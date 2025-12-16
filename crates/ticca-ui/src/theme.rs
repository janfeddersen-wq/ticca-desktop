//! Theme system for Ticca Desktop
//!
//! Provides Claude Desktop-inspired aesthetic with minimalist,
//! typography-focused design. Supports both light and dark modes.

use gpui::{Hsla, SharedString, hsla};

/// Theme configuration for the application.
///
/// Contains all colors, typography settings, and spacing values
/// used throughout the UI.
#[derive(Debug, Clone)]
pub struct Theme {
    // =========================================================================
    // Colors
    // =========================================================================
    /// Primary background color
    pub background: Hsla,
    /// Surface color (cards, panels)
    pub surface: Hsla,
    /// Elevated surface (modals, dropdowns)
    pub surface_elevated: Hsla,
    /// Primary text color
    pub text_primary: Hsla,
    /// Secondary/muted text color
    pub text_secondary: Hsla,
    /// Accent color for interactive elements
    pub accent: Hsla,
    /// Accent color on hover
    pub accent_hover: Hsla,
    /// Error/danger color
    pub error: Hsla,
    /// Success/confirmation color
    pub success: Hsla,
    /// Warning color
    pub warning: Hsla,
    /// Border color
    pub border: Hsla,
    /// Border color on hover
    pub border_hover: Hsla,
    /// Selection/highlight background
    pub selection: Hsla,

    // =========================================================================
    // User message colors
    // =========================================================================
    /// User message bubble background
    pub user_message_bg: Hsla,
    /// User message text
    pub user_message_text: Hsla,

    // =========================================================================
    // Assistant message colors
    // =========================================================================
    /// Assistant message background
    pub assistant_message_bg: Hsla,
    /// Assistant message text
    pub assistant_message_text: Hsla,

    // =========================================================================
    // Typography
    // =========================================================================
    /// Primary font family
    pub font_family: SharedString,
    /// Monospace font for code
    pub font_family_mono: SharedString,
    /// Base font size in pixels
    pub font_size_base: f32,
    /// Small font size
    pub font_size_small: f32,
    /// Large font size
    pub font_size_large: f32,
    /// Extra large font size (headers)
    pub font_size_xl: f32,
    /// Code/monospace font size
    pub font_size_code: f32,
    /// Base line height multiplier
    pub line_height: f32,

    // =========================================================================
    // Spacing
    // =========================================================================
    /// Extra small spacing (4px)
    pub spacing_xs: f32,
    /// Small spacing (8px)
    pub spacing_sm: f32,
    /// Medium spacing (16px)
    pub spacing_md: f32,
    /// Large spacing (24px)
    pub spacing_lg: f32,
    /// Extra large spacing (32px)
    pub spacing_xl: f32,

    // =========================================================================
    // Code block styling
    // =========================================================================
    /// Code block background
    pub code_background: Hsla,
    /// Code block border
    pub code_border: Hsla,
    /// Inline code background
    pub inline_code_background: Hsla,

    // =========================================================================
    // Sidebar
    // =========================================================================
    /// Sidebar background
    pub sidebar_bg: Hsla,
    /// Sidebar item hover
    pub sidebar_item_hover: Hsla,
    /// Sidebar item selected
    pub sidebar_item_selected: Hsla,
    /// Sidebar width in pixels
    pub sidebar_width: f32,

    // =========================================================================
    // Input bar
    // =========================================================================
    /// Input bar background
    pub input_bar_bg: Hsla,
    /// Input field background
    pub input_field_bg: Hsla,
    /// Input field border
    pub input_field_border: Hsla,
    /// Input field border on focus
    pub input_field_focus_border: Hsla,

    // =========================================================================
    // Scrollbar
    // =========================================================================
    /// Scrollbar track color
    pub scrollbar_track: Hsla,
    /// Scrollbar thumb color
    pub scrollbar_thumb: Hsla,
    /// Scrollbar thumb on hover
    pub scrollbar_thumb_hover: Hsla,

