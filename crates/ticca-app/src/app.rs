//! GPUI Application state and main view
//!
//! This is the root component of the Ticca Desktop application.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{
    div, AnyElement, AppContext as _, Context, Entity,
    InteractiveElement as _, IntoElement, ParentElement as _, Render,
    ScrollHandle, Styled as _, Window,
};
use gpui_component::{
    input::{InputEvent, InputState},
    select::{SearchableVec, SelectEvent, SelectState},
    ActiveTheme,
};

use crate::views::settings::{ModelSelectItem, ProviderSelectItem};

use std::collections::HashMap;

use ticca_core::agents::{AgentProfile, AgentType, ChatHistoryMessage, RunnerEvent};
use ticca_core::config::{ApiKeyAccount, McpServer, McpTransport, OAuthAccount, UiMode};
use ticca_core::external_tools::ExternalToolId;
use ticca_core::session::Session;
use ticca_core::tools::{
    SystemExecStore, ToolApprovalDecision as CoreToolApprovalDecision, TodoListState,
};

use tokio::sync::mpsc;

use crate::actions::*;
use crate::chat_message::ChatMessage;
use crate::theme::AppTheme;
use crate::views;

// =============================================================================
// Tool Call Display Helper
// =============================================================================

/// Format a tool call as a nice one-liner for display in chat
fn format_tool_call_display(name: &str, args: &str) -> String {
    // Try to parse the args as JSON to extract relevant fields
    let parsed: serde_json::Value = serde_json::from_str(args).unwrap_or(serde_json::Value::Null);

    match name {
        "list_files" => {
            let dir = parsed.get("directory")
                .or(parsed.get("path"))
                .and_then(|v| v.as_str())
                .unwrap_or(".");
            let recursive = parsed.get("recursive")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let rec_str = if recursive { " (recursive)" } else { "" };
            format!("📂 `{}`{}", dir, rec_str)
        }
        "read_file" => {
            let path = parsed.get("file_path")
                .or(parsed.get("path"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            format!("📄 `{}`", path)
        }
        "write_file" => {
            let path = parsed.get("file_path")
                .or(parsed.get("path"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            format!("✏️ `{}`", path)
        }
        "edit_file" => {
            let path = parsed.get("file_path")
                .or(parsed.get("path"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            format!("🔧 `{}`", path)
        }
        "delete_file" => {
            let path = parsed.get("file_path")
                .or(parsed.get("path"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            format!("🗑️ `{}`", path)
        }
        "grep" | "search" => {
            let pattern = parsed.get("pattern")
                .or(parsed.get("search_string"))
                .or(parsed.get("query"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let dir = parsed.get("directory")
                .or(parsed.get("path"))
                .and_then(|v| v.as_str())
                .unwrap_or(".");
            format!("🔍 `{}` in `{}`", pattern, dir)
        }
        "execute_shell" | "run_shell_command" | "agent_run_shell_command" => {
            let cmd = parsed.get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let preview = if cmd.len() > 60 {
                format!("{}...", &cmd[..57])
            } else {
                cmd.to_string()
            };
            format!("💻 `{}`", preview)
        }
        "invoke_agent" => {
            let agent = parsed.get("agent_name")
                .or(parsed.get("agent"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            format!("🤖 → {}", agent)
        }
        "agent_share_your_reasoning" => {
            "💭 reasoning...".to_string()
        }
        _ => {
            // For unknown tools, show name only
            format!("🔧 {}", name)
        }
    }
}

// =============================================================================
// Toast Notifications
// =============================================================================

/// Toast notification state
#[derive(Debug, Clone)]
pub struct Toast {
    /// The message to display
    pub message: String,
    /// When the toast was created (for auto-dismiss)
    pub created_at: Instant,
}

impl Toast {
    /// Duration before toast auto-dismisses
    pub const DURATION: Duration = Duration::from_secs(4);

    /// Create a new toast notification
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            created_at: Instant::now(),
        }
    }

    /// Check if the toast has expired
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= Self::DURATION
    }
}

// =============================================================================
// Update Available State
// =============================================================================

/// State for the update available notification
#[derive(Debug, Clone)]
pub struct UpdateAvailableState {
    /// Current app version
    pub current_version: String,
    /// Latest available version
    pub latest_version: String,
    /// URL to the release page
    pub release_url: String,
}

// =============================================================================
// Agent Call Graph (for Agent Flow visualization)
// =============================================================================

/// Status of an agent node during execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentStatus {
    #[default]
    Running,
    Completed,
    Failed,
}

/// A node in the agent call graph
#[derive(Debug, Clone)]
pub struct AgentNode {
    pub id: usize,
    pub agent_type: AgentType,
    pub label: String,
    pub status: AgentStatus,
    pub run_id: usize,
    pub started_at: Option<Instant>,
    pub completed_at: Option<Instant>,
}

impl AgentNode {
    /// Create a new running agent node
    pub fn new(id: usize, agent_type: AgentType, run_id: usize) -> Self {
        Self {
            id,
            agent_type,
            label: format!("{} #{}", agent_type.display_name(), id),
            status: AgentStatus::Running,
            run_id,
            started_at: Some(Instant::now()),
            completed_at: None,
        }
    }

    /// Get elapsed time since started
    pub fn elapsed(&self) -> Option<Duration> {
        self.started_at.map(|start| start.elapsed())
    }

    /// Get duration (if completed)
    pub fn duration(&self) -> Option<Duration> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => Some(end.duration_since(start)),
            _ => None,
        }
    }
}

/// An edge (call) between agent nodes
#[derive(Debug, Clone)]
pub struct AgentEdge {
    pub from: usize,
    pub to: usize,
    pub prompt_preview: String,
}

/// The agent call graph tracking execution flow
#[derive(Debug, Clone)]
pub struct AgentCallGraph {
    pub nodes: HashMap<usize, AgentNode>,
    pub edges: Vec<AgentEdge>,
    pub order: Vec<usize>,
    pub current_run: usize,
}

impl Default for AgentCallGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentCallGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            order: Vec::new(),
            current_run: 0,
        }
    }

    /// Start a new run with a root agent
    pub fn start_run(&mut self, agent_type: AgentType) {
        self.current_run += 1;
        let id = self.next_id();
        let node = AgentNode::new(id, agent_type, self.current_run);
        self.nodes.insert(id, node);
        self.order.push(id);
    }

    /// Add a child agent call
    pub fn add_call(&mut self, parent_id: usize, child_type: AgentType, prompt: &str) -> usize {
        let child_id = self.next_id();
        let node = AgentNode::new(child_id, child_type, self.current_run);
        self.nodes.insert(child_id, node);
        self.order.push(child_id);

        let prompt_preview = if prompt.len() > 100 {
            format!("{}...", &prompt[..100])
        } else {
            prompt.to_string()
        };
        self.edges.push(AgentEdge {
            from: parent_id,
            to: child_id,
            prompt_preview,
        });

        child_id
    }

    /// Mark a node as completed
    pub fn mark_completed(&mut self, node_id: usize) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.status = AgentStatus::Completed;
            node.completed_at = Some(Instant::now());
        }
    }

    /// Mark a node as failed
    pub fn mark_failed(&mut self, node_id: usize) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.status = AgentStatus::Failed;
            node.completed_at = Some(Instant::now());
        }
    }

    /// Check if any node is running
    pub fn is_any_running(&self) -> bool {
        self.nodes.values().any(|n| n.status == AgentStatus::Running)
    }

    /// Clear all data
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.order.clear();
        self.current_run = 0;
    }

    fn next_id(&self) -> usize {
        self.nodes.keys().max().map(|m| m + 1).unwrap_or(1)
    }
}

