//! Theme definitions for Ticca Desktop
//!
//! Provides multiple color schemes including popular editor themes.

pub mod dark;
pub mod light;
pub mod styles;
pub mod zinc;

// Popular themes
pub mod catppuccin_latte;
pub mod catppuccin_mocha;
pub mod dracula;
pub mod gruvbox_dark;
pub mod gruvbox_light;
pub mod nord;
pub mod one_dark;
pub mod tokyo_night;

use iced::Theme;

/// Application theme variants
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AppTheme {
    #[default]
    Dark,
    Light,
    Zinc,
    Dracula,
    Nord,
    CatppuccinMocha,
    CatppuccinLatte,
    TokyoNight,
    OneDark,
    GruvboxDark,
    GruvboxLight,
}

/// All available themes for iteration
pub const ALL_THEMES: &[AppTheme] = &[
    AppTheme::Dark,
    AppTheme::Light,
    AppTheme::Zinc,
    AppTheme::Dracula,
    AppTheme::Nord,
    AppTheme::CatppuccinMocha,
    AppTheme::CatppuccinLatte,
    AppTheme::TokyoNight,
    AppTheme::OneDark,
    AppTheme::GruvboxDark,
    AppTheme::GruvboxLight,
];

#[allow(dead_code)]
impl AppTheme {
    /// Cycle to the next theme
    pub fn next(&self) -> Self {
        let idx = ALL_THEMES.iter().position(|t| t == self).unwrap_or(0);
        ALL_THEMES[(idx + 1) % ALL_THEMES.len()]
    }

    /// Get the Iced Theme for this AppTheme
    pub fn to_iced_theme(self) -> Theme {
        match self {
            AppTheme::Dark => dark::theme(),
            AppTheme::Light => light::theme(),
            AppTheme::Zinc => zinc::theme(),
            AppTheme::Dracula => dracula::theme(),
            AppTheme::Nord => nord::theme(),
            AppTheme::CatppuccinMocha => catppuccin_mocha::theme(),
            AppTheme::CatppuccinLatte => catppuccin_latte::theme(),
            AppTheme::TokyoNight => tokyo_night::theme(),
            AppTheme::OneDark => one_dark::theme(),
            AppTheme::GruvboxDark => gruvbox_dark::theme(),
            AppTheme::GruvboxLight => gruvbox_light::theme(),
        }
    }

    /// Convert to string for storage
    pub fn as_str(&self) -> &'static str {
        match self {
            AppTheme::Dark => "dark",
            AppTheme::Light => "light",
            AppTheme::Zinc => "zinc",
            AppTheme::Dracula => "dracula",
            AppTheme::Nord => "nord",
            AppTheme::CatppuccinMocha => "catppuccin-mocha",
            AppTheme::CatppuccinLatte => "catppuccin-latte",
            AppTheme::TokyoNight => "tokyo-night",
            AppTheme::OneDark => "one-dark",
            AppTheme::GruvboxDark => "gruvbox-dark",
            AppTheme::GruvboxLight => "gruvbox-light",
        }
    }

    /// Parse from string
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "light" => AppTheme::Light,
            "zinc" => AppTheme::Zinc,
            "dracula" => AppTheme::Dracula,
            "nord" => AppTheme::Nord,
            "catppuccin-mocha" => AppTheme::CatppuccinMocha,
            "catppuccin-latte" => AppTheme::CatppuccinLatte,
            "tokyo-night" => AppTheme::TokyoNight,
            "one-dark" => AppTheme::OneDark,
            "gruvbox-dark" => AppTheme::GruvboxDark,
            "gruvbox-light" => AppTheme::GruvboxLight,
            _ => AppTheme::Dark,
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            AppTheme::Dark => "Dark",
            AppTheme::Light => "Light",
            AppTheme::Zinc => "Zinc",
            AppTheme::Dracula => "Dracula",
            AppTheme::Nord => "Nord",
            AppTheme::CatppuccinMocha => "Catppuccin Mocha",
            AppTheme::CatppuccinLatte => "Catppuccin Latte",
            AppTheme::TokyoNight => "Tokyo Night",
            AppTheme::OneDark => "One Dark",
            AppTheme::GruvboxDark => "Gruvbox Dark",
            AppTheme::GruvboxLight => "Gruvbox Light",
        }
    }

    /// Check if this is a dark theme
    pub fn is_dark(&self) -> bool {
        !matches!(
            self,
            AppTheme::Light | AppTheme::CatppuccinLatte | AppTheme::GruvboxLight
        )
    }
}

impl std::fmt::Display for AppTheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}
