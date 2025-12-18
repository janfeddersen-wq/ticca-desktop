//! Catppuccin Mocha - Soothing pastel dark theme
//!
//! Based on catppuccin.com

use iced::theme::{Palette, Theme};
use iced::Color;

pub mod colors {
    use iced::Color;

    // Background layers
    pub const BG_BASE: Color = Color::from_rgb(0.118, 0.118, 0.180);      // #1e1e2e Base
    pub const BG_SURFACE: Color = Color::from_rgb(0.094, 0.094, 0.145);   // #181825 Mantle
    pub const BG_ELEVATED: Color = Color::from_rgb(0.192, 0.196, 0.267);  // #313244 Surface0
    pub const BG_HOVER: Color = Color::from_rgb(0.271, 0.278, 0.353);     // #45475a Surface1

    // Text colors
    pub const TEXT_PRIMARY: Color = Color::from_rgb(0.804, 0.839, 0.957); // #cdd6f4 Text
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.729, 0.761, 0.871); // #bac2de Subtext1
    pub const TEXT_MUTED: Color = Color::from_rgb(0.651, 0.678, 0.784);   // #a6adc8 Subtext0

    // Border colors
    pub const BORDER_SUBTLE: Color = Color::from_rgb(0.192, 0.196, 0.267); // #313244 Surface0
    pub const BORDER_DEFAULT: Color = Color::from_rgb(0.424, 0.439, 0.525); // #6c7086 Overlay0

    // Accent - Blue
    pub const ACCENT: Color = Color::from_rgb(0.537, 0.706, 0.980);       // #89b4fa Blue
    pub const ACCENT_HOVER: Color = Color::from_rgb(0.537, 0.863, 0.922); // #89dceb Sky
    pub const ACCENT_MUTED: Color = Color::from_rgb(0.706, 0.745, 0.996); // #b4befe Lavender

    // Status colors
    pub const SUCCESS: Color = Color::from_rgb(0.651, 0.890, 0.631);      // #a6e3a1 Green
    pub const WARNING: Color = Color::from_rgb(0.976, 0.886, 0.686);      // #f9e2af Yellow
    pub const DANGER: Color = Color::from_rgb(0.953, 0.545, 0.659);       // #f38ba8 Red
}

pub fn theme() -> Theme {
    Theme::custom(
        "Catppuccin Mocha".to_string(),
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
