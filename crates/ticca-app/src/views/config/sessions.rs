//! Sessions section view.

use iced::widget::{Column, button, column, container, row, text};
use iced::{Element, Length};

use crate::material_icons::{icon, icons};
use crate::messages::{Message, chat};
use crate::theme::styles;

use ticca_core::session::Session;

/// Build the recent sessions section
pub(super) fn build_sessions_section<'a>(sessions: &'a [Session]) -> Element<'a, Message> {
    let session_list: Vec<Element<'a, Message>> = if sessions.is_empty() {
        vec![text("No saved sessions yet.").size(14).into()]
    } else {
        sessions
            .iter()
            .take(10) // Show last 10 sessions
            .map(|session| {
                let session_id = session.id.clone();
                let session_name = session.name.clone();
                let agent_icon = if session.agent_type == "coding" {
                    icons::CODE
                } else {
                    icons::CHECKLIST
                };

                let updated = session
                    .updated_at
                    .as_ref()
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.format("%m/%d %H:%M").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                let info_text = format!("{} messages • {}", session.message_count, updated);

                container(
                    row![
                        icon(agent_icon).size(16),
                        column![text(session_name).size(14), text(info_text).size(11),]
                            .spacing(2)
                            .width(Length::Fill),
                        button(
                            row![icon(icons::FOLDER_OPEN).size(14), text(" Load").size(12),]
                                .spacing(4)
                        )
                        .on_press(Message::Chat(chat::Msg::LoadSession(session_id)))
                        .style(styles::secondary_button)
                        .padding([6, 10]),
                    ]
                    .spacing(10)
                    .align_y(iced::Alignment::Center),
                )
                .padding(8)
                .width(Length::Fill)
                .into()
            })
            .collect()
    };

    container(
        column![
            text("Recent Sessions").size(18),
            Column::with_children(session_list).spacing(4),
        ]
        .spacing(15),
    )
    .padding(20)
    .style(styles::card_container)
    .into()
}
