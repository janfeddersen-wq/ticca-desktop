//! Markdown rendering widget with Twemoji SVG emoji support and table rendering

use iced::widget::{container, text, Column, Row};
use iced::{Color, Element, Font, Length};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use unicode_segmentation::UnicodeSegmentation;

use super::emoji::{lookup_emoji, render_emoji, EMOJI_SIZE};
use super::syntax::render_code_block;
use crate::icons;
use crate::messages::Message;
use crate::theme::styles;

/// Bootstrap Icons font
const ICONS_FONT: Font = Font::with_name("bootstrap-icons");

/// Markers for inline formatting (using private use characters)
const INLINE_CODE_START: char = '\u{E000}';
const INLINE_CODE_END: char = '\u{E001}';
const BOLD_START: char = '\u{E002}';
const BOLD_END: char = '\u{E003}';
const ITALIC_START: char = '\u{E004}';
const ITALIC_END: char = '\u{E005}';
const STRIKETHROUGH_START: char = '\u{E006}';
const STRIKETHROUGH_END: char = '\u{E007}';

/// Formatting state for inline text
#[derive(Default, Clone)]
struct FormatState {
    bold: bool,
    italic: bool,
    strikethrough: bool,
    code: bool,
}

/// Render a text string with inline formatting, emojis, and code
fn render_text_with_emojis(content: &str, font_size: u16) -> Element<'static, Message> {
    let mut elements: Vec<Element<'static, Message>> = Vec::new();
    let mut current_text = String::new();
    let mut format = FormatState::default();

    for grapheme in content.graphemes(true) {
        let c = grapheme.chars().next();

        // Handle format markers - flush BEFORE changing state
        if let Some(ch) = c {
            let is_marker = matches!(ch,
                INLINE_CODE_START | INLINE_CODE_END | BOLD_START | BOLD_END |
                ITALIC_START | ITALIC_END | STRIKETHROUGH_START | STRIKETHROUGH_END);
            if is_marker {
                flush_formatted_text(&mut current_text, &format, font_size, &mut elements);
                match ch {
                    INLINE_CODE_START => format.code = true,
                    INLINE_CODE_END => format.code = false,
                    BOLD_START => format.bold = true,
                    BOLD_END => format.bold = false,
                    ITALIC_START => format.italic = true,
                    ITALIC_END => format.italic = false,
                    STRIKETHROUGH_START => format.strikethrough = true,
                    STRIKETHROUGH_END => format.strikethrough = false,
                    _ => {}
                }
                continue;
            }
        }

        // Handle emojis (but not inside code)
        if !format.code {
            if let Some(asset) = lookup_emoji(grapheme) {
                flush_formatted_text(&mut current_text, &format, font_size, &mut elements);
                elements.push(render_emoji(&asset, EMOJI_SIZE));
                continue;
            }
        }

        current_text.push_str(grapheme);
    }

    // Flush remaining text
    flush_formatted_text(&mut current_text, &format, font_size, &mut elements);

    // Return appropriate element
    if elements.len() == 1 {
        elements.pop().unwrap()
    } else if elements.is_empty() {
        text("").size(font_size).into()
    } else {
        Row::with_children(elements)
            .spacing(0)
            .align_y(iced::Alignment::Center)
            .into()
    }
}

/// Flush accumulated text with current formatting
fn flush_formatted_text(
    current_text: &mut String,
    format: &FormatState,
    font_size: u16,
    elements: &mut Vec<Element<'static, Message>>,
) {
    if current_text.is_empty() {
        return;
    }

    let content = std::mem::take(current_text);

    if format.code {
        // Inline code styling
        elements.push(
            container(
                text(content)
                    .size(font_size.saturating_sub(1))
                    .font(Font::with_name("Noto Sans Mono")),
            )
            .padding([1, 4])
            .style(|_theme: &iced::Theme| container::Style {
                background: Some(Color::from_rgba(0.5, 0.5, 0.5, 0.15).into()),
                border: iced::Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 3.0.into(),
                },
                ..Default::default()
            })
            .into(),
        );
    } else if format.strikethrough {
        // Strikethrough - use gray color to indicate (Iced doesn't have native strikethrough)
        elements.push(
            text(format!("̶{}̶", content.chars().map(|c| format!("{}\u{0336}", c)).collect::<String>()))
                .size(font_size)
                .into(),
        );
    } else {
        // Normal, bold, italic, or bold+italic
        let font = match (format.bold, format.italic) {
            (true, true) => Font {
                weight: iced::font::Weight::Bold,
                style: iced::font::Style::Italic,
                ..Font::with_name("Noto Sans")
            },
            (true, false) => Font {
                weight: iced::font::Weight::Bold,
                ..Font::with_name("Noto Sans")
            },
            (false, true) => Font {
                style: iced::font::Style::Italic,
                ..Font::with_name("Noto Sans")
            },
            (false, false) => Font::default(),
        };

        elements.push(text(content).size(font_size).font(font).into());
    }
}

