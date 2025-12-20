//! Widget styling - Clean, minimal design
//!
//! Theme-aware styles that adapt to all supported themes.

use iced::widget::{button, container, text_editor, text_input};
use iced::{Border, Color, Theme};

use super::{
    catppuccin_latte, catppuccin_mocha, dark, dracula, gruvbox_dark, gruvbox_light, light, nord,
    one_dark, tokyo_night, zinc,
};

/// Border radius constants
const RADIUS_SM: f32 = 4.0;
const RADIUS_MD: f32 = 8.0;
const RADIUS_LG: f32 = 12.0;
const RADIUS_FULL: f32 = 999.0;

// ============================================================================
// Theme detection helper
// ============================================================================

fn get_theme_name(theme: &Theme) -> String {
    format!("{:?}", theme)
}

fn is_dark_theme(theme: &Theme) -> bool {
    let name = get_theme_name(theme);
    !name.contains("Light") && !name.contains("Latte")
}

// ============================================================================
// Theme-aware color helpers - dispatch to correct theme module
// ============================================================================

fn bg_base(theme: &Theme) -> Color {
    let name = get_theme_name(theme);
    if name.contains("Dracula") {
        dracula::colors::BG_BASE
    } else if name.contains("Nord") {
        nord::colors::BG_BASE
    } else if name.contains("Mocha") {
        catppuccin_mocha::colors::BG_BASE
    } else if name.contains("Latte") {
        catppuccin_latte::colors::BG_BASE
    } else if name.contains("Tokyo") {
        tokyo_night::colors::BG_BASE
    } else if name.contains("One Dark") {
        one_dark::colors::BG_BASE
    } else if name.contains("Gruvbox Dark") {
        gruvbox_dark::colors::BG_BASE
    } else if name.contains("Gruvbox Light") {
        gruvbox_light::colors::BG_BASE
    } else if name.contains("Light") {
        light::colors::ZINC_50
    } else if name.contains("Zinc") {
        zinc::colors::BG_BASE
    } else {
        dark::colors::BG_BASE
    }
}

fn bg_surface(theme: &Theme) -> Color {
    let name = get_theme_name(theme);
    if name.contains("Dracula") {
        dracula::colors::BG_SURFACE
    } else if name.contains("Nord") {
        nord::colors::BG_SURFACE
    } else if name.contains("Mocha") {
        catppuccin_mocha::colors::BG_SURFACE
    } else if name.contains("Latte") {
        catppuccin_latte::colors::BG_SURFACE
    } else if name.contains("Tokyo") {
        tokyo_night::colors::BG_SURFACE
    } else if name.contains("One Dark") {
        one_dark::colors::BG_SURFACE
    } else if name.contains("Gruvbox Dark") {
        gruvbox_dark::colors::BG_SURFACE
    } else if name.contains("Gruvbox Light") {
        gruvbox_light::colors::BG_SURFACE
    } else if name.contains("Light") {
        light::colors::WHITE
    } else if name.contains("Zinc") {
        zinc::colors::BG_SURFACE
    } else {
        dark::colors::BG_SURFACE
    }
}

fn bg_elevated(theme: &Theme) -> Color {
    let name = get_theme_name(theme);
    if name.contains("Dracula") {
        dracula::colors::BG_ELEVATED
    } else if name.contains("Nord") {
        nord::colors::BG_ELEVATED
    } else if name.contains("Mocha") {
        catppuccin_mocha::colors::BG_ELEVATED
    } else if name.contains("Latte") {
        catppuccin_latte::colors::BG_ELEVATED
    } else if name.contains("Tokyo") {
        tokyo_night::colors::BG_ELEVATED
    } else if name.contains("One Dark") {
        one_dark::colors::BG_ELEVATED
    } else if name.contains("Gruvbox Dark") {
        gruvbox_dark::colors::BG_ELEVATED
    } else if name.contains("Gruvbox Light") {
        gruvbox_light::colors::BG_ELEVATED
    } else if name.contains("Light") {
        light::colors::ZINC_100
    } else if name.contains("Zinc") {
        zinc::colors::BG_ELEVATED
    } else {
        dark::colors::BG_ELEVATED
    }
}

