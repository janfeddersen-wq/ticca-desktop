//! GPU-native Markdown renderer.
//!
//! Provides efficient markdown parsing and rendering to GPUI elements.
//! Uses pulldown-cmark for parsing and syntect for syntax highlighting.

use gpui::{div, prelude::*, px, Div, SharedString, Styled};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use syntect::{
    highlighting::{Theme as SyntectTheme, ThemeSet},
    parsing::SyntaxSet,
};

use crate::theme::Theme;

/// Pre-rendered markdown content.
///
/// Contains parsed markdown ready for efficient rendering.
#[derive(Debug, Clone)]
pub struct RenderedContent {
    /// Parsed blocks ready for rendering
    pub blocks: Vec<MarkdownBlock>,
}

impl RenderedContent {
    /// Create empty rendered content.
    pub fn empty() -> Self {
        Self { blocks: Vec::new() }
    }

    /// Check if content is empty.
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}

/// A block-level markdown element.
#[derive(Debug, Clone)]
pub enum MarkdownBlock {
    /// Plain paragraph text
    Paragraph(Vec<InlineElement>),
    /// Header (H1-H6)
    Heading {
        level: u8,
        content: Vec<InlineElement>,
    },
    /// Code block with optional language
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    /// Unordered list
    UnorderedList(Vec<Vec<InlineElement>>),
    /// Ordered list
    OrderedList(Vec<Vec<InlineElement>>),
    /// Block quote
    BlockQuote(Vec<MarkdownBlock>),
    /// Horizontal rule
    HorizontalRule,
}

/// Inline text element with styling.
#[derive(Debug, Clone)]
pub enum InlineElement {
    /// Plain text
    Text(String),
    /// Bold text
    Bold(String),
    /// Italic text
    Italic(String),
    /// Strikethrough text
    Strikethrough(String),
    /// Inline code
    Code(String),
    /// Link with text and URL
    Link { text: String, url: String },
    /// Soft break (newline that's not a hard break)
    SoftBreak,
    /// Hard break
    HardBreak,
}

/// Markdown renderer with syntax highlighting support.
pub struct MarkdownRenderer {
    syntax_set: SyntaxSet,
    theme: SyntectTheme,
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownRenderer {
    /// Create a new markdown renderer with default syntax themes.
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        // Use a dark theme by default (Monokai-inspired)
        let theme = theme_set.themes["base16-ocean.dark"].clone();

        Self { syntax_set, theme }
    }

    /// Create renderer with a specific syntect theme.
    pub fn with_theme(theme_name: &str) -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set
            .themes
            .get(theme_name)
            .cloned()
            .unwrap_or_else(|| theme_set.themes["base16-ocean.dark"].clone());

