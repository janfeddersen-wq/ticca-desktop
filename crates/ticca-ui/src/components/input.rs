//! Input bar component.
//!
//! Sticky bottom prompt bar with multi-line input, attachments,
//! model/agent selection, and send functionality.

use gpui::{div, prelude::*, px, SharedString, Styled};

use crate::theme::Theme;

/// Types of file attachments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentKind {
    /// Image file (PNG, JPEG, etc.)
    Image,
    /// Generic file
    File,
}

/// An attachment in the input.
#[derive(Debug, Clone)]
pub struct Attachment {
    /// Type of attachment
    pub kind: AttachmentKind,
    /// Raw file data
    pub data: Vec<u8>,
    /// File name
    pub name: String,
    /// Preview image data (for images)
    pub preview: Option<Vec<u8>>,
}

impl Attachment {
    /// Create an image attachment.
    pub fn image(name: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            kind: AttachmentKind::Image,
            preview: Some(data.clone()), // Use same data for preview
            data,
            name: name.into(),
        }
    }

    /// Create a file attachment.
    pub fn file(name: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            kind: AttachmentKind::File,
            data,
            name: name.into(),
            preview: None,
        }
    }

    /// Get file size in bytes.
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Format file size for display.
    pub fn formatted_size(&self) -> String {
        let bytes = self.size();
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        }
    }
}

/// Model option for the model selector.
#[derive(Debug, Clone)]
pub struct ModelOption {
    /// Model ID (e.g., "gpt-4", "claude-3-opus")
    pub id: String,
    /// Display name
    pub name: String,
    /// Provider name
    pub provider: String,
}

impl ModelOption {
    pub fn new(id: impl Into<String>, name: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            provider: provider.into(),
        }
    }
}

/// Agent option for the agent selector.
#[derive(Debug, Clone)]
pub struct AgentOption {
    /// Agent ID
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
}

impl AgentOption {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
        }
    }
}

/// Input bar state.
#[derive(Debug, Clone)]
pub struct InputBar {
    /// Current text input
    pub text_buffer: String,
    /// Attached files
    pub attachments: Vec<Attachment>,
    /// Selected model ID
    pub selected_model: String,
    /// Selected agent ID
    pub selected_agent: String,
    /// Whether currently sending
    pub is_sending: bool,
    /// Whether input is focused
    pub is_focused: bool,
    /// Available models
    pub available_models: Vec<ModelOption>,
    /// Available agents
    pub available_agents: Vec<AgentOption>,
    /// Whether model selector is open
    pub model_selector_open: bool,
    /// Whether agent selector is open
    pub agent_selector_open: bool,
    /// Placeholder text
    pub placeholder: String,
    /// Max input length (characters)
    pub max_length: usize,
}

impl Default for InputBar {
    fn default() -> Self {
        Self::new()
    }
}

impl InputBar {
    /// Create a new input bar with defaults.
    pub fn new() -> Self {
        Self {
            text_buffer: String::new(),
            attachments: Vec::new(),
            selected_model: "default".to_string(),
            selected_agent: "code-puppy".to_string(),
            is_sending: false,
            is_focused: false,
            available_models: Self::default_models(),
            available_agents: Self::default_agents(),
            model_selector_open: false,
            agent_selector_open: false,
            placeholder: "Type a message...".to_string(),
            max_length: 100_000,
        }
    }

    /// Default model options.
    fn default_models() -> Vec<ModelOption> {
        vec![
            ModelOption::new("gpt-4", "GPT-4", "OpenAI"),
            ModelOption::new("gpt-4-turbo", "GPT-4 Turbo", "OpenAI"),
            ModelOption::new("claude-3-opus", "Claude 3 Opus", "Anthropic"),
            ModelOption::new("claude-3-sonnet", "Claude 3 Sonnet", "Anthropic"),
            ModelOption::new("gemini-pro", "Gemini Pro", "Google"),
        ]
    }

