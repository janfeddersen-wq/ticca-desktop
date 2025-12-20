//! Iced Message types for the application

use crate::theme::AppTheme;
use std::path::PathBuf;
use std::sync::Arc;
use ticca_core::agents::AgentType;
use ticca_core::tools::{AgentCallEvent, AgentStreamEvent, SystemExecRequest, TodoListEvent};

/// Image attachment data for sending to the LLM
#[derive(Debug, Clone)]
pub struct ImageAttachment {
    /// Raw image bytes (PNG format)
    pub data: Arc<Vec<u8>>,
    /// Original width
    #[allow(dead_code)]
    pub width: u32,
    /// Original height
    #[allow(dead_code)]
    pub height: u32,
    /// Optional filename (for dropped files)
    #[allow(dead_code)]
    pub filename: Option<String>,
}

/// Main application message type
#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum Message {
    // Chat messages
    InputChanged(String),
    SendMessage,

    /// Chat scroll position changed
    ChatScrolled(iced::widget::scrollable::Viewport),

    /// Copy message content to clipboard
    CopyMessage(usize),

    /// Toggle between markdown and raw text view for a message
    ToggleRawView(usize),

    /// Handle text editor actions in raw view (for selection/copy)
    RawViewEditorAction(usize, iced::widget::text_editor::Action),

    /// Pane grid resize event
    PaneResized(iced::widget::pane_grid::ResizeEvent),

    /// Streaming chunk received from LLM
    StreamChunk(String),

    /// Streaming completed
    StreamComplete,

    /// Streaming stopped by user
    StreamStopped,

    /// Streaming error
    StreamError(String),

    /// Tool call started
    ToolCall {
        name: String,
        args: String,
    },

    /// Agent invocation event for the call graph
    AgentCall(AgentCallEvent),

    /// Streaming output from an invoked agent
    SubagentStream(AgentStreamEvent),

    /// To Do list events (agent-scoped)
    TodoEvent(TodoListEvent),

    /// Tool result received
    #[allow(dead_code)]
    ToolResult {
        name: String,
        result: String,
    },

    /// System execution request (from LLM tools)
    SystemExecRequest(SystemExecRequest),

    /// Terminal widget event (System Executions)
    SystemExecTerminalEvent(iced_term::Event),

    /// New terminal name changed
    SystemExecNewTerminalNameChanged(String),

    /// Create user terminal
    SystemExecCreateUserTerminal,

    /// Close terminal UI panel
    SystemExecCloseTerminal(String),

    /// Kill/terminate terminal process
    SystemExecKillTerminal(String),

    /// Copy current terminal output to clipboard
    SystemExecCopyTerminal(String),

    /// Approval required before running a protected tool
    ToolApprovalRequested {
        id: u64,
        name: String,
        args: String,
    },
    /// User decision for a tool approval
    ToolApprovalDecision {
        id: u64,
        approved: bool,
    },

    /// Reasoning/thinking content from the model
    Reasoning(String),

    /// Stream statistics - emitted periodically during streaming
    /// Contains chars received in the last interval
    StreamStats {
        chars_in_window: usize,
        window_ms: u64,
    },

    /// Poll the byte counter for streaming stats (from iced subscription timer)
    PollStreamStats,

    /// Animation tick for smooth 60 FPS spinner animation
    AnimationTick,

    /// Stop the active streaming request
    StopStreaming,

    // Navigation
    OpenSettings,
    CloseSettings,
    SwitchSettingsTab(SettingsTab),

    // Appearance
    ThemeToggle,
    SetTheme(AppTheme),
    SetYoloMode(bool),

    // OAuth accounts
    RemoveOAuthAccount(String),
    ToggleOAuthAccountActive {
        account_id: String,
        is_active: bool,
    },
    ResetOAuthCooldown(String),
    AdjustOAuthAccountPriority {
        account_id: String,
        delta: i64,
    },

    // Agent selection
    SwitchAgent(AgentType),

    // UI layout
    ToggleFlowPanel,
    SelectSidebarTab(RightSidebarTab),
    SelectTodoNode(TodoNodeOption),

    // Working directory
    SelectWorkingDirectory,
    WorkingDirectoryChanged(PathBuf),

    // OAuth
    StartOAuth(OAuthProvider),
    OAuthComplete(OAuthProvider, Result<(), String>),

    // Session
    NewSession,
    LoadSession(String),

    // Model selection
    RefreshModels,
    #[allow(dead_code)]
    RefreshModelsForProvider(OAuthProvider),
    ModelsLoaded(Result<Vec<String>, String>),
    SetDefaultModel(String),
    SetAgentModel(AgentType, Option<String>),

    // Image attachments
    /// Open file picker to select image (workaround for Wayland DnD)
    SelectImageFile,
    /// File dropped into window (drag & drop - X11 only)
    FileDropped(PathBuf),
    /// Image loaded from file (dropped or selected via dialog)
    ImageLoaded(Result<ImageAttachment, String>),
    /// Paste image from clipboard (Ctrl+V)
    PasteImage,
    /// Image pasted from clipboard
    ImagePasted(Result<ImageAttachment, String>),
    /// Remove attached image
    RemoveAttachment(usize),

    // Markdown
    /// Link clicked in markdown content
    LinkClicked(iced::widget::markdown::Uri),

    // Errors
    /// Internal no-op for async command completions
    Noop,
    DismissError,
}

/// Right sidebar tab selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightSidebarTab {
    AgentsFlow,
    TodoList,
    SystemExecutions,
}

/// Pick-list option for selecting an agent node's To Do list
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoNodeOption {
    pub node_id: usize,
    pub label: String,
}

impl std::fmt::Display for RightSidebarTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RightSidebarTab::AgentsFlow => write!(f, "Agents Flow"),
            RightSidebarTab::TodoList => write!(f, "To Do List"),
            RightSidebarTab::SystemExecutions => write!(f, "System Executions"),
        }
    }
}

impl std::fmt::Display for TodoNodeOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

/// OAuth provider selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthProvider {
    Claude,
    Gemini,
    ChatGpt,
}

/// Settings tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Accounts,
    Models,
    Tools,
    Appearance,
    Sessions,
}

impl std::fmt::Display for OAuthProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuthProvider::Claude => write!(f, "Claude"),
            OAuthProvider::Gemini => write!(f, "Gemini"),
            OAuthProvider::ChatGpt => write!(f, "ChatGPT"),
        }
    }
}
