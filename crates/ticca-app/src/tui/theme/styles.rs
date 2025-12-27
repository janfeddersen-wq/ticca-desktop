#![allow(dead_code)]

//! TUI Style Helpers
//!
//! Provides pre-built ratatui styles for consistent widget rendering.

use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders};

use super::TuiColors;

/// Style builder for TUI widgets
pub struct TuiStyles {
    colors: TuiColors,
}

impl TuiStyles {
    pub fn new(colors: TuiColors) -> Self {
        Self { colors }
    }

    pub fn colors(&self) -> &TuiColors {
        &self.colors
    }

    // === Block Styles ===

    /// Standard block with border
    pub fn block<'a>(&self, title: &'a str) -> Block<'a> {
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.colors.border_default))
            .style(Style::default().bg(self.colors.bg_surface))
    }

    /// Focused block with accent border
    pub fn block_focused<'a>(&self, title: &'a str) -> Block<'a> {
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.colors.accent))
            .style(Style::default().bg(self.colors.bg_surface))
    }

    /// Card-style block (elevated)
    pub fn card<'a>(&self, title: &'a str) -> Block<'a> {
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.colors.border_subtle))
            .style(Style::default().bg(self.colors.bg_elevated))
    }

    /// Modal/popup block
    pub fn modal<'a>(&self, title: &'a str) -> Block<'a> {
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(self.colors.accent))
            .style(Style::default().bg(self.colors.bg_elevated))
    }

    // === Text Styles ===

    /// Primary text style
    pub fn text(&self) -> Style {
        Style::default().fg(self.colors.text_primary)
    }

    /// Secondary text style
    pub fn text_secondary(&self) -> Style {
        Style::default().fg(self.colors.text_secondary)
    }

    /// Muted text style
    pub fn text_muted(&self) -> Style {
        Style::default().fg(self.colors.text_muted)
    }

    /// Bold text
    pub fn text_bold(&self) -> Style {
        Style::default()
            .fg(self.colors.text_primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Title style
    pub fn title(&self) -> Style {
        Style::default()
            .fg(self.colors.text_primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Accent colored text
    pub fn text_accent(&self) -> Style {
        Style::default().fg(self.colors.accent)
    }

    // === Status Styles ===

    pub fn success(&self) -> Style {
        Style::default().fg(self.colors.success)
    }

    pub fn warning(&self) -> Style {
        Style::default().fg(self.colors.warning)
    }

    pub fn danger(&self) -> Style {
        Style::default().fg(self.colors.danger)
    }

    // === Interactive Element Styles ===

    /// Normal button style
    pub fn button(&self) -> Style {
        Style::default()
            .fg(self.colors.text_primary)
            .bg(self.colors.bg_elevated)
    }

    /// Focused button style
    pub fn button_focused(&self) -> Style {
        Style::default()
            .fg(self.colors.bg_base)
            .bg(self.colors.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Selected item in list
    pub fn list_selected(&self) -> Style {
        Style::default()
            .fg(self.colors.text_primary)
            .bg(self.colors.bg_hover)
            .add_modifier(Modifier::BOLD)
    }

    /// Highlighted item (hover)
    pub fn list_highlight(&self) -> Style {
        Style::default()
            .fg(self.colors.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Normal list item
    pub fn list_item(&self) -> Style {
        Style::default().fg(self.colors.text_primary)
    }

    // === Input Styles ===

    /// Text input normal
    pub fn input(&self) -> Style {
        Style::default()
            .fg(self.colors.text_primary)
            .bg(self.colors.bg_base)
    }

    /// Text input focused
    pub fn input_focused(&self) -> Style {
        Style::default()
            .fg(self.colors.text_primary)
            .bg(self.colors.bg_base)
    }

    /// Input placeholder text
    pub fn input_placeholder(&self) -> Style {
        Style::default().fg(self.colors.text_muted)
    }

    // === Tab Styles ===

    /// Active tab
    pub fn tab_active(&self) -> Style {
        Style::default()
            .fg(self.colors.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Inactive tab
    pub fn tab_inactive(&self) -> Style {
        Style::default().fg(self.colors.text_secondary)
    }

    // === Chat Message Styles ===

    /// User message block style
    pub fn user_message(&self) -> Style {
        Style::default().bg(self.colors.user_msg_bg)
    }

    /// Assistant message block style
    pub fn assistant_message(&self) -> Style {
        Style::default().bg(self.colors.assistant_msg_bg)
    }

    /// Tool call block style
    pub fn tool_call(&self) -> Style {
        Style::default()
            .fg(self.colors.text_secondary)
            .bg(self.colors.tool_call_bg)
    }

    /// Reasoning block style
    pub fn reasoning(&self) -> Style {
        Style::default()
            .fg(self.colors.text_secondary)
            .bg(self.colors.reasoning_bg)
    }

    /// Code block style
    pub fn code_block(&self) -> Style {
        Style::default()
            .fg(self.colors.text_primary)
            .bg(self.colors.code_bg)
    }

    // === Status Bar ===

    pub fn status_bar(&self) -> Style {
        Style::default()
            .fg(self.colors.text_secondary)
            .bg(self.colors.bg_surface)
    }

    pub fn status_bar_key(&self) -> Style {
        Style::default()
            .fg(self.colors.accent)
            .bg(self.colors.bg_surface)
            .add_modifier(Modifier::BOLD)
    }

    // === Progress Bar ===

    pub fn progress_filled(&self) -> Style {
        Style::default().fg(self.colors.success)
    }

    pub fn progress_empty(&self) -> Style {
        Style::default().fg(self.colors.border_subtle)
    }

    pub fn progress_warning(&self) -> Style {
        Style::default().fg(self.colors.warning)
    }

    pub fn progress_danger(&self) -> Style {
        Style::default().fg(self.colors.danger)
    }

    // === Checkbox/Toggle ===

    pub fn checkbox_checked(&self) -> Style {
        Style::default().fg(self.colors.success)
    }

    pub fn checkbox_unchecked(&self) -> Style {
        Style::default().fg(self.colors.text_muted)
    }

    // === Scrollbar ===

    pub fn scrollbar(&self) -> Style {
        Style::default().fg(self.colors.border_default)
    }

    pub fn scrollbar_thumb(&self) -> Style {
        Style::default().fg(self.colors.text_muted)
    }
}