// =============================================================================
// System Executions (command outputs)
// =============================================================================

/// Status of a system execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExecutionStatus {
    #[default]
    Running,
    Success,
    Failed,
}

/// A system command execution
#[derive(Debug, Clone)]
pub struct SystemExecution {
    pub id: u64,
    pub command: String,
    pub output: String,
    pub status: ExecutionStatus,
    pub started_at: Instant,
    pub exit_code: Option<i32>,
}

impl SystemExecution {
    pub fn new(id: u64, command: String) -> Self {
        Self {
            id,
            command,
            output: String::new(),
            status: ExecutionStatus::Running,
            started_at: Instant::now(),
            exit_code: None,
        }
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Mark as completed with exit code
    pub fn complete(&mut self, exit_code: i32, output: String) {
        self.exit_code = Some(exit_code);
        self.output = output;
        self.status = if exit_code == 0 {
            ExecutionStatus::Success
        } else {
            ExecutionStatus::Failed
        };
    }

    /// Append output
    pub fn append_output(&mut self, text: &str) {
        self.output.push_str(text);
    }
}

// =============================================================================
// Chat State
// =============================================================================

/// State for the chat feature
pub struct ChatState {
    /// Input state for the chat input field
    pub input_state: Entity<InputState>,
    /// Chat messages
    pub messages: Vec<ChatMessage>,
    /// Whether we're currently streaming a response
    pub is_streaming: bool,
    /// Handle for the current stream (for cancellation)
    pub stream_handle: Option<crate::llm_stream::StreamHandle>,
    /// Current working directory
    pub working_directory: PathBuf,
    /// Currently selected agent
    pub current_agent: AgentType,
    /// Pending image attachments
    pub pending_attachments: Vec<ImageAttachment>,
    /// Whether the flow panel is visible
    pub flow_panel_visible: bool,
    /// Currently selected sidebar tab
    pub sidebar_tab: RightSidebarTab,
    /// Current session ID
    pub session_id: Option<String>,
    /// Todo list state (from agent graph)
    pub todo_state: Option<TodoListState>,
    /// Agent call graph (for Agent Flow visualization)
    pub agent_graph: AgentCallGraph,
    /// System executions (command outputs)
    pub system_executions: Vec<SystemExecution>,
    /// Next execution ID
    pub next_execution_id: u64,
    /// Cancel channel for stopping the stream
    pub cancel_tx: Option<tokio::sync::oneshot::Sender<()>>,
    /// Approval decision sender for tool approvals
    pub approval_tx: Option<mpsc::UnboundedSender<CoreToolApprovalDecision>>,
    
    /// Streaming stats: characters per second
    pub streaming_chars_per_sec: Option<f64>,
    /// Streaming stats: total characters streamed in current response
    pub streaming_total_chars: usize,
    
    /// Context window: usage percentage (0-100)
    pub context_usage_percent: Option<u32>,
    /// Context window: tokens used
    pub context_tokens_used: Option<usize>,
    /// Context window: total window size
    pub context_window_size: Option<u64>,
    
    /// Scroll handle for programmatic scroll control in messages list
    pub scroll_handle: ScrollHandle,
    /// Whether auto-scroll to bottom is enabled (true when user is at bottom)
    pub auto_scroll_enabled: bool,
}



// =============================================================================
// Settings State
// =============================================================================

/// Provider authentication status
#[derive(Debug, Clone, Default)]
pub struct ProviderAuthStatus {
    pub claude: bool,
    pub gemini: bool,
    pub chatgpt: bool,
}

/// State for the settings feature
pub struct SettingsState {
    /// Currently selected settings tab
    pub current_tab: SettingsTab,
    /// YOLO mode (auto-approve tools)
    pub yolo_mode: bool,
    /// UI mode (Normal/Compact/Debug)
    pub ui_mode: UiMode,
    
    // OAuth accounts
    /// Provider authentication status
    pub provider_auth_status: ProviderAuthStatus,
    /// Claude OAuth accounts
    pub accounts_claude: Vec<OAuthAccount>,
    /// Gemini OAuth accounts  
    pub accounts_gemini: Vec<OAuthAccount>,
    /// ChatGPT OAuth accounts
    pub accounts_chatgpt: Vec<OAuthAccount>,
    
    // API Key accounts
    /// API key accounts by provider ID
    pub api_key_accounts: HashMap<String, Vec<ApiKeyAccount>>,
    /// API key form: currently editing provider ID
    pub api_key_form_provider: Option<String>,
    /// API key form: current key value
    pub api_key_form_value: String,
    /// API key form: current label value
    pub api_key_form_label: String,
    /// Provider search query (for adding new providers)
    pub provider_search_query: String,
    /// Whether the provider search dropdown is expanded
    pub provider_search_expanded: bool,
    
    // Models
    /// Available models from all providers
    pub available_models: Vec<String>,
    /// Default model for new chats
    pub default_model: Option<String>,
    /// Pinned models per agent type
    pub agent_pinned_models: HashMap<AgentType, String>,
    
    // MCP Servers
    /// Configured MCP servers
    pub mcp_servers: Vec<McpServer>,
    /// MCP form: currently editing server ID (None = new)
    pub mcp_form_server_id: Option<String>,
    /// MCP form: server name
    pub mcp_form_name: String,
    /// MCP form: transport type
    pub mcp_form_transport: McpTransport,
    /// MCP form: command
    pub mcp_form_command: String,
    /// MCP form: args JSON
    pub mcp_form_args_json: String,
    /// MCP form: env JSON
    pub mcp_form_env_json: String,
    /// MCP form: endpoint URL (for HTTP transport)
    pub mcp_form_endpoint_url: String,
    /// MCP form: enabled flag
    pub mcp_form_enabled: bool,
    /// Agent to MCP server mappings
    pub agent_mcp_server_ids: HashMap<AgentType, Vec<String>>,
    
