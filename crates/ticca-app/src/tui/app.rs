//! TUI Application State
//!
//! Contains all state for the terminal UI, including chat messages,
//! settings, streaming state, and UI focus management.

use std::cell::Cell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use ratatui::layout::Rect;
use tokio::sync::{mpsc, oneshot};
use tui_textarea::TextArea;

use crate::chat_message::{ChatMessage, ContentBlock, MessageId};
use crate::cli::Args;
use crate::theme::AppTheme;
use super::theme::{TuiColors, TuiStyles};

use ticca_core::agents::AgentType;
use ticca_core::config::{
    setting_keys, ApiKeyAccount, CompressionSettings, ConfigService, McpServer, OAuthAccount,
    UiMode,
};
use ticca_core::external_tools::ExternalToolId;
use ticca_core::session::{LoadedSessionData, Session, SessionService};
use ticca_core::tools::{SystemExecStore, ToolApprovalDecision};

/// Current view in the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Chat,
    Settings,
    Help,
}

/// Settings tab selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    Accounts,
    Models,
    Agents,
    McpServers,
    Tools,
    Appearance,
    Sessions,
}

impl SettingsTab {
    pub const ALL: &'static [SettingsTab] = &[
        SettingsTab::Accounts,
        SettingsTab::Models,
        SettingsTab::Agents,
        SettingsTab::McpServers,
        SettingsTab::Tools,
        SettingsTab::Appearance,
        SettingsTab::Sessions,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            SettingsTab::Accounts => "Accounts",
            SettingsTab::Models => "Models",
            SettingsTab::Agents => "Agents",
            SettingsTab::McpServers => "MCP",
            SettingsTab::Tools => "Tools",
            SettingsTab::Appearance => "Appearance",
            SettingsTab::Sessions => "Sessions",
        }
    }

    pub fn next(&self) -> Self {
        let idx = Self::ALL.iter().position(|t| t == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(&self) -> Self {
        let idx = Self::ALL.iter().position(|t| t == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// Focus areas within the chat view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChatFocus {
    #[default]
    Input,
    Messages,
    AgentPicker,
    ModelPicker,
}

/// Focus areas within settings view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsFocus {
    #[default]
    TabBar,
    Content,
    Form,
    List,
}

/// Kind of modal popup currently displayed
#[derive(Debug, Clone)]
pub enum ModalKind {
    ToolApproval(ToolApprovalPrompt),
    AgentPicker,
    ModelPicker,
    ThemePicker,
    FilePicker {
        path: PathBuf,
        entries: Vec<(String, bool)>,
        selected: usize,
    },
    Confirmation {
        title: String,
        message: String,
        action: ConfirmAction,
    },
    Error(String),
    ApiKeyForm(FormState),
    McpServerForm(FormState),
}

/// Actions that can be confirmed via modal
#[derive(Debug, Clone)]
pub enum ConfirmAction {
    DeleteSession(String),
    DeleteMcpServer(String),
    DeleteAccount(String),
    NewSession,
}

/// Tool approval prompt data
#[derive(Debug, Clone)]
pub struct ToolApprovalPrompt {
    pub id: u64,
    pub tool_name: String,
    pub args: String,
}

/// External tool status for display
#[derive(Debug, Clone, Default)]
pub struct ToolStatusInfo {
    pub is_installed: bool,
    pub version: Option<String>,
    pub is_installing: bool,
    pub install_progress: u8,
    pub is_supported: bool,
}

/// Form state for editing
#[derive(Debug, Clone, Default)]
pub struct FormState {
    pub fields: Vec<FormField>,
    pub focused_field: usize,
}

#[derive(Debug, Clone)]
pub struct FormField {
    pub label: String,
    pub value: String,
    pub field_type: FormFieldType,
}

#[derive(Debug, Clone)]
pub enum FormFieldType {
    Text,
    Password,
    Select(Vec<String>),
}

impl FormState {
    pub fn new(fields: Vec<FormField>) -> Self {
        Self {
            fields,
            focused_field: 0,
        }
    }

    pub fn next_field(&mut self) {
        let total = self.fields.len().max(1);
        self.focused_field = (self.focused_field + 1) % total;
    }

    pub fn prev_field(&mut self) {
        let total = self.fields.len().max(1);
        self.focused_field = (self.focused_field + total - 1) % total;
    }
}

/// Settings-specific state
#[derive(Debug, Default)]
pub struct SettingsState {
    pub active_tab: SettingsTab,
    pub focus: SettingsFocus,
    pub list_index: usize,

    // Accounts tab
    pub oauth_accounts_claude: Vec<OAuthAccount>,
    pub oauth_accounts_gemini: Vec<OAuthAccount>,
    pub oauth_accounts_chatgpt: Vec<OAuthAccount>,
    pub api_key_accounts: HashMap<String, Vec<ApiKeyAccount>>,
    pub api_key_form: Option<FormState>,
    pub provider_search: String,

    // Models tab
    pub default_model: Option<String>,
    pub agent_pinned_models: HashMap<AgentType, String>,
    pub is_loading_models: bool,

    // MCP tab
    pub mcp_servers: Vec<McpServer>,
    pub mcp_form: Option<FormState>,
    pub agent_mcp_ids: HashMap<AgentType, Vec<String>>,
    pub mcp_import_text: String,

    // Tools tab
    pub external_tools: HashMap<ExternalToolId, ToolStatusInfo>,
    pub compression: CompressionSettings,

    // Sessions tab
    pub recent_sessions: Vec<Session>,
}

impl SettingsState {
    pub fn load() -> Self {
        let mut state = Self::default();
        state.refresh_all();
        state
    }

    pub fn refresh_all(&mut self) {
        self.refresh_accounts();
        self.refresh_models();
        self.refresh_mcp();
        self.refresh_sessions();
        self.compression = CompressionSettings::load();
    }

    pub fn refresh_accounts(&mut self) {
        use ticca_core::config::models::providers;
        self.oauth_accounts_claude =
            ConfigService::list_oauth_accounts_pruned(providers::CLAUDE).unwrap_or_default();
        self.oauth_accounts_gemini =
            ConfigService::list_oauth_accounts_pruned(providers::GEMINI).unwrap_or_default();
        self.oauth_accounts_chatgpt =
            ConfigService::list_oauth_accounts_pruned(providers::CHATGPT).unwrap_or_default();

        self.api_key_accounts.clear();
        for provider in ticca_core::RegistryService::api_key_providers() {
            let accounts =
                ConfigService::list_api_key_accounts(Some(&provider.id)).unwrap_or_default();
            if !accounts.is_empty() {
                self.api_key_accounts.insert(provider.id.clone(), accounts);
            }
        }
    }

    pub fn refresh_models(&mut self) {
        self.agent_pinned_models.clear();
        match ConfigService::load_settings_snapshot() {
            Ok(snapshot) => {
                self.default_model = snapshot.settings.default_model;
                for (agent_str, model) in snapshot.agent_pinned_models {
                    if let Some(agent) = AgentType::parse(&agent_str) {
                        self.agent_pinned_models.insert(agent, model);
                    }
                }
            }
            Err(_) => {
                self.default_model = None;
            }
        }
    }

    pub fn refresh_mcp(&mut self) {
        self.mcp_servers = ConfigService::list_mcp_servers().unwrap_or_default();

        self.agent_mcp_ids.clear();
        for agent in [AgentType::Planning, AgentType::Coding] {
            let ids = ConfigService::get_agent_mcp_server_ids(agent.as_str()).unwrap_or_default();
            self.agent_mcp_ids.insert(agent, ids);
        }
    }

    pub fn refresh_sessions(&mut self) {
        self.recent_sessions = SessionService::list_recent(10).unwrap_or_default();
    }
}

/// Main TUI application state
pub struct TuiApp<'a> {
    // === Core State ===
    pub view: View,
    pub theme: AppTheme,
    pub colors: TuiColors,
    pub styles: TuiStyles,
    pub ui_mode: UiMode,
    pub should_quit: bool,

    // === Chat State ===
    pub chat_focus: ChatFocus,
    pub messages: Vec<ChatMessage>,
    pub input: TextArea<'a>,
    pub send_button_area: Cell<Option<Rect>>,
    pub current_agent: AgentType,
    pub current_model: Option<String>,
    pub available_models: Vec<String>,
    pub working_directory: PathBuf,
    pub current_session: Option<Session>,
    pub message_scroll: usize,
    pub yolo_mode: bool,
    pub max_tool_rounds: u32,

    // === Streaming State ===
    pub is_streaming: bool,
    pub stream_buffer: String,
    pub current_tps: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub context_window: u64,
    pub stream_cancel_tx: Option<oneshot::Sender<()>>,
    pub approval_tx: Option<mpsc::UnboundedSender<ToolApprovalDecision>>,

    // === Pending Approval ===
    pub pending_approval: Option<ToolApprovalPrompt>,

    // === Settings State ===
    pub settings: SettingsState,

    // === Modal State ===
    pub active_modal: Option<ModalKind>,
    pub modal_list_index: usize,

    // === Error Display ===
    pub error_message: Option<String>,
    pub error_dismiss_time: Option<std::time::Instant>,

    // === System Execution ===
    pub system_exec_store: Arc<SystemExecStore>,
    pub system_exec_tx: mpsc::UnboundedSender<ticca_core::tools::SystemExecRequest>,
    pub system_exec_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<ticca_core::tools::SystemExecRequest>>>,

    // === Animation ===
    pub spinner_frame: usize,
    pub last_tick: std::time::Instant,
}

impl<'a> TuiApp<'a> {
    /// Create a new TUI application with the given CLI arguments
    pub fn new(args: &Args) -> anyhow::Result<Self> {
        // Initialize data directories and runtime
        Self::initialize_runtime();

        // Load configuration
        let config = crate::app_config::load_config();

        // Determine theme (CLI overrides config)
        let theme = args
            .theme
            .as_ref()
            .map(|t| AppTheme::parse(t))
            .unwrap_or(config.theme);
        let colors = TuiColors::from_theme(theme);
        let styles = TuiStyles::new(colors);

        // Determine working directory
        let working_directory = args
            .working_dir
            .clone()
            .or(config.working_directory)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        // Determine starting agent
        let current_agent = args.agent_type().unwrap_or(AgentType::Coding);

        // YOLO mode
        let yolo_mode = args.yolo || config.yolo_mode_enabled;

        // Create input textarea with styling
        let mut input = TextArea::default();
        input.set_cursor_line_style(ratatui::style::Style::default());
        input.set_placeholder_text("Type your message... (Enter to send)");

        // System exec channels
        let system_exec_store = Arc::new(SystemExecStore::new());
        let (system_exec_tx, system_exec_rx) = mpsc::unbounded_channel();
        let system_exec_rx = Arc::new(tokio::sync::Mutex::new(system_exec_rx));

        // Load available models
        let available_models = ConfigService::list_discovered_models(None)
            .map(|models| models.into_iter().map(|model| model.canonical_id).collect())
            .unwrap_or_default();

        let default_model = config.default_model.clone();

        let (current_session, messages) = match args.session.as_deref() {
            Some(session_id) => Self::load_session_data(session_id),
            None => (None, Vec::new()),
        };

        Ok(Self {
            // Core
            view: View::Chat,
            theme,
            colors,
            styles,
            ui_mode: config.ui_mode,
            should_quit: false,

            // Chat
            chat_focus: ChatFocus::Input,
            messages,
            input,
            send_button_area: Cell::new(None),
            current_agent,
            current_model: default_model.clone(),
            available_models,
            working_directory,
            current_session,
            message_scroll: 0,
            yolo_mode,
            max_tool_rounds: config.max_tool_rounds,

            // Streaming
            is_streaming: false,
            stream_buffer: String::new(),
            current_tps: 0.0,
            input_tokens: 0,
            output_tokens: 0,
            context_window: 200_000,
            stream_cancel_tx: None,
            approval_tx: None,

            // Approval
            pending_approval: None,

            // Settings
            settings: SettingsState::load(),

            // Modal
            active_modal: None,
            modal_list_index: 0,

            // Error
            error_message: None,
            error_dismiss_time: None,

            // System exec
            system_exec_store,
            system_exec_tx,
            system_exec_rx,

            // Animation
            spinner_frame: 0,
            last_tick: std::time::Instant::now(),
        })
    }

    fn initialize_runtime() {
        use tracing::{info, warn};

        if let Err(error) = ticca_core::config::ensure_dirs_exist() {
            warn!("Failed to create data directories: {}", error);
        }

        match ticca_core::extract_skills_if_needed() {
            Ok(skills_dir) => {
                info!("Skills available at: {}", skills_dir.display());
            }
            Err(error) => {
                warn!("Failed to extract skills bundle: {}", error);
            }
        }

        match ticca_core::ensure_uv_available() {
            Ok(uv_path) => {
                info!("UV binary available at: {}", uv_path.display());
            }
            Err(error) => {
                warn!("Failed to extract UV binary: {}", error);
            }
        }
    }

    fn load_session_data(session_id: &str) -> (Option<Session>, Vec<ChatMessage>) {
        let Ok(Some(LoadedSessionData { session, messages })) = SessionService::load(session_id)
        else {
            return (None, Vec::new());
        };

        let chat_messages = messages
            .into_iter()
            .map(|message| {
                let message_id = message
                    .message_id
                    .as_deref()
                    .and_then(MessageId::from_string)
                    .unwrap_or_else(MessageId::new);
                ChatMessage {
                    id: message_id,
                    role: message.role,
                    content: message.content.clone(),
                    is_streaming: false,
                    author_label: None,
                    reasoning: message.reasoning,
                    reasoning_signature: message.reasoning_signature,
                    content_blocks: vec![ContentBlock::text(message.content)],
                    last_was_tool_call: false,
                }
            })
            .collect();

        (Some(session), chat_messages)
    }

    /// Set the current theme
    pub fn set_theme(&mut self, theme: AppTheme) {
        self.theme = theme;
        self.colors = TuiColors::from_theme(theme);
        self.styles = TuiStyles::new(self.colors);
        let _ = ConfigService::set_setting(setting_keys::THEME, theme.as_str());
    }

    /// Toggle YOLO mode
    pub fn toggle_yolo(&mut self) {
        self.yolo_mode = !self.yolo_mode;
        let value = if self.yolo_mode { "true" } else { "false" };
        let _ = ConfigService::set_setting(setting_keys::YOLO_MODE, value);
    }

    /// Switch to a different agent
    pub fn switch_agent(&mut self, agent: AgentType) {
        self.current_agent = agent;
        // Check for pinned model
        if let Some(pinned) = self.settings.agent_pinned_models.get(&agent) {
            self.current_model = Some(pinned.clone());
        }
    }

    /// Send a message and start streaming
    pub fn send_message(&mut self, event_tx: mpsc::UnboundedSender<super::event::Event>) {
        let input_text = self.input.lines().join("\n");
        if input_text.trim().is_empty() {
            return;
        }

        let user_msg = ChatMessage::user(input_text.clone());
        self.messages.push(user_msg);

        let mut assistant_msg = ChatMessage::assistant_streaming();
        assistant_msg.is_streaming = true;
        self.messages.push(assistant_msg);

        self.input = TextArea::default();
        self.input.set_cursor_line_style(ratatui::style::Style::default());
        self.input
            .set_placeholder_text("Type your message... (Enter to send)");

        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
        let (approval_tx, approval_rx) = tokio::sync::mpsc::unbounded_channel();

        self.stream_cancel_tx = Some(cancel_tx);
        self.approval_tx = Some(approval_tx);
        self.is_streaming = true;
        self.stream_buffer.clear();

        super::streaming::start_streaming(
            event_tx,
            self.get_system_prompt(),
            input_text,
            self.current_model.clone(),
            self.working_directory.clone(),
            self.max_tool_rounds,
            self.messages[..self.messages.len() - 1].to_vec(),
            None,
            vec![],
            self.yolo_mode,
            self.current_agent,
            approval_rx,
            cancel_rx,
            self.system_exec_store.clone(),
            self.system_exec_tx.clone(),
        );

        self.message_scroll = usize::MAX;
    }

    pub fn open_directory_picker(&mut self) {
        let path = self.working_directory.clone();
        let entries = read_directory_entries(&path);
        self.active_modal = Some(ModalKind::FilePicker {
            path,
            entries,
            selected: 0,
        });
    }

    /// Start a new session
    pub fn new_session(&mut self) {
        self.messages.clear();
        self.current_session = None;
        self.input_tokens = 0;
        self.output_tokens = 0;
        self.message_scroll = 0;
    }

    /// Load a session by ID
    pub fn load_session(&mut self, session_id: &str) {
        let (session, messages) = Self::load_session_data(session_id);
        self.current_session = session;
        self.messages = messages;
        self.message_scroll = 0;
    }

    /// Show an error message
    pub fn show_error(&mut self, message: impl Into<String>) {
        self.error_message = Some(message.into());
        self.error_dismiss_time = Some(
            std::time::Instant::now() + std::time::Duration::from_secs(5),
        );
    }

    /// Clear error if expired
    pub fn tick_error(&mut self) {
        if let Some(dismiss_time) = self.error_dismiss_time {
            if std::time::Instant::now() >= dismiss_time {
                self.error_message = None;
                self.error_dismiss_time = None;
            }
        }
    }

    /// Update spinner animation frame
    pub fn tick_animation(&mut self) {
        if self.last_tick.elapsed() >= std::time::Duration::from_millis(100) {
            self.spinner_frame = (self.spinner_frame + 1) % 8;
            self.last_tick = std::time::Instant::now();
        }
    }

    /// Get the system prompt for the current agent
    pub fn get_system_prompt(&self) -> String {
        ticca_core::agents::get_agent(self.current_agent).system_prompt()
    }

    /// Get spinner character
    pub fn spinner_char(&self) -> char {
        const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];
        SPINNER[self.spinner_frame % SPINNER.len()]
    }
}

pub fn read_directory_entries(path: &std::path::Path) -> Vec<(String, bool)> {
    let mut entries = vec![("..".to_string(), true)];

    if let Ok(read_dir) = std::fs::read_dir(path) {
        let mut items: Vec<(String, bool)> = read_dir
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    return None;
                }
                let is_dir = e.file_type().ok()?.is_dir();
                Some((name, is_dir))
            })
            .collect();

        items.sort_by(|a, b| match (a.1, b.1) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.0.cmp(&b.0),
        });

        entries.extend(items);
    }

    entries
}