    // =========================================================================
    // Animation
    // =========================================================================
    /// Default animation duration in ms
    pub animation_duration_ms: u32,
    /// Scroll friction for smooth scrolling
    pub scroll_friction: f32,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    /// Create the dark theme (Claude Desktop inspired).
    ///
    /// Features deep backgrounds with warm accent colors
    /// and comfortable contrast ratios.
    pub fn dark() -> Self {
        Self {
            // Colors - Dark mode palette
            background: hsla(240.0 / 360.0, 0.11, 0.09, 1.0),      // #16161d - deep dark
            surface: hsla(240.0 / 360.0, 0.10, 0.12, 1.0),         // #1e1e24 - card bg
            surface_elevated: hsla(240.0 / 360.0, 0.10, 0.15, 1.0), // #252530 - modal bg
            text_primary: hsla(220.0 / 360.0, 0.14, 0.90, 1.0),    // #e3e5eb - main text
            text_secondary: hsla(220.0 / 360.0, 0.10, 0.55, 1.0),  // #858994 - muted text
            accent: hsla(24.0 / 360.0, 0.85, 0.55, 1.0),           // #e67e22 - warm orange
            accent_hover: hsla(24.0 / 360.0, 0.85, 0.65, 1.0),     // lighter orange
            error: hsla(0.0, 0.70, 0.55, 1.0),                     // #d64545 - red
            success: hsla(142.0 / 360.0, 0.60, 0.45, 1.0),         // #34b759 - green
            warning: hsla(45.0 / 360.0, 0.90, 0.55, 1.0),          // #e6b422 - yellow
            border: hsla(240.0 / 360.0, 0.08, 0.20, 1.0),          // #313138 - subtle border
            border_hover: hsla(240.0 / 360.0, 0.08, 0.30, 1.0),    // lighter border
            selection: hsla(24.0 / 360.0, 0.85, 0.55, 0.2),        // accent with low alpha

            // User messages
            user_message_bg: hsla(24.0 / 360.0, 0.85, 0.55, 1.0),  // accent orange
            user_message_text: hsla(0.0, 0.0, 1.0, 1.0),           // white

            // Assistant messages
            assistant_message_bg: hsla(240.0 / 360.0, 0.10, 0.15, 1.0), // surface elevated
            assistant_message_text: hsla(220.0 / 360.0, 0.14, 0.90, 1.0), // primary text

            // Typography
            font_family: SharedString::from("Inter, system-ui, sans-serif"),
            font_family_mono: SharedString::from("JetBrains Mono, Menlo, monospace"),
            font_size_base: 14.0,
            font_size_small: 12.0,
            font_size_large: 16.0,
            font_size_xl: 20.0,
            font_size_code: 13.0,
            line_height: 1.5,

            // Spacing
            spacing_xs: 4.0,
            spacing_sm: 8.0,
            spacing_md: 16.0,
            spacing_lg: 24.0,
            spacing_xl: 32.0,

            // Code blocks
            code_background: hsla(240.0 / 360.0, 0.15, 0.08, 1.0), // #121218
            code_border: hsla(240.0 / 360.0, 0.08, 0.20, 1.0),     // border color
            inline_code_background: hsla(240.0 / 360.0, 0.10, 0.18, 1.0),

            // Sidebar
            sidebar_bg: hsla(240.0 / 360.0, 0.12, 0.07, 1.0),      // #111115
            sidebar_item_hover: hsla(240.0 / 360.0, 0.10, 0.12, 1.0),
            sidebar_item_selected: hsla(24.0 / 360.0, 0.85, 0.55, 0.15),
            sidebar_width: 280.0,

            // Input bar
            input_bar_bg: hsla(240.0 / 360.0, 0.10, 0.10, 1.0),
            input_field_bg: hsla(240.0 / 360.0, 0.10, 0.14, 1.0),
            input_field_border: hsla(240.0 / 360.0, 0.08, 0.25, 1.0),
            input_field_focus_border: hsla(24.0 / 360.0, 0.85, 0.55, 1.0),

            // Scrollbar
            scrollbar_track: hsla(0.0, 0.0, 0.0, 0.0),             // transparent
            scrollbar_thumb: hsla(240.0 / 360.0, 0.08, 0.25, 1.0),
            scrollbar_thumb_hover: hsla(240.0 / 360.0, 0.08, 0.35, 1.0),

            // Animation
            animation_duration_ms: 150,
            scroll_friction: 0.92,
        }
    }

