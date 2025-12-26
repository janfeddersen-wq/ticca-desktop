//! Theme colors trait and registry for clean dispatch.
//!
//! This module replaces the massive if-else chains in styles.rs with
//! a trait-based dispatch system.

use iced::{Color, Theme};
use std::sync::LazyLock;
use std::collections::HashMap;

/// Trait defining all theme colors.
/// Each theme module implements this trait.
pub trait ThemeColors: Send + Sync {
    // Background layers
    fn bg_base(&self) -> Color;
    fn bg_surface(&self) -> Color;
    fn bg_elevated(&self) -> Color;
    fn bg_hover(&self) -> Color;

    // Text colors
    fn text_primary(&self) -> Color;
    fn text_secondary(&self) -> Color;
    fn text_muted(&self) -> Color;

    // Border colors
    fn border_subtle(&self) -> Color;
    fn border_default(&self) -> Color;

    // Accent colors
    fn accent(&self) -> Color;
    fn accent_hover(&self) -> Color;
    fn accent_muted(&self) -> Color;

    // Status colors
    #[allow(dead_code)]
    fn success(&self) -> Color;
    #[allow(dead_code)]
    fn warning(&self) -> Color;
    fn danger(&self) -> Color;
}

/// Static struct for Dark theme colors
pub struct DarkColors;
impl ThemeColors for DarkColors {
    fn bg_base(&self) -> Color { super::dark::colors::BG_BASE }
    fn bg_surface(&self) -> Color { super::dark::colors::BG_SURFACE }
    fn bg_elevated(&self) -> Color { super::dark::colors::BG_ELEVATED }
    fn bg_hover(&self) -> Color { super::dark::colors::BG_HOVER }
    fn text_primary(&self) -> Color { super::dark::colors::TEXT_PRIMARY }
    fn text_secondary(&self) -> Color { super::dark::colors::TEXT_SECONDARY }
    fn text_muted(&self) -> Color { super::dark::colors::TEXT_MUTED }
    fn border_subtle(&self) -> Color { super::dark::colors::BORDER_SUBTLE }
    fn border_default(&self) -> Color { super::dark::colors::BORDER_DEFAULT }
    fn accent(&self) -> Color { super::dark::colors::ACCENT }
    fn accent_hover(&self) -> Color { super::dark::colors::ACCENT_HOVER }
    fn accent_muted(&self) -> Color { super::dark::colors::ACCENT_MUTED }
    fn success(&self) -> Color { super::dark::colors::SUCCESS }
    fn warning(&self) -> Color { super::dark::colors::WARNING }
    fn danger(&self) -> Color { super::dark::colors::DANGER }
}

/// Static struct for Light theme colors
pub struct LightColors;
impl ThemeColors for LightColors {
    fn bg_base(&self) -> Color { super::light::colors::ZINC_50 }
    fn bg_surface(&self) -> Color { super::light::colors::WHITE }
    fn bg_elevated(&self) -> Color { super::light::colors::ZINC_100 }
    fn bg_hover(&self) -> Color { super::light::colors::ZINC_200 }
    fn text_primary(&self) -> Color { super::light::colors::ZINC_900 }
    fn text_secondary(&self) -> Color { super::light::colors::ZINC_600 }
    fn text_muted(&self) -> Color { super::light::colors::ZINC_400 }
    fn border_subtle(&self) -> Color { super::light::colors::ZINC_200 }
    fn border_default(&self) -> Color { super::light::colors::ZINC_300 }
    fn accent(&self) -> Color { super::light::colors::BLUE_600 }
    fn accent_hover(&self) -> Color { super::light::colors::BLUE_500 }
    fn accent_muted(&self) -> Color { super::light::colors::BLUE_700 }
    fn success(&self) -> Color { super::light::colors::GREEN_600 }
    fn warning(&self) -> Color { super::light::colors::YELLOW_600 }
    fn danger(&self) -> Color { super::light::colors::RED_600 }
}

