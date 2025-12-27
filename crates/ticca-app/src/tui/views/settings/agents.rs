//! Agents Settings Tab
//!
//! MCP server assignment per agent.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::tui::app::TuiApp;
use ticca_core::agents::AgentType;

pub fn render(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .margin(1)
        .split(area);

    render_agent_mcp(frame, app, AgentType::Planning, "Planning Agent", chunks[0]);
    render_agent_mcp(frame, app, AgentType::Coding, "Coding Agent", chunks[1]);
}

fn render_agent_mcp(frame: &mut Frame, app: &TuiApp, agent: AgentType, title: &str, area: Rect) {
    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.colors.border_default));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let assigned_ids = app
        .settings
        .agent_mcp_ids
        .get(&agent)
        .cloned()
        .unwrap_or_default();

    if app.settings.mcp_servers.is_empty() {
        let paragraph = Paragraph::new(vec![
            Line::styled("No MCP servers configured", Style::default().fg(app.colors.text_muted)),
            Line::raw(""),
            Line::styled(
                "Go to MCP Servers tab to add some",
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
        .map(|server| {
            let is_assigned = assigned_ids.contains(&server.id);
            let checkbox = if is_assigned {
                Span::styled("[✓]", Style::default().fg(app.colors.success))
            } else {
                Span::styled("[ ]", Style::default().fg(app.colors.text_muted))
            };

            let enabled_indicator = if server.is_enabled {
                Span::raw("")
            } else {
                Span::styled(" (disabled)", Style::default().fg(app.colors.text_muted))
            };

            ListItem::new(Line::from(vec![
                checkbox,
                Span::raw(" "),
                Span::styled(&server.name, Style::default().fg(app.colors.text_primary)),
                enabled_indicator,
            ]))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}
