//! Token Usage Gauge Widget
//!
//! Displays token usage as a progress bar with percentage.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::tui::theme::TuiColors;

/// Token gauge widget
pub struct TokenGauge<'a> {
    tokens_used: u64,
    context_limit: u64,
    colors: &'a TuiColors,
}

impl<'a> TokenGauge<'a> {
    pub fn new(tokens_used: u64, context_limit: u64, colors: &'a TuiColors) -> Self {
        Self {
            tokens_used,
            context_limit,
            colors,
        }
    }

    fn percentage(&self) -> f64 {
        if self.context_limit == 0 {
            0.0
        } else {
            (self.tokens_used as f64 / self.context_limit as f64 * 100.0).min(100.0)
        }
    }

    fn bar_color(&self) -> ratatui::style::Color {
        let pct = self.percentage();
        if pct > 90.0 {
            self.colors.danger
        } else if pct > 70.0 {
            self.colors.warning
        } else {
            self.colors.success
        }
    }
}

impl Widget for TokenGauge<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 1 {
            return;
        }

        let percentage = self.percentage();
        let bar_width = (area.width as usize).min(20);
        let filled = (bar_width as f64 * percentage / 100.0) as usize;

        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_width - filled));

        let label = format!(
            " {}/{} ({:.0}%)",
            format_tokens(self.tokens_used),
            format_tokens(self.context_limit),
            percentage
        );

        let spans = vec![
            Span::styled(bar, Style::default().fg(self.bar_color())),
            Span::styled(label, Style::default().fg(self.colors.text_secondary)),
        ];

        let line = Line::from(spans);
        buf.set_line(area.x, area.y, &line, area.width);
    }
}

fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}