fn bg_hover(theme: &Theme) -> Color {
    let name = get_theme_name(theme);
    if name.contains("Dracula") {
        dracula::colors::BG_HOVER
    } else if name.contains("Nord") {
        nord::colors::BG_HOVER
    } else if name.contains("Mocha") {
        catppuccin_mocha::colors::BG_HOVER
    } else if name.contains("Latte") {
        catppuccin_latte::colors::BG_HOVER
    } else if name.contains("Tokyo") {
        tokyo_night::colors::BG_HOVER
    } else if name.contains("One Dark") {
        one_dark::colors::BG_HOVER
    } else if name.contains("Gruvbox Dark") {
        gruvbox_dark::colors::BG_HOVER
    } else if name.contains("Gruvbox Light") {
        gruvbox_light::colors::BG_HOVER
    } else if name.contains("Light") {
        light::colors::ZINC_200
    } else if name.contains("Zinc") {
        zinc::colors::BG_HOVER
    } else {
        dark::colors::BG_HOVER
    }
}

fn text_primary(theme: &Theme) -> Color {
    let name = get_theme_name(theme);
    if name.contains("Dracula") {
        dracula::colors::TEXT_PRIMARY
    } else if name.contains("Nord") {
        nord::colors::TEXT_PRIMARY
    } else if name.contains("Mocha") {
        catppuccin_mocha::colors::TEXT_PRIMARY
    } else if name.contains("Latte") {
        catppuccin_latte::colors::TEXT_PRIMARY
    } else if name.contains("Tokyo") {
        tokyo_night::colors::TEXT_PRIMARY
    } else if name.contains("One Dark") {
        one_dark::colors::TEXT_PRIMARY
    } else if name.contains("Gruvbox Dark") {
        gruvbox_dark::colors::TEXT_PRIMARY
    } else if name.contains("Gruvbox Light") {
        gruvbox_light::colors::TEXT_PRIMARY
    } else if name.contains("Light") {
        light::colors::ZINC_900
    } else if name.contains("Zinc") {
        zinc::colors::TEXT_PRIMARY
    } else {
        dark::colors::TEXT_PRIMARY
    }
}

fn text_secondary(theme: &Theme) -> Color {
    let name = get_theme_name(theme);
    if name.contains("Dracula") {
        dracula::colors::TEXT_SECONDARY
    } else if name.contains("Nord") {
        nord::colors::TEXT_SECONDARY
    } else if name.contains("Mocha") {
        catppuccin_mocha::colors::TEXT_SECONDARY
    } else if name.contains("Latte") {
        catppuccin_latte::colors::TEXT_SECONDARY
    } else if name.contains("Tokyo") {
        tokyo_night::colors::TEXT_SECONDARY
    } else if name.contains("One Dark") {
        one_dark::colors::TEXT_SECONDARY
    } else if name.contains("Gruvbox Dark") {
        gruvbox_dark::colors::TEXT_SECONDARY
    } else if name.contains("Gruvbox Light") {
        gruvbox_light::colors::TEXT_SECONDARY
    } else if name.contains("Light") {
        light::colors::ZINC_600
    } else if name.contains("Zinc") {
        zinc::colors::TEXT_SECONDARY
    } else {
        dark::colors::TEXT_SECONDARY
    }
}

fn text_muted(theme: &Theme) -> Color {
    let name = get_theme_name(theme);
    if name.contains("Dracula") {
        dracula::colors::TEXT_MUTED
    } else if name.contains("Nord") {
        nord::colors::TEXT_MUTED
    } else if name.contains("Mocha") {
        catppuccin_mocha::colors::TEXT_MUTED
    } else if name.contains("Latte") {
        catppuccin_latte::colors::TEXT_MUTED
    } else if name.contains("Tokyo") {
        tokyo_night::colors::TEXT_MUTED
    } else if name.contains("One Dark") {
        one_dark::colors::TEXT_MUTED
    } else if name.contains("Gruvbox Dark") {
        gruvbox_dark::colors::TEXT_MUTED
    } else if name.contains("Gruvbox Light") {
        gruvbox_light::colors::TEXT_MUTED
    } else if name.contains("Light") {
        light::colors::ZINC_400
    } else if name.contains("Zinc") {
        zinc::colors::TEXT_MUTED
    } else {
        dark::colors::TEXT_MUTED
    }
}