    // Sessions
    /// Recent sessions
    pub recent_sessions: Vec<Session>,
    
    // External Tools
    /// External tool status info
    pub external_tools: HashMap<ExternalToolId, ExternalToolStatus>,
    
    // Loading state
    /// Whether we're loading models
    pub is_loading_models: bool,
    
    // Input states for forms
    /// MCP form name input
    pub mcp_name_input: Entity<InputState>,
    /// MCP form command input
    pub mcp_command_input: Entity<InputState>,
    /// MCP form URL input
    pub mcp_url_input: Entity<InputState>,
    /// API key input
    pub api_key_input: Entity<InputState>,
    /// API key label input
    pub api_key_label_input: Entity<InputState>,
    /// Provider select dropdown state
    pub provider_select: Entity<SelectState<SearchableVec<ProviderSelectItem>>>,
    /// Model select dropdown state
    pub model_select: Entity<SelectState<SearchableVec<ModelSelectItem>>>,
}

/// Status info for an external tool
#[derive(Debug, Clone, Default)]
pub struct ExternalToolStatus {
    pub is_installed: bool,
    pub version: Option<String>,
    pub is_installing: bool,
    pub install_progress: u8,
    pub is_supported: bool,
}

impl SettingsState {
    /// Create a new SettingsState with input entities
    pub fn new(window: &mut Window, cx: &mut Context<TiccaApp>) -> Self {
        Self {
            current_tab: SettingsTab::Accounts,
            yolo_mode: false,
            ui_mode: UiMode::Easy,
            provider_auth_status: ProviderAuthStatus::default(),
            accounts_claude: Vec::new(),
            accounts_gemini: Vec::new(),
            accounts_chatgpt: Vec::new(),
            api_key_accounts: HashMap::new(),
            api_key_form_provider: None,
            api_key_form_value: String::new(),
            api_key_form_label: String::new(),
            provider_search_query: String::new(),
            provider_search_expanded: false,
            available_models: Vec::new(),
            default_model: None,
            agent_pinned_models: HashMap::new(),
            mcp_servers: Vec::new(),
            mcp_form_server_id: None,
            mcp_form_name: String::new(),
            mcp_form_transport: McpTransport::Stdio,
            mcp_form_command: String::new(),
            mcp_form_args_json: "[]".to_string(),
            mcp_form_env_json: "{}".to_string(),
            mcp_form_endpoint_url: String::new(),
            mcp_form_enabled: true,
            agent_mcp_server_ids: HashMap::new(),
            recent_sessions: Vec::new(),
            external_tools: HashMap::new(),
            is_loading_models: false,
            // Create input states
            mcp_name_input: cx.new(|cx| InputState::new(window, cx).placeholder("Server name")),
            mcp_command_input: cx.new(|cx| InputState::new(window, cx).placeholder("e.g. mcp-filesystem")),
            mcp_url_input: cx.new(|cx| InputState::new(window, cx).placeholder("http://localhost:8080")),
            api_key_input: cx.new(|cx| InputState::new(window, cx).placeholder("sk-...")),
            api_key_label_input: cx.new(|cx| InputState::new(window, cx).placeholder("Work Account")),
            provider_select: cx.new(|cx| {
                // Start with empty list - will be populated by refresh_provider_select
                SelectState::new(
                    SearchableVec::new(Vec::<ProviderSelectItem>::new()),
                    None,
                    window,
                    cx,
                ).searchable(true)
            }),
            model_select: cx.new(|cx| {
                // Start with empty list - will be populated by refresh_model_select
                SelectState::new(
                    SearchableVec::new(Vec::<ModelSelectItem>::new()),
                    None,
                    window,
                    cx,
                ).searchable(true)
            }),
        }
    }
}

impl SettingsState {
    /// Refresh OAuth accounts from ConfigService
    pub fn refresh_accounts(&mut self) {
        use ticca_core::config::models::providers;
        use ticca_core::config::ConfigService;
        
        self.accounts_claude = ConfigService::list_oauth_accounts_pruned(providers::CLAUDE)
            .unwrap_or_default();
        self.accounts_gemini = ConfigService::list_oauth_accounts_pruned(providers::GEMINI)
            .unwrap_or_default();
        self.accounts_chatgpt = ConfigService::list_oauth_accounts_pruned(providers::CHATGPT)
            .unwrap_or_default();
        self.refresh_api_key_accounts();
        self.refresh_auth_status();
    }
    
    /// Refresh API key accounts
    pub fn refresh_api_key_accounts(&mut self) {
        use ticca_core::config::ConfigService;
        use ticca_core::RegistryService;
        
        self.api_key_accounts.clear();
        for provider in RegistryService::api_key_providers() {
            let accounts = ConfigService::list_api_key_accounts(Some(&provider.id))
                .unwrap_or_default();
            if !accounts.is_empty() {
                self.api_key_accounts.insert(provider.id.clone(), accounts);
            }
        }
    }
    
    /// Refresh the provider select dropdown with unconfigured providers
    pub fn refresh_provider_select(&mut self, window: &mut Window, cx: &mut Context<TiccaApp>) {
        use ticca_core::RegistryService;
        
        // Get unconfigured providers (ones not in api_key_accounts)
        let unconfigured: Vec<ProviderSelectItem> = RegistryService::api_key_providers()
            .iter()
            .filter(|p| !self.api_key_accounts.contains_key(&p.id))
            .map(|p| ProviderSelectItem {
                id: p.id.clone(),
                name: p.name.clone(),
            })
            .collect();
        
        self.provider_select.update(cx, |state, cx| {
            state.set_items(SearchableVec::new(unconfigured), window, cx);
        });
    }
    
    /// Refresh authentication status
    pub fn refresh_auth_status(&mut self) {
        use ticca_core::config::models::providers;
        use ticca_core::llm::auth;
        
        self.provider_auth_status = ProviderAuthStatus {
            claude: auth::has_valid_account(providers::CLAUDE),
            gemini: auth::has_valid_account(providers::GEMINI),
            chatgpt: auth::has_valid_account(providers::CHATGPT),
        };
    }
    
    /// Refresh MCP servers
    pub fn refresh_mcp(&mut self) {
        use ticca_core::config::ConfigService;
        
        self.mcp_servers = ConfigService::list_mcp_servers().unwrap_or_default();
        
        let mut map = HashMap::new();
        for agent in [AgentType::Planning, AgentType::Coding] {
            let ids = ConfigService::get_agent_mcp_server_ids(agent.as_str())
                .unwrap_or_default();
            map.insert(agent, ids);
        }
        self.agent_mcp_server_ids = map;
    }
    
