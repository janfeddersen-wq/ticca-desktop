//! Chat interface component.
//!
//! Displays conversation messages with efficient streaming support,
//! markdown rendering, and smooth scrolling.

use chrono::{DateTime, Utc};
use gpui::{div, prelude::*, px, IntoElement, SharedString, Styled};

use crate::components::markdown::{render_markdown, MarkdownRenderer, RenderedContent};
use crate::components::scroll::SmoothScroll;
use crate::theme::Theme;

/// Message role in conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl MessageRole {
    /// Parse from string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "user" => Self::User,
            "assistant" => Self::Assistant,
            _ => Self::System,
        }
    }
}

/// Display representation of a tool call.
#[derive(Debug, Clone)]
pub struct ToolCallDisplay {
    /// Tool call ID
    pub id: String,
    /// Tool name
    pub name: String,
    /// Arguments (JSON string for display)
    pub arguments: String,
    /// Result (if available)
    pub result: Option<String>,
    /// Whether execution is complete
    pub is_complete: bool,
}

/// A chat message for display.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// Unique message ID
    pub id: String,
    /// Role of message sender
    pub role: MessageRole,
    /// Pre-rendered markdown content
    pub content: RenderedContent,
    /// Raw content for editing/copying
    pub raw_content: String,
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
    /// Associated tool calls (for assistant messages)
    pub tool_calls: Option<Vec<ToolCallDisplay>>,
    /// Whether this message is still streaming
    pub is_streaming: bool,
}

impl ChatMessage {
    /// Create a new user message.
    pub fn user(id: impl Into<String>, content: &str, renderer: &MarkdownRenderer) -> Self {
        Self {
            id: id.into(),
            role: MessageRole::User,
            content: renderer.render(content),
            raw_content: content.to_string(),
            timestamp: Utc::now(),
            tool_calls: None,
            is_streaming: false,
        }
    }

    /// Create a new assistant message.
    pub fn assistant(id: impl Into<String>, content: &str, renderer: &MarkdownRenderer) -> Self {
        Self {
            id: id.into(),
            role: MessageRole::Assistant,
            content: renderer.render(content),
            raw_content: content.to_string(),
            timestamp: Utc::now(),
            tool_calls: None,
            is_streaming: false,
        }
    }

    /// Create a streaming assistant message.
    pub fn assistant_streaming(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            role: MessageRole::Assistant,
            content: RenderedContent::empty(),
            raw_content: String::new(),
            timestamp: Utc::now(),
            tool_calls: None,
            is_streaming: true,
        }
    }

    /// Create a system message.
    pub fn system(id: impl Into<String>, content: &str, renderer: &MarkdownRenderer) -> Self {
        Self {
            id: id.into(),
            role: MessageRole::System,
            content: renderer.render(content),
            raw_content: content.to_string(),
            timestamp: Utc::now(),
            tool_calls: None,
            is_streaming: false,
        }
    }

    /// Update content (for streaming).
    pub fn set_content(&mut self, content: &str, renderer: &MarkdownRenderer) {
        self.raw_content = content.to_string();
        self.content = renderer.render(content);
    }

    /// Mark streaming as complete.
    pub fn finish_streaming(&mut self, renderer: &MarkdownRenderer) {
        self.is_streaming = false;
        // Re-render final content
        self.content = renderer.render(&self.raw_content);
    }
}

/// Chat view component.
///
/// Displays a scrollable list of messages with efficient
/// streaming support and auto-scroll behavior.
pub struct ChatView {
    /// All messages in the conversation
    messages: Vec<ChatMessage>,
    /// Smooth scrolling state
    scroll: SmoothScroll,
    /// Whether currently streaming a response
    is_streaming: bool,
    /// Content being streamed (not yet rendered)
    pending_content: String,
    /// Markdown renderer
    renderer: MarkdownRenderer,
    /// Current theme
    theme: Theme,
    /// Whether auto-scroll is enabled
    auto_scroll: bool,
    /// Viewport height for scroll calculations
    viewport_height: f32,
}

impl ChatView {
    /// Create a new chat view.
    pub fn new(theme: Theme) -> Self {
        Self {
            messages: Vec::new(),
            scroll: SmoothScroll::default(),
            is_streaming: false,
            pending_content: String::new(),
            renderer: MarkdownRenderer::new(),
            theme,
            auto_scroll: true,
            viewport_height: 600.0,
        }
    }

