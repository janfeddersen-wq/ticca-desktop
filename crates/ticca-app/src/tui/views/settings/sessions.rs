//! Sessions Settings Tab
//!
//! Recent sessions list with load and delete functionality.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::tui::app::TuiApp;

pub fn render(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let block = Block::default()
        .title(" Recent Sessions ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.colors.border_default))
        .style(Style::default().bg(app.colors.bg_surface));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.settings.recent_sessions.is_empty() {
        let content = vec![
            Line::raw(""),
            Line::styled("No recent sessions", Style::default().fg(app.colors.text_muted)),
            Line::raw(""),
            Line::styled(
                "Sessions are saved automatically",
                Style::default().fg(app.colors.text_muted),
            ),
            Line::styled(
                "when you chat with an agent",
                Style::default().fg(app.colors.text_muted),
            ),
        ];

        let paragraph = Paragraph::new(content);
        frame.render_widget(paragraph, inner);
        return;
    }

    let items: Vec<ListItem> = app
        .settings
        .recent_sessions
        .iter()
        .enumerate()
        .map(|(i, session)| {
            let is_selected = i == app.settings.list_index % app.settings.recent_sessions.len();
            let is_current = app
                .current_session
                .as_ref()
                .map(|s| s.id == session.id)
                .unwrap_or(false);

            let indicator = if is_current {
                Span::styled("▶ ", Style::default().fg(app.colors.accent))
            } else if is_selected {
                Span::styled("> ",
                    Style::default().fg(app.colors.text_primary),
                )
            } else {
                Span::raw("  ")
            };

            let style = if is_selected {
                Style::default()
                    .fg(app.colors.accent)
                    .add_modifier(Modifier::BOLD)
            } else if is_current {
                Style::default()
                    .fg(app.colors.text_primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.colors.text_secondary)
            };

            let title = session.name.as_str();

            let title_display = if title.len() > 40 {
                format!("{}...", &title[..37])
            } else {
                title.to_string()
            };

            let timestamp = session
                .updated_at
                .as_deref()
                .unwrap_or("unknown")
                .to_string();

            ListItem::new(Line::from(vec![
                indicator,
                Span::styled(title_display, style),
                Span::raw("  "),
                Span::styled(timestamp, Style::default().fg(app.colors.text_muted)),
            ]))
        })
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(3)])
        .split(inner);

    let list = List::new(items);
    frame.render_widget(list, chunks[0]);

    let help = Paragraph::new(Line::from(vec![
        Span::styled("[Enter]", Style::default().fg(app.colors.accent)),
        Span::raw(" Load session  "),
        Span::styled("[d]", Style::default().fg(app.colors.accent)),
        Span::raw(" Delete  "),
        Span::styled("[n]", Style::default().fg(app.colors.accent)),
        Span::raw(" New session"),
    ]));
    frame.render_widget(help, chunks[1]);
}
