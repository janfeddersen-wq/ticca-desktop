//! Widget styling - Clean, minimal design
//!
//! Uses direct color references for consistency across the app.

use iced::widget::{button, container, text_editor, text_input};
use iced::{Border, Color, Theme};

use super::dark::colors as dark;

/// Border radius constants
const RADIUS_SM: f32 = 4.0;
const RADIUS_MD: f32 = 8.0;
const RADIUS_LG: f32 = 12.0;
const RADIUS_FULL: f32 = 999.0;

// ============================================================================
// Helper to check if using dark theme
// ============================================================================

fn is_dark_theme(theme: &Theme) -> bool {
    let name = format!("{:?}", theme);
    name.contains("Dark") || name.contains("Ticca")
}

// ============================================================================
// Button Styles
// ============================================================================

/// Primary button - main actions
#[allow(dead_code)]
pub fn primary_button(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(dark::ACCENT.into()),
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
            background: Some(dark::ACCENT_HOVER.into()),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(dark::ACCENT_MUTED.into()),
            ..base
        },
        button::Status::Disabled => button::Style {
            background: Some(dark::BG_ELEVATED.into()),
            text_color: dark::TEXT_MUTED,
            ..base
        },
    }
}

/// Secondary button - less prominent
pub fn secondary_button(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(dark::BG_SURFACE.into()),
        text_color: dark::TEXT_PRIMARY,
        border: Border {
            color: dark::BORDER_SUBTLE,
            width: 1.0,
            radius: RADIUS_MD.into(),
        },
        ..button::Style::default()
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(dark::BG_ELEVATED.into()),
            border: Border {
                color: dark::BORDER_DEFAULT,
                width: 1.0,
                radius: RADIUS_MD.into(),
            },
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(dark::BG_HOVER.into()),
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: dark::TEXT_MUTED,
            border: Border {
                color: Color::TRANSPARENT,
                ..base.border
            },
            ..base
        },
    }
}

/// Icon button - minimal, transparent background
pub fn icon_button(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: None,
        text_color: dark::TEXT_SECONDARY,
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
            background: Some(dark::BG_ELEVATED.into()),
            text_color: dark::TEXT_PRIMARY,
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(dark::BG_HOVER.into()),
            text_color: dark::ACCENT,
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: dark::TEXT_MUTED,
            ..base
        },
    }
}

/// Tab button - agent switcher
pub fn tab_button(_theme: &Theme, status: button::Status, is_active: bool) -> button::Style {
    if is_active {
        button::Style {
            background: Some(dark::ACCENT.into()),
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
            background: Some(dark::BG_SURFACE.into()),
            text_color: dark::TEXT_SECONDARY,
            border: Border {
                color: dark::BORDER_SUBTLE,
                width: 1.0,
                radius: RADIUS_MD.into(),
            },
            ..button::Style::default()
        };

        match status {
            button::Status::Active => base,
            button::Status::Hovered => button::Style {
                background: Some(dark::BG_ELEVATED.into()),
                text_color: dark::TEXT_PRIMARY,
                border: Border {
                    color: dark::BORDER_DEFAULT,
                    ..base.border
                },
                ..base
            },
            button::Status::Pressed => button::Style {
                background: Some(dark::BG_HOVER.into()),
                ..base
            },
            button::Status::Disabled => base,
        }
    }
}

/// Send button - circular accent button
pub fn send_button(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(dark::ACCENT.into()),
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
            background: Some(dark::ACCENT_HOVER.into()),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(dark::ACCENT_MUTED.into()),
            ..base
        },
        button::Status::Disabled => button::Style {
            background: Some(dark::BG_ELEVATED.into()),
            text_color: dark::TEXT_MUTED,
            ..base
        },
    }
}

/// Remove attachment button
pub fn remove_attachment_button(_theme: &Theme, status: button::Status) -> button::Style {
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
            background: Some(dark::DANGER.into()),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Color::from_rgb(0.7, 0.25, 0.25).into()),
            ..base
        },
        button::Status::Disabled => base,
    }
}

// ============================================================================
// Container Styles
// ============================================================================