fn border_subtle(theme: &Theme) -> Color {
    let name = get_theme_name(theme);
    if name.contains("Dracula") {
        dracula::colors::BORDER_SUBTLE
    } else if name.contains("Nord") {
        nord::colors::BORDER_SUBTLE
    } else if name.contains("Mocha") {
        catppuccin_mocha::colors::BORDER_SUBTLE
    } else if name.contains("Latte") {
        catppuccin_latte::colors::BORDER_SUBTLE
    } else if name.contains("Tokyo") {
        tokyo_night::colors::BORDER_SUBTLE
    } else if name.contains("One Dark") {
        one_dark::colors::BORDER_SUBTLE
    } else if name.contains("Gruvbox Dark") {
        gruvbox_dark::colors::BORDER_SUBTLE
    } else if name.contains("Gruvbox Light") {
        gruvbox_light::colors::BORDER_SUBTLE
    } else if name.contains("Light") {
        light::colors::ZINC_200
    } else if name.contains("Zinc") {
        zinc::colors::BORDER_SUBTLE
    } else {
        dark::colors::BORDER_SUBTLE
    }
}

fn border_default(theme: &Theme) -> Color {
    let name = get_theme_name(theme);
    if name.contains("Dracula") {
        dracula::colors::BORDER_DEFAULT
    } else if name.contains("Nord") {
        nord::colors::BORDER_DEFAULT
    } else if name.contains("Mocha") {
        catppuccin_mocha::colors::BORDER_DEFAULT
    } else if name.contains("Latte") {
        catppuccin_latte::colors::BORDER_DEFAULT
    } else if name.contains("Tokyo") {
        tokyo_night::colors::BORDER_DEFAULT
    } else if name.contains("One Dark") {
        one_dark::colors::BORDER_DEFAULT
    } else if name.contains("Gruvbox Dark") {
        gruvbox_dark::colors::BORDER_DEFAULT
    } else if name.contains("Gruvbox Light") {
        gruvbox_light::colors::BORDER_DEFAULT
    } else if name.contains("Light") {
        light::colors::ZINC_300
    } else if name.contains("Zinc") {
        zinc::colors::BORDER_DEFAULT
    } else {
        dark::colors::BORDER_DEFAULT
    }
}

fn accent(theme: &Theme) -> Color {
    let name = get_theme_name(theme);
    if name.contains("Dracula") {
        dracula::colors::ACCENT
    } else if name.contains("Nord") {
        nord::colors::ACCENT
    } else if name.contains("Mocha") {
        catppuccin_mocha::colors::ACCENT
    } else if name.contains("Latte") {
        catppuccin_latte::colors::ACCENT
    } else if name.contains("Tokyo") {
        tokyo_night::colors::ACCENT
    } else if name.contains("One Dark") {
        one_dark::colors::ACCENT
    } else if name.contains("Gruvbox Dark") {
        gruvbox_dark::colors::ACCENT
    } else if name.contains("Gruvbox Light") {
        gruvbox_light::colors::ACCENT
    } else if name.contains("Light") {
        light::colors::BLUE_600
    } else if name.contains("Zinc") {
        zinc::colors::ACCENT
    } else {
        dark::colors::ACCENT
    }
}

fn accent_hover(theme: &Theme) -> Color {
    let name = get_theme_name(theme);
    if name.contains("Dracula") {
        dracula::colors::ACCENT_HOVER
    } else if name.contains("Nord") {
        nord::colors::ACCENT_HOVER
    } else if name.contains("Mocha") {
        catppuccin_mocha::colors::ACCENT_HOVER
    } else if name.contains("Latte") {
        catppuccin_latte::colors::ACCENT_HOVER
    } else if name.contains("Tokyo") {
        tokyo_night::colors::ACCENT_HOVER
    } else if name.contains("One Dark") {
        one_dark::colors::ACCENT_HOVER
    } else if name.contains("Gruvbox Dark") {
        gruvbox_dark::colors::ACCENT_HOVER
    } else if name.contains("Gruvbox Light") {
        gruvbox_light::colors::ACCENT_HOVER
    } else if name.contains("Light") {
        light::colors::BLUE_500
    } else if name.contains("Zinc") {
        zinc::colors::ACCENT_HOVER
    } else {
        dark::colors::ACCENT_HOVER
    }
}