        Self { syntax_set, theme }
    }

    /// Get available syntax highlighting themes.
    pub fn available_themes() -> Vec<&'static str> {
        vec![
            "base16-ocean.dark",
            "base16-eighties.dark",
            "base16-mocha.dark",
            "InspiredGitHub",
            "Solarized (dark)",
            "Solarized (light)",
        ]
    }

    /// Parse markdown into rendered content.
    pub fn render(&self, markdown: &str) -> RenderedContent {
        let options = Options::all();
        let parser = Parser::new_ext(markdown, options);

        let mut blocks = Vec::new();
        let mut state = ParserState::default();

        for event in parser {
            self.process_event(event, &mut state, &mut blocks);
        }

        // Flush any remaining content
        state.flush_paragraph(&mut blocks);

        RenderedContent { blocks }
    }

    fn process_event(
        &self,
        event: Event,
        state: &mut ParserState,
        blocks: &mut Vec<MarkdownBlock>,
    ) {
        match event {
            Event::Start(tag) => self.handle_start_tag(tag, state),
            Event::End(tag) => self.handle_end_tag(tag, state, blocks),
            Event::Text(text) => state.push_text(text.to_string()),
            Event::Code(code) => state.push_inline(InlineElement::Code(code.to_string())),
            Event::SoftBreak => state.push_inline(InlineElement::SoftBreak),
            Event::HardBreak => state.push_inline(InlineElement::HardBreak),
            Event::Rule => {
                state.flush_paragraph(blocks);
                blocks.push(MarkdownBlock::HorizontalRule);
            }
            _ => {} // Ignore other events for now
        }
    }

    fn handle_start_tag(&self, tag: Tag, state: &mut ParserState) {
        match tag {
            Tag::Paragraph => state.in_paragraph = true,
            Tag::Heading { level, .. } => {
                state.in_heading = Some(heading_level_to_u8(level));
            }
            Tag::CodeBlock(kind) => {
                state.in_code_block = true;
                state.code_block_lang = match kind {
                    CodeBlockKind::Fenced(lang) if !lang.is_empty() => Some(lang.to_string()),
                    _ => None,
                };
            }
            Tag::List(start) => {
                state.list_stack.push(ListContext {
                    ordered: start.is_some(),
                    items: Vec::new(),
                });
            }
            Tag::Item => state.current_list_item = Vec::new(),
            Tag::BlockQuote(_) => state.blockquote_depth += 1,
            Tag::Strong => state.style_stack.push(TextStyle::Bold),
            Tag::Emphasis => state.style_stack.push(TextStyle::Italic),
            Tag::Strikethrough => state.style_stack.push(TextStyle::Strikethrough),
            Tag::Link { dest_url, .. } => {
                state.current_link = Some(dest_url.to_string());
            }
            _ => {}
        }
    }

    fn handle_end_tag(&self, tag: TagEnd, state: &mut ParserState, blocks: &mut Vec<MarkdownBlock>) {
        match tag {
            TagEnd::Paragraph => {
                if state.in_paragraph {
                    state.flush_paragraph(blocks);
                }
                state.in_paragraph = false;
            }
            TagEnd::Heading(_level) => {
                if let Some(level) = state.in_heading.take() {
                    let content = std::mem::take(&mut state.inline_buffer);
                    blocks.push(MarkdownBlock::Heading { level, content });
                }
            }
            TagEnd::CodeBlock => {
                let code = std::mem::take(&mut state.text_buffer);
                let language = state.code_block_lang.take();
                blocks.push(MarkdownBlock::CodeBlock { language, code });
                state.in_code_block = false;
            }
            TagEnd::List(_) => {
                if let Some(list_ctx) = state.list_stack.pop() {
                    let block = if list_ctx.ordered {
                        MarkdownBlock::OrderedList(list_ctx.items)
                    } else {
                        MarkdownBlock::UnorderedList(list_ctx.items)
                    };
                    blocks.push(block);
                }
            }
            TagEnd::Item => {
                let item = std::mem::take(&mut state.current_list_item);
                if let Some(list_ctx) = state.list_stack.last_mut() {
                    list_ctx.items.push(item);
                }
            }
            TagEnd::BlockQuote(_) => {
                state.blockquote_depth = state.blockquote_depth.saturating_sub(1);
            }
            TagEnd::Strong => {
                state.style_stack.pop();
            }
            TagEnd::Emphasis => {
                state.style_stack.pop();
            }
            TagEnd::Strikethrough => {
                state.style_stack.pop();
            }
            TagEnd::Link => {
                // Finalize link
                if let Some(url) = state.current_link.take() {
                    let text = std::mem::take(&mut state.text_buffer);
                    state.push_inline(InlineElement::Link { text, url });
                }
            }
            _ => {}
        }
    }

    /// Get syntax definition for a language.
    pub fn get_syntax(&self, lang: &str) -> Option<&syntect::parsing::SyntaxReference> {
        self.syntax_set
            .find_syntax_by_token(lang)
            .or_else(|| self.syntax_set.find_syntax_by_extension(lang))
    }

    /// Reference to syntax set for external use.
    pub fn syntax_set(&self) -> &SyntaxSet {
        &self.syntax_set
    }

    /// Reference to theme for external use.
    pub fn theme(&self) -> &SyntectTheme {
        &self.theme
    }
}

/// Internal parser state for markdown processing.
#[derive(Default)]
struct ParserState {
    in_paragraph: bool,
    in_heading: Option<u8>,
    in_code_block: bool,
    code_block_lang: Option<String>,
    text_buffer: String,
    inline_buffer: Vec<InlineElement>,
    style_stack: Vec<TextStyle>,
    list_stack: Vec<ListContext>,
    current_list_item: Vec<InlineElement>,
    blockquote_depth: usize,
    current_link: Option<String>,
}

impl ParserState {
    fn push_text(&mut self, text: String) {
        if self.in_code_block || self.current_link.is_some() {
            self.text_buffer.push_str(&text);
            return;
        }

        // Apply current style
        let element = if let Some(style) = self.style_stack.last() {
            match style {
                TextStyle::Bold => InlineElement::Bold(text),
                TextStyle::Italic => InlineElement::Italic(text),
                TextStyle::Strikethrough => InlineElement::Strikethrough(text),
            }
        } else {
            InlineElement::Text(text)
        };

        self.push_inline(element);
    }

