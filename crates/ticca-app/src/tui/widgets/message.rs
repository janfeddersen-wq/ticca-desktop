//! Chat Message Renderer
//!
//! Renders individual chat messages with proper formatting for
//! user messages, assistant responses, and sub-agent blocks.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::chat_message::{ChatMessage, ContentBlock};
use crate::tui::theme::TuiColors;
use ticca_core::session::MessageRole;

use super::markdown;

/// Render a chat message into styled lines
pub fn render_message(msg: &ChatMessage, colors: &TuiColors, width: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();

    let (role_icon, role_label, role_color) = match msg.role {
        MessageRole::User => ("👤", "You", colors.accent),
        MessageRole::Assistant => ("🤖", "Assistant", colors.success),
        MessageRole::System => ("⚙️", "System", colors.text_muted),
        MessageRole::Tool => ("🛠️", "Tool", colors.accent_dim),
    };

    lines.push(Line::from(vec![Span::styled(
        format!("{} {} ", role_icon, role_label),
        Style::default()
            .fg(role_color)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::raw(""));

    if let Some(reasoning) = &msg.reasoning {
        lines.push(Line::from(vec![Span::styled(
            "  ▼ Reasoning",
            Style::default()
                .fg(colors.text_muted)
                .add_modifier(Modifier::ITALIC),
        )]));

        let box_width = width.saturating_sub(6).max(4);
        lines.push(Line::from(vec![Span::styled(
            format!("  ┌{}┐", "─".repeat(box_width)),
            Style::default().fg(colors.border_subtle),
        )]));

        for line in reasoning.lines().take(10) {
            let truncate_at = width.saturating_sub(8).max(1);
            let truncated = if line.len() > truncate_at {
                format!("{}...", &line[..truncate_at.saturating_sub(3)])
            } else {
                line.to_string()
            };
            lines.push(Line::from(vec![
                Span::styled("  │ ", Style::default().fg(colors.border_subtle)),
                Span::styled(truncated, Style::default().fg(colors.text_muted)),
            ]));
        }

        let total_lines = reasoning.lines().count();
        if total_lines > 10 {
            lines.push(Line::from(vec![
                Span::styled("  │ ", Style::default().fg(colors.border_subtle)),
                Span::styled(
                    format!("... ({} more lines)", total_lines - 10),
                    Style::default().fg(colors.text_muted),
                ),
            ]));
        }

        lines.push(Line::from(vec![Span::styled(
            format!("  └{}┘", "─".repeat(box_width)),
            Style::default().fg(colors.border_subtle),
        )]));
        lines.push(Line::raw(""));
    }

    for block in &msg.content_blocks {
        match block {
            ContentBlock::Text { content, .. } => {
                let content_lines =
                    markdown::render_markdown(content, colors, width.saturating_sub(4));
                for line in content_lines {
                    let mut indented = vec![Span::raw("  ")];
                    indented.extend(line.spans.into_iter());
                    lines.push(Line::from(indented));
                }
                lines.push(Line::raw(""));
            }
            ContentBlock::SubAgent(sub_agent) => {
                let indicator = if sub_agent.collapsed { "▶" } else { "▼" };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {} Sub-Agent: ", indicator),
                        Style::default().fg(colors.border_default),
                    ),
                    Span::styled(
                        sub_agent.agent_type.display_name(),
                        Style::default()
                            .fg(colors.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));

                if let Some(reasoning) = &sub_agent.reasoning {
                    if !sub_agent.collapsed {
                        lines.push(Line::from(vec![
                            Span::styled(
                                "    │ Reasoning: ",
                                Style::default().fg(colors.text_muted),
                            ),
                            Span::styled(
                                reasoning.clone(),
                                Style::default().fg(colors.text_muted),
                            ),
                        ]));
                    }
                }

                if !sub_agent.collapsed {
                    let sub_lines = markdown::render_markdown(
                        &sub_agent.content,
                        colors,
                        width.saturating_sub(8),
                    );
                    for line in sub_lines {
                        let mut indented =
                            vec![Span::styled("    │ ", Style::default().fg(colors.border_subtle))];
                        indented.extend(line.spans.into_iter());
                        lines.push(Line::from(indented));
                    }
                }
                lines.push(Line::raw(""));
            }
        }
    }

    let separator_width = width.max(1);
    lines.push(Line::from(vec![Span::styled(
        "─".repeat(separator_width),
        Style::default().fg(colors.border_subtle),
    )]));

    lines
}
