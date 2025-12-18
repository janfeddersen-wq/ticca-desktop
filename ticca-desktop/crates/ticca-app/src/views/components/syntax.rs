//! Syntax highlighting for code blocks using syntect

use iced::widget::{container, text, Column, Row};
use iced::{Color, Element, Font, Length};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SyntectStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use unicode_segmentation::UnicodeSegmentation;

use super::emoji::{lookup_emoji, render_emoji};
use crate::messages::Message;
use crate::theme::styles;

/// Lazy-loaded syntax highlighting resources
fn get_syntax_set() -> &'static SyntaxSet {
    static SYNTAX_SET: std::sync::OnceLock<SyntaxSet> = std::sync::OnceLock::new();
    SYNTAX_SET.get_or_init(|| SyntaxSet::load_defaults_newlines())
}

fn get_theme_set() -> &'static ThemeSet {
    static THEME_SET: std::sync::OnceLock<ThemeSet> = std::sync::OnceLock::new();
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

/// Convert syntect color to iced Color
fn syntect_to_iced_color(style: SyntectStyle) -> Color {
    Color::from_rgba8(
        style.foreground.r,
        style.foreground.g,
        style.foreground.b,
        style.foreground.a as f32 / 255.0,
    )
}

/// Render text with emojis, applying a color to non-emoji text
fn render_text_with_emojis_colored(
    content: &str,
    color: Color,
    elements: &mut Vec<Element<'static, Message>>,
) {
    let mut current_text = String::new();

    for grapheme in content.graphemes(true) {
        if let Some(asset) = lookup_emoji(grapheme) {
            if !current_text.is_empty() {
                elements.push(
                    text(std::mem::take(&mut current_text))
                        .size(13)
                        .font(Font::with_name("Noto Sans Mono"))
                        .color(color)
                        .into(),
                );
            }
            elements.push(render_emoji(&asset, 14.0));
        } else {
            current_text.push_str(grapheme);
        }
    }

    if !current_text.is_empty() {
        elements.push(
            text(current_text)
                .size(13)
                .font(Font::with_name("Noto Sans Mono"))
                .color(color)
                .into(),
        );
    }
}

/// Get the file extension for syntax lookup
fn get_syntax_extension(lang: &str) -> &str {
    match lang.to_lowercase().as_str() {
        "js" | "javascript" | "jsx" => "js",
        "ts" | "typescript" => "ts",
        "tsx" => "tsx",
        "py" | "python" | "python3" => "py",
        "rb" | "ruby" => "rb",
        "rs" | "rust" => "rs",
        "go" | "golang" => "go",
        "c" => "c",
        "cpp" | "c++" | "cxx" => "cpp",
        "cs" | "csharp" | "c#" => "cs",
        "java" => "java",
        "kt" | "kotlin" => "kt",
        "swift" => "swift",
        "php" => "php",
        "html" | "htm" => "html",
        "xml" | "svg" => "xml",
        "css" => "css",
        "scss" | "sass" => "scss",
        "less" => "less",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "md" | "markdown" => "md",
        "sql" => "sql",
        "sh" | "bash" | "shell" | "zsh" => "sh",
        "ps1" | "powershell" => "ps1",
        "dockerfile" | "docker" => "Dockerfile",
        "makefile" | "make" => "Makefile",
        "diff" | "patch" => "diff",
        "lua" => "lua",
        "perl" | "pl" => "pl",
        "r" => "r",
        "scala" => "scala",
        "clj" | "clojure" => "clj",
        "hs" | "haskell" => "hs",
        "erl" | "erlang" => "erl",
        "ex" | "elixir" => "ex",
        "vim" | "vimscript" => "vim",
        "ini" | "conf" | "config" => "ini",
        "tex" | "latex" => "tex",
        _ => lang,
    }
}

/// Render a code block with syntax highlighting
pub fn render_code_block(code: &str, language: &str, theme_is_dark: bool) -> Element<'static, Message> {
    let syntax_set = get_syntax_set();
    let theme_set = get_theme_set();

    // Choose theme based on dark/light mode
    let theme_name = if theme_is_dark {
        "base16-ocean.dark"
    } else {
        "base16-ocean.light"
    };

    let theme = theme_set
        .themes
        .get(theme_name)
        .unwrap_or_else(|| theme_set.themes.values().next().unwrap());

    // Find syntax by extension (most reliable method)
    let ext = get_syntax_extension(language);
    let syntax = syntax_set
        .find_syntax_by_extension(ext)
        .or_else(|| syntax_set.find_syntax_by_token(language))
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut lines: Vec<Element<'static, Message>> = Vec::new();

    for line in code.lines() {
        match highlighter.highlight_line(line, syntax_set) {
            Ok(highlighted_regions) => {
                let mut line_elements: Vec<Element<'static, Message>> = Vec::new();

                for (style, text_content) in highlighted_regions {
                    let color = syntect_to_iced_color(style);
                    // Render text with emoji support
                    render_text_with_emojis_colored(text_content, color, &mut line_elements);
                }

                if line_elements.is_empty() {
                    lines.push(text(" ").size(13).font(Font::with_name("Noto Sans Mono")).into());
                } else {
                    lines.push(Row::with_children(line_elements).spacing(0).into());
                }
            }
            Err(_) => {
                lines.push(text(line.to_string()).size(13).font(Font::with_name("Noto Sans Mono")).into());
            }
        }
    }

    container(Column::with_children(lines).spacing(0))
        .padding(10)
        .width(Length::Fill)
        .style(move |theme| styles::code_block(theme, theme_is_dark))
        .into()
}
