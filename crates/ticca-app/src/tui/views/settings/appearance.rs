//! Appearance Settings Tab
//!
//! Theme selection and UI mode configuration.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::theme::ALL_THEMES;
use crate::tui::app::TuiApp;

pub fn render(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .margin(1)
        .split(area);

    render_theme_list(frame, app, chunks[0]);
    render_preview(frame, app, chunks[1]);
}

fn render_theme_list(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let block = Block::default()
        .title(" Theme ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.colors.border_default));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = ALL_THEMES
        .iter()
        .enumerate()
        .map(|(i, theme)| {
            let is_current = *theme == app.theme;
            let is_selected = i == app.settings.list_index % ALL_THEMES.len();

            let indicator = if is_current {
                Span::styled("● ", Style::default().fg(app.colors.success))
            } else {
                Span::styled("○ ", Style::default().fg(app.colors.text_muted))
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

            let dark_light = if theme.is_dark() { " (dark)" } else { " (light)" };

            ListItem::new(Line::from(vec![
                indicator,
                Span::styled(theme.display_name(), style),
                Span::styled(dark_light, Style::default().fg(app.colors.text_muted)),
            ]))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}

fn render_preview(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let block = Block::default()
        .title(" Preview ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.colors.border_default));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let content = vec![
        Line::from(vec![
            Span::styled("Current: ", Style::default().fg(app.colors.text_secondary)),
            Span::styled(
                app.theme.display_name(),
                Style::default()
                    .fg(app.colors.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("████", Style::default().fg(app.colors.bg_base)),
            Span::raw(" Background"),
        ]),
        Line::from(vec![
            Span::styled("████", Style::default().fg(app.colors.accent)),
            Span::raw(" Accent"),
        ]),
        Line::from(vec![
            Span::styled("████", Style::default().fg(app.colors.success)),
            Span::raw(" Success"),
        ]),
        Line::from(vec![
            Span::styled("████", Style::default().fg(app.colors.warning)),
            Span::raw(" Warning"),
        ]),
        Line::from(vec![
            Span::styled("████", Style::default().fg(app.colors.danger)),
            Span::raw(" Danger"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("[Enter]", Style::default().fg(app.colors.accent)),
            Span::raw(" Apply theme"),
        ]),
    ];

    let paragraph = Paragraph::new(content);
    frame.render_widget(paragraph, inner);
}