    /// Add a message to the chat.
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
        if self.auto_scroll {
            self.scroll.scroll_to_bottom();
        }
    }

    /// Add a user message.
    pub fn add_user_message(&mut self, id: impl Into<String>, content: &str) {
        let message = ChatMessage::user(id, content, &self.renderer);
        self.add_message(message);
    }

    /// Start streaming an assistant response.
    pub fn start_streaming(&mut self, message_id: impl Into<String>) {
        self.is_streaming = true;
        self.pending_content.clear();
        let message = ChatMessage::assistant_streaming(message_id);
        self.messages.push(message);
    }

    /// Append content to streaming message.
    ///
    /// Optimized to avoid re-rendering entire history.
    pub fn append_streaming_content(&mut self, delta: &str) {
        if !self.is_streaming {
            return;
        }

        self.pending_content.push_str(delta);

        // Update only the last message
        if let Some(last) = self.messages.last_mut() {
            if last.is_streaming {
                last.raw_content.push_str(delta);
                // Only re-render periodically to avoid performance issues
                // For real-time streaming, we might want to batch updates
                last.content = self.renderer.render(&last.raw_content);
            }
        }

        // Auto-scroll during streaming
        if self.auto_scroll {
            self.scroll.scroll_to_bottom();
        }
    }

    /// Finish streaming the current response.
    pub fn finish_streaming(&mut self) {
        if let Some(last) = self.messages.last_mut() {
            if last.is_streaming {
                last.finish_streaming(&self.renderer);
            }
        }
        self.is_streaming = false;
        self.pending_content.clear();
    }

    /// Get all messages.
    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    /// Clear all messages.
    pub fn clear(&mut self) {
        self.messages.clear();
        self.scroll.set_position(0.0);
    }

    /// Update theme.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    /// Get current theme.
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Handle scroll input.
    pub fn scroll_by(&mut self, delta: f32) {
        self.scroll.scroll_by(delta);
        // Disable auto-scroll if user scrolls up
        if delta > 0.0 && !self.scroll.is_at_bottom(50.0) {
            self.auto_scroll = false;
        }
        // Re-enable auto-scroll when user scrolls to bottom
        if self.scroll.is_at_bottom(10.0) {
            self.auto_scroll = true;
        }
    }

    /// Update scroll animation.
    pub fn update_scroll(&mut self) -> bool {
        self.scroll.update()
    }

    /// Set content dimensions for scroll.
    pub fn set_dimensions(&mut self, content_height: f32, viewport_height: f32) {
        self.viewport_height = viewport_height;
        self.scroll.set_dimensions(content_height, viewport_height);
    }

    /// Get scroll position.
    pub fn scroll_position(&self) -> f32 {
        self.scroll.position()
    }

    /// Check if streaming.
    pub fn is_streaming(&self) -> bool {
        self.is_streaming
    }
}

/// Render a single message.
pub fn render_message(message: &ChatMessage, theme: &Theme) -> impl IntoElement {
    let is_user = message.role == MessageRole::User;
    let is_system = message.role == MessageRole::System;

    let (bg_color, text_color, _align) = if is_user {
        (theme.user_message_bg, theme.user_message_text, "flex_end")
    } else if is_system {
        (theme.surface, theme.text_secondary, "flex_start")
    } else {
        (theme.assistant_message_bg, theme.assistant_message_text, "flex_start")
    };

    let role_label = match message.role {
        MessageRole::User => "You",
        MessageRole::Assistant => "Assistant",
        MessageRole::System => "System",
    };

    // Message container with proper alignment
    let mut message_row = div().flex().flex_row().w_full();

    if is_user {
        message_row = message_row.justify_end();
    } else {
        message_row = message_row.justify_start();
    }

    // Message bubble
    let _max_width = if is_user { "75%" } else { "85%" };

    let mut bubble = div()
        .flex()
        .flex_col()
        .max_w(px(800.0))
        .bg(bg_color)
        .rounded(px(12.0))
        .p(px(theme.spacing_md));

    // Role label (for non-user messages)
    if !is_user {
        bubble = bubble.child(
            div()
                .text_size(px(theme.font_size_small))
                .text_color(theme.text_secondary)
                .mb(px(theme.spacing_xs))
                .child(SharedString::from(role_label)),
        );
    }

    // Message content
    bubble = bubble.child(
        div()
            .text_color(text_color)
            .child(render_markdown(&message.content, theme)),
    );

    // Streaming indicator
    if message.is_streaming {
        bubble = bubble.child(
            div()
                .mt(px(theme.spacing_xs))
                .text_size(px(theme.font_size_small))
                .text_color(theme.text_secondary)
                .child("● Typing..."),
        );
    }

    // Tool calls display
    if let Some(tool_calls) = &message.tool_calls {
        for tool_call in tool_calls {
            bubble = bubble.child(render_tool_call(tool_call, theme));
        }
    }

    // Timestamp
    let time_str = message.timestamp.format("%H:%M").to_string();
    bubble = bubble.child(
        div()
            .mt(px(theme.spacing_xs))
            .text_size(px(theme.font_size_small))
            .text_color(if is_user {
                theme.user_message_text
            } else {
                theme.text_secondary
            })
            .child(SharedString::from(time_str)),
    );

    message_row.child(bubble)
}