/// Static struct for Zinc theme colors
pub struct ZincColors;
impl ThemeColors for ZincColors {
    fn bg_base(&self) -> Color { super::zinc::colors::BG_BASE }
    fn bg_surface(&self) -> Color { super::zinc::colors::BG_SURFACE }
    fn bg_elevated(&self) -> Color { super::zinc::colors::BG_ELEVATED }
    fn bg_hover(&self) -> Color { super::zinc::colors::BG_HOVER }
    fn text_primary(&self) -> Color { super::zinc::colors::TEXT_PRIMARY }
    fn text_secondary(&self) -> Color { super::zinc::colors::TEXT_SECONDARY }
    fn text_muted(&self) -> Color { super::zinc::colors::TEXT_MUTED }
    fn border_subtle(&self) -> Color { super::zinc::colors::BORDER_SUBTLE }
    fn border_default(&self) -> Color { super::zinc::colors::BORDER_DEFAULT }
    fn accent(&self) -> Color { super::zinc::colors::ACCENT }
    fn accent_hover(&self) -> Color { super::zinc::colors::ACCENT_HOVER }
    fn accent_muted(&self) -> Color { super::zinc::colors::ACCENT_MUTED }
    fn success(&self) -> Color { super::zinc::colors::SUCCESS }
    fn warning(&self) -> Color { super::zinc::colors::WARNING }
    fn danger(&self) -> Color { super::zinc::colors::DANGER }
}

/// Static struct for Dracula theme colors
pub struct DraculaColors;
impl ThemeColors for DraculaColors {
    fn bg_base(&self) -> Color { super::dracula::colors::BG_BASE }
    fn bg_surface(&self) -> Color { super::dracula::colors::BG_SURFACE }
    fn bg_elevated(&self) -> Color { super::dracula::colors::BG_ELEVATED }
    fn bg_hover(&self) -> Color { super::dracula::colors::BG_HOVER }
    fn text_primary(&self) -> Color { super::dracula::colors::TEXT_PRIMARY }
    fn text_secondary(&self) -> Color { super::dracula::colors::TEXT_SECONDARY }
    fn text_muted(&self) -> Color { super::dracula::colors::TEXT_MUTED }
    fn border_subtle(&self) -> Color { super::dracula::colors::BORDER_SUBTLE }
    fn border_default(&self) -> Color { super::dracula::colors::BORDER_DEFAULT }
    fn accent(&self) -> Color { super::dracula::colors::ACCENT }
    fn accent_hover(&self) -> Color { super::dracula::colors::ACCENT_HOVER }
    fn accent_muted(&self) -> Color { super::dracula::colors::ACCENT_MUTED }
    fn success(&self) -> Color { super::dracula::colors::SUCCESS }
    fn warning(&self) -> Color { super::dracula::colors::WARNING }
    fn danger(&self) -> Color { super::dracula::colors::DANGER }
}

/// Static struct for Nord theme colors
pub struct NordColors;
impl ThemeColors for NordColors {
    fn bg_base(&self) -> Color { super::nord::colors::BG_BASE }
    fn bg_surface(&self) -> Color { super::nord::colors::BG_SURFACE }
    fn bg_elevated(&self) -> Color { super::nord::colors::BG_ELEVATED }
    fn bg_hover(&self) -> Color { super::nord::colors::BG_HOVER }
    fn text_primary(&self) -> Color { super::nord::colors::TEXT_PRIMARY }
    fn text_secondary(&self) -> Color { super::nord::colors::TEXT_SECONDARY }
    fn text_muted(&self) -> Color { super::nord::colors::TEXT_MUTED }
    fn border_subtle(&self) -> Color { super::nord::colors::BORDER_SUBTLE }
    fn border_default(&self) -> Color { super::nord::colors::BORDER_DEFAULT }
    fn accent(&self) -> Color { super::nord::colors::ACCENT }
    fn accent_hover(&self) -> Color { super::nord::colors::ACCENT_HOVER }
    fn accent_muted(&self) -> Color { super::nord::colors::ACCENT_MUTED }
    fn success(&self) -> Color { super::nord::colors::SUCCESS }
    fn warning(&self) -> Color { super::nord::colors::WARNING }
    fn danger(&self) -> Color { super::nord::colors::DANGER }
}

