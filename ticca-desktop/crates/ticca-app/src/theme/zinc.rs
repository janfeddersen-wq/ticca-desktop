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

    // Neutral accent - using zinc itself
    pub const ACCENT: Color = ZINC_400;

    // Status colors (muted versions)
    pub const GREEN_400: Color = Color::from_rgb(0.290, 0.824, 0.498); // #4ade80
    pub const YELLOW_400: Color = Color::from_rgb(0.980, 0.761, 0.141); // #facc15
    pub const RED_400: Color = Color::from_rgb(0.969, 0.447, 0.447); // #f87171
}

/// Create the zinc (neutral) theme
pub fn theme() -> Theme {
    Theme::custom(
        "Ticca Zinc".to_string(),
        Palette {
            background: colors::ZINC_800,
            text: colors::ZINC_100,
            primary: colors::ACCENT,
            success: colors::GREEN_400,
            danger: colors::RED_400,
        },
    )
}
