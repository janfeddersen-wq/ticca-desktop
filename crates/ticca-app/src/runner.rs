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

/// App icon (SVG source).
const APP_ICON_SVG: &[u8] = include_bytes!("../assets/icons/ticca-desktop.svg");

fn app_window_icon() -> Option<iced::window::Icon> {
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(APP_ICON_SVG, &options).ok()?;

    let icon_size = 256u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(icon_size, icon_size)?;

    let svg_size = tree.size();
    let scale = (icon_size as f32 / svg_size.width()).min(icon_size as f32 / svg_size.height());
    let x = (icon_size as f32 - svg_size.width() * scale) / 2.0;
    let y = (icon_size as f32 - svg_size.height() * scale) / 2.0;

    let transform = resvg::tiny_skia::Transform::from_translate(x, y).post_scale(scale, scale);

    let mut pixmap_mut = pixmap.as_mut();
    resvg::render(&tree, transform, &mut pixmap_mut);

    iced::window::icon::from_rgba(pixmap.data().to_vec(), icon_size, icon_size).ok()
}

/// Run the application with default settings
pub fn run() -> anyhow::Result<()> {
    let window_icon = app_window_icon();
    let mut window_settings = iced::window::Settings {
        icon: window_icon,
        ..Default::default()
    };

    #[cfg(target_os = "linux")]
    {
        // Helps Wayland taskbar/dock match a `.desktop` entry (and thus the correct icon).
        window_settings.platform_specific.application_id = "ticca-desktop".to_string();
    }

    iced::application(TiccaApp::new, TiccaApp::update, TiccaApp::view)
        .title(TiccaApp::title)
        .subscription(TiccaApp::subscription)
        .theme(TiccaApp::theme)
        .window(window_settings)
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
