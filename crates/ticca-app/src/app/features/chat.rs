use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use iced::widget::{button, column, container, row, text, text_editor};
use iced::{Color, Element, Length, Subscription};

use tokio::sync::mpsc;

use base64::Engine as _;

use crate::agent_graph::AgentCallGraph;
use crate::app_config::AppConfig;
use crate::chat_message::{ChatMessage, MessageId};
use crate::helpers::format_tool_call_oneliner;
use crate::messages::{ImageAttachment, Message, RightSidebarTab, chat};
use crate::session_manager;
use crate::system_executions::SystemExecutionsState;
use ticca_core::agents::{
    AgentConfig as CoreAgentConfig, AgentProfile, AgentType, ModelSelectionContext,
};
use ticca_core::llm::{ProviderId, ProviderRegistry, auth};
use ticca_core::session::Session;
use ticca_core::tools::TodoListState;
use ticca_core::tools::{SystemExecRequest, SystemExecResponse, SystemExecStore};

use super::super::{TiccaApp, Toast, View, effects::Effect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChatPane {
    Chat,
    Flow,
}

const STARTING_TODO_NODE_ID: usize = 0;

fn enforce_sidebar_tab(shows_expert_ui: bool, tab: RightSidebarTab) -> RightSidebarTab {
    if shows_expert_ui {
        return tab;
    }

    match tab {
        RightSidebarTab::AgentsFlow | RightSidebarTab::SystemExecutions => {
            RightSidebarTab::TodoList
        }
        RightSidebarTab::TodoList => RightSidebarTab::TodoList,
    }
}

fn enforce_todo_selected_node(shows_internal_agents: bool, node_id: usize) -> usize {
    if shows_internal_agents {
        node_id
    } else {
        STARTING_TODO_NODE_ID
    }
}

#[derive(Debug, Clone)]
struct ToolApprovalPrompt {
    id: u64,
    name: String,
    args: String,
}

pub(in crate::app) struct ChatState {
    pub(in crate::app) input_value: String,
    pub(in crate::app) messages: Vec<ChatMessage>,
    pub(in crate::app) is_streaming: bool,
    pub(in crate::app) pending_attachments: Vec<ImageAttachment>,
    pub(in crate::app) current_agent: AgentType,
    pub(in crate::app) agent_config: CoreAgentConfig,
    pub(in crate::app) available_models: Vec<String>,
    pub(in crate::app) default_model: Option<String>,
    pub(in crate::app) agent_pinned_models: HashMap<AgentType, String>,
    pub(in crate::app) is_loading_models: bool,
    pub(in crate::app) current_session: Option<Session>,
    pub(in crate::app) working_directory: PathBuf,
    pub(in crate::app) max_tool_rounds: u32,
    pub(in crate::app) yolo_mode_enabled: bool,
    pub(in crate::app) approval_tx:
        Option<mpsc::UnboundedSender<ticca_core::tools::ToolApprovalDecision>>,
    pending_approvals: VecDeque<ToolApprovalPrompt>,
    active_approval: Option<ToolApprovalPrompt>,
    pub(in crate::app) stream_cancel: Option<tokio::sync::oneshot::Sender<()>>,
    pub(in crate::app) raw_view_messages: HashSet<MessageId>,
    pub(in crate::app) raw_view_editors: HashMap<MessageId, text_editor::Content>,
    pub(in crate::app) user_at_bottom: bool,
    pub(in crate::app) stream_start_time: Option<std::time::Instant>,
    pub(in crate::app) stream_chars_received: usize,
    pub(in crate::app) current_tps: f64,
    pub(in crate::app) stream_pulse: bool,
    pub(in crate::app) tps_samples: VecDeque<f64>,
    pub(in crate::app) last_bytes_time: Option<std::time::Instant>,
    pub(in crate::app) spinner_frame: usize,
    /// Token usage from API responses (input_tokens includes system prompt, tools, all messages)
    pub(in crate::app) input_tokens: u64,
    pub(in crate::app) output_tokens: u64,
    /// Pre-request context estimate (from rig's ContextEstimate)
    pub(in crate::app) estimated_tokens: usize,
    pub(in crate::app) context_window: u64,
    pub(in crate::app) call_graph: AgentCallGraph,
    /// Maps subagent node_id to the MessageId of their streaming message
    pub(in crate::app) subagent_messages: HashMap<usize, MessageId>,
    panes: iced::widget::pane_grid::State<ChatPane>,
    chat_pane: iced::widget::pane_grid::Pane,
    flow_pane: Option<iced::widget::pane_grid::Pane>,
    pub(in crate::app) sidebar_tab: RightSidebarTab,
    pub(in crate::app) todo_selected_node: usize,
    pub(in crate::app) todo_lists: HashMap<usize, TodoListState>,
    pub(in crate::app) system_exec: SystemExecutionsState,
    system_exec_request_tx: mpsc::UnboundedSender<SystemExecRequest>,
    system_exec_request_rx:
        std::sync::Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<SystemExecRequest>>>,
}

impl ChatState {
    pub(in crate::app) fn new(config: &AppConfig, working_directory: PathBuf) -> Self {
        let (mut panes, chat_pane) = iced::widget::pane_grid::State::new(ChatPane::Chat);
        let (flow_pane, split) = panes
            .split(
                iced::widget::pane_grid::Axis::Vertical,
                chat_pane,
                ChatPane::Flow,
            )
            .expect("initial pane split should succeed");
        panes.resize(split, 0.75);

        let system_exec_store = std::sync::Arc::new(SystemExecStore::new());
        let (system_exec_request_tx, system_exec_request_rx) = mpsc::unbounded_channel();
        let system_exec_request_rx =
            std::sync::Arc::new(tokio::sync::Mutex::new(system_exec_request_rx));

        Self {
            input_value: String::new(),
            messages: vec![ChatMessage::assistant(
                "Welcome to Ticca. How can I assist you?",
            )],
            is_streaming: false,
            pending_attachments: Vec::new(),
            current_agent: AgentType::Coding,
            agent_config: CoreAgentConfig::coding(),
            available_models: Vec::new(),
            default_model: config.default_model.clone(),
            agent_pinned_models: config.agent_pinned_models.clone(),
            is_loading_models: false,
            current_session: None,
            working_directory,
            max_tool_rounds: config.max_tool_rounds,
            yolo_mode_enabled: config.yolo_mode_enabled,
            approval_tx: None,
            pending_approvals: VecDeque::new(),
            active_approval: None,
            stream_cancel: None,
            raw_view_messages: HashSet::new(),
            raw_view_editors: HashMap::new(),
            user_at_bottom: true,
            stream_start_time: None,
            stream_chars_received: 0,
            current_tps: 0.0,
            stream_pulse: false,
            tps_samples: VecDeque::with_capacity(60),
            last_bytes_time: None,
            spinner_frame: 0,
            input_tokens: 0,
            output_tokens: 0,
            estimated_tokens: 0,
            context_window: 0, // Will be looked up from registry based on selected model
            call_graph: AgentCallGraph::new(AgentType::Coding),
            subagent_messages: HashMap::new(),
            panes,
            chat_pane,
            flow_pane: Some(flow_pane),
            sidebar_tab: enforce_sidebar_tab(
                config.ui_mode.shows_expert_ui(),
                RightSidebarTab::AgentsFlow,
            ),
            todo_selected_node: 0,
            todo_lists: HashMap::new(),
            system_exec: SystemExecutionsState::new(system_exec_store),
            system_exec_request_tx,
            system_exec_request_rx,
        }
    }

    fn push_scroll_if_needed(&self, effects: &mut Vec<Effect>) {
        if self.user_at_bottom {
            effects.push(Effect::ScrollToBottom);
        }
    }

    fn reset_stream_runtime_state(&mut self) {
        self.is_streaming = false;
        self.approval_tx = None;
        self.pending_approvals.clear();
        self.active_approval = None;
        self.stream_cancel = None;
        self.stream_start_time = None;
        self.stream_chars_received = 0;
        self.current_tps = 0.0;
        self.tps_samples.clear();
        self.last_bytes_time = None;
        self.spinner_frame = 0;
    }

    fn save_current_session(&mut self) {
        if let Some(session) = session_manager::save_session(
            self.current_session.as_ref(),
            &self.messages,
            self.current_agent,
            &self.todo_lists,
        ) {
            self.current_session = Some(session);
        }
    }

    /// Find a message by its stable ID (mutable)
    fn message_by_id_mut(&mut self, id: MessageId) -> Option<&mut ChatMessage> {
        self.messages.iter_mut().find(|m| m.id == id)
    }

    /// Find a message by its stable ID (immutable)
    #[allow(dead_code)]
    fn message_by_id(&self, id: MessageId) -> Option<&ChatMessage> {
        self.messages.iter().find(|m| m.id == id)
    }

    /// Get the ID of the current streaming message (main agent)
    #[allow(dead_code)]
    fn streaming_message_id(&self) -> Option<MessageId> {
        self.messages
            .last()
            .filter(|m| m.is_streaming)
            .map(|m| m.id)
    }

    /// Get the currently selected model name (pinned or default)
    fn selected_model_name(&self) -> Option<&str> {
        self.agent_pinned_models
            .get(&self.current_agent)
            .or(self.default_model.as_ref())
            .map(String::as_str)
    }

    /// Get context limit for the current model.
    ///
    /// Uses the API-provided context window if available (from streaming response),
    /// otherwise falls back to looking up the selected model in the registry.
    pub(in crate::app) fn context_limit(&self) -> i64 {
        if self.context_window > 0 {
            self.context_window as i64
        } else {
            // Fall back to registry lookup for the selected model
            self.selected_model_name()
                .map(|name| ticca_core::RegistryService::get_context_window(name) as i64)
                .unwrap_or(ticca_core::registry::DEFAULT_CONTEXT_WINDOW as i64)
        }
    }

    /// Check if the current model supports vision (image input)
    ///
    /// OAuth providers (Claude, Gemini, ChatGPT) all support vision.
    /// For API key providers, we check the model registry.
    pub(in crate::app) fn supports_vision(&self) -> bool {
        // OAuth providers all support vision
        let model_name = match self.selected_model_name() {
            Some(name) => name,
            None => return true, // Default to enabled if no model selected
        };

        // Check if this is an OAuth provider model (always support vision)
        if model_name.starts_with("claude")
            || model_name.starts_with("gemini")
            || model_name.starts_with("gpt-")
            || model_name.starts_with("o1")
            || model_name.starts_with("o3")
        {
            return true;
        }

        // For other models, check the registry
        ticca_core::RegistryService::find_model(model_name)
            .map(|m| m.capabilities.vision)
            .unwrap_or(false)
    }

    /// Get estimated tokens used (pre-request estimate, more accurate than API response)
    pub(in crate::app) fn tokens_used(&self) -> i64 {
        // Use pre-estimation if available, otherwise fall back to API response
        if self.estimated_tokens > 0 {
            self.estimated_tokens as i64
        } else {
            self.input_tokens as i64
        }
    }

    pub(in crate::app) fn subscriptions(&self) -> Vec<Subscription<Message>> {
        use iced::time;

        let mut subs: Vec<Subscription<Message>> = Vec::new();

        // Receive system execution requests from LLM tools
        subs.push(Subscription::run_with(
            SystemExecRequestSubscriptionData {
                key: 0,
                rx: self.system_exec_request_rx.clone(),
            },
            system_exec_request_stream,
        ));

        // Terminal backend events (PTY output, exit codes, etc.)
        for instance in self.system_exec.terminals.values() {
            subs.push(Subscription::run_with(
                TerminalBackendSubscriptionData {
                    terminal_id: instance.terminal_id,
                    rx: instance.terminal.event_receiver(),
                },
                terminal_backend_stream,
            ));
        }

        // Add timer subscriptions while streaming
        if self.is_streaming {
            // Stats polling every 1 second
            subs.push(
                time::every(std::time::Duration::from_secs(1))
                    .map(|_| Message::Chat(chat::Msg::PollStreamStats)),
            );

            // Fast animation timer (~60 FPS) for smooth spinner when waiting
            let is_waiting = self
                .last_bytes_time
                .map(|t| t.elapsed().as_secs() >= 2)
                .unwrap_or(false);

            if is_waiting {
                subs.push(
                    time::every(std::time::Duration::from_millis(16))
                        .map(|_| Message::Chat(chat::Msg::AnimationTick)),
                );
            }
        }

        subs
    }
}

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
            let has_text = !app.chat.input_value.trim().is_empty();
            let has_images = !app.chat.pending_attachments.is_empty();

            if (!has_text && !has_images) || app.chat.is_streaming {
                return effects;
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
                return effects;
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

        chat::Msg::RawViewEditorAction(msg_id, action) => {
            if let Some(editor) = app.chat.raw_view_editors.get_mut(&msg_id) {
                editor.perform(action);
            }
        }

        chat::Msg::StreamChunk(chunk) => {
            app.chat.last_bytes_time = Some(std::time::Instant::now());

            if let Some(last) = app.chat.messages.last_mut()
                && last.is_streaming
            {
                if last.last_was_tool_call && !chunk.trim().is_empty() {
                    last.content.push_str("\n\n💡 ");
                    last.last_was_tool_call = false;
                }
                last.content.push_str(&chunk);
                last.update_parsed_items();
            }
            app.chat.push_scroll_if_needed(&mut effects);
        }

        chat::Msg::Reasoning { text, signature } => {
            app.chat.last_bytes_time = Some(std::time::Instant::now());

            if let Some(last) = app.chat.messages.last_mut()
                && last.is_streaming
            {
                if let Some(ref mut existing) = last.reasoning {
                    existing.push_str(&text);
                } else {
                    last.reasoning = Some(text);
                }
                // Store signature (only set once, first one wins)
                if last.reasoning_signature.is_none() && signature.is_some() {
                    last.reasoning_signature = signature;
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

            app.chat.reset_stream_runtime_state();

            if let Some(last) = app.chat.messages.last_mut()
                && last.is_streaming
            {
                last.is_streaming = false;
                last.update_parsed_items();
            }

            app.chat.save_current_session();
        }

        chat::Msg::StreamError(error) => {
            app.chat.reset_stream_runtime_state();

            if let Some(last) = app.chat.messages.last_mut()
                && last.is_streaming
            {
                // PRESERVE existing content, append error instead of replacing
                if !last.content.trim().is_empty() {
                    last.content.push_str("\n\n---\n");
                }
                last.content.push_str(&format!("❌ Error: {}", error));
                last.is_streaming = false;
                last.update_parsed_items();
            }
        }

        chat::Msg::StreamStopped => {
            app.chat.reset_stream_runtime_state();

            if let Some(last) = app.chat.messages.last_mut()
                && last.is_streaming
            {
                if !last.content.trim().is_empty() {
                    last.content.push_str("\n\n");
                }
                last.content.push_str("[Stopped by user]");
                last.is_streaming = false;
                last.update_parsed_items();
            }

            app.chat.save_current_session();
        }

        chat::Msg::SwitchAgent(agent_type) => {
            app.chat.current_agent = agent_type;
            app.chat.agent_config = CoreAgentConfig::new(agent_type);
            app.chat.call_graph.reset(agent_type);
            app.chat.subagent_messages.clear();
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
                ChatPane::Flow,
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

        chat::Msg::SystemExecRequest(request) => match request {
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
        },

        chat::Msg::SystemExecTerminalEvent(event) => match event {
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
        },

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
            app.chat.subagent_messages.clear();
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
            app.chat.subagent_messages.clear();
            app.current_view = View::Chat;
        }

        chat::Msg::ToolCall { name, args } => {
            if let Some(last) = app.chat.messages.last_mut()
                && last.is_streaming
            {
                let tool_line =
                    format_tool_call_oneliner(&name, &args, Some(&app.chat.working_directory));
                last.content.push_str(&format!("\n\n{}", tool_line));
                last.last_was_tool_call = true;
                last.update_parsed_items();
            }
            app.chat.push_scroll_if_needed(&mut effects);
        }

        chat::Msg::AgentCall(event) => {
            app.chat.call_graph.record_call(&event);
        }

        chat::Msg::TodoEvent(event) => {
            use ticca_core::tools::TodoListEvent;

            match event {
                TodoListEvent::Reset { node_id, state }
                | TodoListEvent::Updated { node_id, state } => {
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

        chat::Msg::SubagentStream(event) => {
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
                    if let Some(&msg_id) = app.chat.subagent_messages.get(&node_id) {
                        if let Some(msg) = app.chat.message_by_id_mut(msg_id) {
                            if msg.last_was_tool_call && !text.trim().is_empty() {
                                msg.content.push_str("\n\n💡 ");
                                msg.last_was_tool_call = false;
                            }
                            msg.content.push_str(&text);
                            msg.update_parsed_items();
                        }
                    }
                    app.chat.push_scroll_if_needed(&mut effects);
                }
                AgentStreamEvent::Reasoning { node_id, text } => {
                    if let Some(&msg_id) = app.chat.subagent_messages.get(&node_id) {
                        if let Some(msg) = app.chat.message_by_id_mut(msg_id) {
                            if let Some(ref mut existing) = msg.reasoning {
                                existing.push_str(&text);
                            } else {
                                msg.reasoning = Some(text);
                            }
                        }
                    }
                    app.chat.push_scroll_if_needed(&mut effects);
                }
                AgentStreamEvent::ToolCall {
                    node_id,
                    name,
                    args,
                } => {
                    // Format tool line before mutable borrow
                    let tool_line = format_tool_call_oneliner(
                        &name,
                        &args,
                        Some(&app.chat.working_directory),
                    );
                    if let Some(&msg_id) = app.chat.subagent_messages.get(&node_id) {
                        if let Some(msg) = app.chat.message_by_id_mut(msg_id) {
                            msg.content.push_str(&format!("\n\n{}", tool_line));
                            msg.last_was_tool_call = true;
                            msg.update_parsed_items();
                        }
                    }
                    app.chat.push_scroll_if_needed(&mut effects);
                }
                AgentStreamEvent::Complete { node_id, output } => {
                    if let Some(msg_id) = app.chat.subagent_messages.remove(&node_id) {
                        if let Some(msg) = app.chat.message_by_id_mut(msg_id) {
                            // Replace streamed content with the final pure output
                            msg.content = output;
                            msg.is_streaming = false;
                            msg.update_parsed_items();
                        }
                    }
                }
            }
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
            app.chat.working_directory = path;
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
            effects.push(Effect::OpenUrl(url.as_str().to_string()));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_config::AppConfig;
    use crate::theme::AppTheme;

    fn test_config() -> AppConfig {
        AppConfig {
            theme: AppTheme::Dark,
            default_model: None,
            agent_pinned_models: HashMap::new(),
            max_tool_rounds: 10,
            yolo_mode_enabled: true,
            expert_mode_enabled: true,
            external_tools_prompt_dismissed: false,
            update_check_skip_remaining: 0,
            update_check_dismissed_version: None,
        }
    }

    #[test]
    fn reset_stream_runtime_state_clears_fields() {
        let mut state = ChatState::new(&test_config(), PathBuf::from("."));
        state.is_streaming = true;
        state.stream_start_time = Some(std::time::Instant::now());
        state.stream_chars_received = 123;
        state.current_tps = 7.0;
        state.tps_samples.push_back(1.0);
        state.last_bytes_time = Some(std::time::Instant::now());
        state.spinner_frame = 9;
        state.pending_approvals.push_back(ToolApprovalPrompt {
            id: 1,
            name: "x".to_string(),
            args: "{}".to_string(),
        });
        state.active_approval = state.pending_approvals.pop_front();

        state.reset_stream_runtime_state();

        assert!(!state.is_streaming);
        assert!(state.approval_tx.is_none());
        assert!(state.pending_approvals.is_empty());
        assert!(state.active_approval.is_none());
        assert!(state.stream_cancel.is_none());
        assert!(state.stream_start_time.is_none());
        assert_eq!(state.stream_chars_received, 0);
        assert_eq!(state.current_tps, 0.0);
        assert!(state.tps_samples.is_empty());
        assert!(state.last_bytes_time.is_none());
        assert_eq!(state.spinner_frame, 0);
    }
}

pub(in crate::app) fn view(app: &TiccaApp) -> Element<'_, Message> {
    let secs_since_bytes = app
        .chat
        .last_bytes_time
        .map(|t| t.elapsed().as_secs())
        .unwrap_or(0);

    iced::widget::pane_grid(&app.chat.panes, |_pane, pane_state, _| {
        let content = match pane_state {
            ChatPane::Chat => crate::views::chat::view(
                app.ui_mode.shows_expert_ui(),
                app.chat.current_agent,
                &app.chat.working_directory,
                &app.chat.messages,
                &app.chat.pending_attachments,
                &app.chat.input_value,
                app.chat.is_streaming,
                app.theme,
                &app.chat.raw_view_messages,
                &app.chat.raw_view_editors,
                app.chat.stream_chars_received,
                app.chat.current_tps,
                app.chat.stream_pulse,
                secs_since_bytes,
                app.chat.spinner_frame,
                app.chat.flow_pane.is_some(),
                app.chat.tokens_used(),
                app.chat.context_limit(),
                app.chat.supports_vision(),
            ),
            ChatPane::Flow => crate::views::right_sidebar::view(
                &app.chat.call_graph,
                &app.chat.todo_lists,
                &app.chat.system_exec,
                app.chat.sidebar_tab,
                app.chat.todo_selected_node,
                app.theme,
                app.ui_mode.shows_expert_ui(),
            ),
        };
        iced::widget::pane_grid::Content::new(content)
    })
    .on_resize(10, |event| Message::Chat(chat::Msg::PaneResized(event)))
    .into()
}

pub(in crate::app) fn wrap_with_approval_modal<'a>(
    app: &'a TiccaApp,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(prompt) = &app.chat.active_approval else {
        return base;
    };

    let overlay = view_approval_modal(prompt);
    iced::widget::stack![base, overlay].into()
}

fn view_approval_modal(prompt: &ToolApprovalPrompt) -> Element<'_, Message> {
    let content = container(
        column![
            text("Tool approval required").size(18),
            text(format!("Tool: {}", prompt.name)).size(14),
            text(prompt.args.clone()).size(12),
            row![
                button("Deny")
                    .on_press(Message::Chat(chat::Msg::ToolApprovalDecision {
                        id: prompt.id,
                        approved: false
                    }))
                    .style(crate::theme::styles::secondary_button)
                    .padding([6, 12]),
                button("Approve")
                    .on_press(Message::Chat(chat::Msg::ToolApprovalDecision {
                        id: prompt.id,
                        approved: true
                    }))
                    .style(crate::theme::styles::success_button)
                    .padding([6, 12]),
            ]
            .spacing(12),
        ]
        .spacing(10),
    )
    .padding(20)
    .style(crate::theme::styles::card_container);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.4).into()),
            ..Default::default()
        })
        .into()
}

