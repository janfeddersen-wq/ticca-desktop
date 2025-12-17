//! Theme definitions for Ticca Desktop
//!
//! Provides Dark, Light, and Zinc color schemes using Tailwind CSS colors.

pub mod dark;
pub mod light;
pub mod styles;
pub mod zinc;

use iced::Theme;

/// Application theme variants
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AppTheme {
    #[default]
    Dark,
    Light,
    Zinc,
}

#[allow(dead_code)]
impl AppTheme {
    /// Cycle to the next theme
    pub fn next(&self) -> Self {
        match self {
            AppTheme::Dark => AppTheme::Light,
            AppTheme::Light => AppTheme::Zinc,
            AppTheme::Zinc => AppTheme::Dark,
        }
    }

    /// Get the Iced Theme for this AppTheme
    pub fn to_iced_theme(&self) -> Theme {
        match self {
            AppTheme::Dark => dark::theme(),
            AppTheme::Light => light::theme(),
            AppTheme::Zinc => zinc::theme(),
        }
    }

    /// Convert to string for storage
    pub fn as_str(&self) -> &'static str {
        match self {
            AppTheme::Dark => "dark",
            AppTheme::Light => "light",
            AppTheme::Zinc => "zinc",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "light" => AppTheme::Light,
            "zinc" => AppTheme::Zinc,
            _ => AppTheme::Dark,
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            AppTheme::Dark => "Dark",
            AppTheme::Light => "Light",
            AppTheme::Zinc => "Zinc",
        }
    }
}

impl std::fmt::Display for AppTheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
