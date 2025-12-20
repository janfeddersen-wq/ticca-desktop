use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use iced::widget::{button, column, container, row, text, text_editor};
use iced::{Color, Element, Length, Subscription};

use tokio::sync::mpsc;

use base64::Engine as _;

use crate::agent_graph::AgentCallGraph;
use crate::app_config::AppConfig;
use crate::chat_message::ChatMessage;
use crate::helpers::format_tool_call_oneliner;
use crate::messages::{chat, ImageAttachment, Message, RightSidebarTab};
use crate::session_manager;
use crate::system_executions::SystemExecutionsState;
use ticca_core::agents::{AgentConfig as CoreAgentConfig, AgentProfile, AgentType, ModelSelectionContext};
use ticca_core::llm::{ProviderId, ProviderRegistry, auth};
use ticca_core::session::Session;
use ticca_core::tools::TodoListState;
use ticca_core::tools::{SystemExecRequest, SystemExecResponse, SystemExecStore};

use super::super::{effects::Effect, TiccaApp, View};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChatPane {
    Chat,
    Flow,
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
    pub(in crate::app) raw_view_messages: HashSet<usize>,
    pub(in crate::app) raw_view_editors: HashMap<usize, text_editor::Content>,
    pub(in crate::app) user_at_bottom: bool,
    pub(in crate::app) stream_start_time: Option<std::time::Instant>,
    pub(in crate::app) stream_chars_received: usize,
    pub(in crate::app) current_tps: f64,
    pub(in crate::app) stream_pulse: bool,
    pub(in crate::app) tps_samples: VecDeque<f64>,
    pub(in crate::app) last_bytes_time: Option<std::time::Instant>,
    pub(in crate::app) spinner_frame: usize,
    pub(in crate::app) call_graph: AgentCallGraph,
    pub(in crate::app) subagent_message_indices: HashMap<usize, usize>,
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
            call_graph: AgentCallGraph::new(AgentType::Coding),
            subagent_message_indices: HashMap::new(),
            panes,
            chat_pane,
            flow_pane: Some(flow_pane),
            sidebar_tab: RightSidebarTab::AgentsFlow,
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
        ) {
            self.current_session = Some(session);
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

pub(in crate::app) fn update(
    app: &mut TiccaApp,
    message: chat::Msg,
) -> Vec<Effect> {
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
            app.chat.subagent_message_indices.clear();

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
                ProviderId::Claude => auth::has_valid_account(ticca_core::config::models::providers::CLAUDE),
                ProviderId::Gemini => auth::has_valid_account(ticca_core::config::models::providers::GEMINI),
                ProviderId::ChatGpt => auth::has_valid_account(ticca_core::config::models::providers::CHATGPT),
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
                    let base64_data =
                        base64::engine::general_purpose::STANDARD.encode(&*att.data);
                    ("image/png".to_string(), base64_data)
                })
                .collect();

            effects.push(Effect::RunStream {
                system_prompt,
                user_message,
                model_name,
                working_directory: working_dir,
                max_tool_rounds,
                history,
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

        chat::Msg::CopyMessage(index) => {
            if let Some(msg) = app.chat.messages.get(index) {
                effects.push(Effect::CopyToClipboard(msg.content.clone()));
            }
        }

        chat::Msg::ToggleRawView(index) => {
            if app.chat.raw_view_messages.contains(&index) {
                app.chat.raw_view_messages.remove(&index);
                app.chat.raw_view_editors.remove(&index);
            } else {
                if let Some(msg) = app.chat.messages.get(index) {
                    let content = text_editor::Content::with_text(&msg.content);
                    app.chat.raw_view_editors.insert(index, content);
                }
                app.chat.raw_view_messages.insert(index);
            }
        }

        chat::Msg::RawViewEditorAction(index, action) => {
            if let Some(editor) = app.chat.raw_view_editors.get_mut(&index) {
                editor.perform(action);
            }
        }

        chat::Msg::StreamChunk(chunk) => {
            if let Some(last) = app.chat.messages.last_mut()
                && last.is_streaming
            {
                app.chat.last_bytes_time = Some(std::time::Instant::now());
                if last.last_was_tool_call && !chunk.trim().is_empty() {
                    last.content.push_str("\n\n💡 ");
                    last.last_was_tool_call = false;
                }
                last.content.push_str(&chunk);
                last.update_parsed_items();
            }
            app.chat.push_scroll_if_needed(&mut effects);
        }

        chat::Msg::Reasoning(reasoning) => {
            if let Some(last) = app.chat.messages.last_mut()
                && last.is_streaming
            {
                app.chat.last_bytes_time = Some(std::time::Instant::now());
                if let Some(ref mut existing) = last.reasoning {
                    existing.push_str(&reasoning);
                } else {
                    last.reasoning = Some(reasoning);
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
                last.content = format!("❌ Error: {}", error);
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
            app.chat.subagent_message_indices.clear();
            app.chat.todo_lists.clear();
            app.chat.todo_selected_node = 0;
            app.chat.sidebar_tab = RightSidebarTab::AgentsFlow;
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
            app.chat.sidebar_tab = tab;
        }

        chat::Msg::SelectTodoNode(option) => {
            app.chat.todo_selected_node = option.node_id;
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
                    app.chat.sidebar_tab = RightSidebarTab::SystemExecutions;
                }
                Err(e) => {
                    app.chat.system_exec.ui_error = Some(e);
                    app.chat.sidebar_tab = RightSidebarTab::SystemExecutions;
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
                        app.chat.system_exec.store.respond(
                            request_id,
                            SystemExecResponse::Started { process_id },
                        );
                        app.chat.sidebar_tab = RightSidebarTab::SystemExecutions;
                    }
                    Err(e) => {
                        app.chat.system_exec.store.respond(
                            request_id,
                            SystemExecResponse::Error { message: e },
                        );
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
                            app.chat.system_exec.store.mark_finished(&process_id, Some(code));
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
            app.chat.sidebar_tab = RightSidebarTab::AgentsFlow;
            app.chat.messages.push(ChatMessage::assistant(
                "New session started. How can I help you?",
            ));
            app.chat.current_session = None;
            app.chat.call_graph.reset(app.chat.current_agent);
            app.chat.subagent_message_indices.clear();
        }

        chat::Msg::LoadSession(session_id) => {
            tracing::info!("Loading session: {}", session_id);
            if let Some(loaded) = session_manager::load_session(&session_id) {
                app.chat.messages = loaded.messages;
                app.chat.current_session = Some(loaded.session);

                app.chat.raw_view_messages.clear();
                app.chat.raw_view_editors.clear();
                app.chat.todo_lists.clear();
                app.chat.todo_selected_node = 0;
                app.chat.sidebar_tab = RightSidebarTab::AgentsFlow;

                if let Some(agent_type) = loaded.agent_type {
                    app.chat.current_agent = agent_type;
                    app.chat.agent_config = CoreAgentConfig::new(agent_type);
                }

                tracing::info!("Loaded session with {} messages", app.chat.messages.len());
            }
            app.chat.call_graph.reset(app.chat.current_agent);
            app.chat.subagent_message_indices.clear();
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
                    if node_id == 0 || !app.chat.todo_lists.contains_key(&app.chat.todo_selected_node) {
                        app.chat.todo_selected_node = node_id;
                    }
                }
            }
        }

        chat::Msg::SubagentStream(event) => {
            use ticca_core::tools::AgentStreamEvent;

            match event {
                AgentStreamEvent::Start { node_id, agent_type } => {
                    let label = format!("{} - {}", agent_type.display_name(), node_id);
                    app.chat
                        .messages
                        .push(ChatMessage::assistant_streaming_named(label));
                    let index = app.chat.messages.len().saturating_sub(1);
                    app.chat.subagent_message_indices.insert(node_id, index);
                    app.chat.user_at_bottom = true;
                }
                AgentStreamEvent::Chunk { node_id, text } => {
                    if let Some(&index) = app.chat.subagent_message_indices.get(&node_id)
                        && let Some(msg) = app.chat.messages.get_mut(index)
                    {
                        if msg.last_was_tool_call && !text.trim().is_empty() {
                            msg.content.push_str("\n\n💡 ");
                            msg.last_was_tool_call = false;
                        }
                        msg.content.push_str(&text);
                        msg.update_parsed_items();
                    }
                    app.chat.push_scroll_if_needed(&mut effects);
                }
                AgentStreamEvent::Reasoning { node_id, text } => {
                    if let Some(&index) = app.chat.subagent_message_indices.get(&node_id)
                        && let Some(msg) = app.chat.messages.get_mut(index)
                    {
                        if let Some(ref mut existing) = msg.reasoning {
                            existing.push_str(&text);
                        } else {
                            msg.reasoning = Some(text);
                        }
                    }
                    app.chat.push_scroll_if_needed(&mut effects);
                }
                AgentStreamEvent::ToolCall { node_id, name, args } => {
                    if let Some(&index) = app.chat.subagent_message_indices.get(&node_id)
                        && let Some(msg) = app.chat.messages.get_mut(index)
                    {
                        let tool_line = format_tool_call_oneliner(
                            &name,
                            &args,
                            Some(&app.chat.working_directory),
                        );
                        msg.content.push_str(&format!("\n\n{}", tool_line));
                        msg.last_was_tool_call = true;
                        msg.update_parsed_items();
                    }
                    app.chat.push_scroll_if_needed(&mut effects);
                }
                AgentStreamEvent::Complete { node_id } => {
                    if let Some(index) = app.chat.subagent_message_indices.remove(&node_id)
                        && let Some(msg) = app.chat.messages.get_mut(index)
                    {
                        msg.is_streaming = false;
                        msg.update_parsed_items();
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
                    app.error_message = Some(format!("Failed to load image: {}", e));
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
                    app.error_message = Some(format!("Failed to paste image: {}", e));
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

        // Reserved for future: we used to have ToolResult here.
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
            ),
            ChatPane::Flow => crate::views::right_sidebar::view(
                &app.chat.call_graph,
                &app.chat.todo_lists,
                &app.chat.system_exec,
                app.chat.sidebar_tab,
                app.chat.todo_selected_node,
                app.theme,
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
