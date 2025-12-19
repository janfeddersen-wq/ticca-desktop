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

    /// Chat scroll position changed
    ChatScrolled(iced::widget::scrollable::Viewport),

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

    /// Stream statistics - emitted periodically during streaming
    /// Contains chars received in the last interval
    StreamStats { chars_in_window: usize, window_ms: u64 },

    /// Poll the byte counter for streaming stats (from iced subscription timer)
    PollStreamStats,

    /// Animation tick for smooth 60 FPS spinner animation
    AnimationTick,

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
