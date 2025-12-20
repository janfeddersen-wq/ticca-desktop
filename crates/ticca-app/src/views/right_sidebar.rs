//! Right sidebar with tabbed panels.

use iced::widget::{Space, button, column, container, row, text};
use iced::{Element, Length, Padding};

use crate::agent_graph::AgentCallGraph;
use crate::material_icons::{icon, icons};
use crate::messages::{Message, RightSidebarTab, TodoNodeOption};
use crate::system_executions::SystemExecutionsState;
use crate::theme::{AppTheme, styles};
use crate::views::{agent_flow, system_executions, todo_list};

use iced::widget::pick_list;
use std::collections::HashMap;
use ticca_core::tools::TodoListState;

pub fn view<'a>(
    graph: &AgentCallGraph,
    todo_lists: &HashMap<usize, TodoListState>,
    system_exec: &'a SystemExecutionsState,
    selected_tab: RightSidebarTab,
    selected_todo_node: usize,
    theme: AppTheme,
) -> Element<'a, Message> {
    let tab_button = |tab: RightSidebarTab,
                      tab_icon: material_icons::Icon,
                      label: &'static str|
     -> Element<'a, Message> {
        let is_active = tab == selected_tab;
        button(row![icon(tab_icon).size(14), text(label).size(14)].spacing(6))
            .style(move |theme, status| styles::sidebar_tab_button(theme, status, is_active))
            .padding([8, 12])
            .on_press(Message::SelectSidebarTab(tab))
            .into()
    };

    let tabs = row![
        tab_button(RightSidebarTab::AgentsFlow, icons::FORUM, "Agents"),
        tab_button(RightSidebarTab::TodoList, icons::CHECKLIST, "To Do"),
        tab_button(RightSidebarTab::SystemExecutions, icons::TERMINAL, "System"),
        Space::new().width(Length::Fill),
    ]
    .spacing(0);

    let tabs_header = column![
        container(tabs)
            .padding(Padding {
                top: 10.0,
                right: 10.0,
                bottom: 9.0,
                left: 10.0,
            })
            .width(Length::Fill)
            .style(styles::header_container),
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fixed(1.0))
            .style(|theme: &iced::Theme| iced::widget::container::Style {
                background: Some(styles::card_container(theme).border.color.into()),
                ..Default::default()
            }),
    ]
    .spacing(0)
    .width(Length::Fill);

    let body: Element<Message> = match selected_tab {
        RightSidebarTab::AgentsFlow => agent_flow::contents(graph, theme),
        RightSidebarTab::TodoList => {
            let node_options: Vec<TodoNodeOption> = graph
                .order()
                .iter()
                .filter_map(|node_id| {
                    graph.nodes().get(node_id).map(|node| TodoNodeOption {
                        node_id: *node_id,
                        label: node.label.clone(),
                    })
                })
                .collect();

            let selected_option = node_options
                .iter()
                .find(|opt| opt.node_id == selected_todo_node)
                .cloned()
                .or_else(|| node_options.first().cloned());

            let selected_state = selected_option
                .as_ref()
                .and_then(|opt| todo_lists.get(&opt.node_id));

            let picker = pick_list(node_options, selected_option, Message::SelectTodoNode)
                .width(Length::Fill);

            column![
                container(
                    column![text("Agent").size(11), picker]
                        .spacing(6)
                        .width(Length::Fill)
                )
                .padding(12),
                todo_list::contents(selected_state, theme),
            ]
            .into()
        }
        RightSidebarTab::SystemExecutions => system_executions::contents(system_exec, theme),
    };

    container(column![tabs_header, body])
        .width(Length::Fixed(280.0))
        .height(Length::Fill)
        .style(styles::flow_panel_container)
        .into()
}
