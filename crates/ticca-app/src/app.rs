//! Iced Application state and main loop

use iced::widget::{button, column, container, row, text, text_editor};
use iced::widget::pane_grid;
use iced::widget::scrollable::AbsoluteOffset;
use iced::{Element, Length, Subscription, Task, Theme, widget, Color};

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use base64::Engine as _;

use ticca_core::agents::{AgentConfig as CoreAgentConfig, AgentType, AgentProfile, ModelSelectionContext};
use ticca_core::config::{ConfigDatabase, setting_keys};
use ticca_core::llm::auth;
use ticca_core::llm::{ProviderId, ProviderRegistry};
use ticca_core::session::Session;
use crate::oauth_handler;
use crate::session_manager;

use crate::agent_graph::AgentCallGraph;
use crate::app_config::{load_config, AppConfig};
use crate::chat_message::ChatMessage;
use crate::helpers::format_tool_call_oneliner;
use crate::image_handler;
use crate::llm_stream;
use crate::messages::{Message, ImageAttachment};
use crate::theme::AppTheme;
use crate::views::chat::CHAT_SCROLLABLE_ID;
use crate::views::config::ProviderAuthStatus;
use crate::messages::SettingsTab;

use tokio::sync::mpsc;

/// Main application state
pub struct TiccaApp {
    current_view: View,
    theme: AppTheme,
    chat: ChatState,
    settings: SettingsState,
    error_message: Option<String>,
}

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

struct ChatState {
    input_value: String,
    messages: Vec<ChatMessage>,
    is_streaming: bool,
    pending_attachments: Vec<ImageAttachment>,
    current_agent: AgentType,
    agent_config: CoreAgentConfig,
    available_models: Vec<String>,
    default_model: Option<String>,
    agent_pinned_models: HashMap<AgentType, String>,
    is_loading_models: bool,
    current_session: Option<Session>,
    working_directory: PathBuf,
    max_tool_rounds: u32,
    yolo_mode_enabled: bool,
    approval_tx: Option<mpsc::UnboundedSender<ticca_core::tools::ToolApprovalDecision>>,
    pending_approvals: VecDeque<ToolApprovalPrompt>,
    active_approval: Option<ToolApprovalPrompt>,
    stream_cancel: Option<tokio::sync::oneshot::Sender<()>>,
    raw_view_messages: HashSet<usize>,
    raw_view_editors: HashMap<usize, text_editor::Content>,
    user_at_bottom: bool,
    stream_start_time: Option<std::time::Instant>,
    stream_chars_received: usize,
    current_tps: f64,
    stream_pulse: bool,
    tps_samples: VecDeque<f64>,
    last_bytes_time: Option<std::time::Instant>,
    spinner_frame: usize,
    call_graph: AgentCallGraph,
    subagent_message_indices: HashMap<usize, usize>,
    panes: iced::widget::pane_grid::State<ChatPane>,
    chat_pane: iced::widget::pane_grid::Pane,
    flow_pane: Option<iced::widget::pane_grid::Pane>,
}

impl ChatState {
    fn new(config: &AppConfig, working_directory: PathBuf) -> Self {
        let (mut panes, chat_pane) = iced::widget::pane_grid::State::new(ChatPane::Chat);
        let (flow_pane, split) = panes
            .split(iced::widget::pane_grid::Axis::Vertical, chat_pane, ChatPane::Flow)
            .expect("initial pane split should succeed");
        panes.resize(split, 0.75);

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
        }
    }
}

struct SettingsState {
    settings_tab: SettingsTab,
    provider_auth_status: ProviderAuthStatus,
}

impl SettingsState {
    fn new(provider_auth_status: ProviderAuthStatus) -> Self {
        Self {
            settings_tab: SettingsTab::Accounts,
            provider_auth_status,
        }
    }
}

/// Views in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Chat,
    Settings,
}

#[derive(Debug)]
enum AppCommand {
    RunStream {
        system_prompt: String,
        user_message: String,
        model_name: Option<String>,
        working_directory: PathBuf,
        max_tool_rounds: u32,
        history: Vec<ChatMessage>,
        image_data: Vec<(String, String)>,
        yolo_mode_enabled: bool,
        current_agent: AgentType,
        approval_rx: mpsc::UnboundedReceiver<ticca_core::tools::ToolApprovalDecision>,
        cancel_rx: tokio::sync::oneshot::Receiver<()>,
    },
    ScrollToBottom,
    StartOAuth(crate::messages::OAuthProvider),
    RefreshModels,
    RefreshModelsForProvider(ProviderId),
    CopyToClipboard(String),
    PickWorkingDirectory,
    PickImageFile,
    LoadImage(PathBuf),
    PasteImage,
    OpenUrl(String),
}

