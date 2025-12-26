//! Tool approval modal UI.

use iced::widget::{button, column, container, row, text};
use iced::{Color, Element, Length};

use crate::messages::{Message, chat};

use super::types::ToolApprovalPrompt;

/// Wraps the base element with an approval modal if one is pending.
pub(in crate::app) fn wrap_with_approval_modal<'a>(
    active_approval: Option<&'a ToolApprovalPrompt>,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(prompt) = active_approval else {
        return base;
    };

    let overlay = view_approval_modal(prompt);
    iced::widget::stack![base, overlay].into()
}

/// Renders the approval modal dialog.
fn view_approval_modal(prompt: &ToolApprovalPrompt) -> Element<'_, Message> {
    let content = container(
        column![
            text("Tool approval required").size(18),
            text(format!("Tool: {}", prompt.name)).size(14),
            text(prompt.args.clone()).size(12),
            row![
                button("Deny")
                    .on_press(Message::Chat(chat::Msg::ToolApprovalDecision {
                        id: prompt.id,
                        approved: false
                    }))
                    .style(crate::theme::styles::secondary_button)
                    .padding([6, 12]),
                button("Approve")
                    .on_press(Message::Chat(chat::Msg::ToolApprovalDecision {
                        id: prompt.id,
                        approved: true
                    }))
                    .style(crate::theme::styles::success_button)
                    .padding([6, 12]),
            ]
            .spacing(12),
        ]
        .spacing(10),
    )
    .padding(20)
    .style(crate::theme::styles::card_container);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.4).into()),
            ..Default::default()
        })
        .into()
}