/// Render a heading with emoji and inline formatting support
fn render_heading(content: &str, level: u8) -> Element<'static, Message> {
    let size = match level {
        1 => 22,
        2 => 18,
        3 => 16,
        _ => 15,
    };

    render_text_with_emojis(content, size)
}

/// Render a list item with emoji and inline formatting support
fn render_list_item(content: &str, depth: usize, task_checked: Option<bool>) -> Element<'static, Message> {
    let indent = "  ".repeat(depth.saturating_sub(1));

    let mut elements: Vec<Element<'static, Message>> = Vec::new();

    // Add indent
    if !indent.is_empty() {
        elements.push(text(indent).size(14).into());
    }

    // Add bullet or checkbox
    match task_checked {
        Some(true) => {
            // Checked task - green filled checkbox
            elements.push(
                text(icons::CHECK_SQUARE_FILL.to_string())
                    .size(14)
                    .font(ICONS_FONT)
                    .color(Color::from_rgb(0.2, 0.7, 0.3))
                    .into(),
            );
            elements.push(text(" ").size(14).into());
        }
        Some(false) => {
            // Unchecked task - gray empty square
            elements.push(
                text(icons::SQUARE.to_string())
                    .size(14)
                    .font(ICONS_FONT)
                    .color(Color::from_rgb(0.5, 0.5, 0.5))
                    .into(),
            );
            elements.push(text(" ").size(14).into());
        }
        None => {
            // Regular bullet point
            elements.push(text("• ").size(14).into());
        }
    }

    // Add content with emoji and inline formatting support
    elements.push(render_text_with_emojis(content.trim(), 14));

    Row::with_children(elements)
        .spacing(0)
        .align_y(iced::Alignment::Center)
        .into()
}

/// Render a table from collected rows
fn render_table(
    header_row: &[String],
    body_rows: &[Vec<String>],
    theme_is_dark: bool,
) -> Element<'static, Message> {
    let mut table_rows: Vec<Element<'static, Message>> = Vec::new();

    // Render header row
    if !header_row.is_empty() {
        let header_cells: Vec<Element<'static, Message>> = header_row
            .iter()
            .map(|cell| {
                container(render_text_with_emojis(cell, 13))
                    .padding([4, 8])
                    .width(Length::FillPortion(1))
                    .into()
            })
            .collect();

        let header = container(
            Row::with_children(header_cells)
                .spacing(2)
                .width(Length::Fill),
        )
        .style(move |theme| styles::table_header(theme, theme_is_dark));

        table_rows.push(header.into());
    }

    // Render body rows
    for (idx, row) in body_rows.iter().enumerate() {
        let row_cells: Vec<Element<'static, Message>> = row
            .iter()
            .map(|cell| {
                container(render_text_with_emojis(cell, 13))
                    .padding([4, 8])
                    .width(Length::FillPortion(1))
                    .into()
            })
            .collect();

        let is_alt = idx % 2 == 1;
        let row_element = container(
            Row::with_children(row_cells)
                .spacing(2)
                .width(Length::Fill),
        )
        .style(move |theme| styles::table_row(theme, theme_is_dark, is_alt));

        table_rows.push(row_element.into());
    }

    container(Column::with_children(table_rows).spacing(1))
        .width(Length::Fill)
        .padding(4)
        .into()
}

