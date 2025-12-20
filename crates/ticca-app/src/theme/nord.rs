//! Nord theme - Arctic-inspired color palette
//!
//! Based on nordtheme.com

use iced::theme::{Palette, Theme};

#[allow(dead_code)]
pub mod colors {
    use iced::Color;

    // Background layers (Polar Night)
    pub const BG_BASE: Color = Color::from_rgb(0.180, 0.204, 0.251); // #2E3440 nord0
    pub const BG_SURFACE: Color = Color::from_rgb(0.231, 0.259, 0.322); // #3B4252 nord1
    pub const BG_ELEVATED: Color = Color::from_rgb(0.263, 0.298, 0.369); // #434C5E nord2
    pub const BG_HOVER: Color = Color::from_rgb(0.298, 0.337, 0.416); // #4C566A nord3

    // Text colors (Snow Storm)
    pub const TEXT_PRIMARY: Color = Color::from_rgb(0.925, 0.937, 0.957); // #ECEFF4 nord6
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.847, 0.871, 0.914); // #D8DEE9 nord4
    pub const TEXT_MUTED: Color = Color::from_rgb(0.298, 0.337, 0.416); // #4C566A nord3

    // Border colors
    pub const BORDER_SUBTLE: Color = Color::from_rgb(0.231, 0.259, 0.322); // #3B4252 nord1
    pub const BORDER_DEFAULT: Color = Color::from_rgb(0.298, 0.337, 0.416); // #4C566A nord3

    // Accent (Frost)
    pub const ACCENT: Color = Color::from_rgb(0.533, 0.753, 0.816); // #88C0D0 nord8
    pub const ACCENT_HOVER: Color = Color::from_rgb(0.561, 0.737, 0.733); // #8FBCBB nord7
    pub const ACCENT_MUTED: Color = Color::from_rgb(0.369, 0.506, 0.675); // #5E81AC nord10

    // Status colors (Aurora)
    pub const SUCCESS: Color = Color::from_rgb(0.639, 0.745, 0.549); // #A3BE8C nord14
    pub const WARNING: Color = Color::from_rgb(0.922, 0.796, 0.545); // #EBCB8B nord13
    pub const DANGER: Color = Color::from_rgb(0.749, 0.380, 0.416); // #BF616A nord11
}

pub fn theme() -> Theme {
    Theme::custom(
        "Nord".to_string(),
        Palette {
            background: colors::BG_BASE,
            text: colors::TEXT_PRIMARY,
            primary: colors::ACCENT,
            success: colors::SUCCESS,
            warning: colors::WARNING,
            danger: colors::DANGER,
        },
    )
}