fn accent_muted(theme: &Theme) -> Color {
    let name = get_theme_name(theme);
    if name.contains("Dracula") {
        dracula::colors::ACCENT_MUTED
    } else if name.contains("Nord") {
        nord::colors::ACCENT_MUTED
    } else if name.contains("Mocha") {
        catppuccin_mocha::colors::ACCENT_MUTED
    } else if name.contains("Latte") {
        catppuccin_latte::colors::ACCENT_MUTED
    } else if name.contains("Tokyo") {
        tokyo_night::colors::ACCENT_MUTED
    } else if name.contains("One Dark") {
        one_dark::colors::ACCENT_MUTED
    } else if name.contains("Gruvbox Dark") {
        gruvbox_dark::colors::ACCENT_MUTED
    } else if name.contains("Gruvbox Light") {
        gruvbox_light::colors::ACCENT_MUTED
    } else if name.contains("Light") {
        light::colors::BLUE_700
    } else if name.contains("Zinc") {
        zinc::colors::ACCENT_MUTED
    } else {
        dark::colors::ACCENT_MUTED
    }
}

fn danger(theme: &Theme) -> Color {
    let name = get_theme_name(theme);
    if name.contains("Dracula") {
        dracula::colors::DANGER
    } else if name.contains("Nord") {
        nord::colors::DANGER
    } else if name.contains("Mocha") {
        catppuccin_mocha::colors::DANGER
    } else if name.contains("Latte") {
        catppuccin_latte::colors::DANGER
    } else if name.contains("Tokyo") {
        tokyo_night::colors::DANGER
    } else if name.contains("One Dark") {
        one_dark::colors::DANGER
    } else if name.contains("Gruvbox Dark") {
        gruvbox_dark::colors::DANGER
    } else if name.contains("Gruvbox Light") {
        gruvbox_light::colors::DANGER
    } else if name.contains("Light") {
        light::colors::RED_600
    } else if name.contains("Zinc") {
        zinc::colors::DANGER
    } else {
        dark::colors::DANGER
    }
}

// ============================================================================
// Button Styles
// ============================================================================

/// Primary button - main actions
#[allow(dead_code)]
pub fn primary_button(theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(accent(theme).into()),
        text_color: Color::WHITE,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: RADIUS_MD.into(),
        },
        ..button::Style::default()
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(accent_hover(theme).into()),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(accent_muted(theme).into()),
            ..base
        },
        button::Status::Disabled => button::Style {
            background: Some(bg_elevated(theme).into()),
            text_color: text_muted(theme),
            ..base
        },
    }
}

/// Secondary button - less prominent
pub fn secondary_button(theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(bg_surface(theme).into()),
        text_color: text_primary(theme),
        border: Border {
            color: border_subtle(theme),
            width: 1.0,
            radius: RADIUS_MD.into(),
        },
        ..button::Style::default()
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(bg_elevated(theme).into()),
            border: Border {
                color: border_default(theme),
                width: 1.0,
                radius: RADIUS_MD.into(),
            },
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(bg_hover(theme).into()),
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: text_muted(theme),
            border: Border {
                color: Color::TRANSPARENT,
                ..base.border
            },
            ..base
        },
    }
}

/// Icon button - minimal, transparent background
pub fn icon_button(theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: None,
        text_color: text_secondary(theme),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: RADIUS_SM.into(),
        },
        ..button::Style::default()
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(bg_elevated(theme).into()),
            text_color: text_primary(theme),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(bg_hover(theme).into()),
            text_color: accent(theme),
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: text_muted(theme),
            ..base
        },
    }
}

/// Tab button - agent switcher
pub fn tab_button(theme: &Theme, status: button::Status, is_active: bool) -> button::Style {
    if is_active {
        button::Style {
            background: Some(accent(theme).into()),
            text_color: Color::WHITE,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: RADIUS_MD.into(),
            },
            ..button::Style::default()
        }
    } else {
        let base = button::Style {
            background: Some(bg_surface(theme).into()),
            text_color: text_secondary(theme),
            border: Border {
                color: border_subtle(theme),
                width: 1.0,
                radius: RADIUS_MD.into(),
            },
            ..button::Style::default()
        };

        match status {
            button::Status::Active => base,
            button::Status::Hovered => button::Style {
                background: Some(bg_elevated(theme).into()),
                text_color: text_primary(theme),
                border: Border {
                    color: border_default(theme),
                    ..base.border
                },
                ..base
            },
            button::Status::Pressed => button::Style {
                background: Some(bg_hover(theme).into()),
                ..base
            },
            button::Status::Disabled => base,
        }
    }
}

