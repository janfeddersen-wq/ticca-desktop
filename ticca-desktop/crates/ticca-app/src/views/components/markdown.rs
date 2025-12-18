//! Markdown rendering widget with Twemoji SVG emoji support and table rendering

use iced::widget::{container, text, Column, Row};
use iced::{Color, Element, Font, Length};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use regex::Regex;
use std::sync::LazyLock;
use unicode_segmentation::UnicodeSegmentation;

/// Regex patterns for math processing
static MATH_BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\$\$([^$]+)\$\$").unwrap()
});
static MATH_INLINE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\$([^$]+)\$").unwrap()
});
static SUPERSCRIPT_BRACED_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\^[{\[]([^}\]]+)[}\]]").unwrap()
});
static SUPERSCRIPT_SINGLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\^([0-9n+-])").unwrap()
});
static SUBSCRIPT_BRACED_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"_[{\[]([^}\]]+)[}\]]").unwrap()
});
static SUBSCRIPT_SINGLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"_([0-9n+-])").unwrap()
});

/// Convert a character to its Unicode superscript equivalent if available
fn to_superscript(c: char) -> Option<char> {
    match c {
        '0' => Some('⁰'),
        '1' => Some('¹'),
        '2' => Some('²'),
        '3' => Some('³'),
        '4' => Some('⁴'),
        '5' => Some('⁵'),
        '6' => Some('⁶'),
        '7' => Some('⁷'),
        '8' => Some('⁸'),
        '9' => Some('⁹'),
        '+' => Some('⁺'),
        '-' => Some('⁻'),
        '=' => Some('⁼'),
        '(' => Some('⁽'),
        ')' => Some('⁾'),
        'n' => Some('ⁿ'),
        'i' => Some('ⁱ'),
        _ => None,
    }
}

/// Convert a character to its Unicode subscript equivalent if available
fn to_subscript(c: char) -> Option<char> {
    match c {
        '0' => Some('₀'),
        '1' => Some('₁'),
        '2' => Some('₂'),
        '3' => Some('₃'),
        '4' => Some('₄'),
        '5' => Some('₅'),
        '6' => Some('₆'),
        '7' => Some('₇'),
        '8' => Some('₈'),
        '9' => Some('₉'),
        '+' => Some('₊'),
        '-' => Some('₋'),
        '=' => Some('₌'),
        '(' => Some('₍'),
        ')' => Some('₎'),
        'a' => Some('ₐ'),
        'e' => Some('ₑ'),
        'o' => Some('ₒ'),
        'x' => Some('ₓ'),
        'h' => Some('ₕ'),
        'k' => Some('ₖ'),
        'l' => Some('ₗ'),
        'm' => Some('ₘ'),
        'n' => Some('ₙ'),
        'p' => Some('ₚ'),
        's' => Some('ₛ'),
        't' => Some('ₜ'),
        _ => None,
    }
}

/// Convert a string to superscript Unicode characters (only if ALL can be converted)
fn string_to_superscript(s: &str) -> Option<String> {
    let converted: Option<String> = s.chars()
        .map(|c| to_superscript(c))
        .collect();
    converted
}

/// Convert a string to subscript Unicode characters (only if ALL can be converted)
fn string_to_subscript(s: &str) -> Option<String> {
    let converted: Option<String> = s.chars()
        .map(|c| to_subscript(c))
        .collect();
    converted
}

/// Pre-process content to convert math notation to Unicode
fn preprocess_math(content: &str) -> String {
    let mut result = content.to_string();

    // Process block math $$...$$ - just strip the delimiters and convert
    result = MATH_BLOCK_RE.replace_all(&result, |caps: &regex::Captures| {
        process_math_content(&caps[1])
    }).to_string();

    // Process inline math $...$ - strip delimiters and convert
    result = MATH_INLINE_RE.replace_all(&result, |caps: &regex::Captures| {
        process_math_content(&caps[1])
    }).to_string();

    // Also process bare super/subscripts outside of math delimiters
    result = process_math_content(&result);

    result
}

/// Extract content from braces, handling nested braces
fn extract_braced_content(s: &str, start: usize) -> Option<(String, usize)> {
    if s.as_bytes().get(start) != Some(&b'{') {
        return None;
    }
    let mut depth = 0;
    let mut content_start = start + 1;
    for (i, c) in s[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((s[content_start..start + i].to_string(), start + i + 1));
                }
            }
            _ => {}
        }
    }
    None
}

