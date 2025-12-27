//! Models Settings Tab
//!
//! Default model selection and per-agent model pinning.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::tui::app::TuiApp;
use ticca_core::agents::AgentType;

pub fn render(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(10)])
        .margin(1)
        .split(area);

    render_default_model(frame, app, chunks[0]);
    render_agent_models(frame, app, chunks[1]);
}

fn render_default_model(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let block = Block::default()
        .title(" Default Model ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.colors.border_default));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let model_name = app.settings.default_model.as_deref().unwrap_or("Not set");

    let loading_indicator = if app.settings.is_loading_models {
        " (loading...)"
    } else {
        ""
    };

    let content = vec![
        Line::from(vec![
            Span::styled("Current: ", Style::default().fg(app.colors.text_secondary)),
            Span::styled(
                model_name,
                Style::default()
                    .fg(app.colors.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(loading_indicator, Style::default().fg(app.colors.text_muted)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("[Enter]", Style::default().fg(app.colors.accent)),
            Span::raw(" Change model  "),
            Span::styled("[r]", Style::default().fg(app.colors.accent)),
            Span::raw(" Refresh models"),
        ]),
    ];

    let paragraph = Paragraph::new(content);
    frame.render_widget(paragraph, inner);
}

fn render_agent_models(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let block = Block::default()
        .title(" Agent Pinned Models ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.colors.border_default));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let agents = [
        (AgentType::Planning, "Planning"),
        (AgentType::Coding, "Coding"),
        (AgentType::Skills, "Skills"),
    ];

    let items: Vec<ListItem> = agents
        .iter()
        .enumerate()
        .map(|(i, (agent_type, name))| {
            let pinned_model = app
                .settings
                .agent_pinned_models
                .get(agent_type)
                .map(|s| s.as_str())
                .unwrap_or("(use default)");

            let is_selected = i == app.settings.list_index % 3;
            let style = if is_selected {
                Style::default()
                    .fg(app.colors.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.colors.text_primary)
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("  {} Agent: ", name), style),
                Span::styled(
                    pinned_model,
                    if pinned_model == "(use default)" {
                        Style::default().fg(app.colors.text_muted)
                    } else {
                        Style::default().fg(app.colors.text_secondary)
                    },
                ),
            ]))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}