    /// Refresh recent sessions
    pub fn refresh_sessions(&mut self) {
        use ticca_core::session::SessionService;
        
        self.recent_sessions = SessionService::list_recent(10).unwrap_or_default();
    }
    
    /// Refresh external tools status
    pub fn refresh_external_tools(&mut self) {
        use ticca_core::external_tools::{ExternalToolManager, ToolStatus, get_all_tool_definitions};
        
        self.external_tools.clear();
        
        // Synchronously check tool status (in real impl, this would be async)
        for def in get_all_tool_definitions() {
            // For now, just show all as not installed - real impl needs async
            self.external_tools.insert(def.id, ExternalToolStatus {
                is_installed: false,
                version: None,
                is_installing: false,
                install_progress: 0,
                is_supported: true,
            });
        }
    }
    
    /// Load default model setting from database (sync)
    pub fn load_default_model(&mut self) {
        use ticca_core::config::{ConfigService, setting_keys};
        self.default_model = ConfigService::get_setting(setting_keys::DEFAULT_MODEL)
            .ok()
            .flatten();
    }
    
    /// Load cached models from database (sync, for fast initial load)
    pub fn load_cached_models(&mut self) {
        use ticca_core::config::ConfigService;
        if let Ok(models) = ConfigService::list_discovered_models(None) {
            self.available_models = models.into_iter()
                .map(|m| m.canonical_id)
                .collect();
        }
    }
    
    /// Load agent pinned models from database (sync)
    pub fn load_agent_pinned_models(&mut self) {
        use ticca_core::config::ConfigService;
        use ticca_core::agents::AgentType;
        
        self.agent_pinned_models.clear();
        if let Ok(snapshot) = ConfigService::load_settings_snapshot() {
            for (agent_str, model) in snapshot.agent_pinned_models {
                // Parse agent type from string
                let agent_type = match agent_str.as_str() {
                    "planning" => Some(AgentType::Planning),
                    "coding" => Some(AgentType::Coding),
                    "skills" => Some(AgentType::Skills),
                    "explore" => Some(AgentType::Explore),
                    _ => None,
                };
                if let Some(agent) = agent_type {
                    self.agent_pinned_models.insert(agent, model);
                }
            }
        }
    }
    
    /// Refresh the model select dropdown with available models
    pub fn refresh_model_select(&mut self, window: &mut Window, cx: &mut Context<TiccaApp>) {
        use ticca_core::config::ConfigService;
        
        // Build model items from available models
        let model_items: Vec<ModelSelectItem> = if let Ok(models) = ConfigService::list_discovered_models(None) {
            models.into_iter()
                .map(|m| ModelSelectItem {
                    id: m.canonical_id.clone(),
                    display_name: m.display_name.unwrap_or_else(|| m.model_id.clone()),
                    provider: m.provider,
                })
                .collect()
        } else {
            // Fallback to parsing available_models
            self.available_models.iter()
                .map(|id| {
                    let (provider, display) = ticca_core::llm::ModelId::parse(id)
                        .map(|m| (m.provider.to_string(), m.display_name()))
                        .unwrap_or_else(|| ("Unknown".to_string(), id.clone()));
                    ModelSelectItem {
                        id: id.clone(),
                        display_name: display,
                        provider,
                    }
                })
                .collect()
        };
        
        self.model_select.update(cx, |state, cx| {
            state.set_items(SearchableVec::new(model_items), window, cx);
        });
        
        // Set selected value if default_model is set
        if let Some(ref default) = self.default_model {
            self.model_select.update(cx, |state, cx| {
                state.set_selected_value(default, window, cx);
            });
        }
    }
    
    /// Clear MCP form (including input states)
    pub fn clear_mcp_form(&mut self, window: &mut Window, cx: &mut Context<TiccaApp>) {
        self.mcp_form_server_id = None;
        self.mcp_form_name.clear();
        self.mcp_form_transport = McpTransport::Stdio;
        self.mcp_form_command.clear();
        self.mcp_form_args_json = "[]".to_string();
        self.mcp_form_env_json = "{}".to_string();
        self.mcp_form_endpoint_url.clear();
        self.mcp_form_enabled = true;
        // Clear input states
        self.mcp_name_input.update(cx, |state, cx| state.set_value("", window, cx));
        self.mcp_command_input.update(cx, |state, cx| state.set_value("", window, cx));
        self.mcp_url_input.update(cx, |state, cx| state.set_value("", window, cx));
    }
    
    /// Load MCP server into form for editing
    pub fn edit_mcp_server(&mut self, server: &McpServer, window: &mut Window, cx: &mut Context<TiccaApp>) {
        // Clone values before updating state
        let name = server.name.clone();
        let command = server.command.clone().unwrap_or_default();
        let url = server.endpoint_url.clone().unwrap_or_default();
        
        self.mcp_form_server_id = Some(server.id.clone());
        self.mcp_form_name = name.clone();
        self.mcp_form_transport = server.transport;
        self.mcp_form_command = command.clone();
        self.mcp_form_args_json = serde_json::to_string(&server.args).unwrap_or_else(|_| "[]".to_string());
        self.mcp_form_env_json = serde_json::to_string(&server.env).unwrap_or_else(|_| "{}".to_string());
        self.mcp_form_endpoint_url = url.clone();
        self.mcp_form_enabled = server.is_enabled;
        
        // Set input state values
        self.mcp_name_input.update(cx, |state, cx| state.set_value(&name, window, cx));
        self.mcp_command_input.update(cx, |state, cx| state.set_value(&command, window, cx));
        self.mcp_url_input.update(cx, |state, cx| state.set_value(&url, window, cx));
    }
    
    /// Save MCP form to server (reads from input states)
    pub fn save_mcp_form(&mut self, window: &mut Window, cx: &mut Context<TiccaApp>) -> Result<(), String> {
        use ticca_core::config::ConfigService;
        
        // Read values from input states
        let name = self.mcp_name_input.read(cx).value().trim().to_string();
        let command = self.mcp_command_input.read(cx).value().trim().to_string();
        let url = self.mcp_url_input.read(cx).value().trim().to_string();
        
        if name.is_empty() {
            return Err("Server name is required".to_string());
        }
        
        let args: Vec<String> = if self.mcp_form_args_json.trim().is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&self.mcp_form_args_json)
                .map_err(|e| format!("Invalid args JSON: {}", e))?
        };
        
