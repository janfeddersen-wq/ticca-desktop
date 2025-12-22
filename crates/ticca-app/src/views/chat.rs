//! Chat view component
//!
//! Renders the main chat interface including header, messages, and input area.

use iced::widget::{
    Column, Space, button, column, container, markdown, row, scrollable, text, text_editor,
    text_input,
};
use iced::{Element, Length, widget};

use std::collections::{HashMap, HashSet};
use std::path::Path;

use ticca_core::agents::AgentType;
use ticca_core::session::MessageRole;

use crate::chat_message::ChatMessage;
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

/// Get the icon for an agent type
fn agent_icon(agent_type: AgentType) -> icons::Icon {
    match agent_type {
        AgentType::Coding => icons::CODE,
        AgentType::Planning => icons::CHECKLIST,
        AgentType::Skills => icons::BUILD,
    }
}

/// Get a short display label for an agent type
fn agent_label(agent_type: AgentType) -> &'static str {
    match agent_type {
        AgentType::Coding => "Coding",
        AgentType::Planning => "Planning",
        AgentType::Skills => "Skills",
    }
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
    raw_view_messages: &'a HashSet<usize>,
    raw_view_editors: &'a HashMap<usize, text_editor::Content>,
    stream_chars: usize,
    current_tps: f64,
    stream_pulse: bool,
    secs_since_bytes: u64,
    spinner_frame: usize,
    flow_panel_visible: bool,
) -> Element<'a, Message> {
    let agent_selector: Element<Message> = if expert_mode_enabled {
        let buttons: Vec<Element<Message>> = AgentType::all()
            .iter()
            .map(|&agent_type| {
                let is_selected = current_agent == agent_type;
                let agent_icon = agent_icon(agent_type);
                let label = agent_label(agent_type);
                button(
                    row![icon(agent_icon).size(14), text(format!(" {}", label)).size(14),]
                        .spacing(4),
                )
                .on_press(Message::Chat(chat::Msg::SwitchAgent(agent_type)))
                .style(move |theme, status| styles::tab_button(theme, status, is_selected))
                .padding([8, 12])
                .into()
            })
            .collect();

        iced::widget::Row::with_children(buttons)
            .spacing(8)
            .into()
    } else {
        Space::new().width(Length::Fixed(0.0)).into()
    };

    // Header with agent switcher and settings
    let header = container(
        row![
            agent_selector,
            // Spacer
            horizontal_space(),
            // Actions
            row![
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
                button(icon(icons::CONTRAST).size(18))
                    .on_press(Message::Settings(settings::Msg::ThemeToggle))
                    .style(styles::icon_button)
                    .padding(8),
                button(icon(icons::SETTINGS).size(18))
                    .on_press(Message::Settings(settings::Msg::OpenSettings))
                    .style(styles::icon_button)
                    .padding(8),
                button(icon(icons::ADD).size(18))
                    .on_press(Message::Chat(chat::Msg::NewSession))
                    .style(styles::icon_button)
                    .padding(8),
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
            icon(icons::FOLDER).size(16),
            text(format!(" {}", dir_display)).size(12),
            horizontal_space(),
            button(text("Change").size(12))
                .on_press(Message::Chat(chat::Msg::SelectWorkingDirectory))
                .style(styles::secondary_button)
                .padding([4, 8]),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .padding([6, 12])
    .style(styles::dir_bar_container);

    // Message list
    let message_widgets: Vec<Element<Message>> = messages
        .iter()
        .enumerate()
        .map(|(idx, msg)| render_message(idx, msg, theme, raw_view_messages, raw_view_editors))
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
    let mut input_row = row![
        // Add image button (works on Wayland via xdg-portal)
        button(icon(icons::ATTACH_FILE).size(20))
            .on_press(Message::Chat(chat::Msg::SelectImageFile))
            .style(styles::icon_button)
            .padding([8, 8]),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    // Add streaming indicator if active
    if let Some(indicator) = streaming_indicator {
        input_row = input_row.push(indicator);
    }

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
        .push(
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
        );

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
    index: usize,
    msg: &'a ChatMessage,
    theme: AppTheme,
    raw_view_messages: &'a HashSet<usize>,
    raw_view_editors: &'a HashMap<usize, text_editor::Content>,
) -> Element<'a, Message> {
    let is_user = msg.role == MessageRole::User;
    let is_dark = theme.is_dark();
    let is_raw_view = raw_view_messages.contains(&index);

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
        if let Some(editor_content) = raw_view_editors.get(&index) {
            text_editor(editor_content)
                .on_action(move |action| {
                    Message::Chat(chat::Msg::RawViewEditorAction(index, action))
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

    // Header row with label, toggle button, and copy button
    let header = row![
        text(label).size(12),
        horizontal_space(),
        // Raw/Markdown toggle button
        button(icon(toggle_icon).size(16))
            .on_press(Message::Chat(chat::Msg::ToggleRawView(index)))
            .style(styles::icon_button)
            .padding([4, 6]),
        // Copy button
        button(icon(icons::CONTENT_COPY).size(16))
            .on_press(Message::Chat(chat::Msg::CopyMessage(index)))
            .style(styles::icon_button)
            .padding([4, 6]),
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
        .style(move |theme| styles::message_bubble(theme, is_user))
        .into()
}