/// Static struct for Catppuccin Mocha theme colors
pub struct CatppuccinMochaColors;
impl ThemeColors for CatppuccinMochaColors {
    fn bg_base(&self) -> Color { super::catppuccin_mocha::colors::BG_BASE }
    fn bg_surface(&self) -> Color { super::catppuccin_mocha::colors::BG_SURFACE }
    fn bg_elevated(&self) -> Color { super::catppuccin_mocha::colors::BG_ELEVATED }
    fn bg_hover(&self) -> Color { super::catppuccin_mocha::colors::BG_HOVER }
    fn text_primary(&self) -> Color { super::catppuccin_mocha::colors::TEXT_PRIMARY }
    fn text_secondary(&self) -> Color { super::catppuccin_mocha::colors::TEXT_SECONDARY }
    fn text_muted(&self) -> Color { super::catppuccin_mocha::colors::TEXT_MUTED }
    fn border_subtle(&self) -> Color { super::catppuccin_mocha::colors::BORDER_SUBTLE }
    fn border_default(&self) -> Color { super::catppuccin_mocha::colors::BORDER_DEFAULT }
    fn accent(&self) -> Color { super::catppuccin_mocha::colors::ACCENT }
    fn accent_hover(&self) -> Color { super::catppuccin_mocha::colors::ACCENT_HOVER }
    fn accent_muted(&self) -> Color { super::catppuccin_mocha::colors::ACCENT_MUTED }
    fn success(&self) -> Color { super::catppuccin_mocha::colors::SUCCESS }
    fn warning(&self) -> Color { super::catppuccin_mocha::colors::WARNING }
    fn danger(&self) -> Color { super::catppuccin_mocha::colors::DANGER }
}

/// Static struct for Catppuccin Latte theme colors
pub struct CatppuccinLatteColors;
impl ThemeColors for CatppuccinLatteColors {
    fn bg_base(&self) -> Color { super::catppuccin_latte::colors::BG_BASE }
    fn bg_surface(&self) -> Color { super::catppuccin_latte::colors::BG_SURFACE }
    fn bg_elevated(&self) -> Color { super::catppuccin_latte::colors::BG_ELEVATED }
    fn bg_hover(&self) -> Color { super::catppuccin_latte::colors::BG_HOVER }
    fn text_primary(&self) -> Color { super::catppuccin_latte::colors::TEXT_PRIMARY }
    fn text_secondary(&self) -> Color { super::catppuccin_latte::colors::TEXT_SECONDARY }
    fn text_muted(&self) -> Color { super::catppuccin_latte::colors::TEXT_MUTED }
    fn border_subtle(&self) -> Color { super::catppuccin_latte::colors::BORDER_SUBTLE }
    fn border_default(&self) -> Color { super::catppuccin_latte::colors::BORDER_DEFAULT }
    fn accent(&self) -> Color { super::catppuccin_latte::colors::ACCENT }
    fn accent_hover(&self) -> Color { super::catppuccin_latte::colors::ACCENT_HOVER }
    fn accent_muted(&self) -> Color { super::catppuccin_latte::colors::ACCENT_MUTED }
    fn success(&self) -> Color { super::catppuccin_latte::colors::SUCCESS }
    fn warning(&self) -> Color { super::catppuccin_latte::colors::WARNING }
    fn danger(&self) -> Color { super::catppuccin_latte::colors::DANGER }
}

/// Static struct for Tokyo Night theme colors
pub struct TokyoNightColors;
impl ThemeColors for TokyoNightColors {
    fn bg_base(&self) -> Color { super::tokyo_night::colors::BG_BASE }
    fn bg_surface(&self) -> Color { super::tokyo_night::colors::BG_SURFACE }
    fn bg_elevated(&self) -> Color { super::tokyo_night::colors::BG_ELEVATED }
    fn bg_hover(&self) -> Color { super::tokyo_night::colors::BG_HOVER }
    fn text_primary(&self) -> Color { super::tokyo_night::colors::TEXT_PRIMARY }
    fn text_secondary(&self) -> Color { super::tokyo_night::colors::TEXT_SECONDARY }
    fn text_muted(&self) -> Color { super::tokyo_night::colors::TEXT_MUTED }
    fn border_subtle(&self) -> Color { super::tokyo_night::colors::BORDER_SUBTLE }
    fn border_default(&self) -> Color { super::tokyo_night::colors::BORDER_DEFAULT }
    fn accent(&self) -> Color { super::tokyo_night::colors::ACCENT }
    fn accent_hover(&self) -> Color { super::tokyo_night::colors::ACCENT_HOVER }
    fn accent_muted(&self) -> Color { super::tokyo_night::colors::ACCENT_MUTED }
    fn success(&self) -> Color { super::tokyo_night::colors::SUCCESS }
    fn warning(&self) -> Color { super::tokyo_night::colors::WARNING }
    fn danger(&self) -> Color { super::tokyo_night::colors::DANGER }
}