        let env: std::collections::BTreeMap<String, String> = if self.mcp_form_env_json.trim().is_empty() {
            std::collections::BTreeMap::new()
        } else {
            serde_json::from_str(&self.mcp_form_env_json)
                .map_err(|e| format!("Invalid env JSON: {}", e))?
        };
        
        let id = self.mcp_form_server_id.clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        
        let server = match self.mcp_form_transport {
            McpTransport::Stdio => {
                if command.is_empty() {
                    return Err("Command is required for stdio transport".to_string());
                }
                McpServer::new(id, name)
                    .with_stdio_command(command)
                    .with_args(args)
                    .with_env(env)
                    .set_enabled(self.mcp_form_enabled)
            }
            McpTransport::StreamableHttp => {
                if url.is_empty() {
                    return Err("Endpoint URL is required for HTTP transport".to_string());
                }
                McpServer::new(id, name)
                    .with_streamable_http(url)
                    .with_args(args)
                    .with_env(env)
                    .set_enabled(self.mcp_form_enabled)
            }
        };
        
        ConfigService::upsert_mcp_server(&server)
            .map_err(|e| format!("Failed to save: {}", e))?;
        
        self.clear_mcp_form(window, cx);
        self.refresh_mcp();
        Ok(())
    }
}
// =============================================================================
// Pending Tool Approval
// =============================================================================

/// A pending tool approval request from the runner
#[derive(Debug, Clone)]
pub struct PendingApproval {
    pub id: u64,
    pub name: String,
    pub args: String,
}

// =============================================================================
// Main Application State
// =============================================================================

/// Main application state
pub struct TiccaApp {
    /// Current view (Chat or Settings)
    pub current_view: View,
    /// Current theme
    pub theme: AppTheme,
    /// Chat feature state
    pub chat: ChatState,
    /// Settings feature state
    pub settings: SettingsState,
    /// Toast notification
    pub toast: Option<Toast>,
    /// Update available notification
    pub update_available: Option<UpdateAvailableState>,
    /// Pending tool approval (modal)
    pub pending_approval: Option<PendingApproval>,
}