/// Render a horizontal rule
fn render_horizontal_rule(theme_is_dark: bool) -> Element<'static, Message> {
    let color = if theme_is_dark {
        Color::from_rgba(1.0, 1.0, 1.0, 0.2)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.2)
    };

    container(text(""))
        .width(Length::Fill)
        .height(1)
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(color.into()),
            ..Default::default()
        })
        .into()
}

/// Render markdown content as Iced widgets with Twemoji SVG support
pub fn render(content: &str, theme_is_dark: bool) -> Element<'static, Message> {
    let mut elements: Vec<Element<'static, Message>> = Vec::new();
    let mut current_paragraph = String::new();
    let mut in_code_block = false;
    let mut code_block_content = String::new();
    let mut code_block_language = String::new();
    let mut current_heading_level: Option<u8> = None;
    let mut list_depth: usize = 0;
    let mut in_list_item = false;
    let mut list_item_content = String::new();

    // Table state
    let mut _in_table = false;
    let mut in_table_head = false;
    let mut _in_table_row = false;
    let mut in_table_cell = false;
    let mut table_header_row: Vec<String> = Vec::new();
    let mut table_body_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();

    // Task list state
    let mut current_task_checked: Option<bool> = None;

    // Enable extensions
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(content, options);

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    flush_paragraph_with_emojis(&mut current_paragraph, &mut elements);
                    current_heading_level = Some(level as u8);
                }
                Tag::CodeBlock(kind) => {
                    flush_paragraph_with_emojis(&mut current_paragraph, &mut elements);
                    in_code_block = true;
                    code_block_content.clear();
                    code_block_language = match kind {
                        CodeBlockKind::Fenced(lang) => lang.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                }
                Tag::List(_) => {
                    flush_paragraph_with_emojis(&mut current_paragraph, &mut elements);
                    list_depth += 1;
                }
                Tag::Item => {
                    in_list_item = true;
                    list_item_content.clear();
                }
                Tag::Table(_alignments) => {
                    flush_paragraph_with_emojis(&mut current_paragraph, &mut elements);
                    _in_table = true;
                    table_header_row.clear();
                    table_body_rows.clear();
                }
                Tag::TableHead => {
                    in_table_head = true;
                    current_row.clear();
                }
                Tag::TableRow => {
                    _in_table_row = true;
                    current_row.clear();
                }
                Tag::TableCell => {
                    in_table_cell = true;
                    current_cell.clear();
                }
                Tag::Paragraph => {}
                Tag::Strong => {
                    // Insert bold start marker
                    if in_table_cell {
                        current_cell.push(BOLD_START);
                    } else if in_list_item {
                        list_item_content.push(BOLD_START);
                    } else {
                        current_paragraph.push(BOLD_START);
                    }
                }
                Tag::Emphasis => {
                    // Insert italic start marker
                    if in_table_cell {
                        current_cell.push(ITALIC_START);
                    } else if in_list_item {
                        list_item_content.push(ITALIC_START);
                    } else {
                        current_paragraph.push(ITALIC_START);
                    }
                }
                Tag::Strikethrough => {
                    // Insert strikethrough start marker
                    if in_table_cell {
                        current_cell.push(STRIKETHROUGH_START);
                    } else if in_list_item {
                        list_item_content.push(STRIKETHROUGH_START);
                    } else {
                        current_paragraph.push(STRIKETHROUGH_START);
                    }
                }
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    let code_text = std::mem::take(&mut code_block_content);
                    let language = std::mem::take(&mut code_block_language);
                    elements.push(render_code_block(&code_text, &language, theme_is_dark));
                }
                TagEnd::Heading(_) => {
                    if !current_paragraph.is_empty() {
                        let level = current_heading_level.unwrap_or(1);
                        let heading_content = std::mem::take(&mut current_paragraph);
                        elements.push(render_heading(&heading_content, level));
                    }
                    current_heading_level = None;
                }
                TagEnd::Paragraph => {
                    flush_paragraph_with_emojis(&mut current_paragraph, &mut elements);
                }
                TagEnd::List(_) => {
                    list_depth = list_depth.saturating_sub(1);
                }
                TagEnd::Item => {
                    if !list_item_content.is_empty() {
                        let item_content = std::mem::take(&mut list_item_content);
                        elements.push(render_list_item(&item_content, list_depth, current_task_checked));
                    }
                    in_list_item = false;
                    current_task_checked = None; // Reset for next item
                }
                TagEnd::Table => {
                    elements.push(render_table(&table_header_row, &table_body_rows, theme_is_dark));
                    _in_table = false;
                    table_header_row.clear();
                    table_body_rows.clear();
                }
                TagEnd::TableHead => {
                    table_header_row = std::mem::take(&mut current_row);
                    in_table_head = false;
                }
                TagEnd::TableRow => {
                    if !in_table_head && !current_row.is_empty() {
                        table_body_rows.push(std::mem::take(&mut current_row));
                    }
                    _in_table_row = false;
                }
                TagEnd::TableCell => {
                    current_row.push(std::mem::take(&mut current_cell));
                    in_table_cell = false;
                }
                TagEnd::Strong => {
                    // Insert bold end marker
                    if in_table_cell {
                        current_cell.push(BOLD_END);
                    } else if in_list_item {
                        list_item_content.push(BOLD_END);
                    } else {
                        current_paragraph.push(BOLD_END);
                    }
                }
                TagEnd::Emphasis => {
                    // Insert italic end marker
                    if in_table_cell {
                        current_cell.push(ITALIC_END);
                    } else if in_list_item {
                        list_item_content.push(ITALIC_END);
                    } else {
                        current_paragraph.push(ITALIC_END);
                    }
                }
                TagEnd::Strikethrough => {
                    // Insert strikethrough end marker
                    if in_table_cell {
                        current_cell.push(STRIKETHROUGH_END);
                    } else if in_list_item {
                        list_item_content.push(STRIKETHROUGH_END);
                    } else {
                        current_paragraph.push(STRIKETHROUGH_END);
                    }
                }
                _ => {}
            },
            Event::Text(t) => {
                if in_code_block {
                    code_block_content.push_str(&t);
                } else if in_table_cell {
                    current_cell.push_str(&t);
                } else if in_list_item {
                    list_item_content.push_str(&t);
                } else {
                    current_paragraph.push_str(&t);
                }
            }
            Event::Code(code) => {
                // Use special markers for inline code (rendered by render_text_with_emojis)
                let formatted = format!("{}{}{}", INLINE_CODE_START, code, INLINE_CODE_END);
                if in_table_cell {
                    current_cell.push_str(&formatted);
                } else if in_list_item {
                    list_item_content.push_str(&formatted);
                } else {
                    current_paragraph.push_str(&formatted);
                }
            }
            Event::SoftBreak => {
                if in_code_block {
                    code_block_content.push('\n');
                } else if in_table_cell {
                    current_cell.push(' ');
                } else if in_list_item {
                    list_item_content.push(' ');
                } else {
                    current_paragraph.push(' ');
                }
            }
            Event::HardBreak => {
                if in_code_block {
                    code_block_content.push('\n');
                } else if in_table_cell {
                    current_cell.push('\n');
                } else if in_list_item {
                    list_item_content.push('\n');
                } else {
                    flush_paragraph_with_emojis(&mut current_paragraph, &mut elements);
                }
            }
            Event::TaskListMarker(checked) => {
                current_task_checked = Some(checked);
            }
            Event::Rule => {
                flush_paragraph_with_emojis(&mut current_paragraph, &mut elements);
                elements.push(render_horizontal_rule(theme_is_dark));
            }
            _ => {}
        }
    }

    // Flush any remaining content
    flush_paragraph_with_emojis(&mut current_paragraph, &mut elements);

    // If nothing was parsed, render as plain text with emoji support
    if elements.is_empty() {
        elements.push(render_text_with_emojis(content, 14));
    }

    Column::with_children(elements).spacing(6).into()
}

/// Flush a paragraph with emoji rendering support
fn flush_paragraph_with_emojis(
    paragraph: &mut String,
    elements: &mut Vec<Element<'static, Message>>,
) {
    if !paragraph.is_empty() {
        let content = std::mem::take(paragraph);
        elements.push(render_text_with_emojis(&content, 14));
    }
}
