//! Message handler helper functions.
//!
//! Contains the implementation of complex message handlers extracted from the
//! main update function to keep file sizes manageable.

use std::path::PathBuf;

use base64::Engine as _;

use crate::app::{TiccaApp, effects::Effect};
use crate::chat_message::ChatMessage;
use crate::helpers::format_tool_call_oneliner;
use crate::messages::RightSidebarTab;
use ticca_core::agents::{AgentProfile, ModelSelectionContext};
use ticca_core::llm::{ProviderId, ProviderRegistry, auth};
use ticca_core::tools::{SystemExecRequest, SystemExecResponse};

use super::types::{enforce_sidebar_tab, enforce_todo_selected_node};

/// Handles the SendMessage action.
pub(super) fn handle_send_message(app: &mut TiccaApp, effects: &mut Vec<Effect>) {
    use tokio::sync::mpsc;

    let has_text = !app.chat.input_value.trim().is_empty();
    let has_images = !app.chat.pending_attachments.is_empty();

    if (!has_text && !has_images) || app.chat.is_streaming {
        return;
    }

    let user_message = app.chat.input_value.clone();
    app.chat.input_value.clear();

    let attachments = std::mem::take(&mut app.chat.pending_attachments);

    let display_message = if attachments.is_empty() {
        user_message.clone()
    } else {
        let img_count = attachments.len();
        let img_text = if img_count == 1 {
            "[1 image attached]".to_string()
        } else {
            format!("[{} images attached]", img_count)
        };
        if user_message.is_empty() {
            img_text
        } else {
            format!("{}\n\n{}", img_text, user_message)
        }
    };

    app.chat.messages.push(ChatMessage::user(&display_message));
    app.chat.user_at_bottom = true;
    app.chat.call_graph.reset(app.chat.current_agent);
    app.chat.subagent_messages.clear();
    // Clear todo lists for fresh start - don't carry over completed state from previous request
    app.chat.todo_lists.clear();
    app.chat.todo_selected_node = 0;

    let profile = AgentProfile::for_type(app.chat.current_agent, app.chat.max_tool_rounds);
    let model_name = profile.resolve_model(ModelSelectionContext {
        pinned: app
            .chat
            .agent_pinned_models
            .get(&app.chat.current_agent)
            .map(String::as_str),
        default_model: app.chat.default_model.as_deref(),
        available_models: &app.chat.available_models,
    });

    let provider = model_name
        .as_deref()
        .map(ProviderRegistry::resolve_provider)
        .unwrap_or(ProviderId::Claude);

    let provider_ok = match provider {
        ProviderId::Claude => {
            auth::has_valid_account(ticca_core::config::models::providers::CLAUDE)
        }
        ProviderId::Gemini => {
            auth::has_valid_account(ticca_core::config::models::providers::GEMINI)
        }
        ProviderId::ChatGpt => {
            auth::has_valid_account(ticca_core::config::models::providers::CHATGPT)
        }
        ProviderId::ApiKey(ref provider_id) => auth::has_valid_api_key(provider_id),
    };

    if !provider_ok {
        let provider_name = ProviderRegistry::info(provider).display_name;
        app.chat.messages.push(ChatMessage::assistant(format!(
            "⚠️ No {} accounts available. Please authenticate in Settings.",
            provider_name
        )));
        return;
    }

    app.chat.messages.push(ChatMessage::assistant_streaming());
    app.chat.is_streaming = true;

    app.chat.stream_start_time = Some(std::time::Instant::now());
    app.chat.stream_chars_received = 0;
    app.chat.current_tps = 0.0;
    app.chat.stream_pulse = false;

    let system_prompt = profile.system_prompt;
    let working_dir = app.chat.working_directory.clone();
    let max_tool_rounds = profile.max_tool_rounds;
    let (approval_tx, approval_rx) = mpsc::unbounded_channel();
    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
    app.chat.approval_tx = Some(approval_tx);
    app.chat.stream_cancel = Some(cancel_tx);
    app.chat.pending_approvals.clear();
    app.chat.active_approval = None;

    let history: Vec<_> = app
        .chat
        .messages
        .iter()
        .take(app.chat.messages.len().saturating_sub(2))
        .filter(|m| !m.is_streaming)
        .cloned()
        .collect();

    let image_data: Vec<(String, String)> = attachments
        .iter()
        .map(|att| {
            let base64_data = base64::engine::general_purpose::STANDARD.encode(&*att.data);
            ("image/png".to_string(), base64_data)
        })
        .collect();

    // Start with fresh todo state - each request begins with empty todo list
    let initial_todo_state = None;

    effects.push(Effect::RunStream {
        system_prompt,
        user_message,
        model_name,
        working_directory: working_dir,
        max_tool_rounds,
        history,
        initial_todo_state,
        image_data,
        yolo_mode_enabled: app.chat.yolo_mode_enabled,
        current_agent: app.chat.current_agent,
        approval_rx,
        cancel_rx,
        system_exec_store: app.chat.system_exec.store.clone(),
        system_exec_tx: app.chat.system_exec_request_tx.clone(),
    });
    effects.push(Effect::ScrollToBottom);
}

