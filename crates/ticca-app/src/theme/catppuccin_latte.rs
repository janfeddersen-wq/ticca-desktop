//! Catppuccin Latte - Soothing pastel light theme
//!
//! Based on catppuccin.com

use iced::theme::{Palette, Theme};
use iced::Color;

pub mod colors {
    use iced::Color;

    // Background layers
    pub const BG_BASE: Color = Color::from_rgb(0.937, 0.945, 0.961);      // #eff1f5 Base
    pub const BG_SURFACE: Color = Color::from_rgb(0.902, 0.914, 0.937);   // #e6e9ef Mantle
    pub const BG_ELEVATED: Color = Color::from_rgb(0.800, 0.816, 0.855);  // #ccd0da Surface0
    pub const BG_HOVER: Color = Color::from_rgb(0.737, 0.753, 0.800);     // #bcc0cc Surface1

    // Text colors
    pub const TEXT_PRIMARY: Color = Color::from_rgb(0.298, 0.310, 0.412); // #4c4f69 Text
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.361, 0.373, 0.467); // #5c5f77 Subtext1
    pub const TEXT_MUTED: Color = Color::from_rgb(0.424, 0.435, 0.522);   // #6c6f85 Subtext0

    // Border colors
    pub const BORDER_SUBTLE: Color = Color::from_rgb(0.800, 0.816, 0.855); // #ccd0da Surface0
    pub const BORDER_DEFAULT: Color = Color::from_rgb(0.612, 0.627, 0.690); // #9ca0b0 Overlay0

    // Accent - Blue
    pub const ACCENT: Color = Color::from_rgb(0.118, 0.400, 0.961);       // #1e66f5 Blue
    pub const ACCENT_HOVER: Color = Color::from_rgb(0.016, 0.647, 0.898); // #04a5e5 Sky
    pub const ACCENT_MUTED: Color = Color::from_rgb(0.447, 0.529, 0.992); // #7287fd Lavender

    // Status colors
    pub const SUCCESS: Color = Color::from_rgb(0.251, 0.627, 0.169);      // #40a02b Green
    pub const WARNING: Color = Color::from_rgb(0.875, 0.557, 0.114);      // #df8e1d Yellow
    pub const DANGER: Color = Color::from_rgb(0.824, 0.059, 0.224);       // #d20f39 Red
}

pub fn theme() -> Theme {
    Theme::custom(
        "Catppuccin Latte".to_string(),
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
