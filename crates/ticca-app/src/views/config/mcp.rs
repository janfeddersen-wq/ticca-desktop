//! MCP servers section view.

use std::collections::HashMap;

use iced::widget::{
    Column, Space, button, checkbox, column, container, pick_list, row, text, text_editor,
    text_input,
};
use iced::{Element, Length};

use crate::material_icons::{icon, icons};
use crate::messages::{Message, settings};
use crate::theme::{AppTheme, styles};

use ticca_core::agents::{AgentRegistry, AgentType};
use ticca_core::config::{McpServer, McpTransport};

use super::types::{McpServerFormState, horizontal_space};

/// Build the MCP servers section
pub(super) fn build_mcp_servers_section<'a>(
    servers: &'a [McpServer],
    form: &'a McpServerFormState,
    import_json: &'a text_editor::Content,
    theme: AppTheme,
) -> Element<'a, Message> {
    let is_dark = theme.is_dark();
    let transport_options = vec![McpTransport::Stdio, McpTransport::StreamableHttp];

    let is_editing = form.editing_id.is_some();
    let form_title = if is_editing {
        "Edit MCP Server"
    } else {
        "Add MCP Server"
    };

    let transport_row = row![
        text("Transport:").size(14).width(Length::Fixed(120.0)),
        pick_list(transport_options, Some(form.transport), |t| {
            Message::Settings(settings::Msg::McpFormTransportChanged(t))
        })
        .width(Length::Fixed(220.0)),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    let name_row = row![
        text("Name:").size(14).width(Length::Fixed(120.0)),
        text_input("e.g. filesystem", &form.name)
            .on_input(|v| Message::Settings(settings::Msg::McpFormNameChanged(v)))
            .width(Length::Fixed(420.0)),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    let enabled_row = row![
        text("Enabled:").size(14).width(Length::Fixed(120.0)),
        checkbox(form.is_enabled)
            .on_toggle(|v| Message::Settings(settings::Msg::McpFormEnabledChanged(v))),
        text(if form.is_enabled { "On" } else { "Off" }).size(12),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    let args_row = row![
        text("Args (JSON):").size(14).width(Length::Fixed(120.0)),
        text_input(r#"e.g. ["--root","/path"]"#, &form.args_json)
            .on_input(|v| Message::Settings(settings::Msg::McpFormArgsJsonChanged(v)))
            .width(Length::Fixed(420.0)),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    let env_row = row![
        text("Env (JSON):").size(14).width(Length::Fixed(120.0)),
        text_input(r#"e.g. {"KEY":"VALUE"}"#, &form.env_json)
            .on_input(|v| Message::Settings(settings::Msg::McpFormEnvJsonChanged(v)))
            .width(Length::Fixed(420.0)),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    let transport_specific = match form.transport {
        McpTransport::Stdio => {
            let command_row = row![
                text("Command:").size(14).width(Length::Fixed(120.0)),
                text_input("e.g. mcp-filesystem", &form.command)
                    .on_input(|v| Message::Settings(settings::Msg::McpFormCommandChanged(v)))
                    .width(Length::Fixed(420.0)),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center);

            column![command_row].spacing(10)
        }
        McpTransport::StreamableHttp => {
            let url_row = row![
                text("Endpoint URL:").size(14).width(Length::Fixed(120.0)),
                text_input("e.g. http://localhost:8080", &form.endpoint_url)
                    .on_input(|v| {
                        Message::Settings(settings::Msg::McpFormEndpointUrlChanged(v))
                    })
                    .width(Length::Fixed(420.0)),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center);

            column![url_row].spacing(10)
        }
    };

    let actions = row![
        button(row![icon(icons::SAVE).size(16), text(" Save").size(14),].spacing(4))
            .on_press(Message::Settings(settings::Msg::McpFormSave))
            .style(styles::primary_button)
            .padding([8, 12]),
        button(row![icon(icons::ADD).size(16), text(" New").size(14),].spacing(4))
            .on_press(Message::Settings(settings::Msg::McpFormNew))
            .style(styles::secondary_button)
            .padding([8, 12]),
        button(row![icon(icons::CLOSE).size(16), text(" Cancel").size(14),].spacing(4))
            .on_press(Message::Settings(settings::Msg::McpFormCancel))
            .style(styles::secondary_button)
            .padding([8, 12]),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    let import_card: Element<'a, Message> = container(
        column![
            text("Import MCP JSON").size(18),
            text("Paste MCP server JSON (Claude Desktop-style `mcpServers`, or an array of server objects) and add them to Ticca.")
                .size(12),
            container(
                text_editor(import_json)
                    .on_action(|action| {
                        Message::Settings(settings::Msg::McpImportEditorAction(action))
                    })
                    .height(Length::Fixed(180.0))
                    .style(move |theme, _status| styles::raw_text_editor(theme, is_dark)),
            )
            .padding(8)
            .style(styles::card_container),
            row![
                button(row![icon(icons::ADD).size(16), text(" Parse & Add").size(14),].spacing(4))
                    .on_press(Message::Settings(settings::Msg::McpImportApply))
                    .style(styles::primary_button)
                    .padding([8, 12]),
                button(row![icon(icons::CLOSE).size(16), text(" Clear").size(14),].spacing(4))
                    .on_press(Message::Settings(settings::Msg::McpImportClear))
                    .style(styles::secondary_button)
                    .padding([8, 12]),
            ]
            .spacing(10),
        ]
        .spacing(10),
    )
    .padding(20)
    .style(styles::card_container)
    .into();

    let form_card: Element<'a, Message> = container(
        column![
            text(form_title).size(18),
            text("Configure MCP servers that provide additional tools.").size(12),
            Space::new().height(Length::Fixed(4.0)),
            name_row,
            transport_row,
            transport_specific,
            args_row,
            env_row,
            enabled_row,
            Space::new().height(Length::Fixed(6.0)),
            actions,
        ]
        .spacing(10),
    )
    .padding(20)
    .style(styles::card_container)
    .into();

    let mut server_rows: Vec<Element<'a, Message>> = Vec::new();
    for server in servers {
        let transport_label = match server.transport {
            McpTransport::Stdio => "stdio",
            McpTransport::StreamableHttp => "http",
        };

        let summary = match server.transport {
            McpTransport::Stdio => server
                .command
                .as_deref()
                .map(|c| format!("{} • {}", transport_label, c))
                .unwrap_or_else(|| transport_label.to_string()),
            McpTransport::StreamableHttp => server
                .endpoint_url
                .as_deref()
                .map(|u| format!("{} • {}", transport_label, u))
                .unwrap_or_else(|| transport_label.to_string()),
        };

        let row_el: Element<'a, Message> = container(
            row![
                checkbox(server.is_enabled).on_toggle({
                    let server_id = server.id.clone();
                    move |enabled| {
                        Message::Settings(settings::Msg::McpSetServerEnabled {
                            server_id: server_id.clone(),
                            enabled,
                        })
                    }
                }),
                column![text(&server.name).size(14), text(summary).size(11),]
                    .spacing(2)
                    .width(Length::Fill),
                button(row![icon(icons::EDIT).size(16)].spacing(6))
                    .on_press(Message::Settings(settings::Msg::McpFormEdit(
                        server.id.clone()
                    )))
                    .style(styles::secondary_button)
                    .padding([4, 8]),
                button(row![icon(icons::DELETE).size(16)].spacing(6))
                    .on_press(Message::Settings(settings::Msg::McpDeleteServer(
                        server.id.clone()
                    )))
                    .style(styles::danger_icon_button)
                    .padding([4, 8]),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
        )
        .padding([10, 12])
        .style(styles::card_container)
        .into();

        server_rows.push(row_el);
    }

    let list_body: Element<'a, Message> = if server_rows.is_empty() {
        text("No MCP servers configured yet.").size(13).into()
    } else {
        Column::with_children(server_rows).spacing(10).into()
    };

    let list_card: Element<'a, Message> = container(
        column![
            row![
                text("Configured Servers").size(18),
                horizontal_space(),
                button(row![icon(icons::REFRESH).size(16), text(" Refresh").size(14),].spacing(4))
                    .on_press(Message::Settings(settings::Msg::RefreshMcp))
                    .style(styles::secondary_button)
                    .padding([6, 10]),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            list_body,
        ]
        .spacing(12),
    )
    .padding(20)
    .style(styles::card_container)
    .into();

    column![import_card, form_card, list_card]
        .spacing(12)
        .into()
}

/// Build the agent MCP assignment section
pub(super) fn build_agent_mcp_section<'a>(
    servers: &'a [McpServer],
    agent_mcp_server_ids: &'a HashMap<AgentType, Vec<String>>,
) -> Element<'a, Message> {
    let help = container(
        column![
            row![
                text("Agent Configuration").size(18),
                horizontal_space(),
                button(row![icon(icons::REFRESH).size(16), text(" Refresh").size(14),].spacing(4))
                    .on_press(Message::Settings(settings::Msg::RefreshMcp))
                    .style(styles::secondary_button)
                    .padding([6, 10]),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            text("Assign MCP servers to agents (rows: servers, columns: agents).").size(12),
        ]
        .spacing(10),
    )
    .padding(20)
    .style(styles::card_container);

    let agents = AgentRegistry::all();

    let table_body: Element<'a, Message> = if servers.is_empty() {
        text("No MCP servers configured. Add one in the MCP Servers tab.")
            .size(13)
            .into()
    } else {
        let server_col_width = 340.0;
        let agent_col_width = 120.0;

        let header_row: Element<'a, Message> = container(
            row![
                text("MCP Server")
                    .size(13)
                    .width(Length::Fixed(server_col_width)),
                row(agents.iter().map(|&agent| {
                    container(text(AgentRegistry::get(agent).display_name).size(13))
                        .width(Length::Fixed(agent_col_width))
                        .center_x(Length::Fixed(agent_col_width))
                        .into()
                }))
                .spacing(10)
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
        )
        .padding([10, 12])
        .style(styles::card_container)
        .into();

        let mut rows: Vec<Element<'a, Message>> = vec![header_row];

        for server in servers {
            let transport_label = match server.transport {
                McpTransport::Stdio => "stdio",
                McpTransport::StreamableHttp => "http",
            };

            let detail = match server.transport {
                McpTransport::Stdio => server
                    .command
                    .as_deref()
                    .map(|c| format!("{} • {}", transport_label, c))
                    .unwrap_or_else(|| transport_label.to_string()),
                McpTransport::StreamableHttp => server
                    .endpoint_url
                    .as_deref()
                    .map(|u| format!("{} • {}", transport_label, u))
                    .unwrap_or_else(|| transport_label.to_string()),
            };

            let name = if server.is_enabled {
                server.name.clone()
            } else {
                format!("{} (disabled)", server.name)
            };

            let server_cell: Element<'a, Message> =
                column![text(name).size(14), text(detail).size(11),]
                    .spacing(2)
                    .width(Length::Fixed(server_col_width))
                    .into();

            let mut agent_cells: Vec<Element<'a, Message>> = Vec::new();
            for &agent in agents {
                let checked = agent_mcp_server_ids
                    .get(&agent)
                    .map(|ids| ids.contains(&server.id))
                    .unwrap_or(false);

                agent_cells.push(
                    container(checkbox(checked).on_toggle({
                        let server_id = server.id.clone();
                        move |enabled| {
                            Message::Settings(settings::Msg::AgentMcpToggled {
                                agent_type: agent,
                                server_id: server_id.clone(),
                                enabled,
                            })
                        }
                    }))
                    .width(Length::Fixed(agent_col_width))
                    .center_x(Length::Fixed(agent_col_width))
                    .into(),
                );
            }

            let row_el: Element<'a, Message> = container(
                row![server_cell, row(agent_cells).spacing(10)]
                    .spacing(10)
                    .align_y(iced::Alignment::Center),
            )
            .padding([10, 12])
            .style(styles::card_container)
            .into();

            rows.push(row_el);
        }

        Column::with_children(rows).spacing(10).into()
    };

    let table_card: Element<'a, Message> = container(table_body)
        .padding(20)
        .style(styles::card_container)
        .into();

    column![help, table_card].spacing(12).into()
}
