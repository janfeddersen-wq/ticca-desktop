//! ChatState definition and core methods.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use iced::widget::text_editor;
use iced::Subscription;
use tokio::sync::mpsc;

use crate::agent_graph::AgentCallGraph;
use crate::app_config::AppConfig;
use crate::chat_message::{ChatMessage, MessageId};
use crate::messages::{Message, RightSidebarTab, chat};
use crate::session_manager;
use crate::system_executions::SystemExecutionsState;
use ticca_core::agents::{AgentConfig as CoreAgentConfig, AgentType};
use ticca_core::tools::{SystemExecRequest, SystemExecStore, TodoListState};

use super::subscriptions::{
    SystemExecRequestSubscriptionData, TerminalBackendSubscriptionData,
    system_exec_request_stream, terminal_backend_stream,
};
use super::types::{ChatPane, ToolApprovalPrompt, enforce_sidebar_tab};
use super::Effect;

/// Main chat state containing all conversation and streaming data.
pub(in crate::app) struct ChatState {
    pub(in crate::app) input_value: String,
    pub(in crate::app) messages: Vec<ChatMessage>,
    pub(in crate::app) is_streaming: bool,
    pub(in crate::app) pending_attachments: Vec<crate::messages::ImageAttachment>,
    pub(in crate::app) current_agent: AgentType,
    pub(in crate::app) agent_config: CoreAgentConfig,
    pub(in crate::app) available_models: Vec<String>,
    pub(in crate::app) default_model: Option<String>,
    pub(in crate::app) agent_pinned_models: HashMap<AgentType, String>,
    pub(in crate::app) is_loading_models: bool,
    pub(in crate::app) current_session: Option<ticca_core::session::Session>,
    pub(in crate::app) working_directory: PathBuf,
    pub(in crate::app) max_tool_rounds: u32,
    pub(in crate::app) yolo_mode_enabled: bool,
    pub(in crate::app) approval_tx:
        Option<mpsc::UnboundedSender<ticca_core::tools::ToolApprovalDecision>>,
    pub(super) pending_approvals: VecDeque<ToolApprovalPrompt>,
    pub(in crate::app) active_approval: Option<ToolApprovalPrompt>,
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
    pub(super) panes: iced::widget::pane_grid::State<ChatPane>,
    pub(super) chat_pane: iced::widget::pane_grid::Pane,
    pub(super) flow_pane: Option<iced::widget::pane_grid::Pane>,
    pub(in crate::app) sidebar_tab: RightSidebarTab,
    pub(in crate::app) todo_selected_node: usize,
    pub(in crate::app) todo_lists: HashMap<usize, TodoListState>,
    pub(in crate::app) system_exec: SystemExecutionsState,
    pub(super) system_exec_request_tx: mpsc::UnboundedSender<SystemExecRequest>,
    system_exec_request_rx:
        std::sync::Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<SystemExecRequest>>>,
}

impl ChatState {
    /// Creates a new ChatState with default values.
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

    /// Pushes a scroll-to-bottom effect if user is at bottom.
    pub(super) fn push_scroll_if_needed(&self, effects: &mut Vec<Effect>) {
        if self.user_at_bottom {
            effects.push(Effect::ScrollToBottom);
        }
    }

    /// Resets all streaming-related runtime state.
    pub(super) fn reset_stream_runtime_state(&mut self) {
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

    /// Saves the current session to disk.
    pub(super) fn save_current_session(&mut self) {
        if let Some(session) = session_manager::save_session(
            self.current_session.as_ref(),
            &self.messages,
            self.current_agent,
            &self.todo_lists,
        ) {
            self.current_session = Some(session);
        }
    }

    /// Find a message by its stable ID (mutable).
    pub(super) fn message_by_id_mut(&mut self, id: MessageId) -> Option<&mut ChatMessage> {
        self.messages.iter_mut().find(|m| m.id == id)
    }

    /// Find a message by its stable ID (immutable).
    #[allow(dead_code)]
    pub(super) fn message_by_id(&self, id: MessageId) -> Option<&ChatMessage> {
        self.messages.iter().find(|m| m.id == id)
    }

    /// Get the ID of the current streaming message (main agent).
    #[allow(dead_code)]
    fn streaming_message_id(&self) -> Option<MessageId> {
        self.messages
            .last()
            .filter(|m| m.is_streaming)
            .map(|m| m.id)
    }

    /// Get the currently selected model name (pinned or default).
    pub(super) fn selected_model_name(&self) -> Option<&str> {
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

    /// Check if the current model supports vision (image input).
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

    /// Get estimated tokens used (pre-request estimate, more accurate than API response).
    pub(in crate::app) fn tokens_used(&self) -> i64 {
        // Use pre-estimation if available, otherwise fall back to API response
        if self.estimated_tokens > 0 {
            self.estimated_tokens as i64
        } else {
            self.input_tokens as i64
        }
    }

    /// Returns subscriptions for system exec requests and terminal events.
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
