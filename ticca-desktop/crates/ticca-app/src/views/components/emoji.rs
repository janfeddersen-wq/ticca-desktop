//! Emoji rendering support using Twemoji SVG assets

use iced::widget::svg;
use iced::Element;
use twemoji_assets::svg::SvgTwemojiAsset;

use crate::messages::Message;

/// Default emoji size in pixels
pub const EMOJI_SIZE: f32 = 18.0;

/// Look up an emoji asset, handling variation selectors
/// Returns a reference to the static twemoji asset
pub fn lookup_emoji(grapheme: &str) -> Option<&'static SvgTwemojiAsset> {
    // Try direct lookup first
    SvgTwemojiAsset::from_emoji(grapheme).or_else(|| {
        // If not found, try stripping variation selectors (U+FE0E text, U+FE0F emoji)
        let stripped: String = grapheme
            .chars()
            .filter(|c| *c != '\u{FE0E}' && *c != '\u{FE0F}')
            .collect();
        if stripped != grapheme && !stripped.is_empty() {
            SvgTwemojiAsset::from_emoji(&stripped)
        } else {
            None
        }
    })
}

/// Render an emoji asset as an SVG element with specified size
pub fn render_emoji(asset: &SvgTwemojiAsset, size: f32) -> Element<'static, Message> {
    let svg_data: &str = asset;
    let svg_handle = svg::Handle::from_memory(svg_data.as_bytes().to_vec());
    svg(svg_handle).width(size).height(size).into()
}
