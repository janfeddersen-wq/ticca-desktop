//! Tools & Safety Settings Tab
//!
//! YOLO mode, external tools management, and compression settings.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::tui::app::TuiApp;

pub fn render(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Length(12), Constraint::Min(8)])
        .margin(1)
        .split(area);

    render_yolo_section(frame, app, chunks[0]);
    render_external_tools(frame, app, chunks[1]);
    render_compression(frame, app, chunks[2]);
}

fn render_yolo_section(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let block = Block::default()
        .title(" YOLO Mode ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.colors.border_default));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let yolo_status = if app.yolo_mode {
        Span::styled(
            "⚡ ENABLED - Tools auto-approved",
            Style::default()
                .fg(app.colors.warning)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            "○ Disabled - Approval required",
            Style::default().fg(app.colors.text_secondary),
        )
    };

    let content = vec![
        Line::from(yolo_status),
        Line::raw(""),
        Line::from(vec![
            Span::styled("[Space]", Style::default().fg(app.colors.accent)),
            Span::raw(" Toggle YOLO mode"),
        ]),
    ];

    let paragraph = Paragraph::new(content);
    frame.render_widget(paragraph, inner);
}

fn render_external_tools(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let block = Block::default()
        .title(" External Tools ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.colors.border_default));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let tools = [
        (
            ticca_core::ExternalToolId::LibreOffice,
            "LibreOffice",
            "Document conversion",
        ),
        (
            ticca_core::ExternalToolId::Node,
            "Node.js",
            "JavaScript execution",
        ),
        (
            ticca_core::ExternalToolId::Uv,
            "UV",
            "Python package manager",
        ),
    ];

    let items: Vec<ListItem> = tools
        .iter()
        .map(|(id, name, desc)| {
            let status = app
                .settings
                .external_tools
                .get(id)
                .map(|s| {
                    if s.is_installing {
                        Line::from(vec![Span::styled(
                            format!("Installing {}%", s.install_progress),
                            Style::default().fg(app.colors.accent),
                        )])
                    } else if s.is_installed {
                        Line::from(vec![
                            Span::styled("✓ Installed", Style::default().fg(app.colors.success)),
                            Span::styled(
                                format!(" v{}", s.version.as_deref().unwrap_or("?")),
                                Style::default().fg(app.colors.text_muted),
                            ),
                        ])
                    } else if !s.is_supported {
                        Line::from(Span::styled(
                            "✗ Unsupported",
                            Style::default().fg(app.colors.text_muted),
                        ))
                    } else {
                        Line::from(Span::styled(
                            "○ Not installed",
                            Style::default().fg(app.colors.text_secondary),
                        ))
                    }
                })
                .unwrap_or_else(|| {
                    Line::from(Span::styled(
                        "○ Unknown",
                        Style::default().fg(app.colors.text_muted),
                    ))
                });

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(
                        *name,
                        Style::default()
                            .fg(app.colors.text_primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" - {}", desc),
                        Style::default().fg(app.colors.text_muted),
                    ),
                ]),
                status,
            ])
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}

fn render_compression(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let block = Block::default()
        .title(" Context Compression ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.colors.border_default));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let enabled_status = if app.settings.compression.enabled {
        Span::styled("✓ Enabled", Style::default().fg(app.colors.success))
    } else {
        Span::styled("○ Disabled", Style::default().fg(app.colors.text_muted))
    };

    let strategy_name = match app.settings.compression.strategy {
        ticca_core::config::CompressionStrategy::Truncation => "Truncation",
        ticca_core::config::CompressionStrategy::Summarizing => "Summarizing",
    };

    let content = vec![
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(app.colors.text_secondary)),
            enabled_status,
        ]),
        Line::from(vec![
            Span::styled("Threshold: ", Style::default().fg(app.colors.text_secondary)),
            Span::styled(
                format!("{}%", app.settings.compression.threshold_percent),
                Style::default().fg(app.colors.text_primary),
            ),
            Span::styled(
                " of context window",
                Style::default().fg(app.colors.text_muted),
            ),
        ]),
        Line::from(vec![
            Span::styled("Strategy: ", Style::default().fg(app.colors.text_secondary)),
            Span::styled(strategy_name, Style::default().fg(app.colors.text_primary)),
        ]),
        Line::from(vec![
            Span::styled("Preserve first: ", Style::default().fg(app.colors.text_secondary)),
            Span::styled(
                format!("{} messages", app.settings.compression.preserve_first),
                Style::default().fg(app.colors.text_primary),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Protected tokens: ",
                Style::default().fg(app.colors.text_secondary),
            ),
            Span::styled(
                format!("{}K", app.settings.compression.protected_tokens / 1000),
                Style::default().fg(app.colors.text_primary),
            ),
        ]),
    ];

    let paragraph = Paragraph::new(content);
    frame.render_widget(paragraph, inner);
}