/// Process math content, converting super/subscripts to Unicode
fn process_math_content(content: &str) -> String {
    let mut result = content.to_string();

    // First, clean up common LaTeX commands (do this before super/subscript conversion)
    result = result
        // Operators and relations
        .replace("\\cdots", "⋯")
        .replace("\\ldots", "…")
        .replace("\\times", "×")
        .replace("\\cdot", "·")
        .replace("\\div", "÷")
        .replace("\\pm", "±")
        .replace("\\mp", "∓")
        .replace("\\leq", "≤")
        .replace("\\geq", "≥")
        .replace("\\neq", "≠")
        .replace("\\approx", "≈")
        .replace("\\equiv", "≡")
        .replace("\\sim", "∼")
        .replace("\\propto", "∝")
        .replace("\\infty", "oo")
        .replace("\\partial", "d")
        .replace("\\nabla", "V")
        .replace("\\forall", "for all ")
        .replace("\\exists", "exists ")
        .replace("\\in", " in ")
        .replace("\\notin", " not in ")
        .replace("\\subset", " subset ")
        .replace("\\supset", " superset ")
        .replace("\\cup", " u ")
        .replace("\\cap", " n ")
        .replace("\\emptyset", "{}")
        // Big operators
        .replace("\\sum", "Σ")
        .replace("\\prod", "Π")
        .replace("\\int", "∫")
        .replace("\\iint", "∬")
        .replace("\\oint", "∮")
        .replace("\\lim", "lim")
        // Greek letters (lowercase)
        .replace("\\alpha", "α")
        .replace("\\beta", "β")
        .replace("\\gamma", "γ")
        .replace("\\delta", "δ")
        .replace("\\epsilon", "ε")
        .replace("\\zeta", "ζ")
        .replace("\\eta", "η")
        .replace("\\theta", "θ")
        .replace("\\iota", "ι")
        .replace("\\kappa", "κ")
        .replace("\\lambda", "λ")
        .replace("\\mu", "μ")
        .replace("\\nu", "ν")
        .replace("\\xi", "ξ")
        .replace("\\pi", "π")
        .replace("\\rho", "ρ")
        .replace("\\sigma", "σ")
        .replace("\\tau", "τ")
        .replace("\\upsilon", "υ")
        .replace("\\phi", "φ")
        .replace("\\chi", "χ")
        .replace("\\psi", "ψ")
        .replace("\\omega", "ω")
        // Greek letters (uppercase)
        .replace("\\Gamma", "Γ")
        .replace("\\Delta", "Δ")
        .replace("\\Theta", "Θ")
        .replace("\\Lambda", "Λ")
        .replace("\\Xi", "Ξ")
        .replace("\\Pi", "Π")
        .replace("\\Sigma", "Σ")
        .replace("\\Phi", "Φ")
        .replace("\\Psi", "Ψ")
        .replace("\\Omega", "Ω")
        // Arrows
        .replace("\\rightarrow", "->")
        .replace("\\leftarrow", "<-")
        .replace("\\leftrightarrow", "<->")
        .replace("\\Rightarrow", "=>")
        .replace("\\Leftarrow", "<=")
        .replace("\\Leftrightarrow", "<=>")
        .replace("\\to", "->")
        .replace("\\xrightarrow", "->")
        // Misc
        .replace("\\hbar", "h")
        .replace("\\ell", "l")
        .replace("\\Re", "Re")
        .replace("\\Im", "Im")
        // Delimiters - \left and \right just modify sizing, remove them
        .replace("\\left(", "(")
        .replace("\\right)", ")")
        .replace("\\left[", "[")
        .replace("\\right]", "]")
        .replace("\\left\\{", "{")
        .replace("\\right\\}", "}")
        .replace("\\left|", "|")
        .replace("\\right|", "|")
        .replace("\\left", "")
        .replace("\\right", "")
        // Line breaks
        .replace("\\\\", " ")
        .replace("\\newline", " ")
        // Spacing commands
        .replace("\\,", " ")
        .replace("\\;", " ")
        .replace("\\:", " ")
        .replace("\\!", "")
        .replace("\\quad", "  ")
        .replace("\\qquad", "    ");

    // Handle \frac{a}{b} -> (a)/(b) - with nested brace support
    while let Some(pos) = result.find("\\frac{") {
        // Find first braced content
        if let Some((num, after_num)) = extract_braced_content(&result, pos + 5) {
            // Find second braced content
            if let Some((den, after_den)) = extract_braced_content(&result, after_num) {
                let replacement = format!("({})/({})", num, den);
                result = format!("{}{}{}", &result[..pos], replacement, &result[after_den..]);
                continue;
            }
        }
        // If we can't parse it properly, just remove \frac and continue
        result = result.replacen("\\frac", "frac", 1);
    }

    // Handle \sqrt{x} -> sqrt(x)
    while let Some(pos) = result.find("\\sqrt{") {
        if let Some((content, after)) = extract_braced_content(&result, pos + 5) {
            let replacement = format!("sqrt({})", content);
            result = format!("{}{}{}", &result[..pos], replacement, &result[after..]);
        } else {
            result = result.replacen("\\sqrt", "sqrt", 1);
        }
    }

    // Handle \mathrm{...}, \text{...}, \mathbf{...} etc - just extract content
    for cmd in &["\\mathrm", "\\text", "\\mathbf", "\\mathit", "\\mathsf", "\\textrm", "\\textbf"] {
        while let Some(pos) = result.find(&format!("{}{{", cmd)) {
            if let Some((content, after)) = extract_braced_content(&result, pos + cmd.len()) {
                result = format!("{}{}{}", &result[..pos], content, &result[after..]);
            } else {
                result = result.replacen(cmd, "", 1);
            }
        }
    }

    // Handle \hat{x} -> x̂, \bar{x} -> x̄, \vec{x} -> x⃗, \tilde{x} -> x̃
    for (cmd, combining) in &[("\\hat", "\u{0302}"), ("\\bar", "\u{0304}"), ("\\vec", "\u{20D7}"), ("\\tilde", "\u{0303}")] {
        while let Some(pos) = result.find(&format!("{}{{", cmd)) {
            if let Some((content, after)) = extract_braced_content(&result, pos + cmd.len()) {
                let replacement = format!("{}{}", content, combining);
                result = format!("{}{}{}", &result[..pos], replacement, &result[after..]);
            } else {
                result = result.replacen(cmd, "", 1);
            }
        }
    }

    // Handle chemistry \ce{...} - just extract and clean up
    while let Some(pos) = result.find("\\ce{") {
        if let Some((content, after)) = extract_braced_content(&result, pos + 3) {
            // Clean up chemistry notation: -> becomes →, <=> becomes ⇌
            let chem = content
                .replace("->", "→")
                .replace("<->", "↔")
                .replace("<=>", "⇌");
            result = format!("{}{}{}", &result[..pos], chem, &result[after..]);
        } else {
            result = result.replacen("\\ce", "", 1);
        }
    }

    // Handle \begin{...} and \end{...} environments - strip them
    let begin_re = Regex::new(r"\\begin\{[^}]*\}").unwrap();
    result = begin_re.replace_all(&result, "").to_string();
    let end_re = Regex::new(r"\\end\{[^}]*\}").unwrap();
    result = end_re.replace_all(&result, "").to_string();

    // Convert ^{...} or ^[...] to superscript (only if all chars can be converted)
    result = SUPERSCRIPT_BRACED_RE.replace_all(&result, |caps: &regex::Captures| {
        string_to_superscript(&caps[1]).unwrap_or_else(|| format!("^({})", &caps[1]))
    }).to_string();

    // Convert ^n (single character) to superscript
    result = SUPERSCRIPT_SINGLE_RE.replace_all(&result, |caps: &regex::Captures| {
        string_to_superscript(&caps[1]).unwrap_or_else(|| format!("^{}", &caps[1]))
    }).to_string();

    // Convert _{...} or _[...] to subscript (only if all chars can be converted)
    result = SUBSCRIPT_BRACED_RE.replace_all(&result, |caps: &regex::Captures| {
        string_to_subscript(&caps[1]).unwrap_or_else(|| format!("_({})", &caps[1]))
    }).to_string();

    // Convert _n (single character) to subscript
    result = SUBSCRIPT_SINGLE_RE.replace_all(&result, |caps: &regex::Captures| {
        string_to_subscript(&caps[1]).unwrap_or_else(|| format!("_{}", &caps[1]))
    }).to_string();

    // Clean up remaining backslashes from unknown LaTeX commands with braces
    let cmd_with_braces_re = Regex::new(r"\\([a-zA-Z]+)\{([^}]*)\}").unwrap();
    result = cmd_with_braces_re.replace_all(&result, "$2").to_string();

    // Clean up remaining backslashes from unknown LaTeX commands without braces
    let unknown_cmd_re = Regex::new(r"\\([a-zA-Z]+)").unwrap();
    result = unknown_cmd_re.replace_all(&result, "$1").to_string();

    // Clean up any remaining empty braces
    result = result.replace("{}", "");

    result
}

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
fn render_text_with_emojis(content: &str, font_size: f32) -> Element<'static, Message> {
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
        text("").size(font_size as u32).into()
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
    font_size: f32,
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
                    .size((font_size - 1.0).max(10.0))
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
        1 => 22.0,
        2 => 18.0,
        3 => 16.0,
        _ => 15.0,
    };

    render_text_with_emojis(content, size)
}

