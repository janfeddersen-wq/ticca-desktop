//! Gruvbox Dark - Retro groove with warm earthy tones
//!
//! Based on morhetz/gruvbox

use iced::theme::{Palette, Theme};
use iced::Color;

pub mod colors {
    use iced::Color;

    // Background layers
    pub const BG_BASE: Color = Color::from_rgb(0.157, 0.157, 0.157);      // #282828 dark0
    pub const BG_SURFACE: Color = Color::from_rgb(0.235, 0.220, 0.212);   // #3c3836 dark1
    pub const BG_ELEVATED: Color = Color::from_rgb(0.314, 0.286, 0.271);  // #504945 dark2
    pub const BG_HOVER: Color = Color::from_rgb(0.400, 0.361, 0.329);     // #665c54 dark3

    // Text colors (light palette)
    pub const TEXT_PRIMARY: Color = Color::from_rgb(0.922, 0.859, 0.698); // #ebdbb2 light1
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.835, 0.769, 0.631); // #d5c4a1 light2
    pub const TEXT_MUTED: Color = Color::from_rgb(0.741, 0.682, 0.576);   // #bdae93 light3

    // Border colors
    pub const BORDER_SUBTLE: Color = Color::from_rgb(0.314, 0.286, 0.271); // #504945 dark2
    pub const BORDER_DEFAULT: Color = Color::from_rgb(0.486, 0.435, 0.392); // #7c6f64 dark4

    // Accent - Aqua/Blue
    pub const ACCENT: Color = Color::from_rgb(0.514, 0.647, 0.596);       // #83a598 bright_blue
    pub const ACCENT_HOVER: Color = Color::from_rgb(0.557, 0.753, 0.486); // #8ec07c bright_aqua
    pub const ACCENT_MUTED: Color = Color::from_rgb(0.271, 0.522, 0.533); // #458588 neutral_blue

    // Status colors (bright variants)
    pub const SUCCESS: Color = Color::from_rgb(0.722, 0.733, 0.149);      // #b8bb26
    pub const WARNING: Color = Color::from_rgb(0.980, 0.741, 0.184);      // #fabd2f
    pub const DANGER: Color = Color::from_rgb(0.984, 0.286, 0.204);       // #fb4934
}

pub fn theme() -> Theme {
    Theme::custom(
        "Gruvbox Dark".to_string(),
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