/// Check if all providers have valid (non-expired) tokens
fn check_provider_auth_status() -> ProviderAuthStatus {
    ProviderAuthStatus {
        claude: auth::has_valid_account(ticca_core::config::models::providers::CLAUDE),
        gemini: auth::has_valid_account(ticca_core::config::models::providers::GEMINI),
        chatgpt: auth::has_valid_account(ticca_core::config::models::providers::CHATGPT),
    }
}

impl TiccaApp {
    /// Create a new application instance
    pub fn new() -> (Self, Task<Message>) {
        // Load configuration
        let config = load_config();

        // Load working directory from config or use current directory
        let working_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let provider_auth_status = check_provider_auth_status();

        let app = Self {
            current_view: View::Chat,
            theme: config.theme,
            chat: ChatState::new(&config, working_directory),
            settings: SettingsState::new(provider_auth_status),
            error_message: None,
        };

        // Automatically fetch models on startup if we have credentials
        let startup_task = if auth::has_any_valid_account() {
            Task::done(Message::RefreshModels)
        } else {
            Task::none()
        };

        (app, startup_task)
    }

    /// Get the window title
    pub fn title(&self) -> String {
        format!("Ticca Desktop - {}", self.chat.current_agent.display_name())
    }

    /// Handle a message and return any resulting tasks
    pub fn update(&mut self, message: Message) -> Task<Message> {
        let mut commands = Vec::new();

        match message {
            Message::Noop => {}
            Message::InputChanged(value) => {
                self.chat.input_value = value;
            }

            Message::PaneResized(event) => {
                self.chat.panes.resize(event.split, event.ratio);
            }

            Message::ChatScrolled(viewport) => {
                // Check if user is at or near the bottom of the chat
                // We consider "at bottom" if within 50 pixels of the end
                let content_height = viewport.content_bounds().height;
                let viewport_height = viewport.bounds().height;
                let scroll_offset = viewport.absolute_offset().y;
                let max_scroll = (content_height - viewport_height).max(0.0);
                let distance_from_bottom = max_scroll - scroll_offset;
                self.chat.user_at_bottom = distance_from_bottom < 50.0;
            }

            Message::SendMessage => {
                let has_text = !self.chat.input_value.trim().is_empty();
                let has_images = !self.chat.pending_attachments.is_empty();

                if (!has_text && !has_images) || self.chat.is_streaming {
                    return self.execute_commands(commands);
                }

                let user_message = self.chat.input_value.clone();
                self.chat.input_value.clear();

                let attachments = std::mem::take(&mut self.chat.pending_attachments);

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

                self.chat.messages.push(ChatMessage::user(&display_message));
                self.chat.user_at_bottom = true;
                self.chat.call_graph.reset(self.chat.current_agent);
                self.chat.subagent_message_indices.clear();

                let profile = AgentProfile::for_type(self.chat.current_agent, self.chat.max_tool_rounds);
                let model_name = profile.resolve_model(ModelSelectionContext {
                    pinned: self.chat.agent_pinned_models.get(&self.chat.current_agent).map(String::as_str),
                    default_model: self.chat.default_model.as_deref(),
                    available_models: &self.chat.available_models,
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
                    self.chat.messages.push(ChatMessage::assistant(
                        format!("⚠️ No {} accounts available. Please authenticate in Settings.", provider_name)
                    ));
                    return self.execute_commands(commands);
                }

                self.chat.messages.push(ChatMessage::assistant_streaming());
                self.chat.is_streaming = true;

                self.chat.stream_start_time = Some(std::time::Instant::now());
                self.chat.stream_chars_received = 0;
                self.chat.current_tps = 0.0;
                self.chat.stream_pulse = false;

                let system_prompt = profile.system_prompt;
                let working_dir = self.chat.working_directory.clone();
                let max_tool_rounds = profile.max_tool_rounds;
                let (approval_tx, approval_rx) = mpsc::unbounded_channel();
                let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
                self.chat.approval_tx = Some(approval_tx);
                self.chat.stream_cancel = Some(cancel_tx);
                self.chat.pending_approvals.clear();
                self.chat.active_approval = None;

                let history: Vec<_> = self.chat.messages.iter()
                    .take(self.chat.messages.len().saturating_sub(2))
                    .filter(|m| !m.is_streaming)
                    .cloned()
                    .collect();

                let image_data: Vec<(String, String)> = attachments.iter()
                    .map(|att| {
                        let base64_data = base64::engine::general_purpose::STANDARD.encode(&*att.data);
                        ("image/png".to_string(), base64_data)
                    })
                    .collect();

                commands.push(AppCommand::RunStream {
                    system_prompt,
                    user_message,
                    model_name,
                    working_directory: working_dir,
                    max_tool_rounds,
                    history,
                    image_data,
                    yolo_mode_enabled: self.chat.yolo_mode_enabled,
                    current_agent: self.chat.current_agent,
                    approval_rx,
                    cancel_rx,
                });
                commands.push(AppCommand::ScrollToBottom);
            }

            Message::CopyMessage(index) => {
                if let Some(msg) = self.chat.messages.get(index) {
                    commands.push(AppCommand::CopyToClipboard(msg.content.clone()));
                }
            }

            Message::ToggleRawView(index) => {
                if self.chat.raw_view_messages.contains(&index) {
                    self.chat.raw_view_messages.remove(&index);
                    self.chat.raw_view_editors.remove(&index);
                } else {
                    if let Some(msg) = self.chat.messages.get(index) {
                        let content = text_editor::Content::with_text(&msg.content);
                        self.chat.raw_view_editors.insert(index, content);
                    }
                    self.chat.raw_view_messages.insert(index);
                }
            }

            Message::RawViewEditorAction(index, action) => {
                if let Some(editor) = self.chat.raw_view_editors.get_mut(&index) {
                    editor.perform(action);
                }
            }

            Message::StreamChunk(chunk) => {
                if let Some(last) = self.chat.messages.last_mut() {
                    if last.is_streaming {
                        self.chat.last_bytes_time = Some(std::time::Instant::now());
                        if last.last_was_tool_call && !chunk.trim().is_empty() {
                            last.content.push_str("\n\n💡 ");
                            last.last_was_tool_call = false;
                        }
                        last.content.push_str(&chunk);
                        last.update_parsed_items();
                    }
                }
                self.push_scroll_if_needed(&mut commands);
            }

            Message::Reasoning(reasoning) => {
                if let Some(last) = self.chat.messages.last_mut() {
                    if last.is_streaming {
                        self.chat.last_bytes_time = Some(std::time::Instant::now());
                        if let Some(ref mut existing) = last.reasoning {
                            existing.push_str(&reasoning);
                        } else {
                            last.reasoning = Some(reasoning);
                        }
                    }
                }
                self.push_scroll_if_needed(&mut commands);
            }

            Message::StreamStats { chars_in_window, window_ms } => {
                self.chat.stream_chars_received = chars_in_window;
                self.chat.stream_pulse = !self.chat.stream_pulse;

                if window_ms > 0 {
                    let seconds = window_ms as f64 / 1000.0;
                    let sample_tps = (chars_in_window as f64) / seconds / 4.0;

                    self.chat.tps_samples.push_back(sample_tps);
                    while self.chat.tps_samples.len() > 60 {
                        self.chat.tps_samples.pop_front();
                    }

                    if !self.chat.tps_samples.is_empty() {
                        let sum: f64 = self.chat.tps_samples.iter().sum();
                        self.chat.current_tps = sum / self.chat.tps_samples.len() as f64;
                    }
                }
            }

            Message::AnimationTick => {
                self.chat.spinner_frame = self.chat.spinner_frame.wrapping_add(1);
            }

            Message::PollStreamStats => {
                self.chat.stream_pulse = !self.chat.stream_pulse;
            }

            Message::StopStreaming => {
                if let Some(cancel) = self.chat.stream_cancel.take() {
                    let _ = cancel.send(());
                }
            }

            Message::StreamComplete => {
                if !self.chat.is_streaming {
                    return self.execute_commands(commands);
                }
                self.chat.is_streaming = false;
                self.chat.approval_tx = None;
                self.chat.pending_approvals.clear();
                self.chat.active_approval = None;
                self.chat.stream_cancel = None;
                self.chat.stream_start_time = None;
                self.chat.stream_chars_received = 0;
                self.chat.current_tps = 0.0;
                self.chat.tps_samples.clear();
                self.chat.last_bytes_time = None;
                self.chat.spinner_frame = 0;

                if let Some(last) = self.chat.messages.last_mut() {
                    if last.is_streaming {
                        last.is_streaming = false;
                        last.update_parsed_items();
                    }
                }
                self.save_current_session();
            }

            Message::StreamError(error) => {
                self.chat.is_streaming = false;
                self.chat.approval_tx = None;
                self.chat.pending_approvals.clear();
                self.chat.active_approval = None;
                self.chat.stream_cancel = None;
                self.chat.stream_start_time = None;
                self.chat.stream_chars_received = 0;
                self.chat.current_tps = 0.0;
                self.chat.tps_samples.clear();
                self.chat.last_bytes_time = None;
                self.chat.spinner_frame = 0;

                if let Some(last) = self.chat.messages.last_mut() {
                    if last.is_streaming {
                        last.content = format!("❌ Error: {}", error);
                        last.is_streaming = false;
                    }
                }
            }

            Message::StreamStopped => {
                self.chat.is_streaming = false;
                self.chat.approval_tx = None;
                self.chat.pending_approvals.clear();
                self.chat.active_approval = None;
                self.chat.stream_cancel = None;
                self.chat.stream_start_time = None;
                self.chat.stream_chars_received = 0;
                self.chat.current_tps = 0.0;
                self.chat.tps_samples.clear();
                self.chat.last_bytes_time = None;
                self.chat.spinner_frame = 0;

                if let Some(last) = self.chat.messages.last_mut() {
                    if last.is_streaming {
                        if !last.content.trim().is_empty() {
                            last.content.push_str("\n\n");
                        }
                        last.content.push_str("[Stopped by user]");
                        last.is_streaming = false;
                        last.update_parsed_items();
                    }
                }
                self.save_current_session();
            }

            Message::OpenSettings => {
                self.current_view = View::Settings;
            }

            Message::SwitchSettingsTab(tab) => {
                self.settings.settings_tab = tab;
            }

            Message::CloseSettings => {
                self.current_view = View::Chat;
            }

            Message::ThemeToggle => {
                self.theme = self.theme.next();
                if let Ok(db) = ConfigDatabase::open() {
                    let _ = db.set_setting(setting_keys::THEME, self.theme.as_str());
                }
            }

            Message::SetTheme(theme) => {
                self.theme = theme;
                if let Ok(db) = ConfigDatabase::open() {
                    let _ = db.set_setting(setting_keys::THEME, self.theme.as_str());
                }
            }

            Message::SetYoloMode(enabled) => {
                self.chat.yolo_mode_enabled = enabled;
                if let Ok(db) = ConfigDatabase::open() {
                    let value = if enabled { "true" } else { "false" };
                    let _ = db.set_setting(setting_keys::YOLO_MODE, value);
                }
            }

            Message::SwitchAgent(agent_type) => {
                self.chat.current_agent = agent_type;
                self.chat.agent_config = CoreAgentConfig::new(agent_type);
                self.chat.call_graph.reset(agent_type);
                self.chat.subagent_message_indices.clear();
            }

            Message::ToggleFlowPanel => {
                if let Some(flow_pane) = self.chat.flow_pane.take() {
                    if let Some((_state, remaining)) = self.chat.panes.close(flow_pane) {
                        self.chat.chat_pane = remaining;
                    }
                } else {
                    if let Some((new_pane, split)) = self
                        .chat
                        .panes
                    .split(
                        iced::widget::pane_grid::Axis::Vertical,
                        self.chat.chat_pane,
                        ChatPane::Flow,
                    )
                    {
                        self.chat.panes.resize(split, 0.75);
                        self.chat.flow_pane = Some(new_pane);
                    }
                }
            }

            Message::StartOAuth(provider) => {
                commands.push(AppCommand::StartOAuth(provider));
            }

            Message::OAuthComplete(provider, result) => {
                match result {
                    Ok(()) => {
                        self.error_message = None;
                        self.settings.provider_auth_status = check_provider_auth_status();
                        commands.push(AppCommand::RefreshModelsForProvider(match provider {
                            crate::messages::OAuthProvider::Claude => ProviderId::Claude,
                            crate::messages::OAuthProvider::Gemini => ProviderId::Gemini,
                            crate::messages::OAuthProvider::ChatGpt => ProviderId::ChatGpt,
                        }));
                    }
                    Err(e) => {
                        self.error_message = Some(e);
                    }
                }
            }

            Message::RemoveOAuthAccount(account_id) => {
                if let Ok(db) = ConfigDatabase::open() {
                    let _ = db.delete_oauth_account(&account_id);
                }
                self.settings.provider_auth_status = check_provider_auth_status();
            }

            Message::ToggleOAuthAccountActive { account_id, is_active } => {
                if let Ok(db) = ConfigDatabase::open() {
                    let _ = db.set_oauth_account_active(&account_id, is_active);
                }
                self.settings.provider_auth_status = check_provider_auth_status();
            }

            Message::ResetOAuthCooldown(account_id) => {
                if let Ok(db) = ConfigDatabase::open() {
                    let _ = db.clear_oauth_account_cooldown(&account_id);
                }
            }

            Message::AdjustOAuthAccountPriority { account_id, delta } => {
                if let Ok(db) = ConfigDatabase::open() {
                    if let Ok(Some(account)) = db.get_oauth_account(&account_id) {
                        let new_priority = account.priority.saturating_add(delta);
                        let _ = db.set_oauth_account_priority(&account_id, new_priority);
                    }
                }
            }

            Message::NewSession => {
                self.chat.messages.clear();
                self.chat.raw_view_messages.clear();
                self.chat.raw_view_editors.clear();
                self.chat.messages.push(ChatMessage::assistant(
                    "New session started. How can I help you?",
                ));
                self.chat.current_session = None;
                self.chat.call_graph.reset(self.chat.current_agent);
                self.chat.subagent_message_indices.clear();
            }

            Message::LoadSession(session_id) => {
                tracing::info!("Loading session: {}", session_id);
                if let Some(loaded) = session_manager::load_session(&session_id) {
                    self.chat.messages = loaded.messages;
                    self.chat.current_session = Some(loaded.session);

                    self.chat.raw_view_messages.clear();
                    self.chat.raw_view_editors.clear();

                    if let Some(agent_type) = loaded.agent_type {
                        self.chat.current_agent = agent_type;
                        self.chat.agent_config = CoreAgentConfig::new(agent_type);
                    }

                    tracing::info!("Loaded session with {} messages", self.chat.messages.len());
                }
                self.chat.call_graph.reset(self.chat.current_agent);
                self.chat.subagent_message_indices.clear();
                self.current_view = View::Chat;
            }

            Message::RefreshModels => {
                if self.chat.is_loading_models {
                    return self.execute_commands(commands);
                }
                self.chat.is_loading_models = true;

                if !auth::has_any_valid_account() {
                    self.chat.is_loading_models = false;
                    return self.execute_commands(commands);
                }

                commands.push(AppCommand::RefreshModels);
            }

            Message::RefreshModelsForProvider(provider) => {
                if self.chat.is_loading_models {
                    return self.execute_commands(commands);
                }
                self.chat.is_loading_models = true;

                let mapped = match provider {
                    crate::messages::OAuthProvider::Claude => ProviderId::Claude,
                    crate::messages::OAuthProvider::Gemini => ProviderId::Gemini,
                    crate::messages::OAuthProvider::ChatGpt => ProviderId::ChatGpt,
                };
                commands.push(AppCommand::RefreshModelsForProvider(mapped));
            }

            Message::ModelsLoaded(result) => {
                self.chat.is_loading_models = false;
                match result {
                    Ok(models) => {
                        tracing::info!("Loaded {} models: {:?}", models.len(), models);
                        self.chat.available_models = models;
                    }
                    Err(e) => {
                        tracing::error!("Failed to load models: {}", e);
                        self.error_message = Some(format!("Failed to load models: {}", e));
                    }
                }
            }

            Message::SetDefaultModel(model_name) => {
                self.chat.default_model = Some(model_name.clone());
                if let Ok(db) = ConfigDatabase::open() {
                    let _ = db.set_setting(setting_keys::DEFAULT_MODEL, &model_name);
                }
                tracing::info!("Set default model: {}", model_name);
            }

            Message::SetAgentModel(agent_type, model) => {
                match &model {
                    Some(model_name) => {
                        self.chat.agent_pinned_models.insert(agent_type, model_name.clone());
                        if let Ok(db) = ConfigDatabase::open() {
                            let _ = db.set_agent_pinned_model(agent_type.as_str(), model_name);
                        }
                        tracing::info!("Pinned {} to model: {}", agent_type.as_str(), model_name);
                    }
                    None => {
                        self.chat.agent_pinned_models.remove(&agent_type);
                        if let Ok(db) = ConfigDatabase::open() {
                            let _ = db.clear_agent_pinned_model(agent_type.as_str());
                        }
                        tracing::info!("Cleared pinned model for {}", agent_type.as_str());
                    }
                }
            }

            Message::ToolCall { name, args } => {
                if let Some(last) = self.chat.messages.last_mut() {
                    if last.is_streaming {
                        let tool_line = format_tool_call_oneliner(
                            &name,
                            &args,
                            Some(&self.chat.working_directory),
                        );
                        last.content.push_str(&format!("\n\n{}", tool_line));
                        last.last_was_tool_call = true;
                        last.update_parsed_items();
                    }
                }
                self.push_scroll_if_needed(&mut commands);
            }

            Message::AgentCall(event) => {
                self.chat.call_graph.record_call(&event);
            }

            Message::SubagentStream(event) => {
                use ticca_core::tools::AgentStreamEvent;

                match event {
                    AgentStreamEvent::Start { node_id, agent_type } => {
                        let label = format!("{} - {}", agent_type.display_name(), node_id);
                        self.chat.messages.push(ChatMessage::assistant_streaming_named(label));
                        let index = self.chat.messages.len().saturating_sub(1);
                        self.chat.subagent_message_indices.insert(node_id, index);
                        self.chat.user_at_bottom = true;
                    }
                    AgentStreamEvent::Chunk { node_id, text } => {
                        if let Some(&index) = self.chat.subagent_message_indices.get(&node_id) {
                            if let Some(msg) = self.chat.messages.get_mut(index) {
                                if msg.last_was_tool_call && !text.trim().is_empty() {
                                    msg.content.push_str("\n\n💡 ");
                                    msg.last_was_tool_call = false;
                                }
                                msg.content.push_str(&text);
                                msg.update_parsed_items();
                            }
                        }
                        self.push_scroll_if_needed(&mut commands);
                    }
                    AgentStreamEvent::Reasoning { node_id, text } => {
                        if let Some(&index) = self.chat.subagent_message_indices.get(&node_id) {
                            if let Some(msg) = self.chat.messages.get_mut(index) {
                                if let Some(ref mut existing) = msg.reasoning {
                                    existing.push_str(&text);
                                } else {
                                    msg.reasoning = Some(text);
                                }
                            }
                        }
                        self.push_scroll_if_needed(&mut commands);
                    }
                    AgentStreamEvent::ToolCall { node_id, name, args } => {
                        if let Some(&index) = self.chat.subagent_message_indices.get(&node_id) {
                            if let Some(msg) = self.chat.messages.get_mut(index) {
                                let tool_line = format_tool_call_oneliner(
                                    &name,
                                    &args,
                                    Some(&self.chat.working_directory),
                                );
                                msg.content.push_str(&format!("\n\n{}", tool_line));
                                msg.last_was_tool_call = true;
                                msg.update_parsed_items();
                            }
                        }
                        self.push_scroll_if_needed(&mut commands);
                    }
                    AgentStreamEvent::Complete { node_id } => {
                        if let Some(index) = self.chat.subagent_message_indices.remove(&node_id) {
                            if let Some(msg) = self.chat.messages.get_mut(index) {
                                msg.is_streaming = false;
                                msg.update_parsed_items();
                            }
                        }
                    }
                }
            }

            Message::ToolResult { name: _, result: _ } => {}

            Message::ToolApprovalRequested { id, name, args } => {
                self.chat.pending_approvals.push_back(ToolApprovalPrompt { id, name, args });
                if self.chat.active_approval.is_none() {
                    self.chat.active_approval = self.chat.pending_approvals.pop_front();
                }
            }

            Message::ToolApprovalDecision { id, approved } => {
                if let Some(tx) = &self.chat.approval_tx {
                    let _ = tx.send(ticca_core::tools::ToolApprovalDecision { id, approved });
                }
                self.chat.active_approval = self.chat.pending_approvals.pop_front();
            }

            Message::SelectWorkingDirectory => {
                commands.push(AppCommand::PickWorkingDirectory);
            }

            Message::WorkingDirectoryChanged(path) => {
                self.chat.working_directory = path;
            }

            Message::SelectImageFile => {
                commands.push(AppCommand::PickImageFile);
            }

            Message::FileDropped(path) => {
                commands.push(AppCommand::LoadImage(path));
            }

            Message::ImageLoaded(result) => {
                match result {
                    Ok(attachment) => {
                        self.chat.pending_attachments.push(attachment);
                    }
                    Err(e) => {
                        if !e.contains("No file selected") {
                            self.error_message = Some(format!("Failed to load image: {}", e));
                        }
                    }
                }
            }

            Message::PasteImage => {
                commands.push(AppCommand::PasteImage);
            }

            Message::ImagePasted(result) => {
                match result {
                    Ok(attachment) => {
                        self.chat.pending_attachments.push(attachment);
                    }
                    Err(e) => {
                        if !e.contains("No image") {
                            self.error_message = Some(format!("Failed to paste image: {}", e));
                        }
                    }
                }
            }

            Message::RemoveAttachment(index) => {
                if index < self.chat.pending_attachments.len() {
                    self.chat.pending_attachments.remove(index);
                }
            }

            Message::LinkClicked(url) => {
                commands.push(AppCommand::OpenUrl(url.as_str().to_string()));
            }

            Message::DismissError => {
                self.error_message = None;
            }
        }

        self.execute_commands(commands)
    }

    fn execute_commands(&self, commands: Vec<AppCommand>) -> Task<Message> {
        let tasks: Vec<Task<Message>> = commands
            .into_iter()
            .map(|command| self.command_task(command))
            .collect();

        Task::batch(tasks)
    }

    fn command_task(&self, command: AppCommand) -> Task<Message> {
        match command {
            AppCommand::RunStream {
                system_prompt,
                user_message,
                model_name,
                working_directory,
                max_tool_rounds,
                history,
                image_data,
                yolo_mode_enabled,
                current_agent,
                approval_rx,
                cancel_rx,
            } => Task::run(
                llm_stream::run_rig_agent_stream(
                    system_prompt,
                    user_message,
                    model_name,
                    working_directory,
                    max_tool_rounds,
                    history,
                    image_data,
                    yolo_mode_enabled,
                    current_agent,
                    approval_rx,
                    cancel_rx,
                ),
                |event| event,
            ),
            AppCommand::ScrollToBottom => widget::operation::scroll_to(
                widget::Id::new(CHAT_SCROLLABLE_ID),
                AbsoluteOffset { x: 0.0, y: f32::MAX },
            ),
            AppCommand::StartOAuth(provider) => Task::perform(
                oauth_handler::start_oauth(provider),
                move |result| Message::OAuthComplete(provider, result),
            ),
            AppCommand::RefreshModels => Task::perform(
                async move { ticca_core::llm::ModelService::fetch_all().await },
                Message::ModelsLoaded,
            ),
            AppCommand::RefreshModelsForProvider(provider) => Task::perform(
                async move { ticca_core::llm::ModelService::fetch_for(provider).await },
                Message::ModelsLoaded,
            ),
            AppCommand::CopyToClipboard(content) => Task::perform(
                async move {
                    use arboard::Clipboard;
                    match Clipboard::new() {
                        Ok(mut clipboard) => {
                            if let Err(e) = clipboard.set_text(&content) {
                                tracing::error!("Failed to copy to clipboard: {}", e);
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to access clipboard: {}", e);
                        }
                    }
                },
                |_| Message::Noop,
            ),
            AppCommand::PickWorkingDirectory => Task::perform(
                async {
                    let dialog = rfd::AsyncFileDialog::new()
                        .set_title("Select Working Directory")
                        .pick_folder()
                        .await;

                    dialog.map(|handle| handle.path().to_path_buf())
                },
                |result| match result {
                    Some(path) => Message::WorkingDirectoryChanged(path),
                    None => Message::Noop,
                },
            ),
            AppCommand::PickImageFile => Task::perform(
                async {
                    let dialog = rfd::AsyncFileDialog::new()
                        .set_title("Select Image")
                        .add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp", "bmp"])
                        .pick_file()
                        .await;

                    match dialog {
                        Some(handle) => {
                            let path = handle.path().to_path_buf();
                            image_handler::load_image_from_path(&path).await
                        }
                        None => Err("No file selected".to_string()),
                    }
                },
                Message::ImageLoaded,
            ),
            AppCommand::LoadImage(path) => Task::perform(
                async move { image_handler::load_image_from_path(&path).await },
                Message::ImageLoaded,
            ),
            AppCommand::PasteImage => Task::perform(
                async { image_handler::paste_image_from_clipboard().await },
                Message::ImagePasted,
            ),
            AppCommand::OpenUrl(url) => Task::perform(
                async move {
                    if let Err(e) = open::that(&url) {
                        tracing::warn!("Failed to open URL {}: {}", url, e);
                    }
                },
                |_| Message::Noop,
            ),
        }
    }

    fn push_scroll_if_needed(&self, commands: &mut Vec<AppCommand>) {
        if self.chat.user_at_bottom {
            commands.push(AppCommand::ScrollToBottom);
        }
    }

    /// Render the application view
    pub fn view(&self) -> Element<'_, Message> {
        let content = match self.current_view {
            View::Chat => self.view_chat(),
            View::Settings => self.view_settings(),
        };
        
        // Wrap in container with error overlay if needed
        let main = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(0);
        
        let base: Element<Message> = if let Some(ref error) = self.error_message {
            // Show error toast at top
            let error_banner = container(
                row![
                    text(error).size(14),
                    button("×")
                        .on_press(Message::DismissError)
                        .padding(4)
                ]
                .spacing(10)
            )
            .padding(10)
            .style(container::rounded_box);
            
            column![
                error_banner,
                main,
            ]
            .into()
        } else {
            main.into()
        };

        if let Some(prompt) = &self.chat.active_approval {
            let overlay = self.view_approval_modal(prompt);
            iced::widget::stack![base, overlay].into()
        } else {
            base
        }
    }

    /// Get the current theme
    pub fn theme(&self) -> Theme {
        self.theme.to_iced_theme()
    }
    
    /// Get subscriptions (keyboard shortcuts, file drop events, and streaming stats timer)
    pub fn subscription(&self) -> Subscription<Message> {
        use iced::time;

        let keybindings = crate::keybindings::subscription();

        // Add timer subscriptions while streaming
        if self.chat.is_streaming {
            // Stats polling every 1 second
            let stats_timer = time::every(std::time::Duration::from_secs(1))
                .map(|_| Message::PollStreamStats);

            // Fast animation timer (~60 FPS) for smooth spinner when waiting
            let is_waiting = self.chat.last_bytes_time
                .map(|t| t.elapsed().as_secs() >= 2)
                .unwrap_or(false);

            if is_waiting {
                let animation_timer = time::every(std::time::Duration::from_millis(16))
                    .map(|_| Message::AnimationTick);
                Subscription::batch([keybindings, stats_timer, animation_timer])
            } else {
                Subscription::batch([keybindings, stats_timer])
            }
        } else {
            keybindings
        }
    }
    
    /// Render the chat view
    fn view_chat(&self) -> Element<'_, Message> {
        // Calculate seconds since last bytes received (for "waiting" indicator)
        let secs_since_bytes = self.chat.last_bytes_time
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0);

