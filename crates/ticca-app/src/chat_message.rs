//! Chat message types for display

use iced::widget::markdown;
use ticca_core::session::MessageRole;
use uuid::Uuid;

/// Stable identifier for a chat message (survives index changes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MessageId(pub Uuid);

impl MessageId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parse from string (for session loading)
    pub fn from_string(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(Self)
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A chat message for display
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// Stable identifier (survives index changes)
    pub id: MessageId,
    pub role: MessageRole,
    pub content: String,
    pub is_streaming: bool,
    pub author_label: Option<String>,
    /// Reasoning/thinking content (collapsible)
    pub reasoning: Option<String>,
    /// Signature for reasoning content (required by Claude for verification)
    pub reasoning_signature: Option<String>,
    /// Parsed markdown items (cached for rendering)
    pub parsed_items: Vec<markdown::Item>,
    /// Track if last content added was a tool call (for formatting)
    pub last_was_tool_call: bool,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        let content = content.into();
        let parsed_items = markdown::parse(&content).collect();
        Self {
            id: MessageId::new(),
            role: MessageRole::User,
            content,
            is_streaming: false,
            author_label: None,
            reasoning: None,
            reasoning_signature: None,
            parsed_items,
            last_was_tool_call: false,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        let content = content.into();
        let parsed_items = markdown::parse(&content).collect();
        Self {
            id: MessageId::new(),
            role: MessageRole::Assistant,
            content,
            is_streaming: false,
            author_label: None,
            reasoning: None,
            reasoning_signature: None,
            parsed_items,
            last_was_tool_call: false,
        }
    }

    pub fn assistant_streaming() -> Self {
        Self {
            id: MessageId::new(),
            role: MessageRole::Assistant,
            content: String::new(),
            is_streaming: true,
            author_label: None,
            reasoning: None,
            reasoning_signature: None,
            parsed_items: Vec::new(),
            last_was_tool_call: false,
        }
    }

    pub fn assistant_streaming_named(label: impl Into<String>) -> Self {
        Self {
            id: MessageId::new(),
            role: MessageRole::Assistant,
            content: String::new(),
            is_streaming: true,
            author_label: Some(label.into()),
            reasoning: None,
            reasoning_signature: None,
            parsed_items: Vec::new(),
            last_was_tool_call: false,
        }
    }

    /// System message for notifications (compression, etc.)
    pub fn system(content: impl Into<String>) -> Self {
        let content = content.into();
        let parsed_items = markdown::parse(&content).collect();
        Self {
            id: MessageId::new(),
            role: MessageRole::System,
            content,
            is_streaming: false,
            author_label: None,
            reasoning: None,
            reasoning_signature: None,
            parsed_items,
            last_was_tool_call: false,
        }
    }

    /// Create message with a specific ID (for session loading)
    #[allow(dead_code)]
    pub fn with_id(mut self, id: MessageId) -> Self {
        self.id = id;
        self
    }

    /// Update parsed items when content changes
    pub fn update_parsed_items(&mut self) {
        self.parsed_items = markdown::parse(&self.content).collect();
    }
}
