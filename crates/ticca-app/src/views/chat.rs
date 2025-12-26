//! Chat view component
//!
//! Renders the main chat interface including header, messages, and input area.

use iced::widget::{
    Column, Space, button, column, container, markdown, row, scrollable, text, text_editor,
    text_input, tooltip,
};
use iced::{Element, Length, widget};

use std::collections::{HashMap, HashSet};
use std::path::Path;

use ticca_core::agents::{AgentRegistry, AgentType};
use ticca_core::session::MessageRole;

use crate::chat_message::{ChatMessage, MessageId};
use crate::material_icons::{icon, icons};
use crate::messages::{ImageAttachment, Message, chat, settings};
use crate::theme::{AppTheme, styles};
use crate::widgets::spinner;

/// ID for the chat messages scrollable
pub const CHAT_SCROLLABLE_ID: &str = "chat_messages";

/// Create horizontal space that fills available width
fn horizontal_space() -> Space {
    Space::new().width(Length::Fill)
}

/// Create a styled tooltip with consistent appearance
fn styled_tooltip<'a>(
    content: impl Into<Element<'a, Message>>,
    tip: &'a str,
    position: tooltip::Position,
) -> tooltip::Tooltip<'a, Message, iced::Theme, iced::Renderer> {
    tooltip(content, text(tip).size(12), position)
        .padding(8)
        .gap(4)
        .style(styles::tooltip_style)
}

