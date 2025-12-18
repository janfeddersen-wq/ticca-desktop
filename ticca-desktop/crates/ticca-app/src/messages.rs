//! Iced Message types for the application

use crate::theme::AppTheme;
use ticca_core::agents::AgentType;
use std::path::PathBuf;
use std::sync::Arc;

/// Image attachment data for sending to the LLM
#[derive(Debug, Clone)]
pub struct ImageAttachment {
    /// Raw image bytes (PNG format)
    pub data: Arc<Vec<u8>>,
    /// Original width
    pub width: u32,
    /// Original height
    pub height: u32,
    /// Optional filename (for dropped files)
    pub filename: Option<String>,
}

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
    WorkingDirectoryChanged(PathBuf),

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

    // Image attachments
    /// File dropped into window (drag & drop)
    FileDropped(PathBuf),
    /// Image loaded from dropped file
    ImageLoaded(Result<ImageAttachment, String>),
    /// Paste image from clipboard (Ctrl+V)
    PasteImage,
    /// Image pasted from clipboard
    ImagePasted(Result<ImageAttachment, String>),
    /// Remove attached image
    RemoveAttachment(usize),

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
