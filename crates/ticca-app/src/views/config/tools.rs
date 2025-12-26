//! Tools & Safety section view.

use std::collections::HashMap;

use iced::widget::{button, checkbox, column, container, pick_list, row, text, Column, Space};
use iced::{Border, Color, Element, Length};

use crate::material_icons::{icon, icons};
use crate::messages::{Message, settings};
use crate::theme::styles;

use ticca_core::config::{CompressionSettings, CompressionStrategy};
use ticca_core::external_tools::{
    ExternalToolId, FUSE_DOCS_URL, get_all_tool_definitions, is_fuse_available,
};

use super::types::horizontal_space;

/// Build the tools & safety section
pub(super) fn build_tools_section<'a>(
    yolo_mode_enabled: bool,
    external_tools: &'a HashMap<ExternalToolId, settings::ToolStatusInfo>,
    compression: &'a CompressionSettings,
) -> Element<'a, Message> {
    let status_label = if yolo_mode_enabled {
        "On (no prompts)"
    } else {
        "Off (ask first)"
    };

    let status_button = if yolo_mode_enabled {
        button(text(status_label).size(12))
            .on_press(Message::Settings(settings::Msg::SetYoloMode(
                !yolo_mode_enabled,
            )))
            .style(styles::success_button)
            .padding([6, 10])
    } else {
        button(text(status_label).size(12))
            .on_press(Message::Settings(settings::Msg::SetYoloMode(
                !yolo_mode_enabled,
            )))
            .style(styles::secondary_button)
            .padding([6, 10])
    };

    let yolo_section = container(
        column![
            text("Tools & Safety").size(18),
            row![
                icon(icons::SECURITY).size(16),
                text("Yolo Mode:").size(14).width(Length::Fixed(120.0)),
                status_button,
                text("Require approval for edit/delete/execute_shell when Off.")
                    .size(12)
                    .style(|_theme: &iced::Theme| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(120, 120, 120)),
                    }),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(12),
    )
    .padding(20)
    .style(styles::card_container);

    // Build compression settings section
    let compression_section = build_compression_section(compression);

    // Build external tools section
    let external_tools_section = build_external_tools_section(external_tools);

    column![yolo_section, compression_section, external_tools_section,]
        .spacing(16)
        .into()
}