    /// Create the light theme.
    ///
    /// Features clean white backgrounds with subtle grays
    /// and the same warm accent palette.
    pub fn light() -> Self {
        Self {
            // Colors - Light mode palette
            background: hsla(0.0, 0.0, 0.98, 1.0),                 // #fafafa
            surface: hsla(0.0, 0.0, 1.0, 1.0),                     // #ffffff
            surface_elevated: hsla(0.0, 0.0, 1.0, 1.0),            // #ffffff
            text_primary: hsla(220.0 / 360.0, 0.15, 0.15, 1.0),    // #1f2328 - dark text
            text_secondary: hsla(220.0 / 360.0, 0.10, 0.45, 1.0),  // #656d76 - muted
            accent: hsla(24.0 / 360.0, 0.85, 0.50, 1.0),           // #e07916 - orange
            accent_hover: hsla(24.0 / 360.0, 0.85, 0.45, 1.0),     // darker orange
            error: hsla(0.0, 0.70, 0.50, 1.0),                     // #d63c3c
            success: hsla(142.0 / 360.0, 0.60, 0.40, 1.0),         // #2da44e
            warning: hsla(45.0 / 360.0, 0.90, 0.45, 1.0),          // #d4a017
            border: hsla(220.0 / 360.0, 0.10, 0.88, 1.0),          // #dfe1e5 - subtle border
            border_hover: hsla(220.0 / 360.0, 0.10, 0.78, 1.0),    // darker border
            selection: hsla(24.0 / 360.0, 0.85, 0.50, 0.15),       // accent with low alpha

            // User messages
            user_message_bg: hsla(24.0 / 360.0, 0.85, 0.50, 1.0),  // accent orange
            user_message_text: hsla(0.0, 0.0, 1.0, 1.0),           // white

            // Assistant messages
            assistant_message_bg: hsla(220.0 / 360.0, 0.10, 0.96, 1.0), // #f4f5f7
            assistant_message_text: hsla(220.0 / 360.0, 0.15, 0.15, 1.0), // primary text

            // Typography (same as dark)
            font_family: SharedString::from("Inter, system-ui, sans-serif"),
            font_family_mono: SharedString::from("JetBrains Mono, Menlo, monospace"),
            font_size_base: 14.0,
            font_size_small: 12.0,
            font_size_large: 16.0,
            font_size_xl: 20.0,
            font_size_code: 13.0,
            line_height: 1.5,

            // Spacing (same as dark)
            spacing_xs: 4.0,
            spacing_sm: 8.0,
            spacing_md: 16.0,
            spacing_lg: 24.0,
            spacing_xl: 32.0,

            // Code blocks
            code_background: hsla(220.0 / 360.0, 0.15, 0.95, 1.0), // #f0f2f5
            code_border: hsla(220.0 / 360.0, 0.10, 0.88, 1.0),
            inline_code_background: hsla(220.0 / 360.0, 0.10, 0.92, 1.0),

            // Sidebar
            sidebar_bg: hsla(220.0 / 360.0, 0.10, 0.96, 1.0),      // #f4f5f7
            sidebar_item_hover: hsla(220.0 / 360.0, 0.10, 0.92, 1.0),
            sidebar_item_selected: hsla(24.0 / 360.0, 0.85, 0.50, 0.12),
            sidebar_width: 280.0,

            // Input bar
            input_bar_bg: hsla(0.0, 0.0, 1.0, 1.0),
            input_field_bg: hsla(0.0, 0.0, 1.0, 1.0),
            input_field_border: hsla(220.0 / 360.0, 0.10, 0.85, 1.0),
            input_field_focus_border: hsla(24.0 / 360.0, 0.85, 0.50, 1.0),

            // Scrollbar
            scrollbar_track: hsla(0.0, 0.0, 0.0, 0.0),
            scrollbar_thumb: hsla(220.0 / 360.0, 0.08, 0.78, 1.0),
            scrollbar_thumb_hover: hsla(220.0 / 360.0, 0.08, 0.68, 1.0),

            // Animation
            animation_duration_ms: 150,
            scroll_friction: 0.92,
        }
    }

    /// Create theme from name ("dark" or "light").
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "light" => Self::light(),
            _ => Self::dark(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dark_theme_creation() {
        let theme = Theme::dark();
        assert_eq!(theme.font_size_base, 14.0);
        assert_eq!(theme.sidebar_width, 280.0);
    }

    #[test]
    fn test_light_theme_creation() {
        let theme = Theme::light();
        assert_eq!(theme.font_size_base, 14.0);
    }

    #[test]
    fn test_theme_from_name() {
        let dark = Theme::from_name("dark");
        let light = Theme::from_name("light");
        let default = Theme::from_name("unknown");

        // Light theme has white background (1.0 lightness)
        assert!(light.background.l > 0.9);
        // Dark theme has dark background
        assert!(dark.background.l < 0.2);
        // Default should be dark
        assert!(default.background.l < 0.2);
    }
}