/// Format token count for display with thousands separator (e.g., 45000 -> "45 000")
fn format_tokens(tokens: i64) -> String {
    let s = tokens.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(' ');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// Render the chat view
#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    expert_mode_enabled: bool,
    current_agent: AgentType,
    working_directory: &Path,
    messages: &'a [ChatMessage],
    pending_attachments: &'a [ImageAttachment],
    input_value: &str,
    is_streaming: bool,
    theme: AppTheme,
    raw_view_messages: &'a HashSet<MessageId>,
    raw_view_editors: &'a HashMap<MessageId, text_editor::Content>,
    stream_chars: usize,
    current_tps: f64,
    stream_pulse: bool,
    secs_since_bytes: u64,
    spinner_frame: usize,
    flow_panel_visible: bool,
    tokens_used: i64,
    context_limit: i64,
    supports_vision: bool,
) -> Element<'a, Message> {
    let agent_selector: Element<Message> = if expert_mode_enabled {
        let buttons: Vec<Element<Message>> = AgentType::all()
            .iter()
            .map(|&agent_type| {
                let is_selected = current_agent == agent_type;
                let metadata = AgentRegistry::get(agent_type);
                let agent_icon = metadata.icon;
                let label = metadata.label;
                let description = metadata.description;
                styled_tooltip(
                    button(
                        row![
                            icon(agent_icon).size(14),
                            text(format!(" {}", label)).size(14),
                        ]
                        .spacing(4),
                    )
                    .on_press(Message::Chat(chat::Msg::SwitchAgent(agent_type)))
                    .style(move |theme, status| styles::tab_button(theme, status, is_selected))
                    .padding([8, 12]),
                    description,
                    tooltip::Position::Bottom,
                )
                .into()
            })
            .collect();

        iced::widget::Row::with_children(buttons).spacing(8).into()
    } else {
        Space::new().width(Length::Fixed(0.0)).into()
    };

    // Token usage indicator
    let usage_percentage = if context_limit > 0 {
        (tokens_used as f64 / context_limit as f64).min(1.0)
    } else {
        0.0
    };
    let usage_text = format!(
        "{} / {}",
        format_tokens(tokens_used),
        format_tokens(context_limit)
    );

    // Progress bar dimensions
    let bar_width = 80.0;
    let bar_height = 6.0;
    let filled_width = (bar_width * usage_percentage) as f32;

    // Color based on usage percentage
    let bar_color = if usage_percentage > 0.9 {
        iced::Color::from_rgb(0.9, 0.2, 0.2) // Red when > 90%
    } else if usage_percentage > 0.7 {
        iced::Color::from_rgb(0.9, 0.7, 0.2) // Orange when > 70%
    } else {
        iced::Color::from_rgb(0.3, 0.7, 0.4) // Green otherwise
    };

    let token_usage_display = container(
        row![
            text(usage_text).size(11),
            // Progress bar using nested containers
            container(
                container(
                    Space::new()
                        .width(Length::Fixed(filled_width))
                        .height(Length::Fixed(bar_height as f32))
                )
                .style(move |_theme: &iced::Theme| container::Style {
                    background: Some(bar_color.into()),
                    border: iced::Border {
                        radius: 3.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
            )
            .width(Length::Fixed(bar_width as f32))
            .height(Length::Fixed(bar_height as f32))
            .style(|theme: &iced::Theme| {
                let palette = theme.extended_palette();
                container::Style {
                    background: Some(palette.background.weak.color.into()),
                    border: iced::Border {
                        radius: 3.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .padding([4, 8]);

    // Header with agent switcher and settings
    let header = container(
        row![
            agent_selector,
            // Spacer
            horizontal_space(),
            // Token usage indicator
            token_usage_display,
            // Actions
            row![
                styled_tooltip(
                    button(icon(icons::DELETE).size(18))
                        .on_press(Message::Chat(chat::Msg::NewSession))
                        .style(styles::icon_button)
                        .padding(8),
                    "New session",
                    tooltip::Position::Bottom,
                ),
                styled_tooltip(
                    button(icon(icons::SETTINGS).size(18))
                        .on_press(Message::Settings(settings::Msg::OpenSettings))
                        .style(styles::icon_button)
                        .padding(8),
                    "Settings",
                    tooltip::Position::Bottom,
                ),
                styled_tooltip(
                    button(
                        icon(if flow_panel_visible {
                            icons::CLOSE
                        } else {
                            icons::MENU
                        })
                        .size(18)
                    )
                    .on_press(Message::Chat(chat::Msg::ToggleFlowPanel))
                    .style(styles::icon_button)
                    .padding(8),
                    if flow_panel_visible {
                        "Hide sidebar"
                    } else {
                        "Show sidebar"
                    },
                    tooltip::Position::Bottom,
                ),
            ]
            .spacing(4),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
    )
    .padding(10)
    .style(styles::header_container);

    // Working directory selector bar
    let dir_display = working_directory.to_string_lossy();
    let dir_bar = container(
        row![
            styled_tooltip(
                button(
                    row![
                        icon(icons::FOLDER_OPEN).size(14),
                        text(" Select Workdir").size(12)
                    ]
                    .spacing(2),
                )
                .on_press(Message::Chat(chat::Msg::SelectWorkingDirectory))
                .style(styles::secondary_button)
                .padding([4, 8]),
                "Change working directory",
                tooltip::Position::Bottom,
            ),
            icon(icons::FOLDER).size(16),
            text(format!(" {}", dir_display)).size(12),
            horizontal_space(),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .padding([6, 12])
    .style(styles::dir_bar_container);

    // Message list
    let message_widgets: Vec<Element<Message>> = messages
        .iter()
        .map(|msg| render_message(msg, theme, raw_view_messages, raw_view_editors))
        .collect();

    let messages_view: Element<Message> = scrollable(
        Column::with_children(message_widgets)
            .spacing(12)
            .padding(20),
    )
    .id(widget::Id::new(CHAT_SCROLLABLE_ID))
    .on_scroll(|viewport| Message::Chat(chat::Msg::ChatScrolled(viewport)))
    .height(Length::Fill)
    .into();

    // Build attachment previews if any
    let attachment_preview = build_attachment_preview(pending_attachments);

    // Check if we can send (has text or attachments)
    let can_send =
        !is_streaming && (!input_value.trim().is_empty() || !pending_attachments.is_empty());

    // Streaming indicator - shows LLM output rate or waiting animation
    let streaming_indicator: Option<Element<'_, Message>> = if is_streaming {
        // Determine if we're "waiting" (no bytes for > 2 seconds)
        let is_waiting = secs_since_bytes >= 2;

        if is_waiting {
            // Show GPU-rendered animated spinner with elapsed time
            Some(
                container(
                    row![
                        spinner(spinner_frame),
                        text(format!("{}s", secs_since_bytes)).size(12),
                    ]
                    .spacing(6)
                    .align_y(iced::Alignment::Center),
                )
                .padding([4, 8])
                .style(styles::streaming_indicator_container)
                .into(),
            )
        } else {
            // Show chars/s rate when data is flowing
            let pulse_icon = if stream_pulse {
                icons::RADIO_BUTTON_CHECKED
            } else {
                icons::RADIO_BUTTON_UNCHECKED
            };

            // Show chars/s rate (current_tps is now chars per second, not tokens)
            let display_text = if current_tps > 0.0 {
                format!("{:.0} #/s", current_tps)
            } else if stream_chars > 0 {
                // Have chars but no rate yet
                format!("~{} #", stream_chars)
            } else {
                "...".to_string()
            };

            Some(
                container(
                    row![icon(pulse_icon).size(12), text(display_text).size(12),]
                        .spacing(6)
                        .align_y(iced::Alignment::Center),
                )
                .padding([4, 8])
                .style(styles::streaming_indicator_container)
                .into(),
            )
        }
    } else {
        None
    };

    // Input area
    let image_button_tooltip = if supports_vision {
        "Attach image"
    } else {
        "Current model doesn't support images"
    };
    let mut input_row = row![
        // Add image button (works on Wayland via xdg-portal)
        // Disabled when current model doesn't support vision
        styled_tooltip(
            button(icon(icons::ATTACH_FILE).size(20))
                .on_press_maybe(if supports_vision {
                    Some(Message::Chat(chat::Msg::SelectImageFile))
                } else {
                    None
                })
                .style(styles::icon_button)
                .padding([8, 8]),
            image_button_tooltip,
            tooltip::Position::Top,
        ),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    // Add streaming indicator if active
    if let Some(indicator) = streaming_indicator {
        input_row = input_row.push(indicator);
    }

    // Send/Stop button with tooltip
    let send_button_tooltip = if is_streaming {
        "Stop generation"
    } else {
        "Send message"
    };
    let send_button = styled_tooltip(
        button(if is_streaming {
            icon(icons::CANCEL).size(20)
        } else {
            icon(icons::ARROW_UPWARD).size(20)
        })
        .on_press_maybe(if is_streaming {
            Some(Message::Chat(chat::Msg::StopStreaming))
        } else if can_send {
            Some(Message::Chat(chat::Msg::SendMessage))
        } else {
            None
        })
        .style(styles::send_button)
        .padding([8, 8]),
        send_button_tooltip,
        tooltip::Position::Top,
    );

    // Add the text input and send button
    input_row = input_row
        .push(
            text_input("Type a message...", input_value)
                .on_input(|value| Message::Chat(chat::Msg::InputChanged(value)))
                .on_submit(if is_streaming {
                    Message::Chat(chat::Msg::StopStreaming)
                } else {
                    Message::Chat(chat::Msg::SendMessage)
                })
                .style(styles::text_input_style)
                .padding(12)
                .size(14)
                .width(Length::Fill),
        )
        .push(send_button);

    let input_content: Element<'_, Message> = if let Some(preview) = attachment_preview {
        column![preview, input_row].spacing(0).into()
    } else {
        input_row.into()
    };
    let input = container(input_content)
        .padding(12)
        .style(styles::input_area_container);

    let chat_column: Element<'_, Message> =
        container(column![header, dir_bar, messages_view, input,])
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    chat_column
}

/// Build the attachment preview bar
fn build_attachment_preview(
    pending_attachments: &[ImageAttachment],
) -> Option<Element<'_, Message>> {
    if pending_attachments.is_empty() {
        return None;
    }

    let previews: Vec<Element<Message>> = pending_attachments
        .iter()
        .enumerate()
        .map(|(idx, attachment)| {
            // Create thumbnail from PNG data
            let handle = iced::widget::image::Handle::from_bytes((*attachment.data).clone());
            let thumbnail = iced::widget::image(handle)
                .width(Length::Fixed(60.0))
                .height(Length::Fixed(60.0));

            let size_kb = attachment.data.len() / 1024;

            container(
                column![
                    // Thumbnail with remove button overlay
                    iced::widget::stack![
                        container(thumbnail).style(styles::image_thumbnail_container),
                        container(
                            button(icon(icons::CLOSE).size(12))
                                .on_press(Message::Chat(chat::Msg::RemoveAttachment(idx)))
                                .style(styles::remove_attachment_button)
                                .padding(2)
                        )
                        .align_x(iced::alignment::Horizontal::Right)
                        .width(Length::Fill),
                    ]
                    .width(Length::Fixed(60.0))
                    .height(Length::Fixed(60.0)),
                    // Filename and size
                    text(format!("{}KB", size_kb)).size(10),
                ]
                .spacing(2)
                .align_x(iced::Alignment::Center),
            )
            .padding(4)
            .into()
        })
        .collect();

    Some(
        container(
            row![
                icon(icons::ATTACH_FILE).size(14),
                iced::widget::Row::with_children(previews).spacing(8),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .padding([8, 12])
        .width(Length::Fill)
        .style(styles::attachment_bar_container)
        .into(),
    )
}

/// Render a single message
fn render_message<'a>(
    msg: &'a ChatMessage,
    theme: AppTheme,
    raw_view_messages: &'a HashSet<MessageId>,
    raw_view_editors: &'a HashMap<MessageId, text_editor::Content>,
) -> Element<'a, Message> {
    let msg_id = msg.id;
    let is_user = msg.role == MessageRole::User;
    let is_system = msg.role == MessageRole::System;
    let is_dark = theme.is_dark();
    let is_raw_view = raw_view_messages.contains(&msg_id);

    let default_label = match msg.role {
        MessageRole::User => "You",
        MessageRole::Assistant => "Assistant",
        MessageRole::System => "System",
        MessageRole::Tool => "Tool",
    };
    let label = msg.author_label.as_deref().unwrap_or(default_label);

    let content: Element<Message> = if msg.is_streaming && msg.content.is_empty() {
        row![icon(icons::PENDING).size(16), text(" Thinking...").size(14),]
            .spacing(6)
            .into()
    } else if msg.is_streaming {
        // Render markdown while streaming
        markdown::view(
            &msg.parsed_items,
            markdown::Settings::with_text_size(14, theme.to_iced_theme()),
        )
        .map(|uri| Message::Chat(chat::Msg::LinkClicked(uri)))
    } else if is_raw_view {
        // Raw view: show selectable plain text
        if let Some(editor_content) = raw_view_editors.get(&msg_id) {
            text_editor(editor_content)
                .on_action(move |action| {
                    Message::Chat(chat::Msg::RawViewEditorAction(msg_id, action))
                })
                .style(move |theme, _status| styles::raw_text_editor(theme, is_dark))
                .into()
        } else {
            // Fallback if editor not yet created
            text(&msg.content).size(14).into()
        }
    } else {
        // Render markdown for completed messages
        markdown::view(
            &msg.parsed_items,
            markdown::Settings::with_text_size(14, theme.to_iced_theme()),
        )
        .map(|uri| Message::Chat(chat::Msg::LinkClicked(uri)))
    };

    // Toggle icon: CODE for raw view, DESCRIPTION for markdown view
    let toggle_icon = if is_raw_view {
        icons::DESCRIPTION
    } else {
        icons::CODE
    };
    let toggle_tooltip = if is_raw_view {
        "Show formatted"
    } else {
        "Show raw markdown"
    };

    // Header row with label, toggle button, and copy button
    let header = row![
        text(label).size(12),
        horizontal_space(),
        // Raw/Markdown toggle button
        styled_tooltip(
            button(icon(toggle_icon).size(16))
                .on_press(Message::Chat(chat::Msg::ToggleRawView(msg_id)))
                .style(styles::icon_button)
                .padding([4, 6]),
            toggle_tooltip,
            tooltip::Position::Bottom,
        ),
        // Copy button
        styled_tooltip(
            button(icon(icons::CONTENT_COPY).size(16))
                .on_press(Message::Chat(chat::Msg::CopyMessage(msg_id)))
                .style(styles::icon_button)
                .padding([4, 6]),
            "Copy message",
            tooltip::Position::Bottom,
        ),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center);

    // Build the message column
    let mut msg_column = column![header].spacing(6);

    // Add reasoning section if present
    if let Some(ref reasoning) = msg.reasoning {
        let reasoning_content = container(
            column![
                row![icon(icons::PSYCHOLOGY).size(14), text(" Thinking").size(12),].spacing(4),
                container(text(reasoning).size(12)).padding([4, 8])
            ]
            .spacing(4),
        )
        .padding(8)
        .width(Length::Fill)
        .style(move |theme| styles::reasoning_container(theme, is_dark));

        msg_column = msg_column.push(reasoning_content);
    }

    msg_column = msg_column.push(content);

    container(msg_column)
        .padding(12)
        .width(Length::FillPortion(4))
        .style(move |theme| {
            if is_system {
                styles::system_message_bubble(theme)
            } else {
                styles::message_bubble(theme, is_user)
            }
        })
        .into()
}