/// Handles creating a user terminal.
pub(super) fn handle_create_user_terminal(app: &mut TiccaApp, effects: &mut Vec<Effect>) {
    let name = app.chat.system_exec.new_terminal_name.trim().to_string();
    let name_key = name.clone();
    app.chat.system_exec.ui_error = None;
    match app
        .chat
        .system_exec
        .create_user_terminal(name, Some(app.chat.working_directory.clone()))
    {
        Ok(()) => {
            if let Some(instance) = app.chat.system_exec.terminals.get(&name_key) {
                effects.push(Effect::FocusTerminal(instance.terminal_id));
            }
            app.chat.system_exec.new_terminal_name.clear();
            app.chat.sidebar_tab = enforce_sidebar_tab(
                app.ui_mode.shows_expert_ui(),
                RightSidebarTab::SystemExecutions,
            );
        }
        Err(e) => {
            app.chat.system_exec.ui_error = Some(e);
            app.chat.sidebar_tab = enforce_sidebar_tab(
                app.ui_mode.shows_expert_ui(),
                RightSidebarTab::SystemExecutions,
            );
        }
    }
}

/// Handles system execution requests.
pub(super) fn handle_system_exec_request(
    app: &mut TiccaApp,
    request: SystemExecRequest,
    _effects: &mut Vec<Effect>,
) {
    match request {
        SystemExecRequest::ExecuteShell {
            request_id,
            command,
            cwd,
        } => {
            let process_id = app.chat.system_exec.generate_llm_process_id();
            let cwd = cwd
                .map(PathBuf::from)
                .or_else(|| Some(app.chat.working_directory.clone()));

            match app
                .chat
                .system_exec
                .create_llm_terminal(process_id.clone(), command, cwd)
            {
                Ok(()) => {
                    app.chat
                        .system_exec
                        .store
                        .respond(request_id, SystemExecResponse::Started { process_id });
                    // Don't auto-switch sidebar tab - let user control it
                }
                Err(e) => {
                    app.chat
                        .system_exec
                        .store
                        .respond(request_id, SystemExecResponse::Error { message: e });
                }
            }
        }
        SystemExecRequest::KillProcess {
            request_id,
            process_id,
        } => {
            let resp = match app.chat.system_exec.shutdown_terminal(&process_id) {
                Ok(()) => {
                    app.chat.system_exec.remove_terminal(&process_id);
                    SystemExecResponse::Killed { process_id }
                }
                Err(e) => SystemExecResponse::Error { message: e },
            };
            app.chat.system_exec.store.respond(request_id, resp);
        }
    }
}

/// Handles terminal backend events.
pub(super) fn handle_terminal_event(
    app: &mut TiccaApp,
    event: iced_term::Event,
    effects: &mut Vec<Effect>,
) {
    match event {
        iced_term::Event::Focus(terminal_id) => {
            effects.push(Effect::FocusTerminal(terminal_id));
        }
        iced_term::Event::BackendCall(terminal_id, cmd) => {
            let process_id = app
                .chat
                .system_exec
                .terminal_index
                .get(&terminal_id)
                .cloned();
            if let Some(process_id) = process_id {
                let mut child_exit: Option<i32> = None;
                let mut should_update_output = false;

                if let iced_term::backend::Command::ProcessAlacrittyEvent(ref ev) = cmd {
                    match ev {
                        iced_term::AlacrittyEvent::Wakeup
                        | iced_term::AlacrittyEvent::PtyWrite(_)
                        | iced_term::AlacrittyEvent::Title(_)
                        | iced_term::AlacrittyEvent::ResetTitle => {
                            should_update_output = true;
                        }
                        iced_term::AlacrittyEvent::ChildExit(code) => {
                            should_update_output = true;
                            child_exit = Some(*code);
                        }
                        iced_term::AlacrittyEvent::Exit => {
                            should_update_output = true;
                        }
                        _ => {}
                    }
                }

                let mut remove_terminal = false;
                let mut auto_close_if_fast = false;

                if let Some(instance) = app.chat.system_exec.terminals.get_mut(&process_id) {
                    let action = instance
                        .terminal
                        .handle(iced_term::Command::ProxyToBackend(cmd));

                    if should_update_output {
                        let output = instance.terminal.dump_text();
                        app.chat.system_exec.store.set_output(&process_id, output);
                    }

                    if matches!(action, iced_term::actions::Action::Shutdown) {
                        remove_terminal = true;
                    }

                    if let Some(code) = child_exit {
                        app.chat
                            .system_exec
                            .store
                            .mark_finished(&process_id, Some(code));
                        auto_close_if_fast = instance.auto_close_if_fast
                            && instance.started_at.elapsed()
                                < std::time::Duration::from_secs(30);
                    }
                }

                if remove_terminal || auto_close_if_fast {
                    app.chat.system_exec.remove_terminal(&process_id);
                }
            }
        }
    }
}

