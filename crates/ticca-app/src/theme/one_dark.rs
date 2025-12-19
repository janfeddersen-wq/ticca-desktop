//! One Dark - Atom editor's signature theme
//!
//! Based on atom/one-dark-syntax

use iced::theme::{Palette, Theme};
use iced::Color;

pub mod colors {
    use iced::Color;

    // Background layers
    pub const BG_BASE: Color = Color::from_rgb(0.157, 0.173, 0.204);      // #282c34
    pub const BG_SURFACE: Color = Color::from_rgb(0.129, 0.145, 0.169);   // #21252b
    pub const BG_ELEVATED: Color = Color::from_rgb(0.173, 0.196, 0.235);  // #2c323c
    pub const BG_HOVER: Color = Color::from_rgb(0.243, 0.267, 0.322);     // #3e4452

    // Text colors (mono scale)
    pub const TEXT_PRIMARY: Color = Color::from_rgb(0.671, 0.698, 0.749); // #abb2bf mono-1
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.510, 0.537, 0.592); // #828997 mono-2
    pub const TEXT_MUTED: Color = Color::from_rgb(0.361, 0.388, 0.439);   // #5c6370 mono-3

    // Border colors
    pub const BORDER_SUBTLE: Color = Color::from_rgb(0.231, 0.251, 0.282); // #3b4048
    pub const BORDER_DEFAULT: Color = Color::from_rgb(0.294, 0.322, 0.388); // #4b5263

    // Accent - Blue
    pub const ACCENT: Color = Color::from_rgb(0.380, 0.686, 0.937);       // #61afef
    pub const ACCENT_HOVER: Color = Color::from_rgb(0.322, 0.545, 1.0);   // #528bff
    pub const ACCENT_MUTED: Color = Color::from_rgb(0.227, 0.290, 0.369); // #3a4a5e

    // Status colors
    pub const SUCCESS: Color = Color::from_rgb(0.596, 0.765, 0.475);      // #98c379
    pub const WARNING: Color = Color::from_rgb(0.898, 0.753, 0.482);      // #e5c07b
    pub const DANGER: Color = Color::from_rgb(0.878, 0.424, 0.459);       // #e06c75
}

pub fn theme() -> Theme {
    Theme::custom(
        "One Dark".to_string(),
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
