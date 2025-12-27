//! Code Block Renderer with Syntax Highlighting
//!
//! Uses syntect for syntax highlighting and converts to ratatui styles.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

use crate::tui::theme::TuiColors;

/// Render a code block with syntax highlighting
pub fn render_code_block(
    code: &str,
    language: &str,
    colors: &TuiColors,
    _max_width: usize,
) -> Vec<Line<'static>> {
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();

    let theme_name = if colors.is_dark() {
        "base16-ocean.dark"
    } else {
        "base16-ocean.light"
    };

    let theme = theme_set
        .themes
        .get(theme_name)
        .unwrap_or_else(|| theme_set.themes.values().next().unwrap());

    let syntax = syntax_set
        .find_syntax_by_token(language)
        .or_else(|| syntax_set.find_syntax_by_extension(language))
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut lines: Vec<Line> = Vec::new();

    for line in code.lines() {
        let ranges = highlighter.highlight_line(line, &syntax_set);

        match ranges {
            Ok(ranges) => {
                let spans: Vec<Span> = ranges
                    .iter()
                    .map(|(style, text)| {
                        let fg = syntect_color_to_ratatui(style.foreground);
                        let bg = if style.background.a > 0 {
                            Some(syntect_color_to_ratatui(style.background))
                        } else {
                            Some(colors.code_bg)
                        };

                        let mut ratatui_style = Style::default().fg(fg);
                        if let Some(bg_color) = bg {
                            ratatui_style = ratatui_style.bg(bg_color);
                        }

                        if style
                            .font_style
                            .contains(syntect::highlighting::FontStyle::BOLD)
                        {
                            ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
                        }
                        if style
                            .font_style
                            .contains(syntect::highlighting::FontStyle::ITALIC)
                        {
                            ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
                        }

                        Span::styled(text.to_string(), ratatui_style)
                    })
                    .collect();

                lines.push(Line::from(spans));
            }
            Err(_) => {
                lines.push(Line::styled(
                    line.to_string(),
                    Style::default().fg(colors.text_primary).bg(colors.code_bg),
                ));
            }
        }
    }

    lines
}

/// Convert syntect color to ratatui color
fn syntect_color_to_ratatui(color: syntect::highlighting::Color) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}