/// Handles todo list events.
pub(super) fn handle_todo_event(app: &mut TiccaApp, event: ticca_core::tools::TodoListEvent) {
    use ticca_core::tools::TodoListEvent;

    match event {
        TodoListEvent::Reset { node_id, state } | TodoListEvent::Updated { node_id, state } => {
            app.chat.todo_lists.insert(node_id, state);
            if node_id == 0
                || !app
                    .chat
                    .todo_lists
                    .contains_key(&app.chat.todo_selected_node)
            {
                app.chat.todo_selected_node = node_id;
            }

            app.chat.todo_selected_node = enforce_todo_selected_node(
                app.ui_mode.shows_expert_ui(),
                app.chat.todo_selected_node,
            );
        }
    }
}

/// Handles subagent streaming events.
pub(super) fn handle_subagent_stream(
    app: &mut TiccaApp,
    event: ticca_core::tools::AgentStreamEvent,
    effects: &mut Vec<Effect>,
) {
    use ticca_core::tools::AgentStreamEvent;

    match event {
        AgentStreamEvent::Start {
            node_id,
            agent_type,
        } => {
            let label = format!("{} - {}", agent_type.display_name(), node_id);
            let new_msg = ChatMessage::assistant_streaming_named(label);
            let msg_id = new_msg.id;
            app.chat.messages.push(new_msg);
            app.chat.subagent_messages.insert(node_id, msg_id);
            app.chat.user_at_bottom = true;
        }
        AgentStreamEvent::Chunk { node_id, text } => {
            if let Some(&msg_id) = app.chat.subagent_messages.get(&node_id)
                && let Some(msg) = app.chat.message_by_id_mut(msg_id)
            {
                if msg.last_was_tool_call && !text.trim().is_empty() {
                    msg.content.push_str("\n\n💡 ");
                    msg.last_was_tool_call = false;
                }
                msg.content.push_str(&text);
                msg.update_parsed_items();
            }
            app.chat.push_scroll_if_needed(effects);
        }
        AgentStreamEvent::Reasoning { node_id, text } => {
            if let Some(&msg_id) = app.chat.subagent_messages.get(&node_id)
                && let Some(msg) = app.chat.message_by_id_mut(msg_id)
            {
                if let Some(ref mut existing) = msg.reasoning {
                    existing.push_str(&text);
                } else {
                    msg.reasoning = Some(text);
                }
            }
            app.chat.push_scroll_if_needed(effects);
        }
        AgentStreamEvent::ToolCall {
            node_id,
            name,
            args,
        } => {
            // Format tool line before mutable borrow
            let tool_line =
                format_tool_call_oneliner(&name, &args, Some(&app.chat.working_directory));
            if let Some(&msg_id) = app.chat.subagent_messages.get(&node_id)
                && let Some(msg) = app.chat.message_by_id_mut(msg_id)
            {
                msg.content.push_str(&format!("\n\n{}", tool_line));
                msg.last_was_tool_call = true;
                msg.update_parsed_items();
            }
            app.chat.push_scroll_if_needed(effects);
        }
        AgentStreamEvent::Complete { node_id, output } => {
            if let Some(msg_id) = app.chat.subagent_messages.remove(&node_id)
                && let Some(msg) = app.chat.message_by_id_mut(msg_id)
            {
                // Replace streamed content with the final pure output
                msg.content = output;
                msg.is_streaming = false;
                msg.update_parsed_items();
            }
        }
    }
}