/// Sidebar tab button - classic "tabs" look (not pill buttons)
pub fn sidebar_tab_button(theme: &Theme, status: button::Status, is_active: bool) -> button::Style {
    let base = button::Style {
        background: Some(
            (if is_active {
                bg_surface(theme)
            } else {
                bg_elevated(theme)
            })
            .into(),
        ),
        text_color: if is_active {
            text_primary(theme)
        } else {
            text_secondary(theme)
        },
        border: Border {
            color: if is_active {
                border_default(theme)
            } else {
                border_subtle(theme)
            },
            width: 1.0,
            radius: 0.0.into(),
        },
        ..button::Style::default()
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(
                (if is_active {
                    bg_surface(theme)
                } else {
                    bg_hover(theme)
                })
                .into(),
            ),
            text_color: text_primary(theme),
            border: Border {
                color: border_default(theme),
                width: 1.0,
                radius: 0.0.into(),
            },
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(bg_hover(theme).into()),
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: text_muted(theme),
            ..base
        },
    }
}

/// Send button - circular accent button
pub fn send_button(theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(accent(theme).into()),
        text_color: Color::WHITE,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: RADIUS_FULL.into(),
        },
        ..button::Style::default()
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(accent_hover(theme).into()),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(accent_muted(theme).into()),
            ..base
        },
        button::Status::Disabled => button::Style {
            background: Some(bg_elevated(theme).into()),
            text_color: text_muted(theme),
            ..base
        },
    }
}

/// Success button - authenticated/completed state
pub fn success_button(theme: &Theme, status: button::Status) -> button::Style {
    // Use green tones for success
    let success = Color::from_rgb(0.22, 0.65, 0.38); // A pleasant green
    let success_hover = Color::from_rgb(0.18, 0.55, 0.32);
    let success_pressed = Color::from_rgb(0.16, 0.48, 0.28);

    let base = button::Style {
        background: Some(success.into()),
        text_color: Color::WHITE,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: RADIUS_MD.into(),
        },
        ..button::Style::default()
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(success_hover.into()),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(success_pressed.into()),
            ..base
        },
        button::Status::Disabled => button::Style {
            background: Some(bg_elevated(theme).into()),
            text_color: text_muted(theme),
            ..base
        },
    }
}

/// Remove attachment button
pub fn remove_attachment_button(theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.5).into()),
        text_color: Color::WHITE,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: RADIUS_FULL.into(),
        },
        ..button::Style::default()
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(danger(theme).into()),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Color::from_rgb(0.7, 0.25, 0.25).into()),
            ..base
        },
        button::Status::Disabled => base,
    }
}

/// Danger icon button - used for destructive actions in tool panels
pub fn danger_icon_button(theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(bg_surface(theme).into()),
        text_color: danger(theme),
        border: Border {
            color: border_subtle(theme),
            width: 1.0,
            radius: RADIUS_FULL.into(),
        },
        ..button::Style::default()
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(danger(theme).into()),
            text_color: Color::WHITE,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: RADIUS_FULL.into(),
            },
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Color::from_rgb(0.7, 0.25, 0.25).into()),
            text_color: Color::WHITE,
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: text_muted(theme),
            ..base
        },
    }
}

// ============================================================================
// Container Styles
// ============================================================================

