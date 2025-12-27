//! TUI Theme System
//!
//! Maps all 11 GUI themes to ratatui terminal colors.
//! Provides consistent styling across the entire TUI.

mod colors;
mod styles;

pub use colors::TuiColors;
#[allow(unused_imports)]
pub use styles::TuiStyles;

use crate::theme::AppTheme;

impl TuiColors {
    /// Create TUI colors from the application theme
    pub fn from_theme(theme: AppTheme) -> Self {
        match theme {
            AppTheme::Dark => Self::dark(),
            AppTheme::Light => Self::light(),
            AppTheme::Zinc => Self::zinc(),
            AppTheme::Dracula => Self::dracula(),
            AppTheme::Nord => Self::nord(),
            AppTheme::CatppuccinMocha => Self::catppuccin_mocha(),
            AppTheme::CatppuccinLatte => Self::catppuccin_latte(),
            AppTheme::TokyoNight => Self::tokyo_night(),
            AppTheme::OneDark => Self::one_dark(),
            AppTheme::GruvboxDark => Self::gruvbox_dark(),
            AppTheme::GruvboxLight => Self::gruvbox_light(),
        }
    }
}
