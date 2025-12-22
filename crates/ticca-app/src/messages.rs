//! Iced message types for the application

use std::sync::Arc;

pub mod chat;
pub mod settings;

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

/// Main application message type.
#[derive(Debug, Clone)]
pub enum Message {
    Chat(chat::Msg),
    Settings(settings::Msg),

    // Toast notifications
    /// Internal no-op for async command completions
    Noop,
    /// Dismiss the current toast notification
    DismissToast,
    /// Auto-dismiss tick for toast timeout
    ToastTick,
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
    Agents,
    McpServers,
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
