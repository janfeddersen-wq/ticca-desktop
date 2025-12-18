//! Tokyo Night - City lights inspired theme
//!
//! Based on folke/tokyonight.nvim

use iced::theme::{Palette, Theme};
use iced::Color;

pub mod colors {
    use iced::Color;

    // Background layers
    pub const BG_BASE: Color = Color::from_rgb(0.102, 0.106, 0.149);      // #1a1b26
    pub const BG_SURFACE: Color = Color::from_rgb(0.086, 0.086, 0.118);   // #16161e
    pub const BG_ELEVATED: Color = Color::from_rgb(0.161, 0.180, 0.259);  // #292e42
    pub const BG_HOVER: Color = Color::from_rgb(0.157, 0.204, 0.341);     // #283457

    // Text colors
    pub const TEXT_PRIMARY: Color = Color::from_rgb(0.753, 0.792, 0.961); // #c0caf5
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.663, 0.694, 0.839); // #a9b1d6
    pub const TEXT_MUTED: Color = Color::from_rgb(0.337, 0.373, 0.537);   // #565f89

    // Border colors
    pub const BORDER_SUBTLE: Color = Color::from_rgb(0.082, 0.086, 0.118); // #15161e
    pub const BORDER_DEFAULT: Color = Color::from_rgb(0.231, 0.259, 0.380); // #3b4261

    // Accent - Signature blue
    pub const ACCENT: Color = Color::from_rgb(0.478, 0.635, 0.969);       // #7aa2f7
    pub const ACCENT_HOVER: Color = Color::from_rgb(0.239, 0.349, 0.631); // #3d59a1
    pub const ACCENT_MUTED: Color = Color::from_rgb(0.224, 0.294, 0.439); // #394b70

    // Status colors
    pub const SUCCESS: Color = Color::from_rgb(0.620, 0.808, 0.416);      // #9ece6a
    pub const WARNING: Color = Color::from_rgb(0.878, 0.686, 0.408);      // #e0af68
    pub const DANGER: Color = Color::from_rgb(0.859, 0.294, 0.294);       // #db4b4b
}

pub fn theme() -> Theme {
    Theme::custom(
        "Tokyo Night".to_string(),
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