    /// Default agent options.
    fn default_agents() -> Vec<AgentOption> {
        vec![
            AgentOption::new("code-puppy", "Code Puppy", "General coding assistant"),
            AgentOption::new("code-agent", "Code Agent", "Advanced code generation"),
            AgentOption::new("code-reviewer", "Code Reviewer", "Code review and feedback"),
            AgentOption::new("planner", "Planner", "Project planning and architecture"),
        ]
    }

    /// Set the text content.
    pub fn set_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        if text.len() <= self.max_length {
            self.text_buffer = text;
        }
    }

    /// Append text to the buffer.
    pub fn append_text(&mut self, text: &str) {
        if self.text_buffer.len() + text.len() <= self.max_length {
            self.text_buffer.push_str(text);
        }
    }

    /// Get current text.
    pub fn text(&self) -> &str {
        &self.text_buffer
    }

    /// Clear the input.
    pub fn clear(&mut self) {
        self.text_buffer.clear();
        self.attachments.clear();
    }

    /// Add an attachment.
    pub fn add_attachment(&mut self, attachment: Attachment) {
        self.attachments.push(attachment);
    }

    /// Remove attachment at index.
    pub fn remove_attachment(&mut self, index: usize) {
        if index < self.attachments.len() {
            self.attachments.remove(index);
        }
    }

    /// Check if input is empty.
    pub fn is_empty(&self) -> bool {
        self.text_buffer.trim().is_empty() && self.attachments.is_empty()
    }

    /// Check if can send.
    pub fn can_send(&self) -> bool {
        !self.is_empty() && !self.is_sending
    }

    /// Set selected model.
    pub fn set_model(&mut self, model_id: impl Into<String>) {
        self.selected_model = model_id.into();
        self.model_selector_open = false;
    }

    /// Set selected agent.
    pub fn set_agent(&mut self, agent_id: impl Into<String>) {
        self.selected_agent = agent_id.into();
        self.agent_selector_open = false;
    }

    /// Toggle model selector.
    pub fn toggle_model_selector(&mut self) {
        self.model_selector_open = !self.model_selector_open;
        self.agent_selector_open = false;
    }

    /// Toggle agent selector.
    pub fn toggle_agent_selector(&mut self) {
        self.agent_selector_open = !self.agent_selector_open;
        self.model_selector_open = false;
    }

    /// Close all dropdowns.
    pub fn close_dropdowns(&mut self) {
        self.model_selector_open = false;
        self.agent_selector_open = false;
    }

    /// Get selected model option.
    pub fn selected_model_option(&self) -> Option<&ModelOption> {
        self.available_models
            .iter()
            .find(|m| m.id == self.selected_model)
    }

    /// Get selected agent option.
    pub fn selected_agent_option(&self) -> Option<&AgentOption> {
        self.available_agents
            .iter()
            .find(|a| a.id == self.selected_agent)
    }

    /// Get character count.
    pub fn char_count(&self) -> usize {
        self.text_buffer.len()
    }

    /// Get line count.
    pub fn line_count(&self) -> usize {
        self.text_buffer.lines().count().max(1)
    }

    /// Start sending.
    pub fn start_sending(&mut self) {
        self.is_sending = true;
    }

    /// Finish sending.
    pub fn finish_sending(&mut self) {
        self.is_sending = false;
    }
}

/// Render the input bar.
pub fn render_input_bar(input: &InputBar, theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .w_full()
        .bg(theme.input_bar_bg)
        .border_t_1()
        .border_color(theme.border)
        .p(px(theme.spacing_md))
        // Attachments preview
        .when(!input.attachments.is_empty(), |el| {
            el.child(render_attachments(&input.attachments, theme))
        })
        // Main input row
        .child(
            div()
                .flex()
                .flex_row()
                .items_end()
                .gap(px(theme.spacing_sm))
                // Agent selector button
                .child(render_selector_button(
                    input
                        .selected_agent_option()
                        .map(|a| a.name.as_str())
                        .unwrap_or("Agent"),
                    input.agent_selector_open,
                    theme,
                ))
                // Model selector button
                .child(render_selector_button(
                    input
                        .selected_model_option()
                        .map(|m| m.name.as_str())
                        .unwrap_or("Model"),
                    input.model_selector_open,
                    theme,
                ))
                // Text input area
                .child(render_text_input(input, theme))
                // Attachment button
                .child(render_icon_button("📎", "Attach file", theme))
                // Send button
                .child(render_send_button(input, theme)),
        )
}