/// Render a list item with emoji and inline formatting support
fn render_list_item(
    content: &str,
    depth: usize,
    task_checked: Option<bool>,
    ordered_number: Option<u64>,
) -> Element<'static, Message> {
    let indent = "  ".repeat(depth.saturating_sub(1));

    let mut elements: Vec<Element<'static, Message>> = Vec::new();

    // Add indent
    if !indent.is_empty() {
        elements.push(text(indent).size(14.0).into());
    }

    // Add bullet, number, or checkbox
    match task_checked {
        Some(true) => {
            // Checked task - green filled checkbox
            elements.push(
                text(icons::CHECK_SQUARE_FILL.to_string())
                    .size(14.0)
                    .font(ICONS_FONT)
                    .color(Color::from_rgb(0.2, 0.7, 0.3))
                    .into(),
            );
            elements.push(text(" ").size(14.0).into());
        }
        Some(false) => {
            // Unchecked task - gray empty square
            elements.push(
                text(icons::SQUARE.to_string())
                    .size(14.0)
                    .font(ICONS_FONT)
                    .color(Color::from_rgb(0.5, 0.5, 0.5))
                    .into(),
            );
            elements.push(text(" ").size(14.0).into());
        }
        None => {
            // Ordered (numbered) or unordered (bullet) list
            if let Some(num) = ordered_number {
                elements.push(text(format!("{}. ", num)).size(14.0).into());
            } else {
                elements.push(text("• ").size(14.0).into());
            }
        }
    }

    // Add content with emoji and inline formatting support
    elements.push(render_text_with_emojis(content.trim(), 14.0));

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
                container(render_text_with_emojis(cell, 13.0))
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
                container(render_text_with_emojis(cell, 13.0))
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
    // Pre-process math notation to Unicode
    let content = preprocess_math(content);

    let mut elements: Vec<Element<'static, Message>> = Vec::new();
    let mut current_paragraph = String::new();
    let mut in_code_block = false;
    let mut code_block_content = String::new();
    let mut code_block_language = String::new();
    let mut current_heading_level: Option<u8> = None;

    // List state - use a stack to handle nested lists properly
    // Each entry: (content, is_ordered, current_item_number, task_checked)
    let mut list_stack: Vec<(String, bool, u64, Option<bool>)> = Vec::new();

    // Table state
    let mut _in_table = false;
    let mut in_table_head = false;
    let mut _in_table_row = false;
    let mut in_table_cell = false;
    let mut table_header_row: Vec<String> = Vec::new();
    let mut table_body_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();

    // Enable extensions
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;
    let parser = Parser::new_ext(&content, options);

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
                Tag::List(start_number) => {
                    flush_paragraph_with_emojis(&mut current_paragraph, &mut elements);
                    // Push a new list level onto the stack
                    // start_number is Some(n) for ordered lists, None for unordered
                    let is_ordered = start_number.is_some();
                    let start = start_number.unwrap_or(0);
                    list_stack.push((String::new(), is_ordered, start, None));
                }
                Tag::Item => {
                    // Starting a new item - content will be collected in the top stack entry
                    if let Some(entry) = list_stack.last_mut() {
                        entry.0.clear(); // Clear content for new item
                        entry.3 = None;  // Reset task_checked
                    }
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
                    } else if let Some(entry) = list_stack.last_mut() {
                        entry.0.push(BOLD_START);
                    } else {
                        current_paragraph.push(BOLD_START);
                    }
                }
                Tag::Emphasis => {
                    // Insert italic start marker
                    if in_table_cell {
                        current_cell.push(ITALIC_START);
                    } else if let Some(entry) = list_stack.last_mut() {
                        entry.0.push(ITALIC_START);
                    } else {
                        current_paragraph.push(ITALIC_START);
                    }
                }
                Tag::Strikethrough => {
                    // Insert strikethrough start marker
                    if in_table_cell {
                        current_cell.push(STRIKETHROUGH_START);
                    } else if let Some(entry) = list_stack.last_mut() {
                        entry.0.push(STRIKETHROUGH_START);
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
                    // Pop the list level from the stack
                    list_stack.pop();
                }
                TagEnd::Item => {
                    // Render the current item from the top of the stack
                    let depth = list_stack.len();
                    if let Some(entry) = list_stack.last_mut() {
                        if !entry.0.is_empty() {
                            let item_content = std::mem::take(&mut entry.0);
                            let task_checked = entry.3;
                            let ordered_number = if entry.1 {
                                let num = entry.2;
                                entry.2 += 1; // Increment for next item
                                Some(num)
                            } else {
                                None
                            };
                            elements.push(render_list_item(&item_content, depth, task_checked, ordered_number));
                        }
                    }
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
                    } else if let Some(entry) = list_stack.last_mut() {
                        entry.0.push(BOLD_END);
                    } else {
                        current_paragraph.push(BOLD_END);
                    }
                }
                TagEnd::Emphasis => {
                    // Insert italic end marker
                    if in_table_cell {
                        current_cell.push(ITALIC_END);
                    } else if let Some(entry) = list_stack.last_mut() {
                        entry.0.push(ITALIC_END);
                    } else {
                        current_paragraph.push(ITALIC_END);
                    }
                }
                TagEnd::Strikethrough => {
                    // Insert strikethrough end marker
                    if in_table_cell {
                        current_cell.push(STRIKETHROUGH_END);
                    } else if let Some(entry) = list_stack.last_mut() {
                        entry.0.push(STRIKETHROUGH_END);
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
                } else if let Some(entry) = list_stack.last_mut() {
                    entry.0.push_str(&t);
                } else {
                    current_paragraph.push_str(&t);
                }
            }
            Event::Code(code) => {
                // Use special markers for inline code (rendered by render_text_with_emojis)
                let formatted = format!("{}{}{}", INLINE_CODE_START, code, INLINE_CODE_END);
                if in_table_cell {
                    current_cell.push_str(&formatted);
                } else if let Some(entry) = list_stack.last_mut() {
                    entry.0.push_str(&formatted);
                } else {
                    current_paragraph.push_str(&formatted);
                }
            }
            Event::SoftBreak => {
                if in_code_block {
                    code_block_content.push('\n');
                } else if in_table_cell {
                    current_cell.push(' ');
                } else if let Some(entry) = list_stack.last_mut() {
                    entry.0.push(' ');
                } else {
                    current_paragraph.push(' ');
                }
            }
            Event::HardBreak => {
                if in_code_block {
                    code_block_content.push('\n');
                } else if in_table_cell {
                    current_cell.push('\n');
                } else if let Some(entry) = list_stack.last_mut() {
                    entry.0.push('\n');
                } else {
                    flush_paragraph_with_emojis(&mut current_paragraph, &mut elements);
                }
            }
            Event::TaskListMarker(checked) => {
                // Store the task checked state in the current list entry
                if let Some(entry) = list_stack.last_mut() {
                    entry.3 = Some(checked);
                }
            }
            Event::Rule => {
                flush_paragraph_with_emojis(&mut current_paragraph, &mut elements);
                elements.push(render_horizontal_rule(theme_is_dark));
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                // Handle common HTML elements by stripping tags and keeping content
                let html_str = html.to_string();

                // Skip certain block-level HTML tags entirely
                if html_str.starts_with("<details")
                    || html_str.starts_with("</details")
                    || html_str.starts_with("<summary")
                    || html_str.starts_with("</summary")
                {
                    // These are structural, skip them
                    continue;
                }

                // Handle <kbd> tags - render as inline code style
                if html_str.starts_with("<kbd>") {
                    current_paragraph.push(INLINE_CODE_START);
                } else if html_str.starts_with("</kbd>") {
                    current_paragraph.push(INLINE_CODE_END);
                } else if !html_str.starts_with('<') || html_str.starts_with("<!") {
                    // Not a tag, probably content - add it
                    current_paragraph.push_str(&html_str);
                }
                // Other HTML tags are silently ignored
            }
            Event::FootnoteReference(name) => {
                // Render footnote reference as superscript-like text
                let footnote_ref = format!("[{}]", name);
                if let Some(entry) = list_stack.last_mut() {
                    entry.0.push_str(&footnote_ref);
                } else {
                    current_paragraph.push_str(&footnote_ref);
                }
            }
            _ => {}
        }
    }

    // Flush any remaining content
    flush_paragraph_with_emojis(&mut current_paragraph, &mut elements);

    // If nothing was parsed, render as plain text with emoji support
    if elements.is_empty() {
        elements.push(render_text_with_emojis(&content, 14.0));
    }

    Column::with_children(elements).spacing(12).into()
}

/// Flush a paragraph with emoji rendering support
fn flush_paragraph_with_emojis(
    paragraph: &mut String,
    elements: &mut Vec<Element<'static, Message>>,
) {
    if !paragraph.is_empty() {
        let content = std::mem::take(paragraph);
        elements.push(render_text_with_emojis(&content, 14.0));
    }
}
