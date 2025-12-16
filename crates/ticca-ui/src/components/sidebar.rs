//! Sidebar component for conversation list and navigation.
//!
//! Displays a list of conversations with search, filtering,
//! and collapsible functionality.

use chrono::{DateTime, Utc};
use gpui::{div, prelude::*, px, Div, SharedString, Styled};

use crate::theme::Theme;

/// Preview information for a conversation.
#[derive(Debug, Clone)]
pub struct ConversationPreview {
    /// Unique conversation ID
    pub id: String,
    /// Conversation title
    pub title: String,
    /// Preview of the last message
    pub last_message: String,
    /// Timestamp of last activity
    pub timestamp: DateTime<Utc>,
    /// Number of messages in conversation
    pub message_count: usize,
    /// Agent used in this conversation
    pub agent_name: Option<String>,
}

impl ConversationPreview {
    /// Create a new conversation preview.
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        last_message: impl Into<String>,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            last_message: last_message.into(),
            timestamp,
            message_count: 0,
            agent_name: None,
        }
    }

    /// Set message count.
    pub fn with_message_count(mut self, count: usize) -> Self {
        self.message_count = count;
        self
    }

    /// Set agent name.
    pub fn with_agent(mut self, agent: impl Into<String>) -> Self {
        self.agent_name = Some(agent.into());
        self
    }

    /// Format timestamp for display.
    pub fn formatted_time(&self) -> String {
        let now = Utc::now();
        let diff = now.signed_duration_since(self.timestamp);

        if diff.num_minutes() < 1 {
            "Just now".to_string()
        } else if diff.num_hours() < 1 {
            format!("{}m ago", diff.num_minutes())
        } else if diff.num_days() < 1 {
            format!("{}h ago", diff.num_hours())
        } else if diff.num_days() < 7 {
            format!("{}d ago", diff.num_days())
        } else {
            self.timestamp.format("%b %d").to_string()
        }
    }
}

/// Sidebar state.
#[derive(Debug, Clone)]
pub struct Sidebar {
    /// List of conversation previews
    pub conversations: Vec<ConversationPreview>,
    /// Currently selected conversation ID
    pub selected: Option<String>,
    /// Whether sidebar is collapsed
    pub is_collapsed: bool,
    /// Search/filter query
    pub search_query: String,
    /// Whether search is focused
    pub search_focused: bool,
    /// Hover state for items
    pub hovered_item: Option<String>,
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

impl Sidebar {
    /// Create a new sidebar.
    pub fn new() -> Self {
        Self {
            conversations: Vec::new(),
            selected: None,
            is_collapsed: false,
            search_query: String::new(),
            search_focused: false,
            hovered_item: None,
        }
    }

    /// Set conversations.
    pub fn set_conversations(&mut self, conversations: Vec<ConversationPreview>) {
        self.conversations = conversations;
    }

    /// Add a conversation.
    pub fn add_conversation(&mut self, conversation: ConversationPreview) {
        // Insert at the beginning (most recent)
        self.conversations.insert(0, conversation);
    }

    /// Remove a conversation by ID.
    pub fn remove_conversation(&mut self, id: &str) {
        self.conversations.retain(|c| c.id != id);
        if self.selected.as_deref() == Some(id) {
            self.selected = None;
        }
    }

    /// Update a conversation's preview.
    pub fn update_conversation(
        &mut self,
        id: &str,
        title: Option<&str>,
        last_message: Option<&str>,
        message_count: Option<usize>,
    ) {
        if let Some(conv) = self.conversations.iter_mut().find(|c| c.id == id) {
            if let Some(t) = title {
                conv.title = t.to_string();
            }
            if let Some(msg) = last_message {
                conv.last_message = msg.to_string();
                conv.timestamp = Utc::now();
            }
            if let Some(count) = message_count {
                conv.message_count = count;
            }
        }
    }

    /// Select a conversation.
    pub fn select(&mut self, id: impl Into<String>) {
        self.selected = Some(id.into());
    }

    /// Deselect current conversation.
    pub fn deselect(&mut self) {
        self.selected = None;
    }

    /// Toggle collapsed state.
    pub fn toggle_collapsed(&mut self) {
        self.is_collapsed = !self.is_collapsed;
    }

    /// Set search query.
    pub fn set_search(&mut self, query: impl Into<String>) {
        self.search_query = query.into();
    }

    /// Clear search.
    pub fn clear_search(&mut self) {
        self.search_query.clear();
    }

    /// Get filtered conversations.
    pub fn filtered_conversations(&self) -> Vec<&ConversationPreview> {
        if self.search_query.is_empty() {
            self.conversations.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.conversations
                .iter()
                .filter(|c| {
                    c.title.to_lowercase().contains(&query)
                        || c.last_message.to_lowercase().contains(&query)
                })
                .collect()
        }
    }

    /// Check if a conversation is selected.
    pub fn is_selected(&self, id: &str) -> bool {
        self.selected.as_deref() == Some(id)
    }

    /// Get conversation count.
    pub fn conversation_count(&self) -> usize {
        self.conversations.len()
    }