/// Render attachments preview.
fn render_attachments(attachments: &[Attachment], theme: &Theme) -> impl IntoElement {
    let mut row = div()
        .flex()
        .flex_row()
        .flex_wrap()
        .gap(px(theme.spacing_sm))
        .mb(px(theme.spacing_sm));

    for (idx, attachment) in attachments.iter().enumerate() {
        row = row.child(render_attachment_chip(attachment, idx, theme));
    }

    row
}

/// Render a single attachment chip.
fn render_attachment_chip(attachment: &Attachment, _index: usize, theme: &Theme) -> impl IntoElement {
    let icon = match attachment.kind {
        AttachmentKind::Image => "🖼️",
        AttachmentKind::File => "📄",
    };

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(theme.spacing_xs))
        .bg(theme.surface)
        .border_1()
        .border_color(theme.border)
        .rounded(px(6.0))
        .px(px(theme.spacing_sm))
        .py(px(theme.spacing_xs))
        .child(
            div()
                .text_size(px(theme.font_size_small))
                .child(SharedString::from(icon)),
        )
        .child(
            div()
                .text_size(px(theme.font_size_small))
                .text_color(theme.text_primary)
                .child(SharedString::from(truncate_filename(&attachment.name, 20))),
        )
        .child(
            div()
                .text_size(px(theme.font_size_small - 2.0))
                .text_color(theme.text_secondary)
                .child(SharedString::from(attachment.formatted_size())),
        )
        .child(
            div()
                .text_size(px(theme.font_size_small))
                .text_color(theme.text_secondary)
                .cursor_pointer()
                .child("×"),
        )
}

/// Render a selector button (agent or model).
fn render_selector_button(label: &str, is_open: bool, theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(theme.spacing_xs))
        .bg(if is_open {
            theme.surface_elevated
        } else {
            theme.surface
        })
        .border_1()
        .border_color(if is_open {
            theme.accent
        } else {
            theme.border
        })
        .rounded(px(6.0))
        .px(px(theme.spacing_sm))
        .py(px(theme.spacing_xs))
        .cursor_pointer()
        .child(
            div()
                .text_size(px(theme.font_size_small))
                .text_color(theme.text_primary)
                .child(SharedString::from(label.to_string())),
        )
        .child(
            div()
                .text_size(px(theme.font_size_small - 2.0))
                .text_color(theme.text_secondary)
                .child(if is_open { "▲" } else { "▼" }),
        )
}

/// Render the text input area.
fn render_text_input(input: &InputBar, theme: &Theme) -> impl IntoElement {
    let line_height = theme.font_size_base * theme.line_height;
    let min_height = line_height * 1.5;
    let max_height = line_height * 10.0; // Max 10 lines

    // Calculate dynamic height based on content
    let content_height = (input.line_count() as f32 * line_height).clamp(min_height, max_height);

    let display_text = if input.text_buffer.is_empty() {
        SharedString::from(input.placeholder.clone())
    } else {
        SharedString::from(input.text_buffer.clone())
    };

    let text_color = if input.text_buffer.is_empty() {
        theme.text_secondary
    } else {
        theme.text_primary
    };

    div()
        .flex_1()
        .min_h(px(min_height))
        .max_h(px(max_height))
        .h(px(content_height))
        .bg(theme.input_field_bg)
        .border_1()
        .border_color(if input.is_focused {
            theme.input_field_focus_border
        } else {
            theme.input_field_border
        })
        .rounded(px(8.0))
        .px(px(theme.spacing_md))
        .py(px(theme.spacing_sm))
        .cursor_text()
        .child(
            div()
                .text_size(px(theme.font_size_base))
                .text_color(text_color)
                .child(display_text),
        )
}

/// Render an icon button.
fn render_icon_button(icon: &str, _label: &str, theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .size(px(36.0))
        .bg(theme.surface)
        .border_1()
        .border_color(theme.border)
        .rounded(px(8.0))
        .cursor_pointer()
        .child(
            div()
                .text_size(px(theme.font_size_large))
                .child(SharedString::from(icon.to_string())),
        )
}