    fn push_inline(&mut self, element: InlineElement) {
        if !self.list_stack.is_empty() {
            self.current_list_item.push(element);
        } else {
            self.inline_buffer.push(element);
        }
    }

    fn flush_paragraph(&mut self, blocks: &mut Vec<MarkdownBlock>) {
        if !self.inline_buffer.is_empty() {
            let content = std::mem::take(&mut self.inline_buffer);
            blocks.push(MarkdownBlock::Paragraph(content));
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum TextStyle {
    Bold,
    Italic,
    Strikethrough,
}

#[derive(Default)]
struct ListContext {
    ordered: bool,
    items: Vec<Vec<InlineElement>>,
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

// ============================================================================
// GPUI Rendering
// ============================================================================

/// Render markdown content to GPUI elements.
pub fn render_markdown(content: &RenderedContent, theme: &Theme) -> Div {
    let mut container = div().flex().flex_col().gap(px(theme.spacing_sm));

    for block in &content.blocks {
        container = container.child(render_block(block, theme));
    }

    container
}

fn render_block(block: &MarkdownBlock, theme: &Theme) -> Div {
    match block {
        MarkdownBlock::Paragraph(inlines) => render_paragraph(inlines, theme),
        MarkdownBlock::Heading { level, content } => render_heading(*level, content, theme),
        MarkdownBlock::CodeBlock { language, code } => render_code_block(language.as_deref(), code, theme),
        MarkdownBlock::UnorderedList(items) => render_unordered_list(items, theme),
        MarkdownBlock::OrderedList(items) => render_ordered_list(items, theme),
        MarkdownBlock::BlockQuote(blocks) => render_blockquote(blocks, theme),
        MarkdownBlock::HorizontalRule => render_horizontal_rule(theme),
    }
}

fn render_paragraph(inlines: &[InlineElement], theme: &Theme) -> Div {
    let text = inlines_to_string(inlines);
    div()
        .text_size(px(theme.font_size_base))
        .text_color(theme.text_primary)
        .child(text)
}

fn render_heading(level: u8, content: &[InlineElement], theme: &Theme) -> Div {
    let text = inlines_to_string(content);
    let (size, weight) = match level {
        1 => (theme.font_size_xl * 1.5, 700),
        2 => (theme.font_size_xl * 1.25, 600),
        3 => (theme.font_size_xl, 600),
        4 => (theme.font_size_large, 600),
        5 => (theme.font_size_base, 600),
        _ => (theme.font_size_base, 500),
    };

    div()
        .text_size(px(size))
        .font_weight(gpui::FontWeight(weight as f32))
        .text_color(theme.text_primary)
        .mt(px(theme.spacing_md))
        .mb(px(theme.spacing_sm))
        .child(text)
}

fn render_code_block(language: Option<&str>, code: &str, theme: &Theme) -> Div {
    div()
        .w_full()
        .bg(theme.code_background)
        .border_1()
        .border_color(theme.code_border)
        .rounded(px(6.0))
        .p(px(theme.spacing_md))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(theme.spacing_xs))
                .when_some(language, |el, lang| {
                    el.child(
                        div()
                            .text_size(px(theme.font_size_small))
                            .text_color(theme.text_secondary)
                            .child(SharedString::from(lang.to_string())),
                    )
                })
                .child(
                    div()
                        .text_size(px(theme.font_size_code))
                        .text_color(theme.text_primary)
                        .child(SharedString::from(code.to_string())),
                ),
        )
}

fn render_unordered_list(items: &[Vec<InlineElement>], theme: &Theme) -> Div {
    let mut list = div()
        .flex()
        .flex_col()
        .gap(px(theme.spacing_xs))
        .pl(px(theme.spacing_md));

    for item in items {
        let text = inlines_to_string(item);
        list = list.child(
            div()
                .flex()
                .flex_row()
                .gap(px(theme.spacing_sm))
                .child(
                    div()
                        .text_size(px(theme.font_size_base))
                        .text_color(theme.text_secondary)
                        .child("•"),
                )
                .child(
                    div()
                        .text_size(px(theme.font_size_base))
                        .text_color(theme.text_primary)
                        .child(text),
                ),
        );
    }

    list
}

fn render_ordered_list(items: &[Vec<InlineElement>], theme: &Theme) -> Div {
    let mut list = div()
        .flex()
        .flex_col()
        .gap(px(theme.spacing_xs))
        .pl(px(theme.spacing_md));

    for (idx, item) in items.iter().enumerate() {
        let text = inlines_to_string(item);
        list = list.child(
            div()
                .flex()
                .flex_row()
                .gap(px(theme.spacing_sm))
                .child(
                    div()
                        .text_size(px(theme.font_size_base))
                        .text_color(theme.text_secondary)
                        .min_w(px(20.0))
                        .child(SharedString::from(format!("{}.", idx + 1))),
                )
                .child(
                    div()
                        .text_size(px(theme.font_size_base))
                        .text_color(theme.text_primary)
                        .child(text),
                ),
        );
    }

    list
}

fn render_blockquote(blocks: &[MarkdownBlock], theme: &Theme) -> Div {
    let mut quote = div()
        .flex()
        .flex_col()
        .gap(px(theme.spacing_xs))
        .pl(px(theme.spacing_md))
        .border_l_4()
        .border_color(theme.accent)
        .ml(px(theme.spacing_sm));

    for block in blocks {
        quote = quote.child(render_block(block, theme));
    }

    quote
}

fn render_horizontal_rule(theme: &Theme) -> Div {
    div()
        .w_full()
        .h(px(1.0))
        .bg(theme.border)
        .my(px(theme.spacing_md))
}

/// Convert inline elements to a plain string.
/// This is a simplified approach - for full styling support,
/// you'd want to return styled spans.
fn inlines_to_string(inlines: &[InlineElement]) -> SharedString {
    let mut result = String::new();

    for inline in inlines {
        match inline {
            InlineElement::Text(t) => result.push_str(t),
            InlineElement::Bold(t) => result.push_str(t),
            InlineElement::Italic(t) => result.push_str(t),
            InlineElement::Strikethrough(t) => result.push_str(t),
            InlineElement::Code(t) => {
                result.push('`');
                result.push_str(t);
                result.push('`');
            }
            InlineElement::Link { text, .. } => result.push_str(text),
            InlineElement::SoftBreak => result.push(' '),
            InlineElement::HardBreak => result.push('\n'),
        }
    }

    SharedString::from(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_simple_paragraph() {
        let renderer = MarkdownRenderer::new();
        let content = renderer.render("Hello, world!");

        assert_eq!(content.blocks.len(), 1);
        match &content.blocks[0] {
            MarkdownBlock::Paragraph(inlines) => {
                assert!(!inlines.is_empty());
            }
            _ => panic!("Expected paragraph"),
        }
    }

    #[test]
    fn test_render_heading() {
        let renderer = MarkdownRenderer::new();
        let content = renderer.render("# Hello\n\n## World");

        assert_eq!(content.blocks.len(), 2);
        match &content.blocks[0] {
            MarkdownBlock::Heading { level, .. } => assert_eq!(*level, 1),
            _ => panic!("Expected H1"),
        }
        match &content.blocks[1] {
            MarkdownBlock::Heading { level, .. } => assert_eq!(*level, 2),
            _ => panic!("Expected H2"),
        }
    }

    #[test]
    fn test_render_code_block() {
        let renderer = MarkdownRenderer::new();
        let content = renderer.render("```rust\nfn main() {}\n```");

        assert_eq!(content.blocks.len(), 1);
        match &content.blocks[0] {
            MarkdownBlock::CodeBlock { language, code } => {
                assert_eq!(language.as_deref(), Some("rust"));
                assert!(code.contains("fn main"));
            }
            _ => panic!("Expected code block"),
        }
    }

    #[test]
    fn test_render_unordered_list() {
        let renderer = MarkdownRenderer::new();
        let content = renderer.render("- Item 1\n- Item 2\n- Item 3");

        assert_eq!(content.blocks.len(), 1);
        match &content.blocks[0] {
            MarkdownBlock::UnorderedList(items) => {
                assert_eq!(items.len(), 3);
            }
            _ => panic!("Expected unordered list"),
        }
    }

    #[test]
    fn test_render_ordered_list() {
        let renderer = MarkdownRenderer::new();
        let content = renderer.render("1. First\n2. Second\n3. Third");

        assert_eq!(content.blocks.len(), 1);
        match &content.blocks[0] {
            MarkdownBlock::OrderedList(items) => {
                assert_eq!(items.len(), 3);
            }
            _ => panic!("Expected ordered list"),
        }
    }

    #[test]
    fn test_inline_formatting() {
        let renderer = MarkdownRenderer::new();
        let content = renderer.render("This is **bold** and *italic*");

        assert_eq!(content.blocks.len(), 1);
        match &content.blocks[0] {
            MarkdownBlock::Paragraph(inlines) => {
                assert!(inlines.len() >= 3);
            }
            _ => panic!("Expected paragraph"),
        }
    }
}