    /// Set hovered item.
    pub fn set_hovered(&mut self, id: Option<String>) {
        self.hovered_item = id;
    }
}

/// Render the sidebar.
pub fn render_sidebar(sidebar: &Sidebar, theme: &Theme) -> Div {
    if sidebar.is_collapsed {
        return render_collapsed_sidebar(theme);
    }

    div()
        .flex()
        .flex_col()
        .w(px(theme.sidebar_width))
        .h_full()
        .bg(theme.sidebar_bg)
        .border_r_1()
        .border_color(theme.border)
        // Header
        .child(render_sidebar_header(sidebar, theme))
        // Search
        .child(render_search_bar(sidebar, theme))
        // Conversation list
        .child(render_conversation_list(sidebar, theme))
        // Footer with new chat button
        .child(render_sidebar_footer(theme))
}

/// Render collapsed sidebar.
fn render_collapsed_sidebar(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .w(px(56.0))
        .h_full()
        .bg(theme.sidebar_bg)
        .border_r_1()
        .border_color(theme.border)
        .items_center()
        .pt(px(theme.spacing_md))
        .gap(px(theme.spacing_sm))
        // Expand button
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(40.0))
                .rounded(px(8.0))
                .cursor_pointer()
                .child(
                    div()
                        .text_size(px(theme.font_size_large))
                        .text_color(theme.text_secondary)
                        .child("≡"),
                ),
        )
        // New chat button
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(40.0))
                .bg(theme.accent)
                .rounded(px(8.0))
                .cursor_pointer()
                .child(
                    div()
                        .text_size(px(theme.font_size_large))
                        .text_color(theme.user_message_text)
                        .child("+"),
                ),
        )
}

/// Render sidebar header.
fn render_sidebar_header(_sidebar: &Sidebar, theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .w_full()
        .px(px(theme.spacing_md))
        .py(px(theme.spacing_sm))
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .text_size(px(theme.font_size_large))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .child("Conversations"),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(theme.spacing_xs))
                // Collapse button
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(28.0))
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .child(
                            div()
                                .text_size(px(theme.font_size_base))
                                .text_color(theme.text_secondary)
                                .child("≡"),
                        ),
                ),
        )
}

/// Render search bar.
fn render_search_bar(sidebar: &Sidebar, theme: &Theme) -> impl IntoElement {
    div()
        .w_full()
        .px(px(theme.spacing_md))
        .py(px(theme.spacing_sm))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .w_full()
                .bg(theme.input_field_bg)
                .border_1()
                .border_color(if sidebar.search_focused {
                    theme.input_field_focus_border
                } else {
                    theme.input_field_border
                })
                .rounded(px(6.0))
                .px(px(theme.spacing_sm))
                .py(px(theme.spacing_xs))
                .child(
                    div()
                        .text_size(px(theme.font_size_small))
                        .text_color(theme.text_secondary)
                        .mr(px(theme.spacing_xs))
                        .child("🔍"),
                )
                .child(
                    div()
                        .flex_1()
                        .text_size(px(theme.font_size_small))
                        .text_color(if sidebar.search_query.is_empty() {
                            theme.text_secondary
                        } else {
                            theme.text_primary
                        })
                        .child(SharedString::from(
                            if sidebar.search_query.is_empty() {
                                "Search conversations...".to_string()
                            } else {
                                sidebar.search_query.clone()
                            },
                        )),
                ),
        )
}

/// Render conversation list.
fn render_conversation_list(sidebar: &Sidebar, theme: &Theme) -> impl IntoElement {
    let filtered = sidebar.filtered_conversations();

    let mut list = div()
        .flex()
        .flex_col()
        .flex_1()
        .px(px(theme.spacing_sm));

    if filtered.is_empty() {
        list = list.child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .py(px(theme.spacing_xl))
                .child(
                    div()
                        .text_size(px(theme.font_size_base))
                        .text_color(theme.text_secondary)
                        .child(if sidebar.search_query.is_empty() {
                            "No conversations yet"
                        } else {
                            "No matching conversations"
                        }),
                ),
        );
    } else {
        for conv in filtered {
            list = list.child(render_conversation_item(conv, sidebar.is_selected(&conv.id), theme));
        }
    }

    list
}

