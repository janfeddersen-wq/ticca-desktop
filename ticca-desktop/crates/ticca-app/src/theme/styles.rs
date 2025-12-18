//! Widget styling helpers for consistent theming
//!
//! Provides style functions for buttons, containers, inputs, and other widgets.

use iced::widget::{button, container, text_editor, text_input};
use iced::{Border, Color, Theme};

/// Button border radius constant
const BUTTON_RADIUS: f32 = 6.0;
/// Container border radius constant
const CONTAINER_RADIUS: f32 = 8.0;
/// Input border radius constant
const INPUT_RADIUS: f32 = 6.0;

/// Primary button style (main action buttons)
pub fn primary_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    let base = button::Style {
        background: Some(palette.primary.base.color.into()),
        text_color: palette.primary.base.text,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: BUTTON_RADIUS.into(),
        },
        ..button::Style::default()
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(palette.primary.weak.color.into()),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(palette.primary.strong.color.into()),
            ..base
        },
        button::Status::Disabled => button::Style {
            background: Some(palette.background.weak.color.into()),
            text_color: palette.background.weak.text,
            ..base
        },
    }
}

/// Secondary button style (less prominent actions)
pub fn secondary_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    let base = button::Style {
        background: Some(palette.background.weak.color.into()),
        text_color: palette.background.weak.text,
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: BUTTON_RADIUS.into(),
        },
        ..button::Style::default()
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(palette.background.strong.color.into()),
            border: Border {
                color: palette.primary.base.color,
                width: 1.0,
                radius: BUTTON_RADIUS.into(),
            },
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(palette.primary.weak.color.into()),
            text_color: palette.primary.weak.text,
            ..base
        },
        button::Status::Disabled => button::Style {
            background: Some(palette.background.base.color.into()),
            text_color: Color {
                a: 0.5,
                ..palette.background.base.text
            },
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: BUTTON_RADIUS.into(),
            },
            ..base
        },
    }
}

/// Icon button style (icon-only buttons like settings, theme toggle)
pub fn icon_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    let base = button::Style {
        background: Some(Color::TRANSPARENT.into()),
        text_color: palette.background.base.text,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: BUTTON_RADIUS.into(),
        },
        ..button::Style::default()
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(palette.background.weak.color.into()),
            text_color: palette.primary.base.color,
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(palette.background.strong.color.into()),
            text_color: palette.primary.strong.color,
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: Color {
                a: 0.4,
                ..palette.background.base.text
            },
            ..base
        },
    }
}

/// Tab button style (for agent switcher tabs)
pub fn tab_button(theme: &Theme, status: button::Status, is_active: bool) -> button::Style {
    let palette = theme.extended_palette();

    if is_active {
        // Active tab
        button::Style {
            background: Some(palette.primary.base.color.into()),
            text_color: palette.primary.base.text,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: BUTTON_RADIUS.into(),
            },
            ..button::Style::default()
        }
    } else {
        // Inactive tab
        let base = button::Style {
            background: Some(Color::TRANSPARENT.into()),
            text_color: palette.background.base.text,
            border: Border {
                color: palette.background.strong.color,
                width: 1.0,
                radius: BUTTON_RADIUS.into(),
            },
            ..button::Style::default()
        };

        match status {
            button::Status::Active => base,
            button::Status::Hovered => button::Style {
                background: Some(palette.background.weak.color.into()),
                border: Border {
                    color: palette.primary.base.color,
                    width: 1.0,
                    radius: BUTTON_RADIUS.into(),
                },
                ..base
            },
            button::Status::Pressed => button::Style {
                background: Some(palette.primary.weak.color.into()),
                text_color: palette.primary.weak.text,
                ..base
            },
            button::Status::Disabled => base,
        }
    }
}

/// Header container style
pub fn header_container(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.weak.color.into()),
        text_color: Some(palette.background.weak.text),
        border: Border {
            color: palette.background.strong.color,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Message bubble container style
pub fn message_bubble(theme: &Theme, is_user: bool) -> container::Style {
    let palette = theme.extended_palette();

    if is_user {
        // User message - primary colored
        container::Style {
            background: Some(palette.primary.base.color.into()),
            text_color: Some(palette.primary.base.text),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: CONTAINER_RADIUS.into(),
            },
            ..container::Style::default()
        }
    } else {
        // Assistant/system message - surface colored
        container::Style {
            background: Some(palette.background.weak.color.into()),
            text_color: Some(palette.background.weak.text),
            border: Border {
                color: palette.background.strong.color,
                width: 1.0,
                radius: CONTAINER_RADIUS.into(),
            },
            ..container::Style::default()
        }
    }
}

/// Card container style (for settings sections, etc.)
pub fn card_container(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.weak.color.into()),
        text_color: Some(palette.background.weak.text),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: CONTAINER_RADIUS.into(),
        },
        ..container::Style::default()
    }
}

