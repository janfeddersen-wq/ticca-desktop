//! Light theme using Tailwind Zinc color palette
//!
//! A light theme with zinc-50 background and blue-600 accent.
//!
//! Note: Full Zinc palette is defined for flexibility - not all colors are used yet.

use iced::theme::{Palette, Theme};

/// Tailwind Zinc light palette colors
#[allow(dead_code)]
pub mod colors {
    use iced::Color;

    // Zinc palette (light mode)
    pub const ZINC_50: Color = Color::from_rgb(0.980, 0.980, 0.980); // #fafafa
    pub const ZINC_100: Color = Color::from_rgb(0.957, 0.957, 0.961); // #f4f4f5
    pub const ZINC_200: Color = Color::from_rgb(0.894, 0.894, 0.906); // #e4e4e7
    pub const ZINC_300: Color = Color::from_rgb(0.831, 0.831, 0.847); // #d4d4d8
    pub const ZINC_400: Color = Color::from_rgb(0.631, 0.631, 0.667); // #a1a1aa
    pub const ZINC_500: Color = Color::from_rgb(0.443, 0.443, 0.478); // #71717a
    pub const ZINC_600: Color = Color::from_rgb(0.322, 0.322, 0.353); // #52525b
    pub const ZINC_700: Color = Color::from_rgb(0.247, 0.247, 0.275); // #3f3f46
    pub const ZINC_800: Color = Color::from_rgb(0.153, 0.153, 0.169); // #27272a
    pub const ZINC_900: Color = Color::from_rgb(0.094, 0.094, 0.106); // #18181b
    pub const ZINC_950: Color = Color::from_rgb(0.039, 0.039, 0.043); // #0a0a0b

    pub const WHITE: Color = Color::from_rgb(1.0, 1.0, 1.0); // #ffffff

    // Blue accent
    pub const BLUE_700: Color = Color::from_rgb8(0x1d, 0x4e, 0xd8); // #1d4ed8
    pub const BLUE_600: Color = Color::from_rgb(0.145, 0.388, 0.922); // #2563eb
    pub const BLUE_500: Color = Color::from_rgb(0.231, 0.510, 0.965); // #3b82f6

    // Status colors
    pub const GREEN_600: Color = Color::from_rgb(0.086, 0.639, 0.290); // #16a34a
    pub const YELLOW_600: Color = Color::from_rgb(0.792, 0.498, 0.0); // #ca8a04
    pub const RED_600: Color = Color::from_rgb(0.863, 0.149, 0.149); // #dc2626
}

/// Create the light theme
pub fn theme() -> Theme {
    Theme::custom(
        "Ticca Light".to_string(),
        Palette {
            background: colors::ZINC_50,
            text: colors::ZINC_900,
            primary: colors::BLUE_600,
            success: colors::GREEN_600,
            warning: colors::YELLOW_600,
            danger: colors::RED_600,
        },
    )
}
