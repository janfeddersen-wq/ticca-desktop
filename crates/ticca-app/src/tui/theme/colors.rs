#![allow(dead_code)]

//! TUI Color Palette Definitions
//!
//! Contains color definitions for all 11 themes, mapping GUI theme colors
//! to ratatui Color values for terminal rendering.

use iced::Color as IcedColor;
use ratatui::style::Color;

use crate::theme::colors::{
    CatppuccinLatteColors, CatppuccinMochaColors, DarkColors, DraculaColors, GruvboxDarkColors,
    GruvboxLightColors, LightColors, NordColors, OneDarkColors, ThemeColors, TokyoNightColors,
    ZincColors,
};

/// Complete color palette for TUI rendering
#[derive(Debug, Clone, Copy)]
pub struct TuiColors {
    // === Background Colors ===
    /// Base background (darkest)
    pub bg_base: Color,
    /// Surface background (cards, panels)
    pub bg_surface: Color,
    /// Elevated background (dropdowns, modals)
    pub bg_elevated: Color,
    /// Hover state background
    pub bg_hover: Color,

    // === Text Colors ===
    /// Primary text color
    pub text_primary: Color,
    /// Secondary text color (less emphasis)
    pub text_secondary: Color,
    /// Muted text color (hints, placeholders)
    pub text_muted: Color,

    // === Border Colors ===
    /// Subtle border (dividers)
    pub border_subtle: Color,
    /// Default border (inputs, cards)
    pub border_default: Color,
    /// Focused border (active elements)
    pub border_focused: Color,

    // === Accent Colors ===
    /// Primary accent color
    pub accent: Color,
    /// Dimmed accent (less emphasis)
    pub accent_dim: Color,

    // === Status Colors ===
    /// Success state
    pub success: Color,
    /// Warning state
    pub warning: Color,
    /// Danger/error state
    pub danger: Color,

    // === Chat-Specific Colors ===
    /// User message background
    pub user_msg_bg: Color,
    /// Assistant message background
    pub assistant_msg_bg: Color,
    /// Tool call block background
    pub tool_call_bg: Color,
    /// Reasoning block background
    pub reasoning_bg: Color,
    /// Code block background
    pub code_bg: Color,

    // === Syntax Highlighting (for code blocks) ===
    /// Keywords (fn, let, if, etc.)
    pub syntax_keyword: Color,
    /// Strings
    pub syntax_string: Color,
    /// Comments
    pub syntax_comment: Color,
    /// Numbers
    pub syntax_number: Color,
    /// Functions
    pub syntax_function: Color,
    /// Types
    pub syntax_type: Color,
}

impl TuiColors {
    /// Check if this is a dark theme
    pub fn is_dark(&self) -> bool {
        match self.bg_base {
            Color::Rgb(r, g, b) => {
                let luminance = (r as u32 + g as u32 + b as u32) / 3;
                luminance < 128
            }
            _ => true,
        }
    }

    fn from_iced(color: IcedColor) -> Color {
        Color::Rgb(
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8,
        )
    }

    fn from_theme_colors(colors: &dyn ThemeColors) -> Self {
        let accent = Self::from_iced(colors.accent());
        let accent_dim = Self::from_iced(colors.accent_muted());
        let success = Self::from_iced(colors.success());
        let warning = Self::from_iced(colors.warning());
        let text_secondary = Self::from_iced(colors.text_secondary());
        let text_muted = Self::from_iced(colors.text_muted());
        let bg_surface = Self::from_iced(colors.bg_surface());
        let bg_elevated = Self::from_iced(colors.bg_elevated());
        let bg_hover = Self::from_iced(colors.bg_hover());
        let bg_base = Self::from_iced(colors.bg_base());

        Self {
            bg_base,
            bg_surface,
            bg_elevated,
            bg_hover,
            text_primary: Self::from_iced(colors.text_primary()),
            text_secondary,
            text_muted,
            border_subtle: Self::from_iced(colors.border_subtle()),
            border_default: Self::from_iced(colors.border_default()),
            border_focused: accent,
            accent,
            accent_dim,
            success,
            warning,
            danger: Self::from_iced(colors.danger()),
            user_msg_bg: accent_dim,
            assistant_msg_bg: bg_surface,
            tool_call_bg: bg_elevated,
            reasoning_bg: bg_hover,
            code_bg: bg_base,
            syntax_keyword: accent,
            syntax_string: success,
            syntax_comment: text_muted,
            syntax_number: warning,
            syntax_function: accent_dim,
            syntax_type: text_secondary,
        }
    }

    pub fn dark() -> Self {
        let colors = DarkColors;
        Self::from_theme_colors(&colors)
    }

    pub fn light() -> Self {
        let colors = LightColors;
        Self::from_theme_colors(&colors)
    }

    pub fn zinc() -> Self {
        let colors = ZincColors;
        Self::from_theme_colors(&colors)
    }

    pub fn dracula() -> Self {
        let colors = DraculaColors;
        Self::from_theme_colors(&colors)
    }

    pub fn nord() -> Self {
        let colors = NordColors;
        Self::from_theme_colors(&colors)
    }

    pub fn catppuccin_mocha() -> Self {
        let colors = CatppuccinMochaColors;
        Self::from_theme_colors(&colors)
    }

    pub fn catppuccin_latte() -> Self {
        let colors = CatppuccinLatteColors;
        Self::from_theme_colors(&colors)
    }

    pub fn tokyo_night() -> Self {
        let colors = TokyoNightColors;
        Self::from_theme_colors(&colors)
    }

    pub fn one_dark() -> Self {
        let colors = OneDarkColors;
        Self::from_theme_colors(&colors)
    }

    pub fn gruvbox_dark() -> Self {
        let colors = GruvboxDarkColors;
        Self::from_theme_colors(&colors)
    }

    pub fn gruvbox_light() -> Self {
        let colors = GruvboxLightColors;
        Self::from_theme_colors(&colors)
    }
}