#[derive(Clone)]
struct SystemExecRequestSubscriptionData {
    key: u64,
    rx: std::sync::Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<SystemExecRequest>>>,
}

impl std::hash::Hash for SystemExecRequestSubscriptionData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl PartialEq for SystemExecRequestSubscriptionData {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for SystemExecRequestSubscriptionData {}

fn system_exec_request_stream(
    data: &SystemExecRequestSubscriptionData,
) -> iced::futures::stream::BoxStream<'static, Message> {
    let rx = data.rx.clone();
    Box::pin(iced::stream::channel(100, async move |mut output| {
        use iced::futures::SinkExt;

        loop {
            let req = {
                let mut rx = rx.lock().await;
                rx.recv().await
            };

            match req {
                Some(req) => {
                    let _ = output
                        .send(Message::Chat(chat::Msg::SystemExecRequest(req)))
                        .await;
                }
                None => break,
            }
        }
    }))
}

#[derive(Clone)]
struct TerminalBackendSubscriptionData {
    terminal_id: u64,
    rx: std::sync::Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<iced_term::AlacrittyEvent>>>,
}

impl std::hash::Hash for TerminalBackendSubscriptionData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.terminal_id.hash(state);
    }
}

impl PartialEq for TerminalBackendSubscriptionData {
    fn eq(&self, other: &Self) -> bool {
        self.terminal_id == other.terminal_id
    }
}

impl Eq for TerminalBackendSubscriptionData {}

fn terminal_backend_stream(
    data: &TerminalBackendSubscriptionData,
) -> iced::futures::stream::BoxStream<'static, Message> {
    let terminal_id = data.terminal_id;
    let rx = data.rx.clone();
    Box::pin(iced::stream::channel(100, async move |mut output| {
        use iced::futures::SinkExt;

        loop {
            let ev = {
                let mut rx = rx.lock().await;
                rx.recv().await
            };

            match ev {
                Some(ev) => {
                    let _ = output
                        .send(Message::Chat(chat::Msg::SystemExecTerminalEvent(
                            iced_term::Event::BackendCall(
                                terminal_id,
                                iced_term::backend::Command::ProcessAlacrittyEvent(ev),
                            ),
                        )))
                        .await;
                }
                None => break,
            }
        }
    }))
}
