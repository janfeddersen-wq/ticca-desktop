//! Accounts Settings Tab
//!
//! OAuth accounts (Claude, Gemini, ChatGPT) and API key management.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem};
use ratatui::Frame;

use crate::tui::app::TuiApp;

pub fn render(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(10)])
        .margin(1)
        .split(area);

    render_oauth_section(frame, app, chunks[0]);
    render_account_lists(frame, app, chunks[1]);
}

fn render_oauth_section(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let block = Block::default()
        .title(" OAuth Providers ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.colors.border_default));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let providers = [
        ("Claude", "Anthropic", app.settings.oauth_accounts_claude.len()),
        ("Gemini", "Google", app.settings.oauth_accounts_gemini.len()),
        (
            "ChatGPT",
            "OpenAI",
            app.settings.oauth_accounts_chatgpt.len(),
        ),
    ];

    let items: Vec<ListItem> = providers
        .iter()
        .enumerate()
        .map(|(i, (name, company, count))| {
            let status = if *count > 0 {
                Span::styled(
                    format!(" ✓ {} accounts", count),
                    Style::default().fg(app.colors.success),
                )
            } else {
                Span::styled(" ○ Not connected", Style::default().fg(app.colors.text_muted))
            };

            let is_selected = i == app.settings.list_index % 3;
            let style = if is_selected {
                Style::default()
                    .fg(app.colors.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.colors.text_primary)
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("  {} ", name), style),
                Span::styled(
                    format!("({})", company),
                    Style::default().fg(app.colors.text_muted),
                ),
                status,
                Span::styled(" [Enter to add]", Style::default().fg(app.colors.text_muted)),
            ]))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}

fn render_account_lists(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_oauth_accounts(frame, app, chunks[0]);
    render_api_key_accounts(frame, app, chunks[1]);
}

fn render_oauth_accounts(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let block = Block::default()
        .title(" OAuth Accounts ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.colors.border_default));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut items: Vec<ListItem> = Vec::new();

    for acc in &app.settings.oauth_accounts_claude {
        let status = get_account_status(acc);
        items.push(ListItem::new(Line::from(vec![
            Span::styled("Claude: ", Style::default().fg(app.colors.accent)),
            Span::styled(
                acc.label.clone().unwrap_or_else(|| acc.id[..8].to_string()),
                Style::default().fg(app.colors.text_primary),
            ),
            Span::raw(" "),
            status,
        ])));
    }

    for acc in &app.settings.oauth_accounts_gemini {
        let status = get_account_status(acc);
        items.push(ListItem::new(Line::from(vec![
            Span::styled("Gemini: ", Style::default().fg(app.colors.warning)),
            Span::styled(
                acc.label.clone().unwrap_or_else(|| acc.id[..8].to_string()),
                Style::default().fg(app.colors.text_primary),
            ),
            Span::raw(" "),
            status,
        ])));
    }

    for acc in &app.settings.oauth_accounts_chatgpt {
        let status = get_account_status(acc);
        items.push(ListItem::new(Line::from(vec![
            Span::styled("ChatGPT: ", Style::default().fg(app.colors.success)),
            Span::styled(
                acc.label.clone().unwrap_or_else(|| acc.id[..8].to_string()),
                Style::default().fg(app.colors.text_primary),
            ),
            Span::raw(" "),
            status,
        ])));
    }

    if items.is_empty() {
        items.push(ListItem::new(Line::styled(
            "  No OAuth accounts configured",
            Style::default().fg(app.colors.text_muted),
        )));
    }

    let list = List::new(items);
    frame.render_widget(list, inner);
}

fn render_api_key_accounts(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let block = Block::default()
        .title(" API Key Accounts ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.colors.border_default));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut items: Vec<ListItem> = Vec::new();

    for (provider_id, accounts) in &app.settings.api_key_accounts {
        for acc in accounts {
            let status = if acc.is_active {
                Span::styled("●", Style::default().fg(app.colors.success))
            } else {
                Span::styled("○", Style::default().fg(app.colors.text_muted))
            };

            items.push(ListItem::new(Line::from(vec![
                status,
                Span::raw(" "),
                Span::styled(provider_id, Style::default().fg(app.colors.accent)),
                Span::raw(": "),
                Span::styled(
                    acc.label.clone().unwrap_or_else(|| "unnamed".to_string()),
                    Style::default().fg(app.colors.text_primary),
                ),
            ])));
        }
    }

    if items.is_empty() {
        items.push(ListItem::new(Line::styled(
            "  No API keys configured",
            Style::default().fg(app.colors.text_muted),
        )));
        items.push(ListItem::new(Line::styled(
            "  Press [n] to add one",
            Style::default().fg(app.colors.text_muted),
        )));
    }

    let list = List::new(items);
    frame.render_widget(list, inner);
}

fn get_account_status(acc: &ticca_core::config::OAuthAccount) -> Span<'static> {
    if !acc.is_active {
        Span::styled("[disabled]", Style::default().fg(ratatui::style::Color::Gray))
    } else if acc.is_expired() {
        Span::styled("[expired]", Style::default().fg(ratatui::style::Color::Red))
    } else if acc.is_cooling() {
        Span::styled("[cooldown]", Style::default().fg(ratatui::style::Color::Yellow))
    } else {
        Span::styled("[ready]", Style::default().fg(ratatui::style::Color::Green))
    }
}
