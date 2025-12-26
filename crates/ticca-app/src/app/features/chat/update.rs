//! Chat message update handler.

use iced::widget::text_editor;

use crate::app::{TiccaApp, Toast, View, effects::Effect};
use crate::chat_message::ChatMessage;
use crate::helpers::{extract_diff_markers, format_tool_call_oneliner};
use crate::messages::chat;
use crate::session_manager;
use ticca_core::agents::AgentConfig as CoreAgentConfig;

use super::handlers::{
    handle_create_user_terminal, handle_send_message, handle_subagent_stream,
    handle_system_exec_request, handle_terminal_event, handle_todo_event,
};
use super::types::{ToolApprovalPrompt, enforce_sidebar_tab, enforce_todo_selected_node};

/// Handles chat messages and returns effects to be executed.
pub(in crate::app) fn update(app: &mut TiccaApp, message: chat::Msg) -> Vec<Effect> {
    let mut effects = Vec::new();

    match message {
        chat::Msg::InputChanged(value) => {
            app.chat.input_value = value;
        }

        chat::Msg::PaneResized(event) => {
            app.chat.panes.resize(event.split, event.ratio);
        }

        chat::Msg::ChatScrolled(viewport) => {
            // We consider "at bottom" if within 50 pixels of the end
            let content_height = viewport.content_bounds().height;
            let viewport_height = viewport.bounds().height;
            let scroll_offset = viewport.absolute_offset().y;
            let max_scroll = (content_height - viewport_height).max(0.0);
            let distance_from_bottom = max_scroll - scroll_offset;
            app.chat.user_at_bottom = distance_from_bottom < 50.0;
        }

        chat::Msg::SendMessage => {
            handle_send_message(app, &mut effects);
        }

        chat::Msg::CopyMessage(msg_id) => {
            if let Some(msg) = app.chat.message_by_id(msg_id) {
                effects.push(Effect::CopyToClipboard(msg.content.clone()));
            }
        }

        chat::Msg::ToggleRawView(msg_id) => {
            if app.chat.raw_view_messages.contains(&msg_id) {
                app.chat.raw_view_messages.remove(&msg_id);
                app.chat.raw_view_editors.remove(&msg_id);
            } else {
                if let Some(msg) = app.chat.message_by_id(msg_id) {
                    let content = text_editor::Content::with_text(&msg.content);
                    app.chat.raw_view_editors.insert(msg_id, content);
                }
                app.chat.raw_view_messages.insert(msg_id);
            }
        }

        chat::Msg::ToggleSubAgentCollapsed { msg_id, node_id } => {
            if let Some(msg) = app.chat.message_by_id_mut(msg_id)
                && let Some(sub_agent) = msg.sub_agent_mut(node_id)
            {
                sub_agent.collapsed = !sub_agent.collapsed;
            }
        }

        chat::Msg::RawViewEditorAction(msg_id, action) => {
            if let Some(editor) = app.chat.raw_view_editors.get_mut(&msg_id) {
                editor.perform(action);
            }
        }

        chat::Msg::StreamChunk(chunk) => {
            app.chat.last_bytes_time = Some(std::time::Instant::now());

            let mut extracted_diffs = Vec::new();
            if let Some(msg_id) = app.chat.main_agent_message_id
                && let Some(msg) = app.chat.message_by_id_mut(msg_id)
                && msg.is_streaming
            {
                if msg.last_was_tool_call && !chunk.trim().is_empty() {
                    msg.current_text_block_mut().push_str("\n\n💡 ");
                    msg.last_was_tool_call = false;
                }
                msg.current_text_block_mut().push_str(&chunk);
                msg.content.push_str(&chunk);
                let (cleaned, diffs) = extract_diff_markers(&msg.content);
                if cleaned != msg.content {
                    msg.content = cleaned.clone();
                    *msg.current_text_block_mut() = cleaned;
                }
                extracted_diffs = diffs;
                msg.update_parsed_items();
            }
            for (diff_id, diff_state) in extracted_diffs {
                app.chat.diff_cache.insert(diff_id, diff_state);
            }
            app.chat.push_scroll_if_needed(&mut effects);
        }

        chat::Msg::Reasoning { text, signature } => {
            app.chat.last_bytes_time = Some(std::time::Instant::now());

            if let Some(msg_id) = app.chat.main_agent_message_id
                && let Some(msg) = app.chat.message_by_id_mut(msg_id)
                && msg.is_streaming
            {
                if let Some(ref mut existing) = msg.reasoning {
                    existing.push_str(&text);
                } else {
                    msg.reasoning = Some(text);
                }
                // Store signature (only set once, first one wins)
                if msg.reasoning_signature.is_none() && signature.is_some() {
                    msg.reasoning_signature = signature;
                }
            }
            app.chat.push_scroll_if_needed(&mut effects);
        }

        chat::Msg::StreamStats {
            chars_in_window,
            window_ms,
        } => {
            app.chat.stream_chars_received = chars_in_window;
            app.chat.stream_pulse = !app.chat.stream_pulse;

            if window_ms > 0 {
                let seconds = window_ms as f64 / 1000.0;
                let sample_tps = (chars_in_window as f64) / seconds / 4.0;

                app.chat.tps_samples.push_back(sample_tps);
                while app.chat.tps_samples.len() > 60 {
                    app.chat.tps_samples.pop_front();
                }

                if !app.chat.tps_samples.is_empty() {
                    let sum: f64 = app.chat.tps_samples.iter().sum();
                    app.chat.current_tps = sum / app.chat.tps_samples.len() as f64;
                }
            }
        }

        chat::Msg::Usage {
            input_tokens,
            output_tokens,
        } => {
            // Update token values from the API - input_tokens is the actual context usage
            tracing::info!(
                "UI received Usage: input={}, output={}",
                input_tokens,
                output_tokens
            );
            app.chat.input_tokens = input_tokens;
            app.chat.output_tokens = output_tokens;
            // Also update estimated_tokens so tokens_used() shows the actual value
            app.chat.estimated_tokens = input_tokens as usize;
        }

        chat::Msg::ContextEstimate {
            system_prompt_tokens,
            tool_definitions_tokens,
            messages_tokens,
            total_tokens,
            context_window: _,
            usage_percent: _,
        } => {
            // Update pre-request context estimate for UI display
            // Use our own context window lookup instead of rig's default (200k)
            let our_context_window = app
                .chat
                .selected_model_name()
                .map(|m| ticca_core::llm::ModelService::get_context_length(m) as u64)
                .unwrap_or(app.chat.context_window);
            let our_usage_percent = if our_context_window > 0 {
                (total_tokens as f64 / our_context_window as f64) * 100.0
            } else {
                0.0
            };
            tracing::info!(
                "Context estimate: system={}, tools={}, messages={}, total={} ({:.1}% of {})",
                system_prompt_tokens,
                tool_definitions_tokens,
                messages_tokens,
                total_tokens,
                our_usage_percent,
                our_context_window
            );
            app.chat.estimated_tokens = total_tokens;
            app.chat.context_window = our_context_window;
        }

        chat::Msg::AnimationTick => {
            app.chat.spinner_frame = app.chat.spinner_frame.wrapping_add(1);
        }

        chat::Msg::PollStreamStats => {
            app.chat.stream_pulse = !app.chat.stream_pulse;
        }

        chat::Msg::StopStreaming => {
            if let Some(cancel) = app.chat.stream_cancel.take() {
                let _ = cancel.send(());
            }
        }

        chat::Msg::StreamComplete => {
            if !app.chat.is_streaming {
                return effects;
            }

            let msg_id = app.chat.main_agent_message_id;

            app.chat.reset_stream_runtime_state();

            if let Some(msg_id) = msg_id
                && let Some(msg) = app.chat.message_by_id_mut(msg_id)
                && msg.is_streaming
            {
                msg.is_streaming = false;
                msg.update_parsed_items();
            }

            // Mark main agent (node 0) as completed and finish the run
            let current_run = app.chat.call_graph.current_run_id();
            app.chat.call_graph.mark_node_completed(0);
            app.chat.call_graph.mark_run_completed(current_run);

            app.chat.save_current_session();
        }

        chat::Msg::StreamError(error) => {
            let msg_id = app.chat.main_agent_message_id;

            app.chat.reset_stream_runtime_state();

            if let Some(msg_id) = msg_id
                && let Some(msg) = app.chat.message_by_id_mut(msg_id)
                && msg.is_streaming
            {
                // PRESERVE existing content, append error instead of replacing
                if !msg.content.trim().is_empty() {
                    msg.content.push_str("\n\n---\n");
                }
                msg.content.push_str(&format!("❌ Error: {}", error));
                msg.is_streaming = false;
                msg.update_parsed_items();
            }

            // Mark main agent (node 0) as failed and finish the run
            app.chat.call_graph.mark_node_failed(0);
            let current_run = app.chat.call_graph.current_run_id();
            app.chat.call_graph.mark_run_completed(current_run);
        }

        chat::Msg::StreamStopped => {
            let msg_id = app.chat.main_agent_message_id;

            app.chat.reset_stream_runtime_state();

            if let Some(msg_id) = msg_id
                && let Some(msg) = app.chat.message_by_id_mut(msg_id)
                && msg.is_streaming
            {
                if !msg.content.trim().is_empty() {
                    msg.content.push_str("\n\n");
                }
                msg.content.push_str("[Stopped by user]");
                msg.is_streaming = false;
                msg.update_parsed_items();
            }

            // Mark main agent (node 0) as failed (stopped by user) and finish the run
            app.chat.call_graph.mark_node_failed(0);
            let current_run = app.chat.call_graph.current_run_id();
            app.chat.call_graph.mark_run_completed(current_run);

            app.chat.save_current_session();
        }

        chat::Msg::SwitchAgent(agent_type) => {
            app.chat.current_agent = agent_type;
            app.chat.agent_config = CoreAgentConfig::new(agent_type);
            app.chat.call_graph.reset(agent_type);
            app.chat.todo_lists.clear();
            app.chat.todo_selected_node = 0;
            // Reset context window so it gets looked up fresh for new agent's model
            app.chat.context_window = 0;
            app.chat.estimated_tokens = 0;
            // Don't auto-switch sidebar tab - let user control it
        }

        chat::Msg::ToggleFlowPanel => {
            if let Some(flow_pane) = app.chat.flow_pane.take() {
                if let Some((_state, remaining)) = app.chat.panes.close(flow_pane) {
                    app.chat.chat_pane = remaining;
                }
            } else if let Some((new_pane, split)) = app.chat.panes.split(
                iced::widget::pane_grid::Axis::Vertical,
                app.chat.chat_pane,
                super::types::ChatPane::Flow,
            ) {
                app.chat.panes.resize(split, 0.75);
                app.chat.flow_pane = Some(new_pane);
            }
        }

        chat::Msg::SelectSidebarTab(tab) => {
            app.chat.sidebar_tab = enforce_sidebar_tab(app.ui_mode.shows_expert_ui(), tab);
        }

        chat::Msg::SelectTodoNode(option) => {
            app.chat.todo_selected_node =
                enforce_todo_selected_node(app.ui_mode.shows_internal_agents(), option.node_id);
        }

        chat::Msg::SystemExecNewTerminalNameChanged(name) => {
            app.chat.system_exec.new_terminal_name = name;
            app.chat.system_exec.ui_error = None;
        }

        chat::Msg::SystemExecCreateUserTerminal => {
            handle_create_user_terminal(app, &mut effects);
        }

        chat::Msg::SystemExecCloseTerminal(process_id) => {
            let _ = app.chat.system_exec.shutdown_terminal(&process_id);
            app.chat.system_exec.remove_terminal(&process_id);
        }

        chat::Msg::SystemExecKillTerminal(process_id) => {
            let _ = app.chat.system_exec.shutdown_terminal(&process_id);
            app.chat.system_exec.remove_terminal(&process_id);
        }

        chat::Msg::SystemExecCopyTerminal(process_id) => {
            if let Some(output) = app.chat.system_exec.store.output(&process_id) {
                effects.push(Effect::CopyToClipboard(output));
            }
        }

        chat::Msg::SystemExecRequest(request) => {
            handle_system_exec_request(app, request, &mut effects);
        }

        chat::Msg::SystemExecTerminalEvent(event) => {
            handle_terminal_event(app, event, &mut effects);
        }

        chat::Msg::NewSession => {
            app.chat.messages.clear();
            app.chat.raw_view_messages.clear();
            app.chat.raw_view_editors.clear();
            app.chat.todo_lists.clear();
            app.chat.todo_selected_node = 0;
            app.chat.input_tokens = 0;
            app.chat.output_tokens = 0;
            app.chat.estimated_tokens = 0;
            // Don't auto-switch sidebar tab - let user control it
            app.chat.messages.push(ChatMessage::assistant(
                "New session started. How can I help you?",
            ));
            app.chat.current_session = None;
            app.chat.call_graph.reset(app.chat.current_agent);
        }

        chat::Msg::LoadSession(session_id) => {
            tracing::info!("Loading session: {}", session_id);
            if let Some(loaded) = session_manager::load_session(&session_id) {
                app.chat.messages = loaded.messages;
                app.chat.current_session = Some(loaded.session);

                // Reset token tracking - will be updated on next request
                app.chat.input_tokens = 0;
                app.chat.output_tokens = 0;
                app.chat.estimated_tokens = 0;

                app.chat.raw_view_messages.clear();
                app.chat.raw_view_editors.clear();
                app.chat.todo_lists = loaded.todo_lists;
                app.chat.todo_selected_node = 0;
                // Don't auto-switch sidebar tab - let user control it

                if let Some(agent_type) = loaded.agent_type {
                    app.chat.current_agent = agent_type;
                    app.chat.agent_config = CoreAgentConfig::new(agent_type);
                }

                tracing::info!("Loaded session with {} messages", app.chat.messages.len());
            }
            app.chat.call_graph.reset(app.chat.current_agent);
            app.current_view = View::Chat;
        }

        chat::Msg::ToolCall { name, args } => {
            let working_directory = app.chat.working_directory.clone();
            if let Some(msg_id) = app.chat.main_agent_message_id
                && let Some(msg) = app.chat.message_by_id_mut(msg_id)
                && msg.is_streaming
            {
                let tool_line = format_tool_call_oneliner(&name, &args, Some(&working_directory));
                let formatted = format!("\n\n{}", tool_line);
                msg.current_text_block_mut().push_str(&formatted);
                msg.content.push_str(&formatted);
                msg.last_was_tool_call = true;
                msg.update_parsed_items();
            }
            app.chat.push_scroll_if_needed(&mut effects);
        }

        chat::Msg::AgentCall(event) => {
            app.chat.call_graph.record_call(&event);
        }

        chat::Msg::AgentNodeCompleted { node_id } => {
            app.chat.call_graph.mark_node_completed(node_id);
        }

        chat::Msg::AgentNodeFailed { node_id } => {
            app.chat.call_graph.mark_node_failed(node_id);
        }

        chat::Msg::RunCompleted { run_id } => {
            app.chat.call_graph.mark_run_completed(run_id);
        }

        chat::Msg::FlowAnimationTick => {
            app.chat.flow_animation_frame = app.chat.flow_animation_frame.wrapping_add(1);
        }

        chat::Msg::TodoEvent(event) => {
            handle_todo_event(app, event);
        }

        chat::Msg::SubagentStream(event) => {
            handle_subagent_stream(app, event, &mut effects);
        }

        chat::Msg::ToolApprovalRequested { id, name, args } => {
            app.chat
                .pending_approvals
                .push_back(ToolApprovalPrompt { id, name, args });
            if app.chat.active_approval.is_none() {
                app.chat.active_approval = app.chat.pending_approvals.pop_front();
            }
        }

        chat::Msg::ToolApprovalDecision { id, approved } => {
            if let Some(tx) = &app.chat.approval_tx {
                let _ = tx.send(ticca_core::tools::ToolApprovalDecision { id, approved });
            }
            app.chat.active_approval = app.chat.pending_approvals.pop_front();
        }

        chat::Msg::SelectWorkingDirectory => {
            effects.push(Effect::PickWorkingDirectory);
        }

        chat::Msg::WorkingDirectoryChanged(path) => {
            app.chat.working_directory = path.clone();
            let path_str = path.to_string_lossy().to_string();
            let _ = ticca_core::config::ConfigService::set_setting(
                ticca_core::config::setting_keys::WORKING_DIRECTORY,
                &path_str,
            );
        }

        chat::Msg::SelectImageFile => {
            effects.push(Effect::PickImageFile);
        }

        chat::Msg::FileDropped(path) => {
            effects.push(Effect::LoadImage(path));
        }

        chat::Msg::ImageLoaded(result) => match result {
            Ok(attachment) => {
                app.chat.pending_attachments.push(attachment);
            }
            Err(e) => {
                if !e.contains("No file selected") {
                    app.toast = Some(Toast::new(format!("Failed to load image: {}", e)));
                }
            }
        },

        chat::Msg::PasteImage => {
            effects.push(Effect::PasteImage);
        }

        chat::Msg::ImagePasted(result) => match result {
            Ok(attachment) => {
                app.chat.pending_attachments.push(attachment);
            }
            Err(e) => {
                if !e.contains("No image") {
                    app.toast = Some(Toast::new(format!("Failed to paste image: {}", e)));
                }
            }
        },

        chat::Msg::RemoveAttachment(index) => {
            if index < app.chat.pending_attachments.len() {
                app.chat.pending_attachments.remove(index);
            }
        }

        chat::Msg::LinkClicked(url) => {
            let url_str = url.as_str();
            if let Some(diff_id) = url_str.strip_prefix("ticca-diff://") {
                if let Some(diff_state) = app.chat.diff_cache.get(diff_id).cloned() {
                    app.chat.active_diff_modal = Some(diff_state);
                }
            } else {
                effects.push(Effect::OpenUrl(url_str.to_string()));
            }
        }

        chat::Msg::ShowDiffModal(diff_id) => {
            if let Some(diff_state) = app.chat.diff_cache.get(&diff_id).cloned() {
                app.chat.active_diff_modal = Some(diff_state);
            }
        }

        chat::Msg::CloseDiffModal => {
            app.chat.active_diff_modal = None;
        }

        chat::Msg::CacheDiff { id, state } => {
            app.chat.diff_cache.insert(id, state);
        }

        chat::Msg::ContextCompressed {
            original_messages,
            compressed_messages,
            original_tokens,
            compressed_tokens,
            strategy,
        } => {
            tracing::info!(
                "Context compressed: {} -> {} messages, {} -> {} tokens ({})",
                original_messages,
                compressed_messages,
                original_tokens,
                compressed_tokens,
                strategy
            );
            // Show compression notification as a system message in chat
            let saved_tokens = original_tokens.saturating_sub(compressed_tokens);
            let saved_messages = original_messages.saturating_sub(compressed_messages);
            let notification = format!(
                "📦 **Context compressed** ({} strategy)\n\
                 - Removed {} messages (~{} tokens)\n\
                 - Keeping {} messages (~{} tokens)",
                strategy, saved_messages, saved_tokens, compressed_messages, compressed_tokens
            );
            app.chat.messages.push(ChatMessage::system(notification));
        }

        chat::Msg::ContextUsageWarning {
            current_tokens,
            threshold_tokens,
            context_window,
            usage_percent,
        } => {
            tracing::debug!(
                "Context usage: {}% ({}/{} tokens, threshold: {})",
                usage_percent,
                current_tokens,
                context_window,
                threshold_tokens
            );
            // Update tokens in UI - the usage warning is informational
            // The context bar already shows token usage, this event is for logging
            let _ = (
                current_tokens,
                threshold_tokens,
                context_window,
                usage_percent,
            );
        }
    }

    effects
}
