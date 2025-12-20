//! To Do list sidebar panel.

use iced::widget::{column, row, scrollable, text};
use iced::{Element, Length};

use crate::material_icons::{icon, icons};
use crate::messages::Message;
use crate::theme::AppTheme;

use ticca_core::tools::{TodoListState, TodoStatus};

pub fn contents<'a>(state: Option<&TodoListState>, _theme: AppTheme) -> Element<'a, Message> {
    let state = state.cloned().unwrap_or_default();

    let total = state.items.len();
    let completed = state
        .items
        .iter()
        .filter(|item| item.status == TodoStatus::Completed)
        .count();
    let in_progress = state
        .items
        .iter()
        .filter(|item| item.status == TodoStatus::InProgress)
        .count();

    let status_row = if state.is_completed_and_confirmed() {
        row![
            icon(icons::CHECK_CIRCLE).size(16),
            text("Confirmed complete").size(11),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
    } else {
        row![icon(icons::WARNING).size(16), text("Not confirmed").size(11),]
            .spacing(6)
            .align_y(iced::Alignment::Center)
    };

    let summary = text(format!(
        "{} items • {} completed{}",
        total,
        completed,
        if in_progress > 0 {
            format!(" • {} in progress", in_progress)
        } else {
            String::new()
        }
    ))
    .size(11);

    let list: Element<Message> = if state.items.is_empty() {
        column![
            text("No items yet.").size(12),
            text("The agent can populate this via the todo_list tool.").size(11)
        ]
        .spacing(6)
        .into()
    } else {
        let rows = state.items.iter().map(|item| {
            let status_icon = match item.status {
                TodoStatus::Pending => icons::RADIO_BUTTON_UNCHECKED,
                TodoStatus::InProgress => icons::PENDING,
                TodoStatus::Completed => icons::CHECK_CIRCLE,
            };

            row![
                icon(status_icon).size(16),
                text(item.text.clone()).size(12).width(Length::Fill),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .into()
        });

        scrollable(column(rows).spacing(8))
            .height(Length::Fill)
            .into()
    };

    column![status_row, summary, list]
        .spacing(8)
        .padding(12)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

