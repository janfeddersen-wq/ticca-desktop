# Color emoji rendering in Iced 0.13 remains problematic despite claimed support

**Iced 0.13 theoretically supports color emojis through cosmic-text's swash backend, but critical bugs prevent reliable rendering of common emoji fonts like NotoColorEmoji.** The most practical workaround is replacing emoji codepoints with inline SVG images using the `twemoji-assets` crate, since the underlying font rendering issues remain unresolved in cosmic-text.

## cosmic-text claims support but has documented failures

The architecture looks promising on paper: Iced uses **glyphon → cosmic-text → swash** for text rendering, and swash claims full emoji support for COLR/CPAL (Microsoft), CBDT/CBLC (Google/Android), and sbix (Apple) formats. The cosmic-text README explicitly states "Rendering is provided by swash, which supports ligatures and color emoji."

However, **GitHub Issue #310** in pop-os/cosmic-text reveals that NotoColorEmoji.ttf (the most common Linux emoji font using CBDT/CBLC format) returns empty images with incorrect metadata:

```
image.source = Outline        // Should be ColorBitmap
image.content = Mask          // Should be Color
image.placement = { width: 2, height: 0 }
image.data.len() = 0          // Empty!
```

The font is being misidentified as an outline font rather than a color bitmap font. This issue remains open as of December 2025.

## Font fallback creates an impossible conflict

A more fundamental problem exists in cosmic-text's font fallback system. **Issue #327 and PR #328** document a critical conflict: placing "Noto Color Emoji" in the fallback list breaks rendering in both positions. If placed above "Noto Sans," the emoji font gets used for all glyphs including regular text, breaking typography completely. If placed below "DejaVu Sans," emojis don't render properly.

Iced maintainer @hecrj acknowledged the need for "a separate emoji fallback list and some kind of emoji detection during font selection" — but this fix hasn't been implemented. The current architecture has no mechanism to distinguish emoji characters from regular text during font selection.

## Platform-specific inconsistencies compound the problem

**Issue #210** documents Windows 11 users seeing black-and-white emoji glyphs instead of color versions, while Linux and macOS users with the same code see correct rendering. The suspected cause is cosmic-text selecting `SwashContent::Mask` instead of `SwashContent::Color`, or selecting a monochrome emoji font instead of a color one. COSMIC desktop applications like cosmic-term exhibit the same issues — emojis render inconsistently, sometimes using old B&W fonts, sometimes color but rendered very small.

## Alternative text backends don't exist for Iced

Iced is tightly coupled to cosmic-text through the glyphon integration introduced in PR #1697 (Iced 0.10). There's no supported way to swap in a different text rendering backend. You could theoretically use `iced::widget::canvas` for custom text rendering with full control, or integrate wgpu_glyph through a custom shader pipeline, but both approaches require substantial implementation effort and abandon Iced's built-in text widgets.

## The practical solution is Twemoji SVG replacement

Since Iced's `rich_text()` widget doesn't support inline images (only text styling via `text::Span`), the most reliable approach is detecting emoji codepoints and rendering them as separate SVG widgets in a Row layout:

```rust
use iced::widget::{row, text, svg, Row, Element};
use twemoji_assets::svg::SvgTwemojiAsset;
use unicode_segmentation::UnicodeSegmentation;

fn emoji_text<'a, Message: 'a>(content: &str) -> Row<'a, Message> {
    let mut elements: Vec<Element<'a, Message>> = Vec::new();
    let mut current_text = String::new();
    
    for grapheme in content.graphemes(true) {
        if let Some(emoji_asset) = SvgTwemojiAsset::from_emoji(grapheme) {
            if !current_text.is_empty() {
                elements.push(text(std::mem::take(&mut current_text)).into());
            }
            let svg_handle = svg::Handle::from_memory(emoji_asset.as_bytes());
            elements.push(svg(svg_handle).width(20).height(20).into());
        } else {
            current_text.push_str(grapheme);
        }
    }
    if !current_text.is_empty() {
        elements.push(text(current_text).into());
    }
    row(elements).spacing(2)
}
```

Required dependencies:
```toml
twemoji-assets = { version = "1.4", features = ["svg"] }
unicode-segmentation = "1.12"
iced = { version = "0.13", features = ["svg"] }
```

This approach mirrors how `egui_twemoji` handles the same problem in the egui ecosystem, and provides consistent cross-platform rendering independent of system fonts.

## SVG-in-OpenType fonts are not supported

The swash crate doesn't mention SVG-in-OpenType support, and no evidence suggests this format works with cosmic-text. The supported formats are **COLR/CPAL** (vector layers), **CBDT/CBLC** (bitmaps), and **sbix** (Apple bitmaps) — though as documented, CBDT/CBLC has rendering bugs.

If you want to try native font rendering, COLR/CPAL fonts like Windows' Segoe UI Emoji may work better than CBDT/CBLC fonts like NotoColorEmoji. You can load fonts explicitly:

```rust
iced::application("Chat", App::update, App::view)
    .font(include_bytes!("path/to/emoji_font.ttf"))
    .run()
```

And use advanced shaping: `text("Hello 👋").shaping(text::Shaping::Advanced)`

## Summary of approaches by complexity

| Approach | Viability | Notes |
|----------|-----------|-------|
| Native cosmic-text rendering | Unreliable | Documented bugs with NotoColorEmoji, font fallback conflicts |
| Twemoji SVG replacement | Recommended | Consistent cross-platform, moderate implementation effort |
| Custom EmojiText widget | High effort | Full control but requires implementing Widget trait |
| Alternative text backend | Not supported | Iced is tightly coupled to cosmic-text |
| WebView hybrid | Overkill | Consider only for very complex rich text needs |

## Conclusion

For your chat application with markdown rendering, implement the Twemoji SVG replacement approach. Parse your markdown content, detect emoji sequences using `unicode-segmentation`, and render them as inline SVG elements from `twemoji-assets`. This bypasses cosmic-text's font rendering entirely for emojis while keeping regular text rendering through the standard pipeline. The COSMIC desktop team faces the same limitations — there's no special solution they've implemented that you're missing.