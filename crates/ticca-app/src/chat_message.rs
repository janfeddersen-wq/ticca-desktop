//! Chat message types for display

use iced::widget::markdown;
use ticca_core::session::MessageRole;

/// A chat message for display
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub is_streaming: bool,
    /// Reasoning/thinking content (collapsible)
    pub reasoning: Option<String>,
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
            role: MessageRole::User,
            content,
            is_streaming: false,
            reasoning: None,
            parsed_items,
            last_was_tool_call: false,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        let content = content.into();
        let parsed_items = markdown::parse(&content).collect();
        Self {
            role: MessageRole::Assistant,
            content,
            is_streaming: false,
            reasoning: None,
            parsed_items,
            last_was_tool_call: false,
        }
    }

    pub fn assistant_streaming() -> Self {
        Self {
            role: MessageRole::Assistant,
            content: String::new(),
            is_streaming: true,
            reasoning: None,
            parsed_items: Vec::new(),
            last_was_tool_call: false,
        }
    }

    /// Update parsed items when content changes
    pub fn update_parsed_items(&mut self) {
        self.parsed_items = markdown::parse(&self.content).collect();
    }
}
