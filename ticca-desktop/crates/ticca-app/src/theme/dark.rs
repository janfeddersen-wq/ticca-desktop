//! Dark theme using Tailwind Zinc color palette
//!
//! A dark theme with zinc-950 background and blue-500 accent.
//!
//! Note: Full Zinc palette is defined for flexibility - not all colors are used yet.

use iced::theme::{Palette, Theme};

#[allow(unused_imports)]
use iced::Color;

/// Tailwind Zinc dark palette colors
#[allow(dead_code)]
pub mod colors {
    use iced::Color;

    // Zinc palette (dark mode)
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

    // Blue accent
    pub const BLUE_600: Color = Color::from_rgb(0.145, 0.388, 0.922); // #2563eb
    pub const BLUE_500: Color = Color::from_rgb(0.231, 0.510, 0.965); // #3b82f6
    pub const BLUE_400: Color = Color::from_rgb(0.376, 0.647, 0.996); // #60a5fa

    // Status colors
    pub const GREEN_500: Color = Color::from_rgb(0.133, 0.773, 0.369); // #22c55e
    pub const YELLOW_500: Color = Color::from_rgb(0.961, 0.620, 0.043); // #f59e0b
    pub const RED_500: Color = Color::from_rgb(0.937, 0.267, 0.267); // #ef4444
}

/// Create the dark theme
pub fn theme() -> Theme {
    Theme::custom(
        "Ticca Dark".to_string(),
        Palette {
            background: colors::ZINC_950,
            text: colors::ZINC_50,
            primary: colors::BLUE_500,
            success: colors::GREEN_500,
            danger: colors::RED_500,
        },
    )
}