/// Text input style
pub fn text_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = theme.extended_palette();

    let base = text_input::Style {
        background: palette.background.weak.color.into(),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: INPUT_RADIUS.into(),
        },
        icon: palette.background.base.text,
        placeholder: Color {
            a: 0.5,
            ..palette.background.base.text
        },
        value: palette.background.base.text,
        selection: palette.primary.weak.color,
    };

    match status {
        text_input::Status::Active => base,
        text_input::Status::Hovered => text_input::Style {
            border: Border {
                color: palette.primary.weak.color,
                width: 1.0,
                radius: INPUT_RADIUS.into(),
            },
            ..base
        },
        text_input::Status::Focused => text_input::Style {
            border: Border {
                color: palette.primary.base.color,
                width: 2.0,
                radius: INPUT_RADIUS.into(),
            },
            ..base
        },
        text_input::Status::Disabled => text_input::Style {
            background: palette.background.base.color.into(),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: INPUT_RADIUS.into(),
            },
            placeholder: Color {
                a: 0.3,
                ..palette.background.base.text
            },
            value: Color {
                a: 0.5,
                ..palette.background.base.text
            },
            ..base
        },
    }
}

/// Input area container (wraps text input and send button)
pub fn input_area_container(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.weak.color.into()),
        text_color: Some(palette.background.weak.text),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Code block container style (for markdown code blocks)
pub fn code_block(theme: &Theme, is_dark: bool) -> container::Style {
    let palette = theme.extended_palette();

    // Use a slightly different background for code blocks
    let bg_color = if is_dark {
        Color::from_rgb(0.12, 0.14, 0.16) // Darker background for dark theme
    } else {
        Color::from_rgb(0.95, 0.96, 0.97) // Light gray for light theme
    };

    container::Style {
        background: Some(bg_color.into()),
        text_color: Some(palette.background.base.text),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..container::Style::default()
    }
}

/// Table header row style
pub fn table_header(theme: &Theme, is_dark: bool) -> container::Style {
    let palette = theme.extended_palette();

    let bg_color = if is_dark {
        Color::from_rgb(0.18, 0.20, 0.24)
    } else {
        Color::from_rgb(0.92, 0.93, 0.95)
    };

    container::Style {
        background: Some(bg_color.into()),
        text_color: Some(palette.background.base.text),
        border: Border {
            color: palette.background.strong.color,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Table body row style
pub fn table_row(theme: &Theme, is_dark: bool, is_alternate: bool) -> container::Style {
    let palette = theme.extended_palette();

    let bg_color = if is_dark {
        if is_alternate {
            Color::from_rgb(0.14, 0.16, 0.18)
        } else {
            Color::from_rgb(0.10, 0.12, 0.14)
        }
    } else if is_alternate {
        Color::from_rgb(0.96, 0.97, 0.98)
    } else {
        Color::from_rgb(0.99, 0.99, 1.0)
    };

    container::Style {
        background: Some(bg_color.into()),
        text_color: Some(palette.background.base.text),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Directory bar container style (working directory selector)
pub fn dir_bar_container(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.base.color.into()),
        text_color: Some(Color {
            a: 0.7,
            ..palette.background.base.text
        }),
        border: Border {
            color: palette.background.strong.color,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Reasoning/thinking container style (collapsible thinking section)
pub fn reasoning_container(theme: &Theme, is_dark: bool) -> container::Style {
    let palette = theme.extended_palette();

    let bg_color = if is_dark {
        Color::from_rgba(0.3, 0.3, 0.4, 0.3)
    } else {
        Color::from_rgba(0.9, 0.9, 0.95, 0.8)
    };

    let border_color = if is_dark {
        Color::from_rgba(0.5, 0.5, 0.6, 0.4)
    } else {
        Color::from_rgba(0.7, 0.7, 0.8, 0.5)
    };

    container::Style {
        background: Some(bg_color.into()),
        text_color: Some(Color {
            a: 0.8,
            ..palette.background.base.text
        }),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    }
}

/// Raw text editor style (for selectable plain text view)
pub fn raw_text_editor(theme: &Theme, is_dark: bool) -> text_editor::Style {
    let palette = theme.extended_palette();

    let bg_color = if is_dark {
        Color::from_rgb(0.10, 0.12, 0.14)
    } else {
        Color::from_rgb(0.96, 0.97, 0.98)
    };

    let text_color = if is_dark {
        Color::from_rgb(0.85, 0.85, 0.85)
    } else {
        Color::from_rgb(0.15, 0.15, 0.15)
    };

    let selection_color = if is_dark {
        Color::from_rgba(0.3, 0.5, 0.8, 0.5)
    } else {
        Color::from_rgba(0.2, 0.5, 0.8, 0.4)
    };

    text_editor::Style {
        background: bg_color.into(),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 4.0.into(),
        },
        icon: palette.background.weak.text,
        placeholder: palette.background.weak.text,
        value: text_color,
        selection: selection_color,
    }
}