/// Render a tool call display.
fn render_tool_call(tool_call: &ToolCallDisplay, theme: &Theme) -> impl IntoElement {
    div()
        .mt(px(theme.spacing_sm))
        .p(px(theme.spacing_sm))
        .bg(theme.surface)
        .rounded(px(6.0))
        .border_1()
        .border_color(theme.border)
        .flex()
        .flex_col()
        .gap(px(theme.spacing_xs))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(theme.spacing_xs))
                .child(
                    div()
                        .text_size(px(theme.font_size_small))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.accent)
                        .child(SharedString::from(format!("🛠️ {}", tool_call.name))),
                )
                .child(
                    div()
                        .text_size(px(theme.font_size_small - 2.0))
                        .text_color(theme.text_secondary)
                        .child(if tool_call.is_complete {
                            "✓"
                        } else {
                            "●"
                        }),
                ),
        )
        .when_some(tool_call.result.as_ref(), |el, result| {
            el.child(
                div()
                    .text_size(px(theme.font_size_code))
                    .text_color(theme.text_secondary)
                    .p(px(theme.spacing_xs))
                    .bg(theme.code_background)
                    .rounded(px(4.0))
                    .child(SharedString::from(truncate_str(result, 200))),
            )
        })
}

/// Truncate a string with ellipsis.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

/// Render the full chat view.
pub fn render_chat_view(chat: &ChatView) -> impl IntoElement {
    let theme = &chat.theme;

    let mut container = div()
        .flex()
        .flex_col()
        .size_full()
        .bg(theme.background)
        .p(px(theme.spacing_md))
        .gap(px(theme.spacing_md));

    // Empty state
    if chat.messages.is_empty() {
        container = container.child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .justify_center()
                .items_center()
                .child(
                    div()
                        .text_size(px(theme.font_size_xl))
                        .text_color(theme.text_secondary)
                        .child("Start a conversation"),
                )
                .child(
                    div()
                        .mt(px(theme.spacing_sm))
                        .text_size(px(theme.font_size_base))
                        .text_color(theme.text_secondary)
                        .child("Type a message below to begin."),
                ),
        );
    } else {
        // Message list
        for message in &chat.messages {
            container = container.child(render_message(message, theme));
        }
    }

    container
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_user_message() {
        let renderer = MarkdownRenderer::new();
        let msg = ChatMessage::user("msg-1", "Hello!", &renderer);
        assert_eq!(msg.id, "msg-1");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.raw_content, "Hello!");
        assert!(!msg.is_streaming);
    }

    #[test]
    fn test_create_assistant_message() {
        let renderer = MarkdownRenderer::new();
        let msg = ChatMessage::assistant("msg-2", "Hi there!", &renderer);
        assert_eq!(msg.role, MessageRole::Assistant);
    }

    #[test]
    fn test_streaming_workflow() {
        let theme = Theme::dark();
        let mut chat = ChatView::new(theme);

        // Add user message
        chat.add_user_message("msg-1", "Hello");
        assert_eq!(chat.messages.len(), 1);

        // Start streaming
        chat.start_streaming("msg-2");
        assert!(chat.is_streaming());
        assert_eq!(chat.messages.len(), 2);

        // Append content
        chat.append_streaming_content("Hi ");
        chat.append_streaming_content("there!");

        let last = chat.messages.last().expect("should have message");
        assert_eq!(last.raw_content, "Hi there!");

        // Finish streaming
        chat.finish_streaming();
        assert!(!chat.is_streaming());
    }

    #[test]
    fn test_clear_messages() {
        let theme = Theme::dark();
        let mut chat = ChatView::new(theme);

        chat.add_user_message("msg-1", "Hello");
        chat.add_user_message("msg-2", "World");
        assert_eq!(chat.messages.len(), 2);

        chat.clear();
        assert!(chat.messages.is_empty());
    }

    #[test]
    fn test_message_role_parsing() {
        assert_eq!(MessageRole::from_str("user"), MessageRole::User);
        assert_eq!(MessageRole::from_str("USER"), MessageRole::User);
        assert_eq!(MessageRole::from_str("assistant"), MessageRole::Assistant);
        assert_eq!(MessageRole::from_str("system"), MessageRole::System);
        assert_eq!(MessageRole::from_str("unknown"), MessageRole::System);
    }
}
