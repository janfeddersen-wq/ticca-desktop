//! Ticca Desktop - GPU-accelerated AI agent application
//!
//! This is the main entry point for the GPUI-based frontend,
//! targeting 120FPS smooth UI performance.
//!
//! ## Architecture
//!
//! The application follows a strict separation of concerns:
//! - **UI Thread**: Only handles rendering and user input
//! - **Background Tasks**: All AI/DB operations via tokio
//! - **Message Passing**: Async communication between threads
//!
//! ## Performance Budget
//!
//! - Target frame rate: 120FPS (8.3ms per frame)
//! - UI thread must never block on I/O
//! - All async operations offloaded to background tokio tasks

use anyhow::Result;
use gpui::{
    actions, div, App, AppContext, Application, Context, Menu, MenuItem, ParentElement,
    Render, Styled, Window, WindowOptions,
};
use tracing::{error, info};
use tracing_subscriber::{fmt, EnvFilter};

mod app;
mod components;
mod theme;

use app::{AppConfig, RootView, TiccaApp};
pub use theme::Theme;

// =============================================================================
// Actions
// =============================================================================

actions!(
    app,
    [
        Quit,
        NewConversation,
        ToggleSidebar,
        ToggleTheme,
        OpenSettings,
        SendMessage,
    ]
);

// =============================================================================
// Initialization
// =============================================================================

/// Initialize the tracing subscriber for logging.
///
/// Configures structured logging with:
/// - Environment-based filter (RUST_LOG)
/// - File and line number information
/// - Thread IDs for debugging
fn init_tracing() -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,ticca_ui=debug"));

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    Ok(())
}

/// Initialize the background tokio runtime.
///
/// Creates a multi-threaded runtime for handling:
/// - AI agent requests
/// - Database operations
/// - File I/O
/// - Network requests
fn init_tokio_runtime() -> Result<tokio::runtime::Runtime> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("ticca-worker")
        .enable_all()
        .build()?;

    info!("Tokio runtime initialized with 4 worker threads");
    Ok(runtime)
}

/// Build the application menu bar.
fn build_menus() -> Vec<Menu> {
    vec![
        Menu {
            name: "Ticca".into(),
            items: vec![
                MenuItem::action("About Ticca", Quit), // TODO: Add About action
                MenuItem::separator(),
                MenuItem::action("Settings...", OpenSettings),
                MenuItem::separator(),
                MenuItem::action("Quit Ticca", Quit),
            ],
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New Conversation", NewConversation),
                MenuItem::separator(),
                MenuItem::action("Export Conversation...", Quit), // TODO
            ],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Toggle Sidebar", ToggleSidebar),
                MenuItem::action("Toggle Theme", ToggleTheme),
            ],
        },
    ]
}

// =============================================================================
// Main Entry Point
// =============================================================================

/// Main application entry point.
///
/// Initializes all subsystems in order:
/// 1. Tracing/logging
/// 2. Tokio runtime (background)
/// 3. GPUI application
/// 4. Main window
fn main() -> Result<()> {
    // Initialize tracing first for logging
    init_tracing()?;
    info!("Starting Ticca Desktop v{}", env!("CARGO_PKG_VERSION"));

    // Initialize tokio runtime for background tasks
    let runtime = init_tokio_runtime()?;
    let runtime_handle = runtime.handle().clone();

    // Initialize GPUI application
    Application::new().run(move |cx: &mut App| {
        // Set up menus
        cx.set_menus(build_menus());

        // Register global actions
        register_global_actions(cx);

        // Create application configuration
        let config = AppConfig::default();

        // Create the main application state
        let app = TiccaApp::new()
            .with_config(config)
            .with_runtime(runtime_handle.clone());

        // Create the main window
        let window_options = WindowOptions {
            // Window decorations handled by GPUI
            ..Default::default()
        };

        match cx.open_window(window_options, |_window: &mut Window, cx: &mut App| {
            cx.new(|_cx| RootView::new(app))
        }) {
            Ok(_) => {
                info!("Main window created successfully");
            }
            Err(e) => {
                error!("Failed to create main window: {}", e);
            }
        }

        info!("Ticca Desktop initialized successfully");
    });

    info!("Ticca Desktop shutting down");
    Ok(())
}

/// Register global actions that work across the entire application.
fn register_global_actions(cx: &mut App) {
    // Quit action
    cx.on_action(|_: &Quit, cx: &mut App| {
        info!("Quit action triggered");
        cx.quit();
    });

    // New conversation action
    cx.on_action(|_: &NewConversation, _cx: &mut App| {
        info!("New conversation action triggered");
        // TODO: Dispatch to focused window
    });

    // Toggle sidebar action
    cx.on_action(|_: &ToggleSidebar, _cx: &mut App| {
        info!("Toggle sidebar action triggered");
        // TODO: Dispatch to focused window
    });

    // Toggle theme action
    cx.on_action(|_: &ToggleTheme, _cx: &mut App| {
        info!("Toggle theme action triggered");
        // TODO: Dispatch to focused window
    });

    // Open settings action
    cx.on_action(|_: &OpenSettings, _cx: &mut App| {
        info!("Open settings action triggered");
        // TODO: Open settings panel/window
    });
}

// =============================================================================
// Placeholder view for testing (kept for reference)
// =============================================================================

/// Simple placeholder view for basic testing.
#[allow(dead_code)]
struct PlaceholderView {
    theme: Theme,
}

#[allow(dead_code)]
impl PlaceholderView {
    fn new() -> Self {
        Self {
            theme: Theme::dark(),
        }
    }
}

impl Render for PlaceholderView {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl gpui::IntoElement {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.background)
            .justify_center()
            .items_center()
            .child(
                div()
                    .text_xl()
                    .text_color(theme.text_primary)
                    .child("Welcome to Ticca Desktop"),
            )
            .child(
                div()
                    .mt_2()
                    .text_sm()
                    .text_color(theme.text_secondary)
                    .child("Your AI desktop agent, powered by GPUI"),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracing_init() {
        // Just verify it doesn't panic
        // Note: Can only init once per process, so this may fail in parallel tests
    }

    #[test]
    fn test_tokio_runtime() {
        let runtime = init_tokio_runtime().expect("runtime should initialize");
        
        // Verify we can run async code
        let result = runtime.block_on(async {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            42
        });
        
        assert_eq!(result, 42);
    }

    #[test]
    fn test_build_menus() {
        let menus = build_menus();
        assert!(!menus.is_empty());
        assert_eq!(menus[0].name.as_ref(), "Ticca");
    }
}
