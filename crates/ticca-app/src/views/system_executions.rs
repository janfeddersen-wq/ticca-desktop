//! System Executions sidebar panel.

use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Color, Element, Length};

use crate::material_icons::{icon, icons};
use crate::messages::{Message, chat};
use crate::system_executions::SystemExecutionsState;
use crate::theme::{AppTheme, styles};

pub fn contents<'a>(state: &'a SystemExecutionsState, theme: AppTheme) -> Element<'a, Message> {
    let _ = theme;

    let error_banner: Element<'a, Message> = if let Some(err) = &state.ui_error {
        container(
            text(err)
                .size(12)
                .style(|_theme: &iced::Theme| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(200, 60, 60)),
                }),
        )
        .padding([6, 8])
        .style(styles::card_container)
        .into()
    } else {
        Space::new().into()
    };

    let header = column![
        row![
            text_input("New terminal name", &state.new_terminal_name)
                .on_input(|value| Message::Chat(chat::Msg::SystemExecNewTerminalNameChanged(value)))
                .width(Length::Fill),
            button(
                row![icon(icons::ADD).size(14), text("New Terminal").size(12),]
                    .spacing(6)
                    .align_y(iced::Alignment::Center),
            )
            .on_press(Message::Chat(chat::Msg::SystemExecCreateUserTerminal))
            .style(styles::secondary_button)
            .padding([6, 10]),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
        error_banner
    ]
    .spacing(8);

    let mut items: Vec<Element<'a, Message>> = Vec::new();

    for (process_id, instance) in state.terminals.iter() {
        let title = row![
            text(process_id.clone()).size(12).width(Length::Fill),
            button(row![icon(icons::CONTENT_COPY).size(16)].spacing(6))
                .on_press(Message::Chat(chat::Msg::SystemExecCopyTerminal(
                    process_id.clone()
                )))
                .style(styles::secondary_button)
                .padding([4, 8]),
            button(row![icon(icons::CANCEL).size(16)].spacing(6))
                .on_press(Message::Chat(chat::Msg::SystemExecKillTerminal(
                    process_id.clone()
                )))
                .style(styles::danger_icon_button)
                .padding([4, 8]),
            button(row![icon(icons::CLOSE).size(16)].spacing(6))
                .on_press(Message::Chat(chat::Msg::SystemExecCloseTerminal(
                    process_id.clone()
                )))
                .style(styles::secondary_button)
                .padding([4, 8]),
        ]
        .spacing(8)
        .width(Length::Fill)
        .align_y(iced::Alignment::Center);

        let terminal = container(
            iced_term::TerminalView::show(&instance.terminal)
                .map(|event| Message::Chat(chat::Msg::SystemExecTerminalEvent(event))),
        )
        .width(Length::Fill)
        .height(Length::Fixed(220.0));

        let card = container(column![title, terminal].spacing(8).width(Length::Fill))
            .padding(10)
            .width(Length::Fill)
            .style(styles::card_container);

        items.push(card.into());
    }

    let body: Element<'a, Message> = if items.is_empty() {
        container(text("No active terminals.").size(14))
            .padding(12)
            .into()
    } else {
        scrollable(column(items).spacing(12).width(Length::Fill))
            .height(Length::Fill)
            .into()
    };

    container(column![header, body].spacing(12))
        .padding(12)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(styles::flow_panel_container)
        .into()
}
