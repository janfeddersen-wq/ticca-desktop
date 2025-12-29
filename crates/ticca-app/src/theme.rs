//! Theme management for Ticca Desktop
//!
//! Integrates with gpui-component's theme system and provides our custom themes.

use gpui::{App, SharedString, Window};
use gpui_component::{Theme, ThemeMode, ThemeRegistry};
use std::path::PathBuf;

/// Application theme variants
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AppTheme {
    #[default]
    TiccaDark,
    TiccaLight,
    TiccaZinc,
    Dracula,
    Nord,
    OneDark,
    CatppuccinMocha,
    CatppuccinLatte,
    TokyoNight,
    GruvboxDark,
    GruvboxLight,
}

/// All available themes for iteration
pub const ALL_THEMES: &[AppTheme] = &[
    AppTheme::TiccaDark,
    AppTheme::TiccaLight,
    AppTheme::TiccaZinc,
    AppTheme::Dracula,
    AppTheme::Nord,
    AppTheme::OneDark,
    AppTheme::CatppuccinMocha,
    AppTheme::CatppuccinLatte,
    AppTheme::TokyoNight,
    AppTheme::GruvboxDark,
    AppTheme::GruvboxLight,
];

impl AppTheme {
    /// Cycle to the next theme
    pub fn next(&self) -> Self {
        let idx = ALL_THEMES.iter().position(|t| t == self).unwrap_or(0);
        ALL_THEMES[(idx + 1) % ALL_THEMES.len()]
    }

    /// Get the gpui-component theme name
    pub fn theme_name(&self) -> SharedString {
        match self {
            AppTheme::TiccaDark => "Ticca Dark".into(),
            AppTheme::TiccaLight => "Ticca Light".into(),
            AppTheme::TiccaZinc => "Ticca Zinc".into(),
            AppTheme::Dracula => "Dracula".into(),
            AppTheme::Nord => "Nord".into(),
            AppTheme::OneDark => "One Dark".into(),
            AppTheme::CatppuccinMocha => "Catppuccin Mocha".into(),
            AppTheme::CatppuccinLatte => "Catppuccin Latte".into(),
            AppTheme::TokyoNight => "Tokyo Night".into(),
            AppTheme::GruvboxDark => "Gruvbox Dark".into(),
            AppTheme::GruvboxLight => "Gruvbox Light".into(),
        }
    }

    /// Get the display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            AppTheme::TiccaDark => "Dark",
            AppTheme::TiccaLight => "Light",
            AppTheme::TiccaZinc => "Zinc",
            AppTheme::Dracula => "Dracula",
            AppTheme::Nord => "Nord",
            AppTheme::OneDark => "One Dark",
            AppTheme::CatppuccinMocha => "Catppuccin Mocha",
            AppTheme::CatppuccinLatte => "Catppuccin Latte",
            AppTheme::TokyoNight => "Tokyo Night",
            AppTheme::GruvboxDark => "Gruvbox Dark",
            AppTheme::GruvboxLight => "Gruvbox Light",
        }
    }

    /// Check if this is a dark theme
    pub fn is_dark(&self) -> bool {
        !matches!(
            self,
            AppTheme::TiccaLight | AppTheme::CatppuccinLatte | AppTheme::GruvboxLight
        )
    }

    /// Get the theme mode
    pub fn mode(&self) -> ThemeMode {
        if self.is_dark() {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        }
    }

    /// Apply this theme to the application
    pub fn apply(&self, window: Option<&mut Window>, cx: &mut App) {
        let theme_name = self.theme_name();

        // Try to find the theme in the registry
        if let Some(theme_config) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned() {
            Theme::global_mut(cx).apply_config(&theme_config);
            tracing::info!("Applied theme: {}", theme_name);
            if let Some(window) = window {
                window.refresh();
            }
        } else {
            // Fall back to default dark/light
            tracing::warn!("Theme '{}' not found, falling back to default", theme_name);
            Theme::change(self.mode(), window, cx);
        }
    }
}

impl std::fmt::Display for AppTheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Initialize the theme system
///
/// This loads our custom themes from the themes directory and sets up
/// the theme registry.
pub fn init(cx: &mut App) {
    // Get the themes directory - relative to the executable or current dir
    let themes_dir = get_themes_dir();

    if themes_dir.exists() {
        tracing::info!("Loading themes from: {}", themes_dir.display());

        // Watch the themes directory for changes
        if let Err(err) = ThemeRegistry::watch_dir(themes_dir.clone(), cx, |cx| {
            tracing::info!("Themes loaded/reloaded");
            // Apply default theme after loading
            AppTheme::TiccaDark.apply(None, cx);
        }) {
            tracing::error!("Failed to watch themes directory: {}", err);
        }
    } else {
        tracing::warn!("Themes directory not found: {}", themes_dir.display());
        // Use default dark theme
        Theme::change(ThemeMode::Dark, None, cx);
    }
}

/// Get the themes directory path
fn get_themes_dir() -> PathBuf {
    // First, try relative to the executable (for installed apps)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let themes_dir = exe_dir.join("themes");
            if themes_dir.exists() {
                return themes_dir;
            }
            // Also check parent directory (for target/debug/ticca)
            if let Some(parent) = exe_dir.parent() {
                let themes_dir = parent.join("themes");
                if themes_dir.exists() {
                    return themes_dir;
                }
            }
        }
    }

    // Then try relative to current directory (for development)
    let cwd = std::env::current_dir().unwrap_or_default();

    // Check common locations
    let candidates = [
        cwd.join("crates/ticca-app/themes"),
        cwd.join("themes"),
        PathBuf::from("./themes"),
    ];

    for candidate in candidates {
        if candidate.exists() {
            return candidate;
        }
    }

    // Default to cwd/themes
    cwd.join("themes")
}