impl TiccaApp {
    /// Create a new application instance
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let working_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Create input state for chat
        let input_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Type a message...")
        });

        // Subscribe to input events to handle Enter key
        cx.subscribe(&input_state, |_this, _input, event: &InputEvent, cx| {
            if let InputEvent::PressEnter { secondary: false } = event {
                // We need window access, but subscribe doesn't provide it
                // So we'll dispatch an action instead
                cx.dispatch_action(&SendMessage);
            }
        })
        .detach();

        let chat = ChatState {
            input_state,
            messages: Vec::new(),
            is_streaming: false,
            stream_handle: None,
            working_directory,
            current_agent: AgentType::Coding,
            pending_attachments: Vec::new(),
            flow_panel_visible: false,
            sidebar_tab: RightSidebarTab::TodoList,
            session_id: None,
            todo_state: None,
            agent_graph: AgentCallGraph::new(),
            system_executions: Vec::new(),
            next_execution_id: 1,
            cancel_tx: None,
            approval_tx: None,
            streaming_chars_per_sec: None,
            streaming_total_chars: 0,
            context_usage_percent: None,
            context_tokens_used: None,
            context_window_size: None,
            scroll_handle: ScrollHandle::new(),
            auto_scroll_enabled: true,
        };

        let mut settings = SettingsState::new(window, cx);
        
        // Load initial data from database
        settings.load_default_model();
        settings.load_cached_models();
        settings.load_agent_pinned_models();
        
        // Subscribe to provider select events to handle selection
        cx.subscribe(&settings.provider_select, |this, _state, event: &SelectEvent<SearchableVec<ProviderSelectItem>>, cx| {
            if let SelectEvent::Confirm(Some(provider_id)) = event {
                this.settings.api_key_form_provider = Some(provider_id.clone());
                cx.notify();
            }
        })
        .detach();
        
        // Subscribe to model select events to set default model
        cx.subscribe(&settings.model_select, |this, _state, event: &SelectEvent<SearchableVec<ModelSelectItem>>, cx| {
            if let SelectEvent::Confirm(Some(model_id)) = event {
                use ticca_core::config::{ConfigService, setting_keys};
                this.settings.default_model = Some(model_id.clone());
                let _ = ConfigService::set_setting(setting_keys::DEFAULT_MODEL, model_id);
                cx.notify();
            }
        })
        .detach();
        
        Self {
            current_view: View::Chat,
            theme: AppTheme::TiccaDark,
            chat,
            settings,
            toast: None,
            update_available: None,
            pending_approval: None,
        }
    }

    /// Handle sending a message
    pub fn handle_send_message(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        use ticca_core::session::MessageRole;
        use crate::chat_message::MessageId;

        let input_value = self.chat.input_state.read(cx).value();

        if input_value.trim().is_empty() {
            return;
        }

        let prompt = input_value.to_string();

        // Add user message
        self.chat.messages.push(ChatMessage {
            id: MessageId::new(),
            role: MessageRole::User,
            content: prompt.clone(),
            is_streaming: false,
            author_label: None,
            reasoning: None,
            reasoning_signature: None,
            content_blocks: vec![],
            last_was_tool_call: false,
        });

        // Clear input
        self.chat.input_state.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });

        // Add assistant message (streaming)
        let assistant_msg_id = MessageId::new();
        self.chat.messages.push(ChatMessage {
            id: assistant_msg_id,
            role: MessageRole::Assistant,
            content: String::new(),
            is_streaming: true,
            author_label: None,
            reasoning: None,
            reasoning_signature: None,
            content_blocks: vec![],
            last_was_tool_call: false,
        });

        self.chat.is_streaming = true;
        self.chat.streaming_chars_per_sec = None;
        self.chat.streaming_total_chars = 0;
        
        // Reset auto-scroll on new message and scroll to bottom
        self.chat.auto_scroll_enabled = true;
        self.chat.scroll_handle.scroll_to_bottom();
        
        cx.notify();

        // Spawn async streaming task
        self.start_llm_stream(prompt, assistant_msg_id, cx);
    }

    /// Start LLM streaming for the given prompt
    /// Uses the real ticca_core runner for actual LLM streaming.
    fn start_llm_stream(
        &mut self,
        prompt: String,
        msg_id: crate::chat_message::MessageId,
        cx: &mut Context<Self>,
    ) {
        use futures::StreamExt;
        use ticca_core::agents::run_agent_stream;
        use ticca_core::tools::SystemExecRequest;

        let working_dir = self.chat.working_directory.clone();
        let agent_type = self.chat.current_agent;
        let yolo_mode = self.settings.yolo_mode;
        let todo_state = self.chat.todo_state.clone();

        // Build chat history from existing messages (excluding the current streaming message)
        let chat_history: Vec<ChatHistoryMessage> = self
            .chat
            .messages
            .iter()
            .filter(|m| !m.is_streaming)
            .map(|m| ChatHistoryMessage {
                role: m.role.clone(),
                content: m.content.clone(),
                reasoning: m.reasoning.clone(),
                reasoning_signature: m.reasoning_signature.clone(),
            })
            .collect();

        // Get system prompt from agent profile
        let profile = AgentProfile::for_type(agent_type, 30);
        let system_prompt = profile.system_prompt.clone();

        // Create channels for approval flow
        let (approval_tx, approval_rx) = mpsc::unbounded_channel::<CoreToolApprovalDecision>();
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
        let system_exec_store = Arc::new(SystemExecStore::new());
        let (system_exec_tx, _system_exec_rx) = mpsc::unbounded_channel::<SystemExecRequest>();

        // Store senders for cancellation and approval
        self.chat.cancel_tx = Some(cancel_tx);
        self.chat.approval_tx = Some(approval_tx);

        // Start the agent graph for this run
        self.chat.agent_graph.start_run(agent_type);

        // Collect image data from attachments
        let image_data: Vec<(String, String)> = self
            .chat
            .pending_attachments
            .iter()
            .map(|att| {
                use base64::Engine;
                let base64_data = base64::engine::general_purpose::STANDARD
                    .encode(att.data.as_ref());
                let media_type = "image/png".to_string();
                (media_type, base64_data)
            })
            .collect();

        // Clear pending attachments
        self.chat.pending_attachments.clear();

        // Resolve the model to use:
        // 1. Check for agent-specific pinned model
        // 2. Fall back to default model setting
        // 3. None triggers Claude auto-detect (legacy behavior)
        let model_name = {
            use ticca_core::config::{ConfigDatabase, setting_keys};
            
            // First check for agent-specific pinned model
            let pinned = ConfigDatabase::open()
                .ok()
                .and_then(|db| db.get_agent_pinned_model(agent_type.as_str()).ok().flatten())
                .filter(|s| !s.trim().is_empty());
            
            // Then check default model setting
            pinned.or_else(|| {
                ConfigDatabase::open()
                    .ok()
                    .and_then(|db| db.get_setting(setting_keys::DEFAULT_MODEL).ok().flatten())
                    .map(|s| s.value)
                    .filter(|s| !s.trim().is_empty())
            })
        };
        
        tracing::info!("Using model: {:?}", model_name);

        // Create a channel to send events from the background task to the UI
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<RunnerEvent>();

        // Spawn the streaming task on the background executor
        // Note: We need a Tokio runtime because run_agent_stream uses tokio::spawn internally
        cx.background_executor().spawn(async move {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!("Failed to create Tokio runtime: {}", e);
                    let _ = event_tx.send(RunnerEvent::StreamError(
                        format!("Failed to create async runtime: {}", e)
                    ));
                    let _ = event_tx.send(RunnerEvent::StreamComplete);
                    return;
                }
            };
            
            rt.block_on(async {
                let stream = run_agent_stream(
                    system_prompt,
                    prompt,
                    model_name,
                    working_dir,
                    30, // max tool rounds
                    chat_history,
                    todo_state,
                    image_data,
                    yolo_mode,
                    agent_type,
                    approval_rx,
                    cancel_rx,
                    system_exec_store,
                    system_exec_tx,
                );

                tokio::pin!(stream);

                while let Some(event) = stream.next().await {
                    if event_tx.send(event).is_err() {
                        break;
                    }
                }
            });
        }).detach();

        // Spawn a UI task to receive events and update state
        cx.spawn(async move |this: gpui::WeakEntity<TiccaApp>, cx| {
            while let Some(event) = event_rx.recv().await {
                let is_terminal = matches!(
                    event,
                    RunnerEvent::StreamComplete | RunnerEvent::StreamError(_)
                );

                let update_result = this.update(cx, |app, cx| {
                    app.handle_runner_event(event, msg_id, cx);
                });

                if update_result.is_err() || is_terminal {
                    break;
                }
            }
        }).detach();
    }

    /// Handle a runner event and update the UI accordingly
    fn handle_runner_event(
        &mut self,
        event: RunnerEvent,
        msg_id: crate::chat_message::MessageId,
        cx: &mut Context<Self>,
    ) {
        use ticca_core::tools::{AgentCallEvent, AgentStreamEvent, TodoListEvent};

        match event {
            RunnerEvent::StreamChunk(text) => {
                if let Some(msg) = self.chat.messages.iter_mut().find(|m| m.id == msg_id) {
                    // Append to the current text block
                    let block = msg.current_text_block_mut();
                    block.push_str(&text);
                    msg.content.push_str(&text);
                    msg.last_was_tool_call = false;
                }
                // Auto-scroll to bottom if enabled
                if self.chat.auto_scroll_enabled {
                    self.chat.scroll_handle.scroll_to_bottom();
                }
                cx.notify();
            }
            RunnerEvent::Reasoning { text, signature } => {
                if let Some(msg) = self.chat.messages.iter_mut().find(|m| m.id == msg_id) {
                    let reasoning = msg.reasoning.get_or_insert_with(String::new);
                    reasoning.push_str(&text);
                    msg.reasoning_signature = signature;
                }
                cx.notify();
            }
            RunnerEvent::ToolCall { name, args } => {
                // Add to system executions list
                let id = self.chat.next_execution_id;
                self.chat.next_execution_id += 1;
                let mut exec = SystemExecution::new(id, format!("🔧 {}", name));
                let args_preview = if args.len() > 200 {
                    format!("{}...", &args[..200])
                } else {
                    args.clone()
                };
                exec.output = args_preview;
                self.chat.system_executions.push(exec);

                // Add a compact visual marker for the tool call
                if let Some(msg) = self.chat.messages.iter_mut().find(|m| m.id == msg_id) {
                    let display = format_tool_call_display(&name, &args);
                    let marker = format!("\n{}\n", display);
                    let block = msg.current_text_block_mut();
                    block.push_str(&marker);
                    msg.content.push_str(&marker);
                    msg.last_was_tool_call = true;
                }
                // Auto-scroll to bottom if enabled
                if self.chat.auto_scroll_enabled {
                    self.chat.scroll_handle.scroll_to_bottom();
                }
                cx.notify();
            }
            RunnerEvent::ToolApprovalRequested { id, name, args } => {
                // Show approval modal
                self.pending_approval = Some(PendingApproval { id, name, args });
                cx.notify();
            }
            RunnerEvent::ToolResult { name, success, result: _ } => {
                // Update the system execution with the result
                if let Some(exec) = self.chat.system_executions.iter_mut().rev().find(|e| e.command.contains(&name)) {
                    exec.status = if success { ExecutionStatus::Success } else { ExecutionStatus::Failed };
                }
                cx.notify();
            }
            RunnerEvent::ToolExecution { name, args } => {
                // Tool execution started - could add UI notification here
                tracing::debug!("Tool execution started: {} with args: {}", name, args);
            }
            RunnerEvent::TodoEvent(todo_event) => {
                match todo_event {
                    TodoListEvent::Reset { state, .. } => {
                        self.chat.todo_state = Some(state);
                    }
                    TodoListEvent::Updated { state, .. } => {
                        self.chat.todo_state = Some(state);
                    }
                }
                cx.notify();
            }
            RunnerEvent::AgentCall(agent_event) => {
                // AgentCallEvent is a struct, not an enum
                let AgentCallEvent {
                    parent_id,
                    child_id,
                    child,
                    prompt,
                    ..
                } = agent_event;
                self.chat.agent_graph.add_call(parent_id, child, &prompt);
                tracing::info!(
                    "Agent call: {} -> {} (node {})",
                    parent_id,
                    child.display_name(),
                    child_id
                );
                cx.notify();
            }
            RunnerEvent::SubagentStream(stream_event) => {
                use crate::chat_message::SubAgentMessage;
                match stream_event {
                    AgentStreamEvent::Start {
                        node_id,
                        agent_type,
                    } => {
                        if let Some(msg) =
                            self.chat.messages.iter_mut().find(|m| m.id == msg_id)
                        {
                            msg.add_sub_agent(SubAgentMessage::new_streaming(
                                node_id, agent_type,
                            ));
                        }
                    }
                    AgentStreamEvent::Chunk { node_id, text } => {
                        if let Some(msg) =
                            self.chat.messages.iter_mut().find(|m| m.id == msg_id)
                        {
                            if let Some(sub) = msg.sub_agent_mut(node_id) {
                                sub.content.push_str(&text);
                            }
                        }
                    }
                    AgentStreamEvent::Reasoning { node_id, text } => {
                        if let Some(msg) =
                            self.chat.messages.iter_mut().find(|m| m.id == msg_id)
                        {
                            if let Some(sub) = msg.sub_agent_mut(node_id) {
                                let r = sub.reasoning.get_or_insert_with(String::new);
                                r.push_str(&text);
                            }
                        }
                    }
                    AgentStreamEvent::ToolCall { node_id, name, .. } => {
                        if let Some(msg) =
                            self.chat.messages.iter_mut().find(|m| m.id == msg_id)
                        {
                            if let Some(sub) = msg.sub_agent_mut(node_id) {
                                if !sub.last_was_tool_call {
                                    sub.content
                                        .push_str(&format!("\n\n🔧 *Running {}...*\n\n", name));
                                }
                                sub.last_was_tool_call = true;
                            }
                        }
                    }
                    AgentStreamEvent::Complete { node_id, .. } => {
                        if let Some(msg) =
                            self.chat.messages.iter_mut().find(|m| m.id == msg_id)
                        {
                            if let Some(sub) = msg.sub_agent_mut(node_id) {
                                sub.is_streaming = false;
                            }
                        }
                    }
                }
                // Auto-scroll to bottom if enabled
                if self.chat.auto_scroll_enabled {
                    self.chat.scroll_handle.scroll_to_bottom();
                }
                cx.notify();
            }
            RunnerEvent::Usage {
                input_tokens,
                output_tokens,
            } => {
                tracing::debug!(
                    "Token usage: {} input, {} output",
                    input_tokens,
                    output_tokens
                );
            }
            RunnerEvent::StreamStats { chars_in_window, window_ms } => {
                // Calculate chars per second and update UI
                if window_ms > 0 {
                    let cps = (chars_in_window as f64) / (window_ms as f64 / 1000.0);
                    self.chat.streaming_chars_per_sec = Some(cps);
                    self.chat.streaming_total_chars += chars_in_window;
                    cx.notify();
                }
            }
            RunnerEvent::ContextEstimate { 
                usage_percent, 
                total_tokens, 
                context_window,
                .. 
            } => {
                self.chat.context_usage_percent = Some(usage_percent);
                self.chat.context_tokens_used = Some(total_tokens);
                self.chat.context_window_size = Some(context_window);
                tracing::debug!("Context usage: {}% ({}/{} tokens)", usage_percent, total_tokens, context_window);
                cx.notify();
            }
            RunnerEvent::ContextCompressed {
                original_tokens,
                compressed_tokens,
                strategy,
                ..
            } => {
                tracing::info!(
                    "Context compressed: {} -> {} tokens ({})",
                    original_tokens,
                    compressed_tokens,
                    strategy
                );
                // Add a system message about compression
                self.chat.messages.push(ChatMessage::system(format!(
                    "💾 Context compressed: {} → {} tokens",
                    original_tokens, compressed_tokens
                )));
            }
            RunnerEvent::ContextUsageWarning { usage_percent, .. } => {
                tracing::warn!("Context usage warning: {}%", usage_percent);
            }
            RunnerEvent::StreamComplete => {
                if let Some(msg) = self.chat.messages.iter_mut().find(|m| m.id == msg_id) {
                    msg.is_streaming = false;
                }
                self.chat.is_streaming = false;
                self.chat.cancel_tx = None;
                self.chat.approval_tx = None;
                // Mark all running agents as completed
                let running_ids: Vec<usize> = self
                    .chat
                    .agent_graph
                    .nodes
                    .iter()
                    .filter(|(_, n)| n.status == AgentStatus::Running)
                    .map(|(id, _)| *id)
                    .collect();
                for id in running_ids {
                    self.chat.agent_graph.mark_completed(id);
                }
                cx.notify();
            }
            RunnerEvent::StreamStopped => {
                if let Some(msg) = self.chat.messages.iter_mut().find(|m| m.id == msg_id) {
                    msg.is_streaming = false;
                    if !msg.content.is_empty() {
                        msg.content.push_str("\n\n*[Stopped by user]*");
                    } else {
                        msg.content = "*[Cancelled]*".to_string();
                    }
                }
                self.chat.is_streaming = false;
                self.chat.cancel_tx = None;
                self.chat.approval_tx = None;
                cx.notify();
            }
            RunnerEvent::StreamError(error) => {
                if let Some(msg) = self.chat.messages.iter_mut().find(|m| m.id == msg_id) {
                    msg.content = format!("❌ Error: {}", error);
                    msg.is_streaming = false;
                }
                self.chat.is_streaming = false;
                self.chat.cancel_tx = None;
                self.chat.approval_tx = None;
                tracing::error!("Stream error: {}", error);
                cx.notify();
            }
        }
    }

    // =========================================================================
    // Action Handlers
    // =========================================================================

    fn on_open_settings(&mut self, _: &OpenSettings, window: &mut Window, cx: &mut Context<Self>) {
        self.current_view = View::Settings;
        // Refresh all settings data
        self.settings.refresh_accounts();
        self.settings.refresh_mcp();
        self.settings.refresh_sessions();
        self.settings.refresh_provider_select(window, cx);
        // Refresh model data
        self.settings.load_default_model();
        self.settings.load_cached_models();
        self.settings.load_agent_pinned_models();
        self.settings.refresh_model_select(window, cx);
        cx.notify();
    }

    fn on_close_settings(&mut self, _: &CloseSettings, _: &mut Window, cx: &mut Context<Self>) {
        self.current_view = View::Chat;
        cx.notify();
    }

    fn on_switch_settings_tab(&mut self, action: &SwitchSettingsTab, _: &mut Window, cx: &mut Context<Self>) {
        self.settings.current_tab = action.0;
        cx.notify();
    }

    fn on_theme_toggle(&mut self, _: &ThemeToggle, _: &mut Window, cx: &mut Context<Self>) {
        self.theme = self.theme.next();
        // Dispatch the theme change to the global handler
        cx.dispatch_action(&SwitchTheme(self.theme.theme_name().to_string()));
        cx.notify();
    }

    fn on_switch_theme(&mut self, action: &SwitchTheme, _: &mut Window, cx: &mut Context<Self>) {
        // Find the theme by name and update local state
        use crate::theme::ALL_THEMES;
        for &t in ALL_THEMES {
            if t.theme_name().as_ref() == action.0 {
                self.theme = t;
                tracing::info!("Switched to {} theme", t.display_name());
                break;
            }
        }
        cx.notify();
    }

    fn on_new_session(&mut self, _: &NewSession, window: &mut Window, cx: &mut Context<Self>) {
        // Cancel any ongoing stream
        if let Some(cancel_tx) = self.chat.cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
        if let Some(handle) = &self.chat.stream_handle {
            handle.cancel();
        }
        self.chat.stream_handle = None;
        self.chat.approval_tx = None;

        // Clear input
        self.chat.input_state.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });
        self.chat.messages.clear();
        self.chat.pending_attachments.clear();
        self.chat.is_streaming = false;
        self.chat.session_id = None;
        self.chat.todo_state = None;
        self.chat.agent_graph.clear();
        self.chat.system_executions.clear();
        self.pending_approval = None;
        tracing::info!("New session started");
        cx.notify();
    }

    fn on_switch_agent(&mut self, action: &SwitchAgent, _: &mut Window, cx: &mut Context<Self>) {
        use ticca_core::agents::AgentType;
        let new_agent = match action.0.as_str() {
            "coding" => AgentType::Coding,
            "planning" => AgentType::Planning,
            "skills" => AgentType::Skills,
            "explore" => AgentType::Explore,
            _ => return,
        };
        self.chat.current_agent = new_agent;
        tracing::info!("Switched to {} agent", action.0);
        cx.notify();
    }

    fn on_send_message(&mut self, _: &SendMessage, window: &mut Window, cx: &mut Context<Self>) {
        self.handle_send_message(window, cx);
    }

    fn on_stop_streaming(&mut self, _: &StopStreaming, _: &mut Window, cx: &mut Context<Self>) {
        // Cancel via the runner's cancel channel (triggers StreamStopped event)
        if let Some(cancel_tx) = self.chat.cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
        // Also cancel via the old stream handle if present (for demo mode)
        if let Some(handle) = &self.chat.stream_handle {
            handle.cancel();
        }
        self.chat.stream_handle = None;

        tracing::info!("Streaming stop requested");
        cx.notify();
    }

    fn on_toggle_flow_panel(&mut self, _: &ToggleFlowPanel, _: &mut Window, cx: &mut Context<Self>) {
        self.chat.flow_panel_visible = !self.chat.flow_panel_visible;
        cx.notify();
    }

    fn on_select_sidebar_tab(&mut self, action: &SelectSidebarTab, _: &mut Window, cx: &mut Context<Self>) {
        self.chat.sidebar_tab = action.0;
        cx.notify();
    }

    fn on_dismiss_toast(&mut self, _: &DismissToast, _: &mut Window, cx: &mut Context<Self>) {
        self.toast = None;
        cx.notify();
    }

    fn on_open_url(&mut self, action: &OpenUrl, _: &mut Window, _: &mut Context<Self>) {
        if let Err(e) = open::that(&action.0) {
            tracing::error!("Failed to open URL {}: {}", action.0, e);
        }
    }

    // =========================================================================
    // Public API
    // =========================================================================

    /// Show a toast notification
    #[allow(dead_code)]
    pub fn show_toast(&mut self, message: impl Into<String>, cx: &mut Context<Self>) {
        self.toast = Some(Toast::new(message));
        cx.notify();
    }
}

