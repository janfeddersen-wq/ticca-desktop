//! Markdown to Terminal Text Renderer
//!
//! Converts markdown text to styled ratatui Lines with support for:
//! - Headers (# ## ###)
//! - Bold (**text**)
//! - Italic (*text* or _text_)
//! - Code (`inline` and ```blocks```)
//! - Links [text](url)
//! - Lists (- item, * item, 1. item)

use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::tui::theme::TuiColors;

use super::code_block;

/// Wrap text to fit within max_width, preserving words where possible
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            if word.len() > max_width {
                let mut remaining = word;
                while remaining.len() > max_width {
                    lines.push(remaining[..max_width].to_string());
                    remaining = &remaining[max_width..];
                }
                current_line = remaining.to_string();
            } else {
                current_line = word.to_string();
            }
        } else if current_line.len() + 1 + word.len() <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            if word.len() > max_width {
                let mut remaining = word;
                while remaining.len() > max_width {
                    lines.push(remaining[..max_width].to_string());
                    remaining = &remaining[max_width..];
                }
                current_line = remaining.to_string();
            } else {
                current_line = word.to_string();
            }
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Render markdown text to styled terminal lines
pub fn render_markdown(text: &str, colors: &TuiColors, max_width: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();
    let mut current_line: Vec<Span> = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default().fg(colors.text_primary)];

    let mut in_code_block = false;
    let mut code_block_lang = String::new();
    let mut code_block_content = String::new();
    let mut list_depth: usize = 0;

    let parser = Parser::new(text);

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }

                    let heading_style = match level {
                        pulldown_cmark::HeadingLevel::H1 | pulldown_cmark::HeadingLevel::H2 => {
                            Style::default()
                                .fg(colors.accent)
                                .add_modifier(Modifier::BOLD)
                        }
                        _ => Style::default()
                            .fg(colors.text_primary)
                            .add_modifier(Modifier::BOLD),
                    };
                    style_stack.push(heading_style);
                }
                Tag::Paragraph => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                }
                Tag::Strong => {
                    let current = *style_stack.last().unwrap_or(&Style::default());
                    style_stack.push(current.add_modifier(Modifier::BOLD));
                }
                Tag::Emphasis => {
                    let current = *style_stack.last().unwrap_or(&Style::default());
                    style_stack.push(current.add_modifier(Modifier::ITALIC));
                }
                Tag::CodeBlock(kind) => {
                    in_code_block = true;
                    code_block_lang = match kind {
                        CodeBlockKind::Fenced(lang) => lang.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                    code_block_content.clear();

                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                }
                Tag::List(_) => {
                    list_depth += 1;
                }
                Tag::Item => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }

                    let indent = "  ".repeat(list_depth.saturating_sub(1));
                    current_line.push(Span::styled(
                        format!("{}• ", indent),
                        Style::default().fg(colors.accent),
                    ));
                }
                Tag::Link { .. } => {
                    style_stack.push(
                        Style::default()
                            .fg(colors.accent)
                            .add_modifier(Modifier::UNDERLINED),
                    );
                }
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Heading(_) => {
                    style_stack.pop();
                    lines.push(Line::from(std::mem::take(&mut current_line)));
                    lines.push(Line::raw(""));
                }
                TagEnd::Paragraph => {
                    lines.push(Line::from(std::mem::take(&mut current_line)));
                    lines.push(Line::raw(""));
                }
                TagEnd::Strong | TagEnd::Emphasis | TagEnd::Link => {
                    style_stack.pop();
                }
                TagEnd::CodeBlock => {
                    in_code_block = false;

                    let code_lines = code_block::render_code_block(
                        &code_block_content,
                        &code_block_lang,
                        colors,
                        max_width.saturating_sub(4),
                    );

                    let lang_label = if code_block_lang.is_empty() {
                        "code"
                    } else {
                        code_block_lang.as_str()
                    };
                    let border_width = max_width
                        .saturating_sub(lang_label.len() + 6)
                        .max(1);
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("┌─ {} ", lang_label),
                            Style::default().fg(colors.border_subtle),
                        ),
                        Span::styled(
                            "─".repeat(border_width),
                            Style::default().fg(colors.border_subtle),
                        ),
                    ]));

                    for code_line in code_lines {
                        let mut bordered = vec![Span::styled(
                            "│ ",
                            Style::default().fg(colors.border_subtle),
                        )];
                        bordered.extend(code_line.spans.into_iter());
                        lines.push(Line::from(bordered));
                    }

                    lines.push(Line::from(vec![Span::styled(
                        format!("└{}", "─".repeat(max_width.saturating_sub(2).max(1))),
                        Style::default().fg(colors.border_subtle),
                    )]));
                    lines.push(Line::raw(""));

                    code_block_content.clear();
                }
                TagEnd::List(_) => {
                    list_depth = list_depth.saturating_sub(1);
                    if list_depth == 0 {
                        lines.push(Line::raw(""));
                    }
                }
                TagEnd::Item => {
                    lines.push(Line::from(std::mem::take(&mut current_line)));
                }
                _ => {}
            },
            Event::Text(text) => {
                if in_code_block {
                    code_block_content.push_str(&text);
                } else {
                    let style = *style_stack
                        .last()
                        .unwrap_or(&Style::default().fg(colors.text_primary));

                    let _current_width: usize =
                        current_line.iter().map(|s| s.content.len()).sum();
                    let _remaining_width = max_width.saturating_sub(_current_width);

                    let wrapped = wrap_text(&text, max_width);
                    for (i, line_text) in wrapped.into_iter().enumerate() {
                        if i > 0 {
                            lines.push(Line::from(std::mem::take(&mut current_line)));
                        }
                        if !line_text.is_empty() {
                            current_line.push(Span::styled(line_text, style));
                        }
                    }
                }
            }
            Event::Code(code) => {
                current_line.push(Span::styled(
                    format!("`{}`", code),
                    Style::default().fg(colors.syntax_string).bg(colors.code_bg),
                ));
            }
            Event::SoftBreak | Event::HardBreak => {
                lines.push(Line::from(std::mem::take(&mut current_line)));
            }
            _ => {}
        }
    }

    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
    }

    lines
}
