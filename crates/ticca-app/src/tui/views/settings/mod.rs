//! Settings View
//!
//! Main settings container with tab bar and content rendering for all 7 tabs.

mod accounts;
mod models;
mod agents;
mod mcp;
mod tools;
mod appearance;
mod sessions;

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};
use ratatui::Frame;

use crate::tui::app::{SettingsTab, TuiApp};

/// Render the complete settings view
pub fn render(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(area);

    render_header(frame, app, chunks[0]);
    render_tabs(frame, app, chunks[1]);
    render_content(frame, app, chunks[2]);
    render_help_bar(frame, app, chunks[3]);
}

fn render_header(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let header_spans = vec![
        Span::styled("← ", Style::default().fg(app.colors.accent)),
        Span::styled("[Esc]", Style::default().fg(app.colors.text_muted)),
        Span::styled(" Back to Chat", Style::default().fg(app.colors.text_secondary)),
        Span::raw("    "),
        Span::styled(
            "⚙ Settings",
            Style::default()
                .fg(app.colors.text_primary)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    let header = Paragraph::new(Line::from(header_spans)).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(app.colors.border_subtle)),
    );

    frame.render_widget(header, area);
}

fn render_tabs(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let tab_titles: Vec<Line> = SettingsTab::ALL
        .iter()
        .enumerate()
        .map(|(i, tab)| {
            let style = if *tab == app.settings.active_tab {
                Style::default()
                    .fg(app.colors.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.colors.text_secondary)
            };
            Line::from(Span::styled(format!("[{}] {}", i + 1, tab.name()), style))
        })
        .collect();

    let selected_index = SettingsTab::ALL
        .iter()
        .position(|t| *t == app.settings.active_tab)
        .unwrap_or(0);

    let tabs = Tabs::new(tab_titles)
        .select(selected_index)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(app.colors.border_subtle)),
        )
        .highlight_style(
            Style::default()
                .fg(app.colors.accent)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled(
            " │ ",
            Style::default().fg(app.colors.border_subtle),
        ));

    frame.render_widget(tabs, area);
}

fn render_content(frame: &mut Frame, app: &TuiApp, area: Rect) {
    match app.settings.active_tab {
        SettingsTab::Accounts => accounts::render(frame, app, area),
        SettingsTab::Models => models::render(frame, app, area),
        SettingsTab::Agents => agents::render(frame, app, area),
        SettingsTab::McpServers => mcp::render(frame, app, area),
        SettingsTab::Tools => tools::render(frame, app, area),
        SettingsTab::Appearance => appearance::render(frame, app, area),
        SettingsTab::Sessions => sessions::render(frame, app, area),
    }
}

fn render_help_bar(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let help_spans = vec![
        Span::styled("[1-7]", Style::default().fg(app.colors.accent)),
        Span::raw(" Tab "),
        Span::styled("[←/→]", Style::default().fg(app.colors.accent)),
        Span::raw(" Navigate "),
        Span::styled("[↑/↓]", Style::default().fg(app.colors.accent)),
        Span::raw(" Select "),
        Span::styled("[Enter]", Style::default().fg(app.colors.accent)),
        Span::raw(" Confirm "),
        Span::styled("[n]", Style::default().fg(app.colors.accent)),
        Span::raw(" New "),
        Span::styled("[d]", Style::default().fg(app.colors.accent)),
        Span::raw(" Delete "),
        Span::styled("[Esc]", Style::default().fg(app.colors.accent)),
        Span::raw(" Back"),
    ];

    let help = Paragraph::new(Line::from(help_spans))
        .style(Style::default().bg(app.colors.bg_surface))
        .alignment(Alignment::Center);

    frame.render_widget(help, area);
}