// =============================================================================
// Render Implementation
// =============================================================================

impl Render for TiccaApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use crate::modals::render_approval_modal;

        // Render the main content based on current view
        let content: AnyElement = match self.current_view {
            View::Chat => views::render_chat_view(self, window, cx),
            View::Settings => views::render_settings_view(self, window, cx),
        };

        let theme = cx.theme();

        // Main container with action handlers attached
        // Each view (chat, settings) handles its own layout
        let mut root = div()
            .id("ticca-app-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            // Attach action handlers to the root element
            .on_action(cx.listener(Self::on_open_settings))
            .on_action(cx.listener(Self::on_close_settings))
            .on_action(cx.listener(Self::on_switch_settings_tab))
            .on_action(cx.listener(Self::on_theme_toggle))
            .on_action(cx.listener(Self::on_switch_theme))
            .on_action(cx.listener(Self::on_new_session))
            .on_action(cx.listener(Self::on_send_message))
            .on_action(cx.listener(Self::on_stop_streaming))
            .on_action(cx.listener(Self::on_toggle_flow_panel))
            .on_action(cx.listener(Self::on_select_sidebar_tab))
            .on_action(cx.listener(Self::on_dismiss_toast))
            .on_action(cx.listener(Self::on_open_url))
            .on_action(cx.listener(Self::on_switch_agent))
            .child(content);

        // Show approval modal overlay if there's a pending approval
        if let Some(ref approval) = self.pending_approval {
            root = root.child(render_approval_modal(approval, cx));
        }

        root
    }
}
