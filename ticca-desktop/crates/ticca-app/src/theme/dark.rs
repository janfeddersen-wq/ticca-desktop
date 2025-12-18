//! Dark theme - Clean, modern dark color scheme
//!
//! A cohesive dark theme with subtle contrasts and minimal visual noise.

use iced::theme::{Palette, Theme};
use iced::Color;

/// Clean dark palette colors
pub mod colors {
    use iced::Color;

    // Background layers (darkest to lightest)
    pub const BG_BASE: Color = Color::from_rgb(0.067, 0.067, 0.075);      // #111113 - main background
    pub const BG_SURFACE: Color = Color::from_rgb(0.098, 0.098, 0.110);   // #19191c - cards, panels
    pub const BG_ELEVATED: Color = Color::from_rgb(0.133, 0.133, 0.145);  // #222225 - elevated elements
    pub const BG_HOVER: Color = Color::from_rgb(0.165, 0.165, 0.180);     // #2a2a2e - hover states

    // Text colors
    pub const TEXT_PRIMARY: Color = Color::from_rgb(0.933, 0.933, 0.940); // #eeeeef - primary text
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.600, 0.600, 0.630); // #9999a0 - secondary text
    pub const TEXT_MUTED: Color = Color::from_rgb(0.450, 0.450, 0.480);   // #737379 - muted text

    // Border colors
    pub const BORDER_SUBTLE: Color = Color::from_rgb(0.180, 0.180, 0.200); // #2e2e33 - subtle borders
    pub const BORDER_DEFAULT: Color = Color::from_rgb(0.220, 0.220, 0.245); // #38383e - default borders

    // Accent - soft blue
    pub const ACCENT: Color = Color::from_rgb(0.380, 0.580, 0.920);       // #6194eb - primary accent
    pub const ACCENT_HOVER: Color = Color::from_rgb(0.440, 0.630, 0.950); // #70a0f2 - accent hover
    pub const ACCENT_MUTED: Color = Color::from_rgb(0.250, 0.380, 0.600); // #406099 - muted accent

    // Status colors
    pub const SUCCESS: Color = Color::from_rgb(0.300, 0.750, 0.450);      // #4dbf73
    pub const WARNING: Color = Color::from_rgb(0.920, 0.700, 0.300);      // #ebb34d
    pub const DANGER: Color = Color::from_rgb(0.900, 0.350, 0.350);       // #e65959
}

/// Create the dark theme
pub fn theme() -> Theme {
    Theme::custom(
        "Ticca Dark".to_string(),
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
