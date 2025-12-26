//! Chat view rendering.

use iced::Element;

use crate::app::TiccaApp;
use crate::messages::{Message, chat};

use super::types::ChatPane;

/// Renders the main chat view with pane grid.
pub(in crate::app) fn view(app: &TiccaApp) -> Element<'_, Message> {
    let secs_since_bytes = app
        .chat
        .last_bytes_time
        .map(|t| t.elapsed().as_secs())
        .unwrap_or(0);

    iced::widget::pane_grid(&app.chat.panes, |_pane, pane_state, _| {
        let content = match pane_state {
            ChatPane::Chat => crate::views::chat::view(
                app.ui_mode.shows_expert_ui(),
                app.chat.current_agent,
                &app.chat.working_directory,
                &app.chat.messages,
                &app.chat.pending_attachments,
                &app.chat.input_value,
                app.chat.is_streaming,
                app.theme,
                &app.chat.raw_view_messages,
                &app.chat.raw_view_editors,
                app.chat.stream_chars_received,
                app.chat.current_tps,
                app.chat.stream_pulse,
                secs_since_bytes,
                app.chat.spinner_frame,
                app.chat.flow_pane.is_some(),
                app.chat.tokens_used(),
                app.chat.context_limit(),
                app.chat.supports_vision(),
            ),
            ChatPane::Flow => crate::views::right_sidebar::view(
                &app.chat.call_graph,
                &app.chat.todo_lists,
                &app.chat.system_exec,
                app.chat.sidebar_tab,
                app.chat.todo_selected_node,
                app.theme,
                app.ui_mode.shows_expert_ui(),
                app.chat.flow_animation_frame,
            ),
        };
        iced::widget::pane_grid::Content::new(content)
    })
    .on_resize(10, |event| Message::Chat(chat::Msg::PaneResized(event)))
    .into()
}
