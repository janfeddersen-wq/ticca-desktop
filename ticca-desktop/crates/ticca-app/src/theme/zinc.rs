//! Zinc theme - a neutral gray color scheme
//!
//! A muted theme with zinc-800 background and zinc-400 accent for a minimalist look.
//!
//! Note: Full Zinc palette is defined for flexibility - not all colors are used yet.

use iced::theme::{Palette, Theme};

#[allow(unused_imports)]
use iced::Color;

/// Tailwind Zinc neutral palette colors
#[allow(dead_code)]
pub mod colors {
    use iced::Color;

    // Zinc palette (neutral mode - mid-gray)
    pub const ZINC_950: Color = Color::from_rgb(0.039, 0.039, 0.043); // #0a0a0b
    pub const ZINC_900: Color = Color::from_rgb(0.094, 0.094, 0.106); // #18181b
    pub const ZINC_800: Color = Color::from_rgb(0.153, 0.153, 0.169); // #27272a
    pub const ZINC_700: Color = Color::from_rgb(0.247, 0.247, 0.275); // #3f3f46
    pub const ZINC_600: Color = Color::from_rgb(0.322, 0.322, 0.353); // #52525b
    pub const ZINC_500: Color = Color::from_rgb(0.443, 0.443, 0.478); // #71717a
    pub const ZINC_400: Color = Color::from_rgb(0.631, 0.631, 0.667); // #a1a1aa
    pub const ZINC_300: Color = Color::from_rgb(0.831, 0.831, 0.847); // #d4d4d8
    pub const ZINC_200: Color = Color::from_rgb(0.894, 0.894, 0.906); // #e4e4e7
    pub const ZINC_100: Color = Color::from_rgb(0.957, 0.957, 0.961); // #f4f4f5
    pub const ZINC_50: Color = Color::from_rgb(0.980, 0.980, 0.980); // #fafafa

    // Background layers
    pub const BG_BASE: Color = ZINC_800;
    pub const BG_SURFACE: Color = ZINC_700;
    pub const BG_ELEVATED: Color = ZINC_600;
    pub const BG_HOVER: Color = ZINC_500;

    // Text colors
    pub const TEXT_PRIMARY: Color = ZINC_100;
    pub const TEXT_SECONDARY: Color = ZINC_300;
    pub const TEXT_MUTED: Color = ZINC_400;

    // Border colors
    pub const BORDER_SUBTLE: Color = ZINC_600;
    pub const BORDER_DEFAULT: Color = ZINC_500;

    // Accent colors
    pub const ACCENT: Color = ZINC_400;
    pub const ACCENT_HOVER: Color = ZINC_300;
    pub const ACCENT_MUTED: Color = ZINC_500;

    // Status colors (muted versions)
    pub const SUCCESS: Color = Color::from_rgb(0.290, 0.824, 0.498); // #4ade80
    pub const WARNING: Color = Color::from_rgb(0.980, 0.761, 0.141); // #facc15
    pub const DANGER: Color = Color::from_rgb(0.969, 0.447, 0.447); // #f87171
}

/// Create the zinc (neutral) theme
pub fn theme() -> Theme {
    Theme::custom(
        "Ticca Zinc".to_string(),
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
