//! Popup/Modal Widgets
//!
//! Generic popup forms for settings editing, confirmations, and pickers.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::tui::theme::TuiColors;

/// Calculate a centered rectangle
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Render a list picker popup
pub fn render_list_picker(
    frame: &mut Frame,
    title: &str,
    items: &[(&str, bool)],
    selected_index: usize,
    colors: &TuiColors,
    area: Rect,
) {
    let popup_area = centered_rect(50, 60, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(colors.accent))
        .style(Style::default().bg(colors.bg_elevated));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, (label, is_current))| {
            let is_selected = i == selected_index;

            let prefix = if is_selected && *is_current {
                Span::styled("● ", Style::default().fg(colors.success))
            } else if is_selected {
                Span::styled("> ", Style::default().fg(colors.accent))
            } else if *is_current {
                Span::styled("● ", Style::default().fg(colors.success))
            } else {
                Span::raw("  ")
            };

            let style = if is_selected {
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.text_primary)
            };

            ListItem::new(Line::from(vec![prefix, Span::styled(*label, style)]))
        })
        .collect();

    let list = List::new(list_items);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(inner);

    frame.render_widget(list, chunks[0]);

    let help = Paragraph::new(Line::from(vec![
        Span::styled("[↑/↓]", Style::default().fg(colors.accent)),
        Span::raw(" Navigate "),
        Span::styled("[Enter]", Style::default().fg(colors.accent)),
        Span::raw(" Select "),
        Span::styled("[Esc]", Style::default().fg(colors.accent)),
        Span::raw(" Cancel"),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(help, chunks[1]);
}

/// Render a confirmation dialog
pub fn render_confirmation(
    frame: &mut Frame,
    title: &str,
    message: &str,
    colors: &TuiColors,
    area: Rect,
) {
    let popup_area = centered_rect(50, 30, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(colors.warning))
        .style(Style::default().bg(colors.bg_elevated));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let content = vec![
        Line::raw(""),
        Line::styled(message, Style::default().fg(colors.text_primary)),
        Line::raw(""),
        Line::raw(""),
        Line::from(vec![
            Span::styled("[Y]", Style::default().fg(colors.success)),
            Span::raw(" Yes  "),
            Span::styled("[N]", Style::default().fg(colors.danger)),
            Span::raw(" No  "),
            Span::styled("[Esc]", Style::default().fg(colors.text_muted)),
            Span::raw(" Cancel"),
        ]),
    ];

    let paragraph = Paragraph::new(content)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, inner);
}

/// Render an input form popup
pub fn render_input_form(
    frame: &mut Frame,
    title: &str,
    fields: &[(&str, &str, bool)],
    colors: &TuiColors,
    area: Rect,
) {
    let height = 6 + fields.len() as u16 * 3;
    let popup_area = centered_rect(60, height.min(80), area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(colors.accent))
        .style(Style::default().bg(colors.bg_elevated));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let mut constraints: Vec<Constraint> = fields
        .iter()
        .map(|_| Constraint::Length(3))
        .collect();
    constraints.push(Constraint::Length(2));
    constraints.push(Constraint::Min(0));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(constraints)
        .split(inner);

    for (i, (label, value, is_focused)) in fields.iter().enumerate() {
        let field_block = Block::default()
            .title(*label)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if *is_focused {
                Style::default().fg(colors.accent)
            } else {
                Style::default().fg(colors.border_default)
            });

        let inner_field = field_block.inner(chunks[i]);
        frame.render_widget(field_block, chunks[i]);

        let text = if value.is_empty() && *is_focused {
            Paragraph::new(Line::styled(
                "Type here...",
                Style::default().fg(colors.text_muted),
            ))
        } else {
            Paragraph::new(Line::styled(*value, Style::default().fg(colors.text_primary)))
        };
        frame.render_widget(text, inner_field);
    }

    let help = Paragraph::new(Line::from(vec![
        Span::styled("[Tab]", Style::default().fg(colors.accent)),
        Span::raw(" Next field "),
        Span::styled("[Enter]", Style::default().fg(colors.accent)),
        Span::raw(" Save "),
        Span::styled("[Esc]", Style::default().fg(colors.accent)),
        Span::raw(" Cancel"),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(help, chunks[fields.len()]);
}

/// Render a simple file/directory browser
pub fn render_file_picker(
    frame: &mut Frame,
    current_path: &std::path::Path,
    entries: &[(String, bool)],
    selected_index: usize,
    colors: &TuiColors,
    area: Rect,
) {
    let popup_area = centered_rect(70, 70, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Select Directory ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(colors.accent))
        .style(Style::default().bg(colors.bg_elevated));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(inner);

    let path_str = current_path.display().to_string();
    let available_width = chunks[0].width.saturating_sub(4) as usize;
    let path_display = if path_str.len() > available_width && available_width > 3 {
        format!("...{}", &path_str[path_str.len() - (available_width - 3)..])
    } else {
        path_str
    };
    let path_line = Paragraph::new(Line::from(vec![
        Span::styled("📁 ", Style::default()),
        Span::styled(path_display, Style::default().fg(colors.accent)),
    ]));
    frame.render_widget(path_line, chunks[0]);

    let list_items: Vec<ListItem> = entries
        .iter()
        .enumerate()
        .map(|(i, (name, is_dir))| {
            let is_selected = i == selected_index;
            let icon = if *is_dir { "📁 " } else { "📄 " };

            let style = if is_selected {
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.text_primary)
            };

            let prefix = if is_selected { "> " } else { "  " };

            ListItem::new(Line::from(vec![
                Span::raw(prefix),
                Span::raw(icon),
                Span::styled(name, style),
            ]))
        })
        .collect();

    let list = List::new(list_items);
    frame.render_widget(list, chunks[1]);

    let help = Paragraph::new(Line::from(vec![
        Span::styled("[Enter]", Style::default().fg(colors.accent)),
        Span::raw(" Select "),
        Span::styled("[Backspace]", Style::default().fg(colors.accent)),
        Span::raw(" Parent "),
        Span::styled("[Esc]", Style::default().fg(colors.accent)),
        Span::raw(" Cancel"),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(help, chunks[2]);
}