/// Static struct for One Dark theme colors
pub struct OneDarkColors;
impl ThemeColors for OneDarkColors {
    fn bg_base(&self) -> Color { super::one_dark::colors::BG_BASE }
    fn bg_surface(&self) -> Color { super::one_dark::colors::BG_SURFACE }
    fn bg_elevated(&self) -> Color { super::one_dark::colors::BG_ELEVATED }
    fn bg_hover(&self) -> Color { super::one_dark::colors::BG_HOVER }
    fn text_primary(&self) -> Color { super::one_dark::colors::TEXT_PRIMARY }
    fn text_secondary(&self) -> Color { super::one_dark::colors::TEXT_SECONDARY }
    fn text_muted(&self) -> Color { super::one_dark::colors::TEXT_MUTED }
    fn border_subtle(&self) -> Color { super::one_dark::colors::BORDER_SUBTLE }
    fn border_default(&self) -> Color { super::one_dark::colors::BORDER_DEFAULT }
    fn accent(&self) -> Color { super::one_dark::colors::ACCENT }
    fn accent_hover(&self) -> Color { super::one_dark::colors::ACCENT_HOVER }
    fn accent_muted(&self) -> Color { super::one_dark::colors::ACCENT_MUTED }
    fn success(&self) -> Color { super::one_dark::colors::SUCCESS }
    fn warning(&self) -> Color { super::one_dark::colors::WARNING }
    fn danger(&self) -> Color { super::one_dark::colors::DANGER }
}

/// Static struct for Gruvbox Dark theme colors
pub struct GruvboxDarkColors;
impl ThemeColors for GruvboxDarkColors {
    fn bg_base(&self) -> Color { super::gruvbox_dark::colors::BG_BASE }
    fn bg_surface(&self) -> Color { super::gruvbox_dark::colors::BG_SURFACE }
    fn bg_elevated(&self) -> Color { super::gruvbox_dark::colors::BG_ELEVATED }
    fn bg_hover(&self) -> Color { super::gruvbox_dark::colors::BG_HOVER }
    fn text_primary(&self) -> Color { super::gruvbox_dark::colors::TEXT_PRIMARY }
    fn text_secondary(&self) -> Color { super::gruvbox_dark::colors::TEXT_SECONDARY }
    fn text_muted(&self) -> Color { super::gruvbox_dark::colors::TEXT_MUTED }
    fn border_subtle(&self) -> Color { super::gruvbox_dark::colors::BORDER_SUBTLE }
    fn border_default(&self) -> Color { super::gruvbox_dark::colors::BORDER_DEFAULT }
    fn accent(&self) -> Color { super::gruvbox_dark::colors::ACCENT }
    fn accent_hover(&self) -> Color { super::gruvbox_dark::colors::ACCENT_HOVER }
    fn accent_muted(&self) -> Color { super::gruvbox_dark::colors::ACCENT_MUTED }
    fn success(&self) -> Color { super::gruvbox_dark::colors::SUCCESS }
    fn warning(&self) -> Color { super::gruvbox_dark::colors::WARNING }
    fn danger(&self) -> Color { super::gruvbox_dark::colors::DANGER }
}

/// Static struct for Gruvbox Light theme colors
pub struct GruvboxLightColors;
impl ThemeColors for GruvboxLightColors {
    fn bg_base(&self) -> Color { super::gruvbox_light::colors::BG_BASE }
    fn bg_surface(&self) -> Color { super::gruvbox_light::colors::BG_SURFACE }
    fn bg_elevated(&self) -> Color { super::gruvbox_light::colors::BG_ELEVATED }
    fn bg_hover(&self) -> Color { super::gruvbox_light::colors::BG_HOVER }
    fn text_primary(&self) -> Color { super::gruvbox_light::colors::TEXT_PRIMARY }
    fn text_secondary(&self) -> Color { super::gruvbox_light::colors::TEXT_SECONDARY }
    fn text_muted(&self) -> Color { super::gruvbox_light::colors::TEXT_MUTED }
    fn border_subtle(&self) -> Color { super::gruvbox_light::colors::BORDER_SUBTLE }
    fn border_default(&self) -> Color { super::gruvbox_light::colors::BORDER_DEFAULT }
    fn accent(&self) -> Color { super::gruvbox_light::colors::ACCENT }
    fn accent_hover(&self) -> Color { super::gruvbox_light::colors::ACCENT_HOVER }
    fn accent_muted(&self) -> Color { super::gruvbox_light::colors::ACCENT_MUTED }
    fn success(&self) -> Color { super::gruvbox_light::colors::SUCCESS }
    fn warning(&self) -> Color { super::gruvbox_light::colors::WARNING }
    fn danger(&self) -> Color { super::gruvbox_light::colors::DANGER }
}

