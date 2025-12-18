//! Iced Message types for the application

use crate::theme::AppTheme;
use ticca_core::agents::AgentType;

/// Main application message type
#[derive(Debug, Clone)]
pub enum Message {
    // Chat messages
    InputChanged(String),
    SendMessage,

    /// Copy message content to clipboard
    CopyMessage(usize),

    /// Toggle between markdown and raw text view for a message
    ToggleRawView(usize),

    /// Handle text editor actions in raw view (for selection/copy)
    RawViewEditorAction(usize, iced::widget::text_editor::Action),

    /// Streaming chunk received from LLM
    StreamChunk(String),

    /// Streaming completed
    StreamComplete,

    /// Streaming error
    StreamError(String),

    /// Tool call started
    ToolCall { name: String, args: String },

    /// Tool result received
    ToolResult { name: String, result: String },

    /// Reasoning/thinking content from the model
    Reasoning(String),

    // Navigation
    OpenSettings,
    CloseSettings,

    // Appearance
    ThemeToggle,
    SetTheme(AppTheme),

    // Agent selection
    SwitchAgent(AgentType),

    // Working directory
    SelectWorkingDirectory,
    WorkingDirectoryChanged(std::path::PathBuf),

    // OAuth
    StartOAuth(OAuthProvider),
    OAuthComplete(OAuthProvider, Result<(), String>),

    // Session
    NewSession,
    LoadSession(String),

    // Model selection
    RefreshModels,
    ModelsLoaded(Result<Vec<String>, String>),
    SetDefaultModel(String),
    SetAgentModel(AgentType, Option<String>),

    // Errors
    DismissError,
}

/// OAuth provider selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthProvider {
    Claude,
    Gemini,
    ChatGpt,
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
