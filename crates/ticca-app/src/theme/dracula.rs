//! Dracula theme - Purple-heavy aesthetic with high contrast
//!
//! Based on draculatheme.com

use iced::theme::{Palette, Theme};

#[allow(dead_code)]
pub mod colors {
    use iced::Color;

    // Background layers
    pub const BG_BASE: Color = Color::from_rgb(0.098, 0.102, 0.129); // #191A21
    pub const BG_SURFACE: Color = Color::from_rgb(0.157, 0.165, 0.212); // #282A36
    pub const BG_ELEVATED: Color = Color::from_rgb(0.204, 0.216, 0.275); // #343746
    pub const BG_HOVER: Color = Color::from_rgb(0.267, 0.278, 0.353); // #44475A

    // Text colors
    pub const TEXT_PRIMARY: Color = Color::from_rgb(0.973, 0.973, 0.949); // #F8F8F2
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.384, 0.447, 0.643); // #6272A4
    pub const TEXT_MUTED: Color = Color::from_rgb(0.384, 0.447, 0.643); // #6272A4

    // Border colors
    pub const BORDER_SUBTLE: Color = Color::from_rgb(0.267, 0.278, 0.353); // #44475A
    pub const BORDER_DEFAULT: Color = Color::from_rgb(0.384, 0.447, 0.643); // #6272A4

    // Accent - signature purple
    pub const ACCENT: Color = Color::from_rgb(0.741, 0.576, 0.976); // #BD93F9
    pub const ACCENT_HOVER: Color = Color::from_rgb(0.839, 0.675, 1.0); // #D6ACFF
    pub const ACCENT_MUTED: Color = Color::from_rgb(0.506, 0.361, 0.839); // #815CD6

    // Status colors
    pub const SUCCESS: Color = Color::from_rgb(0.314, 0.980, 0.482); // #50FA7B
    pub const WARNING: Color = Color::from_rgb(1.0, 0.722, 0.424); // #FFB86C
    pub const DANGER: Color = Color::from_rgb(1.0, 0.333, 0.333); // #FF5555
}

pub fn theme() -> Theme {
    Theme::custom(
        "Dracula".to_string(),
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
