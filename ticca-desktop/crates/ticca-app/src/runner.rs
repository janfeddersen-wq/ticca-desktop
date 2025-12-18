//! Application runner with fonts and configuration
//!
//! Handles the Iced application bootstrap and font loading.

use crate::app::TiccaApp;
use crate::material_icons as mi;

/// Bundled Noto Sans font for consistent text rendering
const NOTO_SANS_REGULAR: &[u8] = include_bytes!("../assets/fonts/NotoSans-Regular.ttf");

/// Bundled Noto Sans Bold font
const NOTO_SANS_BOLD: &[u8] = include_bytes!("../assets/fonts/NotoSans-Bold.ttf");

/// Bundled Noto Sans Italic font
const NOTO_SANS_ITALIC: &[u8] = include_bytes!("../assets/fonts/NotoSans-Italic.ttf");

/// Bundled Noto Sans Bold Italic font
const NOTO_SANS_BOLD_ITALIC: &[u8] = include_bytes!("../assets/fonts/NotoSans-BoldItalic.ttf");

/// Bundled Noto Sans Mono font for code blocks
const NOTO_SANS_MONO: &[u8] = include_bytes!("../assets/fonts/NotoSansMono-Regular.ttf");

/// Bundled Noto Sans Symbols font for symbols and special characters
const NOTO_SANS_SYMBOLS: &[u8] = include_bytes!("../assets/fonts/NotoSansSymbols-Regular.ttf");

/// Bundled Noto Sans Symbols 2 font for emoji and extended symbols
/// Note: cosmic-text doesn't support color emoji fonts (CBDT/CBLC), so we use monochrome
const NOTO_SANS_SYMBOLS2: &[u8] = include_bytes!("../assets/fonts/NotoSansSymbols2-Regular.ttf");

/// Run the application with default settings
pub fn run() -> anyhow::Result<()> {
    iced::application(TiccaApp::new, TiccaApp::update, TiccaApp::view)
        .title(TiccaApp::title)
        .subscription(TiccaApp::subscription)
        .theme(TiccaApp::theme)
        // Load bundled fonts - order matters for fallback chain
        .font(NOTO_SANS_REGULAR)
        .font(NOTO_SANS_BOLD)
        .font(NOTO_SANS_ITALIC)
        .font(NOTO_SANS_BOLD_ITALIC)
        .font(NOTO_SANS_MONO)
        .font(NOTO_SANS_SYMBOLS)
        .font(NOTO_SANS_SYMBOLS2)
        // Material Icons font (replaces Bootstrap Icons)
        .font(mi::FONT_BYTES)
        // Set Noto Sans as the default font for consistent rendering
        .default_font(iced::Font::with_name("Noto Sans"))
        .window_size(iced::Size::new(900.0, 700.0))
        .antialiasing(true)
        .run()?;
    Ok(())
}