/// Render a single conversation item.
fn render_conversation_item(
    conv: &ConversationPreview,
    is_selected: bool,
    theme: &Theme,
) -> impl IntoElement {
    let bg = if is_selected {
        theme.sidebar_item_selected
    } else {
        theme.sidebar_bg
    };

    div()
        .flex()
        .flex_col()
        .w_full()
        .bg(bg)
        .rounded(px(8.0))
        .p(px(theme.spacing_sm))
        .mb(px(theme.spacing_xs))
        .cursor_pointer()
        // Title row
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex_1()
                        .text_size(px(theme.font_size_base))
                        .font_weight(if is_selected {
                            gpui::FontWeight::SEMIBOLD
                        } else {
                            gpui::FontWeight::NORMAL
                        })
                        .text_color(theme.text_primary)
                        .overflow_x_hidden()
                        .child(SharedString::from(truncate_str(&conv.title, 25))),
                )
                .child(
                    div()
                        .text_size(px(theme.font_size_small - 2.0))
                        .text_color(theme.text_secondary)
                        .child(SharedString::from(conv.formatted_time())),
                ),
        )
        // Preview row
        .child(
            div()
                .mt(px(theme.spacing_xs))
                .text_size(px(theme.font_size_small))
                .text_color(theme.text_secondary)
                .overflow_x_hidden()
                .child(SharedString::from(truncate_str(&conv.last_message, 50))),
        )
        // Metadata row
        .when(conv.message_count > 0 || conv.agent_name.is_some(), |el| {
            el.child(
                div()
                    .mt(px(theme.spacing_xs))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(theme.spacing_sm))
                    .when_some(conv.agent_name.as_ref(), |el, agent| {
                        el.child(
                            div()
                                .text_size(px(theme.font_size_small - 2.0))
                                .text_color(theme.accent)
                                .child(SharedString::from(agent.clone())),
                        )
                    })
                    .when(conv.message_count > 0, |el| {
                        el.child(
                            div()
                                .text_size(px(theme.font_size_small - 2.0))
                                .text_color(theme.text_secondary)
                                .child(SharedString::from(format!(
                                    "{} messages",
                                    conv.message_count
                                ))),
                        )
                    }),
            )
        })
}

/// Render sidebar footer.
fn render_sidebar_footer(theme: &Theme) -> impl IntoElement {
    div()
        .w_full()
        .p(px(theme.spacing_md))
        .border_t_1()
        .border_color(theme.border)
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_center()
                .w_full()
                .bg(theme.accent)
                .rounded(px(8.0))
                .py(px(theme.spacing_sm))
                .cursor_pointer()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(theme.spacing_xs))
                        .child(
                            div()
                                .text_size(px(theme.font_size_base))
                                .text_color(theme.user_message_text)
                                .child("+"),
                        )
                        .child(
                            div()
                                .text_size(px(theme.font_size_base))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.user_message_text)
                                .child("New Chat"),
                        ),
                ),
        )
}

/// Truncate a string with ellipsis.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidebar_creation() {
        let sidebar = Sidebar::new();
        assert!(sidebar.conversations.is_empty());
        assert!(sidebar.selected.is_none());
        assert!(!sidebar.is_collapsed);
    }

    #[test]
    fn test_add_conversation() {
        let mut sidebar = Sidebar::new();
        let conv = ConversationPreview::new("conv-1", "Test", "Hello", Utc::now());
        sidebar.add_conversation(conv);
        assert_eq!(sidebar.conversations.len(), 1);
    }

    #[test]
    fn test_remove_conversation() {
        let mut sidebar = Sidebar::new();
        sidebar.add_conversation(ConversationPreview::new("conv-1", "Test", "Hello", Utc::now()));
        sidebar.select("conv-1");
        assert!(sidebar.selected.is_some());

        sidebar.remove_conversation("conv-1");
        assert!(sidebar.conversations.is_empty());
        assert!(sidebar.selected.is_none()); // Should deselect
    }

    #[test]
    fn test_search_filter() {
        let mut sidebar = Sidebar::new();
        sidebar.add_conversation(ConversationPreview::new(
            "conv-1",
            "Rust Help",
            "How do I use iterators?",
            Utc::now(),
        ));
        sidebar.add_conversation(ConversationPreview::new(
            "conv-2",
            "Python Tips",
            "Best practices",
            Utc::now(),
        ));

        assert_eq!(sidebar.filtered_conversations().len(), 2);

        sidebar.set_search("rust");
        assert_eq!(sidebar.filtered_conversations().len(), 1);

        sidebar.clear_search();
        assert_eq!(sidebar.filtered_conversations().len(), 2);
    }

    #[test]
    fn test_selection() {
        let mut sidebar = Sidebar::new();
        sidebar.add_conversation(ConversationPreview::new("conv-1", "Test", "Hello", Utc::now()));

        sidebar.select("conv-1");
        assert!(sidebar.is_selected("conv-1"));
        assert!(!sidebar.is_selected("conv-2"));

        sidebar.deselect();
        assert!(!sidebar.is_selected("conv-1"));
    }

    #[test]
    fn test_toggle_collapsed() {
        let mut sidebar = Sidebar::new();
        assert!(!sidebar.is_collapsed);

        sidebar.toggle_collapsed();
        assert!(sidebar.is_collapsed);

        sidebar.toggle_collapsed();
        assert!(!sidebar.is_collapsed);
    }

    #[test]
    fn test_formatted_time() {
        use chrono::Duration;

        let now = Utc::now();

        let recent = ConversationPreview::new("1", "T", "M", now);
        assert_eq!(recent.formatted_time(), "Just now");

        let minutes_ago = ConversationPreview::new(
            "2",
            "T",
            "M",
            now - Duration::try_minutes(30).unwrap_or_default(),
        );
        assert!(minutes_ago.formatted_time().contains("m ago"));

        let hours_ago = ConversationPreview::new(
            "3",
            "T",
            "M",
            now - Duration::try_hours(5).unwrap_or_default(),
        );
        assert!(hours_ago.formatted_time().contains("h ago"));
    }
}