/// Header container - top bar
pub fn header_container(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(bg_surface(theme).into()),
        text_color: Some(text_primary(theme)),
        border: Border {
            color: border_subtle(theme),
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Directory bar - working directory display
pub fn dir_bar_container(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(bg_base(theme).into()),
        text_color: Some(text_secondary(theme)),
        border: Border {
            color: border_subtle(theme),
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Agent flow panel container
pub fn flow_panel_container(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(bg_surface(theme).into()),
        text_color: Some(text_primary(theme)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Message bubble
pub fn message_bubble(theme: &Theme, is_user: bool) -> container::Style {
    if is_user {
        container::Style {
            background: Some(accent_muted(theme).into()),
            text_color: Some(if is_dark_theme(theme) {
                text_primary(theme)
            } else {
                Color::WHITE
            }),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: RADIUS_LG.into(),
            },
            ..container::Style::default()
        }
    } else {
        container::Style {
            background: Some(bg_surface(theme).into()),
            text_color: Some(text_primary(theme)),
            border: Border {
                color: border_subtle(theme),
                width: 1.0,
                radius: RADIUS_LG.into(),
            },
            ..container::Style::default()
        }
    }
}

/// Card container - settings sections
pub fn card_container(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(bg_surface(theme).into()),
        text_color: Some(text_primary(theme)),
        border: Border {
            color: border_subtle(theme),
            width: 1.0,
            radius: RADIUS_MD.into(),
        },
        ..container::Style::default()
    }
}

/// Input area container
pub fn input_area_container(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(bg_surface(theme).into()),
        text_color: Some(text_primary(theme)),
        border: Border {
            color: border_subtle(theme),
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Attachment bar container
pub fn attachment_bar_container(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(bg_surface(theme).into()),
        text_color: Some(text_secondary(theme)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Image thumbnail container
pub fn image_thumbnail_container(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(bg_elevated(theme).into()),
        text_color: Some(text_primary(theme)),
        border: Border {
            color: border_subtle(theme),
            width: 1.0,
            radius: RADIUS_SM.into(),
        },
        ..container::Style::default()
    }
}

/// Code block container
#[allow(dead_code)]
pub fn code_block(theme: &Theme, _is_dark: bool) -> container::Style {
    container::Style {
        background: Some(bg_base(theme).into()),
        text_color: Some(text_primary(theme)),
        border: Border {
            color: border_subtle(theme),
            width: 1.0,
            radius: RADIUS_SM.into(),
        },
        ..container::Style::default()
    }
}

/// Table header
#[allow(dead_code)]
pub fn table_header(theme: &Theme, _is_dark: bool) -> container::Style {
    container::Style {
        background: Some(bg_elevated(theme).into()),
        text_color: Some(text_primary(theme)),
        border: Border {
            color: border_subtle(theme),
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Table row
#[allow(dead_code)]
pub fn table_row(theme: &Theme, _is_dark: bool, is_alternate: bool) -> container::Style {
    let bg = if is_alternate {
        bg_surface(theme)
    } else {
        bg_base(theme)
    };

    container::Style {
        background: Some(bg.into()),
        text_color: Some(text_primary(theme)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Reasoning/thinking container
pub fn reasoning_container(theme: &Theme, _is_dark: bool) -> container::Style {
    let bg = if is_dark_theme(theme) {
        Color::from_rgba(0.15, 0.15, 0.20, 0.5)
    } else {
        Color::from_rgba(0.9, 0.9, 0.95, 0.8)
    };

    container::Style {
        background: Some(bg.into()),
        text_color: Some(text_secondary(theme)),
        border: Border {
            color: border_subtle(theme),
            width: 1.0,
            radius: RADIUS_SM.into(),
        },
        ..container::Style::default()
    }
}

// ============================================================================
// Input Styles
// ============================================================================

/// Text input style
pub fn text_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let base = text_input::Style {
        background: bg_elevated(theme).into(),
        border: Border {
            color: border_subtle(theme),
            width: 1.0,
            radius: RADIUS_MD.into(),
        },
        icon: text_secondary(theme),
        placeholder: text_muted(theme),
        value: text_primary(theme),
        selection: accent_muted(theme),
    };

    match status {
        text_input::Status::Active => base,
        text_input::Status::Hovered => text_input::Style {
            border: Border {
                color: border_default(theme),
                ..base.border
            },
            ..base
        },
        text_input::Status::Focused { .. } => text_input::Style {
            border: Border {
                color: accent(theme),
                width: 2.0,
                ..base.border
            },
            ..base
        },
        text_input::Status::Disabled => text_input::Style {
            background: bg_base(theme).into(),
            placeholder: text_muted(theme),
            value: text_muted(theme),
            border: Border {
                color: Color::TRANSPARENT,
                ..base.border
            },
            ..base
        },
    }
}

/// Raw text editor style
pub fn raw_text_editor(theme: &Theme, _is_dark: bool) -> text_editor::Style {
    text_editor::Style {
        background: bg_base(theme).into(),
        border: Border {
            color: border_subtle(theme),
            width: 1.0,
            radius: RADIUS_SM.into(),
        },
        placeholder: text_muted(theme),
        value: text_primary(theme),
        selection: accent_muted(theme),
    }
}

/// Streaming indicator container - shows TPS and pulse
pub fn streaming_indicator_container(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(bg_elevated(theme).into()),
        text_color: Some(accent(theme)),
        border: Border {
            color: accent(theme),
            width: 1.0,
            radius: RADIUS_FULL.into(),
        },
        ..container::Style::default()
    }
}