        pane_grid(&self.chat.panes, |_pane, pane_state, _| {
            let content = match pane_state {
                ChatPane::Chat => crate::views::chat::view(
                    self.chat.current_agent,
                    &self.chat.working_directory,
                    &self.chat.messages,
                    &self.chat.pending_attachments,
                    &self.chat.input_value,
                    self.chat.is_streaming,
                    self.theme,
                    &self.chat.raw_view_messages,
                    &self.chat.raw_view_editors,
                    self.chat.stream_chars_received,
                    self.chat.current_tps,
                    self.chat.stream_pulse,
                    secs_since_bytes,
                    self.chat.spinner_frame,
                    self.chat.flow_pane.is_some(),
                ),
                ChatPane::Flow => crate::views::agent_flow::view(&self.chat.call_graph, self.theme),
            };
            iced::widget::pane_grid::Content::new(content)
        })
        .on_resize(10, Message::PaneResized)
        .into()
    }

    /// Save the current session to the database
    fn save_current_session(&mut self) {
        if let Some(session) = session_manager::save_session(
            self.chat.current_session.as_ref(),
            &self.chat.messages,
            self.chat.current_agent,
        ) {
            self.chat.current_session = Some(session);
        }
    }
    
    /// Render the settings view
    fn view_settings(&self) -> Element<'_, Message> {
        crate::views::config::view(
            self.theme,
            &self.chat.available_models,
            self.chat.default_model.as_deref(),
            &self.chat.agent_pinned_models,
            self.chat.is_loading_models,
            &self.settings.provider_auth_status,
            self.chat.yolo_mode_enabled,
            self.settings.settings_tab,
        )
    }

    fn view_approval_modal(&self, prompt: &ToolApprovalPrompt) -> Element<'_, Message> {
        let content = container(
            column![
                text("Tool approval required").size(18),
                text(format!("Tool: {}", prompt.name)).size(14),
                text(prompt.args.clone()).size(12),
                row![
                    button("Deny")
                        .on_press(Message::ToolApprovalDecision { id: prompt.id, approved: false })
                        .style(crate::theme::styles::secondary_button)
                        .padding([6, 12]),
                    button("Approve")
                        .on_press(Message::ToolApprovalDecision { id: prompt.id, approved: true })
                        .style(crate::theme::styles::success_button)
                        .padding([6, 12]),
                ]
                .spacing(12),
            ]
            .spacing(10)
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
}

impl Default for TiccaApp {
    fn default() -> Self {
        Self::new().0
    }
}