/// Header container - top bar
pub fn header_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(dark::BG_SURFACE.into()),
        text_color: Some(dark::TEXT_PRIMARY),
        border: Border {
            color: dark::BORDER_SUBTLE,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Directory bar - working directory display
pub fn dir_bar_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(dark::BG_BASE.into()),
        text_color: Some(dark::TEXT_SECONDARY),
        border: Border {
            color: dark::BORDER_SUBTLE,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Message bubble
pub fn message_bubble(_theme: &Theme, is_user: bool) -> container::Style {
    if is_user {
        container::Style {
            background: Some(dark::ACCENT_MUTED.into()),
            text_color: Some(dark::TEXT_PRIMARY),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: RADIUS_LG.into(),
            },
            ..container::Style::default()
        }
    } else {
        container::Style {
            background: Some(dark::BG_SURFACE.into()),
            text_color: Some(dark::TEXT_PRIMARY),
            border: Border {
                color: dark::BORDER_SUBTLE,
                width: 1.0,
                radius: RADIUS_LG.into(),
            },
            ..container::Style::default()
        }
    }
}

/// Card container - settings sections
pub fn card_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(dark::BG_SURFACE.into()),
        text_color: Some(dark::TEXT_PRIMARY),
        border: Border {
            color: dark::BORDER_SUBTLE,
            width: 1.0,
            radius: RADIUS_MD.into(),
        },
        ..container::Style::default()
    }
}

/// Input area container
pub fn input_area_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(dark::BG_SURFACE.into()),
        text_color: Some(dark::TEXT_PRIMARY),
        border: Border {
            color: dark::BORDER_SUBTLE,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Attachment bar container
pub fn attachment_bar_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(dark::BG_SURFACE.into()),
        text_color: Some(dark::TEXT_SECONDARY),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Image thumbnail container
pub fn image_thumbnail_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(dark::BG_ELEVATED.into()),
        text_color: Some(dark::TEXT_PRIMARY),
        border: Border {
            color: dark::BORDER_SUBTLE,
            width: 1.0,
            radius: RADIUS_SM.into(),
        },
        ..container::Style::default()
    }
}

/// Code block container
pub fn code_block(_theme: &Theme, _is_dark: bool) -> container::Style {
    container::Style {
        background: Some(dark::BG_BASE.into()),
        text_color: Some(dark::TEXT_PRIMARY),
        border: Border {
            color: dark::BORDER_SUBTLE,
            width: 1.0,
            radius: RADIUS_SM.into(),
        },
        ..container::Style::default()
    }
}

/// Table header
pub fn table_header(_theme: &Theme, _is_dark: bool) -> container::Style {
    container::Style {
        background: Some(dark::BG_ELEVATED.into()),
        text_color: Some(dark::TEXT_PRIMARY),
        border: Border {
            color: dark::BORDER_SUBTLE,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Table row
pub fn table_row(_theme: &Theme, _is_dark: bool, is_alternate: bool) -> container::Style {
    let bg = if is_alternate {
        dark::BG_SURFACE
    } else {
        dark::BG_BASE
    };

    container::Style {
        background: Some(bg.into()),
        text_color: Some(dark::TEXT_PRIMARY),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Reasoning/thinking container
pub fn reasoning_container(_theme: &Theme, _is_dark: bool) -> container::Style {
    container::Style {
        background: Some(Color::from_rgba(0.15, 0.15, 0.20, 0.5).into()),
        text_color: Some(dark::TEXT_SECONDARY),
        border: Border {
            color: dark::BORDER_SUBTLE,
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
pub fn text_input_style(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let base = text_input::Style {
        background: dark::BG_ELEVATED.into(),
        border: Border {
            color: dark::BORDER_SUBTLE,
            width: 1.0,
            radius: RADIUS_MD.into(),
        },
        icon: dark::TEXT_SECONDARY,
        placeholder: dark::TEXT_MUTED,
        value: dark::TEXT_PRIMARY,
        selection: dark::ACCENT_MUTED,
    };

    match status {
        text_input::Status::Active => base,
        text_input::Status::Hovered => text_input::Style {
            border: Border {
                color: dark::BORDER_DEFAULT,
                ..base.border
            },
            ..base
        },
        text_input::Status::Focused { .. } => text_input::Style {
            border: Border {
                color: dark::ACCENT,
                width: 2.0,
                ..base.border
            },
            ..base
        },
        text_input::Status::Disabled => text_input::Style {
            background: dark::BG_BASE.into(),
            placeholder: dark::TEXT_MUTED,
            value: dark::TEXT_MUTED,
            border: Border {
                color: Color::TRANSPARENT,
                ..base.border
            },
            ..base
        },
    }
}

/// Raw text editor style
pub fn raw_text_editor(_theme: &Theme, _is_dark: bool) -> text_editor::Style {
    text_editor::Style {
        background: dark::BG_BASE.into(),
        border: Border {
            color: dark::BORDER_SUBTLE,
            width: 1.0,
            radius: RADIUS_SM.into(),
        },
        placeholder: dark::TEXT_MUTED,
        value: dark::TEXT_PRIMARY,
        selection: dark::ACCENT_MUTED,
    }
}