// Static instances for the registry
static DARK: DarkColors = DarkColors;
static LIGHT: LightColors = LightColors;
static ZINC: ZincColors = ZincColors;
static DRACULA: DraculaColors = DraculaColors;
static NORD: NordColors = NordColors;
static CATPPUCCIN_MOCHA: CatppuccinMochaColors = CatppuccinMochaColors;
static CATPPUCCIN_LATTE: CatppuccinLatteColors = CatppuccinLatteColors;
static TOKYO_NIGHT: TokyoNightColors = TokyoNightColors;
static ONE_DARK: OneDarkColors = OneDarkColors;
static GRUVBOX_DARK: GruvboxDarkColors = GruvboxDarkColors;
static GRUVBOX_LIGHT: GruvboxLightColors = GruvboxLightColors;

/// Theme registry mapping theme names to their color implementations.
static THEME_REGISTRY: LazyLock<HashMap<&'static str, &'static dyn ThemeColors>> = LazyLock::new(|| {
    let mut map: HashMap<&'static str, &'static dyn ThemeColors> = HashMap::new();
    map.insert("Dark", &DARK);
    map.insert("Light", &LIGHT);
    map.insert("Zinc", &ZINC);
    map.insert("Dracula", &DRACULA);
    map.insert("Nord", &NORD);
    map.insert("Catppuccin Mocha", &CATPPUCCIN_MOCHA);
    map.insert("Catppuccin Latte", &CATPPUCCIN_LATTE);
    map.insert("Tokyo Night", &TOKYO_NIGHT);
    map.insert("One Dark", &ONE_DARK);
    map.insert("Gruvbox Dark", &GRUVBOX_DARK);
    map.insert("Gruvbox Light", &GRUVBOX_LIGHT);
    // Partial matches for Debug format
    map.insert("Mocha", &CATPPUCCIN_MOCHA);
    map.insert("Latte", &CATPPUCCIN_LATTE);
    map.insert("Tokyo", &TOKYO_NIGHT);
    map
});

/// Get the theme colors for a given Iced Theme.
/// Falls back to Dark theme if not found.
pub fn get_theme_colors(theme: &Theme) -> &'static dyn ThemeColors {
    let name = format!("{:?}", theme);

    // Try exact match first
    if let Some(colors) = THEME_REGISTRY.get(name.as_str()) {
        return *colors;
    }

    // Try partial matches
    for (key, colors) in THEME_REGISTRY.iter() {
        if name.contains(key) {
            return *colors;
        }
    }

    // Default to Dark
    &DARK
}

/// Convenience function to check if a theme is dark.
pub fn is_dark_theme(theme: &Theme) -> bool {
    let name = format!("{:?}", theme);
    !name.contains("Light") && !name.contains("Latte")
}

// ============================================================================
// Color helper functions using the registry
// ============================================================================

pub fn bg_base(theme: &Theme) -> Color {
    get_theme_colors(theme).bg_base()
}

pub fn bg_surface(theme: &Theme) -> Color {
    get_theme_colors(theme).bg_surface()
}

pub fn bg_elevated(theme: &Theme) -> Color {
    get_theme_colors(theme).bg_elevated()
}

pub fn bg_hover(theme: &Theme) -> Color {
    get_theme_colors(theme).bg_hover()
}

pub fn text_primary(theme: &Theme) -> Color {
    get_theme_colors(theme).text_primary()
}

pub fn text_secondary(theme: &Theme) -> Color {
    get_theme_colors(theme).text_secondary()
}

pub fn text_muted(theme: &Theme) -> Color {
    get_theme_colors(theme).text_muted()
}

pub fn border_subtle(theme: &Theme) -> Color {
    get_theme_colors(theme).border_subtle()
}

pub fn border_default(theme: &Theme) -> Color {
    get_theme_colors(theme).border_default()
}

pub fn accent(theme: &Theme) -> Color {
    get_theme_colors(theme).accent()
}

pub fn accent_hover(theme: &Theme) -> Color {
    get_theme_colors(theme).accent_hover()
}

pub fn accent_muted(theme: &Theme) -> Color {
    get_theme_colors(theme).accent_muted()
}

#[allow(dead_code)]
pub fn success(theme: &Theme) -> Color {
    get_theme_colors(theme).success()
}

#[allow(dead_code)]
pub fn warning(theme: &Theme) -> Color {
    get_theme_colors(theme).warning()
}

pub fn danger(theme: &Theme) -> Color {
    get_theme_colors(theme).danger()
}
