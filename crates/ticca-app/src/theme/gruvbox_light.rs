//! Gruvbox Light - Retro groove light variant
//!
//! Based on morhetz/gruvbox

use iced::theme::{Palette, Theme};
use iced::Color;

pub mod colors {
    use iced::Color;

    // Background layers
    pub const BG_BASE: Color = Color::from_rgb(0.984, 0.945, 0.780);      // #fbf1c7 light0
    pub const BG_SURFACE: Color = Color::from_rgb(0.922, 0.859, 0.698);   // #ebdbb2 light1
    pub const BG_ELEVATED: Color = Color::from_rgb(0.835, 0.769, 0.631);  // #d5c4a1 light2
    pub const BG_HOVER: Color = Color::from_rgb(0.741, 0.682, 0.576);     // #bdae93 light3

    // Text colors (dark palette)
    pub const TEXT_PRIMARY: Color = Color::from_rgb(0.235, 0.220, 0.212); // #3c3836 dark1
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.314, 0.286, 0.271); // #504945 dark2
    pub const TEXT_MUTED: Color = Color::from_rgb(0.400, 0.361, 0.329);   // #665c54 dark3

    // Border colors
    pub const BORDER_SUBTLE: Color = Color::from_rgb(0.835, 0.769, 0.631); // #d5c4a1 light2
    pub const BORDER_DEFAULT: Color = Color::from_rgb(0.659, 0.600, 0.518); // #a89984 light4

    // Accent - Faded blue
    pub const ACCENT: Color = Color::from_rgb(0.027, 0.400, 0.471);       // #076678 faded_blue
    pub const ACCENT_HOVER: Color = Color::from_rgb(0.271, 0.522, 0.533); // #458588 neutral_blue
    pub const ACCENT_MUTED: Color = Color::from_rgb(0.514, 0.647, 0.596); // #83a598 bright_blue

    // Status colors (faded variants)
    pub const SUCCESS: Color = Color::from_rgb(0.475, 0.455, 0.055);      // #79740e
    pub const WARNING: Color = Color::from_rgb(0.710, 0.463, 0.078);      // #b57614
    pub const DANGER: Color = Color::from_rgb(0.616, 0.0, 0.024);         // #9d0006
}

pub fn theme() -> Theme {
    Theme::custom(
        "Gruvbox Light".to_string(),
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
