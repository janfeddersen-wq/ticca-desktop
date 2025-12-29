//! Application runner with fonts and configuration
//!
//! Handles the GPUI application bootstrap and font loading.

use std::borrow::Cow;

use gpui::*;
use gpui_component::Root;

use crate::actions::SwitchTheme;
use crate::app::TiccaApp;
use crate::keybindings;
use crate::theme;

// =============================================================================
// Bundled Fonts
// =============================================================================

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
const NOTO_SANS_SYMBOLS2: &[u8] = include_bytes!("../assets/fonts/NotoSansSymbols2-Regular.ttf");

// =============================================================================
// App Icon
// =============================================================================

/// App icon (SVG source)
const APP_ICON_SVG: &[u8] = include_bytes!("../assets/icons/ticca-desktop.svg");

/// Load and rasterize the app icon from SVG
fn load_app_icon() -> Option<Vec<u8>> {
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

    Some(pixmap.data().to_vec())
}

// =============================================================================
// Font Loading
// =============================================================================

/// Load all bundled fonts into the GPUI text system
fn load_fonts(cx: &App) -> anyhow::Result<()> {
    let font_bytes: Vec<Cow<'static, [u8]>> = vec![
        Cow::Borrowed(NOTO_SANS_REGULAR),
        Cow::Borrowed(NOTO_SANS_BOLD),
        Cow::Borrowed(NOTO_SANS_ITALIC),
        Cow::Borrowed(NOTO_SANS_BOLD_ITALIC),
        Cow::Borrowed(NOTO_SANS_MONO),
        Cow::Borrowed(NOTO_SANS_SYMBOLS),
        Cow::Borrowed(NOTO_SANS_SYMBOLS2),
    ];

    cx.text_system()
        .add_fonts(font_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to load fonts: {:?}", e))?;

    tracing::debug!("Loaded {} bundled fonts", 7);
    Ok(())
}

// =============================================================================
// Global Action Handlers
// =============================================================================

/// Register global action handlers (at the App level)
fn register_global_actions(cx: &mut App) {
    use gpui_component::{Theme, ThemeRegistry};

    // Handle theme switching
    cx.on_action(|action: &SwitchTheme, cx: &mut App| {
        let theme_name = gpui::SharedString::from(action.0.clone());
        if let Some(theme_config) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned() {
            Theme::global_mut(cx).apply_config(&theme_config);
            tracing::info!("Switched to theme: {}", action.0);
        } else {
            tracing::warn!("Theme not found: {}", action.0);
        }
        cx.refresh_windows();
    });
}

// =============================================================================
// Application Entry Point
// =============================================================================

/// Run the GPUI application
pub fn run() -> anyhow::Result<()> {
    // Pre-load the app icon (SVG rasterization)
    let _app_icon = load_app_icon();

    // Create the GPUI application with gpui-component assets
    let app = Application::new().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        // Initialize gpui-component (REQUIRED - sets up themes, icons, etc.)
        gpui_component::init(cx);

        // Initialize our custom themes
        theme::init(cx);

        // Register keybindings
        keybindings::register(cx);

        // Register global action handlers
        register_global_actions(cx);

        // Load custom fonts
        if let Err(e) = load_fonts(cx) {
            tracing::warn!("Failed to load custom fonts: {}", e);
        }

        // Activate the application
        cx.activate(true);

        // Open the main window
        let window_result = cx.open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("Ticca Desktop".into()),
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1200.), px(800.)),
                    cx,
                ))),
                #[cfg(target_os = "linux")]
                app_id: Some("ticca-desktop".to_string()),
                ..Default::default()
            },
            |window, cx| {
                // Create the main app view
                let app_view = cx.new(|cx| TiccaApp::new(window, cx));
                // Wrap in Root (required by gpui-component)
                cx.new(|cx| Root::new(app_view, window, cx))
            },
        );

        match window_result {
            Ok(window) => {
                tracing::info!("Main window opened successfully");
                window
                    .update(cx, |_, window, _| {
                        window.activate_window();
                    })
                    .ok();
            }
            Err(e) => {
                tracing::error!("Failed to open window: {:?}", e);
            }
        }
    });

    Ok(())
}
