//! Toolbar component.
//!
//! Provides main application toolbar with navigation,
//! settings access, and status display.

use gpui::{div, prelude::*, px, SharedString, Styled};

use crate::theme::Theme;

/// Connection status indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Connected and ready
    Connected,
    /// Connecting...
    Connecting,
    /// Disconnected/offline
    Disconnected,
    /// Error state
    Error,
}

impl ConnectionStatus {
    /// Get status text.
    pub fn text(&self) -> &'static str {
        match self {
            Self::Connected => "Connected",
            Self::Connecting => "Connecting...",
            Self::Disconnected => "Disconnected",
            Self::Error => "Error",
        }
    }

    /// Get status icon.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Connected => "●",
            Self::Connecting => "○",
            Self::Disconnected => "○",
            Self::Error => "✖",
        }
    }
}

/// Toolbar state.
#[derive(Debug, Clone)]
pub struct Toolbar {
    /// Current conversation title
    pub title: String,
    /// Connection status
    pub status: ConnectionStatus,
    /// Current agent name
    pub agent_name: String,
    /// Current model name
    pub model_name: String,
    /// Whether settings panel is open
    pub settings_open: bool,
    /// Whether in fullscreen mode
    pub is_fullscreen: bool,
}

impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}

impl Toolbar {
    /// Create a new toolbar.
    pub fn new() -> Self {
        Self {
            title: "New Conversation".to_string(),
            status: ConnectionStatus::Connected,
            agent_name: "Code Puppy".to_string(),
            model_name: "GPT-4".to_string(),
            settings_open: false,
            is_fullscreen: false,
        }
    }

    /// Set the conversation title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    /// Set connection status.
    pub fn set_status(&mut self, status: ConnectionStatus) {
        self.status = status;
    }

    /// Set current agent name.
    pub fn set_agent(&mut self, name: impl Into<String>) {
        self.agent_name = name.into();
    }

    /// Set current model name.
    pub fn set_model(&mut self, name: impl Into<String>) {
        self.model_name = name.into();
    }

    /// Toggle settings panel.
    pub fn toggle_settings(&mut self) {
        self.settings_open = !self.settings_open;
    }

    /// Toggle fullscreen mode.
    pub fn toggle_fullscreen(&mut self) {
        self.is_fullscreen = !self.is_fullscreen;
    }
}

/// Render the toolbar.
pub fn render_toolbar(toolbar: &Toolbar, theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .w_full()
        .h(px(48.0))
        .bg(theme.surface)
        .border_b_1()
        .border_color(theme.border)
        .px(px(theme.spacing_md))
        // Left section: Sidebar toggle + Title
        .child(render_left_section(toolbar, theme))
        // Center section: Status
        .child(render_center_section(toolbar, theme))
        // Right section: Actions
        .child(render_right_section(toolbar, theme))
}

/// Render left section of toolbar.
fn render_left_section(toolbar: &Toolbar, theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(theme.spacing_md))
        // Sidebar toggle (hamburger)
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(32.0))
                .rounded(px(6.0))
                .cursor_pointer()
                .child(
                    div()
                        .text_size(px(theme.font_size_large))
                        .text_color(theme.text_secondary)
                        .child("≡"),
                ),
        )
        // Conversation title
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(theme.font_size_base))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text_primary)
                        .child(SharedString::from(truncate_str(&toolbar.title, 40))),
                )
                .child(
                    div()
                        .text_size(px(theme.font_size_small - 2.0))
                        .text_color(theme.text_secondary)
                        .child(SharedString::from(format!(
                            "{} • {}",
                            toolbar.agent_name, toolbar.model_name
                        ))),
                ),
        )
}

/// Render center section of toolbar.
fn render_center_section(toolbar: &Toolbar, theme: &Theme) -> impl IntoElement {
    let status_color = match toolbar.status {
        ConnectionStatus::Connected => theme.success,
        ConnectionStatus::Connecting => theme.warning,
        ConnectionStatus::Disconnected => theme.text_secondary,
        ConnectionStatus::Error => theme.error,
    };

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(theme.spacing_xs))
        .child(
            div()
                .text_size(px(theme.font_size_small))
                .text_color(status_color)
                .child(SharedString::from(toolbar.status.icon().to_string())),
        )
        .child(
            div()
                .text_size(px(theme.font_size_small))
                .text_color(theme.text_secondary)
                .child(SharedString::from(toolbar.status.text().to_string())),
        )
}

/// Render right section of toolbar.
fn render_right_section(toolbar: &Toolbar, theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(theme.spacing_sm))
        // Export button
        .child(render_toolbar_button("💾", "Export", theme))
        // Share button
        .child(render_toolbar_button("🔗", "Share", theme))
        // Settings button
        .child(render_toolbar_button(
            "⚙️",
            "Settings",
            theme,
        ))
        // Fullscreen toggle
        .child(render_toolbar_button(
            if toolbar.is_fullscreen { "⇱" } else { "⇲" },
            "Fullscreen",
            theme,
        ))
}

/// Render a toolbar button.
fn render_toolbar_button(icon: &str, _label: &str, theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .size(px(32.0))
        .rounded(px(6.0))
        .cursor_pointer()
        .child(
            div()
                .text_size(px(theme.font_size_base))
                .text_color(theme.text_secondary)
                .child(SharedString::from(icon.to_string())),
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
    fn test_toolbar_creation() {
        let toolbar = Toolbar::new();
        assert_eq!(toolbar.title, "New Conversation");
        assert_eq!(toolbar.status, ConnectionStatus::Connected);
        assert!(!toolbar.settings_open);
    }

    #[test]
    fn test_set_title() {
        let mut toolbar = Toolbar::new();
        toolbar.set_title("My Chat");
        assert_eq!(toolbar.title, "My Chat");
    }

    #[test]
    fn test_connection_status() {
        let mut toolbar = Toolbar::new();
        
        toolbar.set_status(ConnectionStatus::Connecting);
        assert_eq!(toolbar.status, ConnectionStatus::Connecting);
        assert_eq!(toolbar.status.text(), "Connecting...");

        toolbar.set_status(ConnectionStatus::Error);
        assert_eq!(toolbar.status.icon(), "✖");
    }

    #[test]
    fn test_toggle_settings() {
        let mut toolbar = Toolbar::new();
        assert!(!toolbar.settings_open);

        toolbar.toggle_settings();
        assert!(toolbar.settings_open);

        toolbar.toggle_settings();
        assert!(!toolbar.settings_open);
    }

    #[test]
    fn test_toggle_fullscreen() {
        let mut toolbar = Toolbar::new();
        assert!(!toolbar.is_fullscreen);

        toolbar.toggle_fullscreen();
        assert!(toolbar.is_fullscreen);
    }
}