/// Render the send button.
fn render_send_button(input: &InputBar, theme: &Theme) -> impl IntoElement {
    let can_send = input.can_send();

    let bg = if can_send {
        theme.accent
    } else {
        theme.surface
    };

    let icon = if input.is_sending { "●" } else { "↑" };

    div()
        .flex()
        .items_center()
        .justify_center()
        .size(px(36.0))
        .bg(bg)
        .rounded(px(8.0))
        .cursor(if can_send {
            gpui::CursorStyle::PointingHand
        } else {
            gpui::CursorStyle::Arrow
        })
        .child(
            div()
                .text_size(px(theme.font_size_large))
                .text_color(if can_send {
                    theme.user_message_text
                } else {
                    theme.text_secondary
                })
                .font_weight(gpui::FontWeight::BOLD)
                .child(SharedString::from(icon.to_string())),
        )
}

/// Truncate a filename for display.
fn truncate_filename(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        return name.to_string();
    }

    // Try to preserve extension
    if let Some(dot_pos) = name.rfind('.') {
        let ext = &name[dot_pos..];
        let name_part = &name[..dot_pos];
        let available = max_len.saturating_sub(ext.len()).saturating_sub(3);
        if available > 0 && name_part.len() > available {
            return format!("{}...{}", &name_part[..available], ext);
        }
    }

    format!("{}...", &name[..max_len - 3])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_bar_creation() {
        let input = InputBar::new();
        assert!(input.text_buffer.is_empty());
        assert!(input.attachments.is_empty());
        assert!(!input.is_sending);
    }

    #[test]
    fn test_set_text() {
        let mut input = InputBar::new();
        input.set_text("Hello, world!");
        assert_eq!(input.text(), "Hello, world!");
    }

    #[test]
    fn test_append_text() {
        let mut input = InputBar::new();
        input.set_text("Hello");
        input.append_text(", world!");
        assert_eq!(input.text(), "Hello, world!");
    }

    #[test]
    fn test_can_send() {
        let mut input = InputBar::new();
        assert!(!input.can_send()); // Empty

        input.set_text("Hello");
        assert!(input.can_send());

        input.is_sending = true;
        assert!(!input.can_send()); // Already sending
    }

    #[test]
    fn test_attachments() {
        let mut input = InputBar::new();
        assert!(input.attachments.is_empty());

        input.add_attachment(Attachment::image("test.png", vec![1, 2, 3]));
        assert_eq!(input.attachments.len(), 1);

        input.remove_attachment(0);
        assert!(input.attachments.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut input = InputBar::new();
        input.set_text("Hello");
        input.add_attachment(Attachment::file("test.txt", vec![]));

        input.clear();
        assert!(input.text_buffer.is_empty());
        assert!(input.attachments.is_empty());
    }

    #[test]
    fn test_model_selection() {
        let mut input = InputBar::new();
        input.set_model("claude-3-opus");
        assert_eq!(input.selected_model, "claude-3-opus");
    }

    #[test]
    fn test_agent_selection() {
        let mut input = InputBar::new();
        input.set_agent("code-reviewer");
        assert_eq!(input.selected_agent, "code-reviewer");
    }

    #[test]
    fn test_attachment_size_formatting() {
        let small = Attachment::file("a.txt", vec![0; 100]);
        assert_eq!(small.formatted_size(), "100 B");

        let medium = Attachment::file("b.txt", vec![0; 2048]);
        assert_eq!(medium.formatted_size(), "2.0 KB");

        let large = Attachment::file("c.txt", vec![0; 1_500_000]);
        assert!(large.formatted_size().contains("MB"));
    }

    #[test]
    fn test_truncate_filename() {
        assert_eq!(truncate_filename("short.txt", 20), "short.txt");
        // Test truncation preserves extension
        let truncated = truncate_filename("very_long_filename_here.txt", 15);
        assert!(truncated.len() <= 15);
        assert!(truncated.ends_with(".txt"));
    }
}