/// Build the context compression settings section
fn build_compression_section(compression: &CompressionSettings) -> Element<'_, Message> {
    // Compression enabled toggle
    let enabled_row = row![
        icon(icons::COMPRESS).size(16),
        text("Context Compression:")
            .size(14)
            .width(Length::Fixed(140.0)),
        checkbox(compression.enabled)
            .on_toggle(|v| Message::Settings(settings::Msg::SetCompressionEnabled(v))),
        text(if compression.enabled {
            "Enabled"
        } else {
            "Disabled"
        })
        .size(12),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    // Threshold slider
    let threshold_row = row![
        text("Threshold:").size(14).width(Length::Fixed(140.0)),
        iced::widget::slider(50..=95, compression.threshold_percent, |v| {
            Message::Settings(settings::Msg::SetCompressionThreshold(v))
        })
        .width(Length::Fixed(200.0)),
        text(format!("{}%", compression.threshold_percent)).size(12),
        text("of context window")
            .size(11)
            .style(|_theme: &iced::Theme| iced::widget::text::Style {
                color: Some(Color::from_rgb8(120, 120, 120)),
            }),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    // Strategy picker
    let strategy_options = vec![
        CompressionStrategy::Truncation,
        CompressionStrategy::Summarizing,
    ];
    let strategy_row = row![
        text("Strategy:").size(14).width(Length::Fixed(140.0)),
        pick_list(strategy_options, Some(compression.strategy), |s| {
            Message::Settings(settings::Msg::SetCompressionStrategy(s))
        })
        .width(Length::Fixed(180.0)),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    // Strategy description
    let strategy_desc = match compression.strategy {
        CompressionStrategy::Truncation => "Keeps system prompt + recent messages (by token count)",
        CompressionStrategy::Summarizing => {
            "Generates a summary of removed context (uses LLM tokens)"
        }
    };
    let strategy_desc_row = row![
        Space::new().width(Length::Fixed(140.0)),
        text(strategy_desc)
            .size(11)
            .style(|_theme: &iced::Theme| iced::widget::text::Style {
                color: Some(Color::from_rgb8(120, 120, 120)),
            }),
    ]
    .spacing(10);

    // Preserve first messages
    let preserve_first_row = row![
        text("Preserve First:").size(14).width(Length::Fixed(140.0)),
        iced::widget::slider(0..=5, compression.preserve_first, |v| {
            Message::Settings(settings::Msg::SetCompressionPreserveFirst(v))
        })
        .width(Length::Fixed(120.0)),
        text(format!("{} messages", compression.preserve_first)).size(12),
        text("(system prompt)")
            .size(11)
            .style(|_theme: &iced::Theme| iced::widget::text::Style {
                color: Some(Color::from_rgb8(120, 120, 120)),
            }),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    // Protected tokens for recent messages
    // Range: 10k to 100k tokens, step by 5k
    let protected_tokens_k = compression.protected_tokens / 1000;
    let preserve_recent_row = row![
        text("Protected Tokens:")
            .size(14)
            .width(Length::Fixed(140.0)),
        iced::widget::slider(10..=100, protected_tokens_k, |v| {
            Message::Settings(settings::Msg::SetCompressionProtectedTokens(v * 1000))
        })
        .width(Length::Fixed(120.0)),
        text(format!("{}k tokens", protected_tokens_k)).size(12),
        text("(recent context budget)")
            .size(11)
            .style(|_theme: &iced::Theme| iced::widget::text::Style {
                color: Some(Color::from_rgb8(120, 120, 120)),
            }),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    container(
        column![
            text("Context Compression").size(18),
            text("Automatically compress context when approaching token limits.")
                .size(12)
                .style(|_theme: &iced::Theme| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(120, 120, 120)),
                }),
            enabled_row,
            threshold_row,
            strategy_row,
            strategy_desc_row,
            preserve_first_row,
            preserve_recent_row,
        ]
        .spacing(12),
    )
    .padding(20)
    .style(styles::card_container)
    .into()
}

/// Build the external tools management section
fn build_external_tools_section<'a>(
    external_tools: &'a HashMap<ExternalToolId, settings::ToolStatusInfo>,
) -> Element<'a, Message> {
    let header = row![
        text("External Tools").size(18),
        horizontal_space(),
        button(row![icon(icons::REFRESH).size(14), text(" Refresh").size(12),].spacing(4))
            .on_press(Message::Settings(settings::Msg::RefreshExternalTools))
            .style(styles::secondary_button)
            .padding([6, 10]),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    let description = text(
        "Download optional tools for advanced features like document conversion and JavaScript execution.",
    )
    .size(12)
    .style(|_theme: &iced::Theme| iced::widget::text::Style {
        color: Some(Color::from_rgb8(120, 120, 120)),
    });

    // Check if LibreOffice needs FUSE warning (installed but FUSE 2 missing)
    let libreoffice_installed = external_tools
        .get(&ExternalToolId::LibreOffice)
        .map(|s| s.is_installed)
        .unwrap_or(false);
    let needs_fuse_warning = libreoffice_installed && !is_fuse_available();

    // Build tool cards
    let tool_cards: Vec<Element<'a, Message>> =
        get_all_tool_definitions()
            .iter()
            .map(|def| {
                let tool_id = def.id;
                let status = external_tools.get(&tool_id);

                let status_text = if let Some(status) = status {
                    if status.is_installing {
                        format!("Installing... {}%", status.install_progress)
                    } else if status.is_installed {
                        format!("Installed v{}", status.version.as_deref().unwrap_or("?"))
                    } else if !status.is_supported {
                        "Unsupported Platform".to_string()
                    } else {
                        "Not Installed".to_string()
                    }
                } else {
                    "Unknown".to_string()
                };

                let is_installed = status.map(|s| s.is_installed).unwrap_or(false);
                let is_installing = status.map(|s| s.is_installing).unwrap_or(false);
                let is_supported = status.map(|s| s.is_supported).unwrap_or(true);

                let action_button: Element<'a, Message> = if is_installing {
                    text(format!(
                        "{}%",
                        status.map(|s| s.install_progress).unwrap_or(0)
                    ))
                    .size(12)
                    .into()
                } else if is_installed {
                    button(text("Uninstall").size(12))
                        .on_press(Message::Settings(settings::Msg::UninstallExternalTool(
                            tool_id,
                        )))
                        .style(styles::secondary_button)
                        .padding([4, 8])
                        .into()
                } else if is_supported {
                    button(text("Install").size(12))
                        .on_press(Message::Settings(settings::Msg::InstallExternalTool(
                            tool_id,
                        )))
                        .style(styles::primary_button)
                        .padding([4, 8])
                        .into()
                } else {
                    text("N/A").size(12).into()
                };

                let required_by = def.required_by.join(", ");

                container(
                    row![
                        column![
                            text(def.display_name).size(14),
                            text(def.description)
                                .size(11)
                                .style(|_theme: &iced::Theme| iced::widget::text::Style {
                                    color: Some(Color::from_rgb8(120, 120, 120)),
                                }),
                            text(format!(
                                "~{} MB • Required by: {}",
                                def.size_mb, required_by
                            ))
                            .size(10)
                            .style(|_theme: &iced::Theme| iced::widget::text::Style {
                                color: Some(Color::from_rgb8(100, 100, 100)),
                            }),
                        ]
                        .spacing(2)
                        .width(Length::Fill),
                        column![text(status_text).size(11), action_button,]
                            .spacing(4)
                            .align_x(iced::Alignment::End),
                    ]
                    .spacing(10)
                    .align_y(iced::Alignment::Center),
                )
                .padding(10)
                .width(Length::Fill)
                .into()
            })
            .collect();

    // FUSE warning banner (only shown when LibreOffice is installed but FUSE 2 is missing)
    let fuse_warning: Option<Element<'a, Message>> = if needs_fuse_warning {
        Some(
            container(
                column![
                    row![
                        icon(icons::WARNING).size(16),
                        text("LibreOffice requires FUSE 2").size(13),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                    text("AppImages need FUSE 2 (libfuse.so.2) to run. FUSE 3 is not compatible.")
                        .size(11)
                        .style(|_theme: &iced::Theme| iced::widget::text::Style {
                            color: Some(Color::from_rgb8(180, 140, 80)),
                        }),
                    text("Install: Arch: pacman -S fuse2 | Debian/Ubuntu: apt install libfuse2")
                        .size(10)
                        .style(|_theme: &iced::Theme| iced::widget::text::Style {
                            color: Some(Color::from_rgb8(150, 120, 70)),
                        }),
                    button(text("View installation guide →").size(10).style(
                        |_theme: &iced::Theme| iced::widget::text::Style {
                            color: Some(Color::from_rgb8(100, 160, 255)),
                        }
                    ))
                    .on_press(Message::Settings(settings::Msg::OpenUrl(
                        FUSE_DOCS_URL.to_string()
                    )))
                    .style(|_theme: &iced::Theme, _status| iced::widget::button::Style {
                        background: None,
                        text_color: Color::from_rgb8(100, 160, 255),
                        ..Default::default()
                    })
                    .padding(0),
                ]
                .spacing(4),
            )
            .padding(12)
            .width(Length::Fill)
            .style(|_theme: &iced::Theme| container::Style {
                background: Some(Color::from_rgb8(60, 50, 30).into()),
                border: Border {
                    color: Color::from_rgb8(180, 140, 50),
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            })
            .into(),
        )
    } else {
        None
    };

    let mut content = column![header, description,].spacing(12);

    if let Some(warning) = fuse_warning {
        content = content.push(warning);
    }

    content = content.push(Column::with_children(tool_cards).spacing(8));

    container(content)
        .padding(20)
        .style(styles::card_container)
        .into()
}
