//! Chat View Rendering
//!
//! Renders the main chat interface including header, messages, input, and status bar.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::Frame;

use crate::tui::app::{ChatFocus, TuiApp};
use crate::tui::widgets;
use ticca_core::agents::AgentRegistry;

/// Render the complete chat view
pub fn render(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(5),
            Constraint::Length(2),
        ])
        .split(area);

    render_header(frame, app, chunks[0]);
    render_messages(frame, app, chunks[1]);
    render_input(frame, app, chunks[2]);
    render_status_bar(frame, app, chunks[3]);
}

/// Render the header with agent, model, and working directory
fn render_header(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let agent_meta = AgentRegistry::get(app.current_agent);
    let agent_label = agent_meta.label;

    let model_name = app.current_model.as_deref().unwrap_or("No model");

    let work_dir = app
        .working_directory
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(".");

    let yolo_indicator = if app.yolo_mode {
        Span::styled(
            " ⚡YOLO ",
            Style::default()
                .fg(app.colors.warning)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("")
    };

    let streaming_indicator = if app.is_streaming {
        Span::styled(
            format!(" {} ", app.spinner_char()),
            Style::default()
                .fg(app.colors.accent)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("")
    };

    let header_spans = vec![
        Span::styled("🤖 ", Style::default()),
        Span::styled(
            agent_label,
            Style::default()
                .fg(app.colors.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" │ "),
        Span::styled(model_name, Style::default().fg(app.colors.text_secondary)),
        Span::raw(" │ "),
        Span::styled(
            format!("📁 {}", work_dir),
            Style::default().fg(app.colors.text_muted),
        ),
        yolo_indicator,
        streaming_indicator,
    ];

    let header = Paragraph::new(Line::from(header_spans))
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(app.colors.border_subtle))
                .border_type(BorderType::Plain),
        )
        .alignment(Alignment::Left);

    frame.render_widget(header, area);
}

/// Render the message list
fn render_messages(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let inner_area = Rect {
        x: area.x + 1,
        y: area.y,
        width: area.width.saturating_sub(3),
        height: area.height,
    };

    if app.messages.is_empty() {
        let empty_msg = Paragraph::new(vec![
            Line::raw(""),
            Line::raw(""),
            Line::styled("No messages yet", Style::default().fg(app.colors.text_muted)),
            Line::raw(""),
            Line::styled(
                "Type a message and press Enter to send",
                Style::default().fg(app.colors.text_muted),
            ),
        ])
        .alignment(Alignment::Center)
        .block(Block::default());

        frame.render_widget(empty_msg, inner_area);
        return;
    }

    let mut all_lines: Vec<Line> = Vec::new();

    for msg in &app.messages {
        let msg_lines =
            widgets::message::render_message(msg, &app.colors, inner_area.width as usize);
        all_lines.extend(msg_lines);
    }

    let total_lines = all_lines.len();
    let max_scroll = total_lines.saturating_sub(inner_area.height as usize);
    let scroll_offset = app.message_scroll.min(max_scroll);
    let scroll_offset_u16 = scroll_offset.min(u16::MAX as usize) as u16;

    let content = Paragraph::new(all_lines)
        .block(Block::default())
        .scroll((scroll_offset_u16, 0));

    frame.render_widget(content, inner_area);

    let scrollbar_area = Rect {
        x: area.x + area.width.saturating_sub(1),
        y: area.y,
        width: 1,
        height: area.height,
    };

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("▲"))
        .end_symbol(Some("▼"))
        .track_symbol(Some("│"))
        .thumb_symbol("█");

    let mut scrollbar_state = ScrollbarState::new(total_lines).position(scroll_offset);

    frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
}

/// Render the input area with Send button
fn render_input(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let is_focused = app.chat_focus == ChatFocus::Input;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(10)])
        .split(area);

    app.send_button_area.set(Some(chunks[1]));

    let border_color = if is_focused {
        app.colors.accent
    } else {
        app.colors.border_default
    };

    let block = Block::default()
        .title(if is_focused { " Input (focused) " } else { " Input " })
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(chunks[0]);
    frame.render_widget(block, chunks[0]);

    frame.render_widget(&app.input, inner);

    let can_send = !app.is_streaming && !app.input.lines().join("").trim().is_empty();

    let button_style = if can_send {
        Style::default()
            .fg(app.colors.bg_base)
            .bg(app.colors.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(app.colors.text_muted)
            .bg(app.colors.bg_elevated)
    };

    let button_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if can_send {
            Style::default().fg(app.colors.accent)
        } else {
            Style::default().fg(app.colors.border_subtle)
        });

    let button_text = if app.is_streaming { " ⏳ " } else { " Send " };

    let button = Paragraph::new(button_text)
        .style(button_style)
        .alignment(Alignment::Center)
        .block(button_block);

    frame.render_widget(button, chunks[1]);
}

/// Render the status bar
fn render_status_bar(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(35), Constraint::Min(20)])
        .split(area);

    let total_tokens = app.input_tokens + app.output_tokens;
    let token_text = format_tokens(total_tokens as i64);
    let context_text = format_tokens(app.context_window as i64);
    let percentage = if app.context_window > 0 {
        (total_tokens as f64 / app.context_window as f64 * 100.0) as u8
    } else {
        0
    };

    let bar_width: usize = 15;
    let filled = (bar_width as f64 * percentage as f64 / 100.0) as usize;
    let bar_color = if percentage > 90 {
        app.colors.danger
    } else if percentage > 70 {
        app.colors.warning
    } else {
        app.colors.success
    };

    let bar = format!(
        "{}{}",
        "█".repeat(filled),
        "░".repeat(bar_width.saturating_sub(filled))
    );

    let gauge_spans = vec![
        Span::styled(bar, Style::default().fg(bar_color)),
        Span::raw(" "),
        Span::styled(token_text, Style::default().fg(app.colors.text_secondary)),
        Span::raw("/"),
        Span::styled(context_text, Style::default().fg(app.colors.text_muted)),
    ];

    let gauge = Paragraph::new(Line::from(gauge_spans))
        .style(Style::default().bg(app.colors.bg_surface));
    frame.render_widget(gauge, chunks[0]);

    let tps_text = if app.is_streaming && app.current_tps > 0.0 {
        format!("{:.1} t/s │ ", app.current_tps)
    } else {
        String::new()
    };

    let help_spans = vec![
        Span::styled(tps_text, Style::default().fg(app.colors.success)),
        Span::styled("[Enter]", Style::default().fg(app.colors.accent)),
        Span::raw(" Send "),
        Span::styled("[Shift+Enter]", Style::default().fg(app.colors.text_muted)),
        Span::raw(" Newline "),
        Span::styled("[F1]", Style::default().fg(app.colors.accent)),
        Span::raw(" Agent "),
        Span::styled("[F2]", Style::default().fg(app.colors.accent)),
        Span::raw(" Model "),
        Span::styled("[F9/F10]", Style::default().fg(app.colors.accent)),
        Span::raw(" Settings "),
        Span::styled("[?]", Style::default().fg(app.colors.accent)),
        Span::raw(" Help"),
    ];

    let help = Paragraph::new(Line::from(help_spans))
        .style(Style::default().bg(app.colors.bg_surface))
        .alignment(Alignment::Right);
    frame.render_widget(help, chunks[1]);
}

fn format_tokens(tokens: i64) -> String {
    let s = tokens.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(' ');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}
