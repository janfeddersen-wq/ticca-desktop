//! Input Widget Helpers
//!
//! Helpers for rendering the chat input area.

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, BorderType, Borders};
use ratatui::Frame;
use tui_textarea::TextArea;

use crate::tui::theme::TuiColors;

pub fn render_input(frame: &mut Frame, input: &TextArea, area: Rect, colors: &TuiColors, focused: bool) {
    let border_color = if focused {
        colors.accent
    } else {
        colors.border_default
    };

    let block = Block::default()
        .title(if focused { " Input (focused) " } else { " Input " })
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(input, inner);
}
