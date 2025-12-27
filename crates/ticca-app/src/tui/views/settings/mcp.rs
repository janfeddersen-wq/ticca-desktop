//! MCP Servers Settings Tab
//!
//! MCP server configuration and management.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::tui::app::TuiApp;

pub fn render(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(15), Constraint::Length(8)])
        .margin(1)
        .split(area);

    render_server_list(frame, app, chunks[0]);
    render_import_section(frame, app, chunks[1]);
}

fn render_server_list(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let block = Block::default()
        .title(" MCP Servers ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.colors.border_default));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.settings.mcp_servers.is_empty() {
        let paragraph = Paragraph::new(vec![
            Line::styled("No MCP servers configured", Style::default().fg(app.colors.text_muted)),
            Line::raw(""),
            Line::styled(
                "Press [n] to add a new server",
                Style::default().fg(app.colors.text_muted),
            ),
            Line::styled(
                "or use the JSON import below",
                Style::default().fg(app.colors.text_muted),
            ),
        ]);
        frame.render_widget(paragraph, inner);
        return;
    }

    let items: Vec<ListItem> = app
        .settings
        .mcp_servers
        .iter()
        .enumerate()
        .map(|(i, server)| {
            let is_selected = i == app.settings.list_index;
            let style = if is_selected {
                Style::default()
                    .fg(app.colors.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.colors.text_primary)
            };

            let status = if server.is_enabled {
                Span::styled("●", Style::default().fg(app.colors.success))
            } else {
                Span::styled("○", Style::default().fg(app.colors.text_muted))
            };

            let transport = match &server.transport {
                ticca_core::config::McpTransport::Stdio => "stdio",
                ticca_core::config::McpTransport::StreamableHttp => "http",
            };

            ListItem::new(Line::from(vec![
                if is_selected { Span::raw("> ") } else { Span::raw("  ") },
                status,
                Span::raw(" "),
                Span::styled(&server.name, style),
                Span::styled(
                    format!(" [{}]", transport),
                    Style::default().fg(app.colors.text_muted),
                ),
            ]))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}

fn render_import_section(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let block = Block::default()
        .title(" Import MCP JSON ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.colors.border_default));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let content = vec![
        Line::styled(
            "Paste Claude Desktop-style mcpServers JSON and press [Enter]",
            Style::default().fg(app.colors.text_secondary),
        ),
        Line::raw(""),
        Line::from(vec![
            Span::styled("[n]", Style::default().fg(app.colors.accent)),
            Span::raw(" New server  "),
            Span::styled("[e]", Style::default().fg(app.colors.accent)),
            Span::raw(" Edit  "),
            Span::styled("[d]", Style::default().fg(app.colors.accent)),
            Span::raw(" Delete  "),
            Span::styled("[Space]", Style::default().fg(app.colors.accent)),
            Span::raw(" Toggle enabled"),
        ]),
    ];

    let paragraph = Paragraph::new(content);
    frame.render_widget(paragraph, inner);
}
