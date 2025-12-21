//! Settings/Configuration view

use iced::widget::{
    Column, Space, button, checkbox, column, container, pick_list, row, scrollable, text,
    text_editor, text_input,
};
use iced::{Border, Color, Element, Length};

use std::collections::HashMap;

use crate::material_icons::{icon, icons};
use crate::messages::{Message, OAuthProvider, SettingsTab, chat, settings};
use crate::theme::{AppTheme, styles};

use ticca_core::agents::AgentType;
use ticca_core::config::{McpServer, McpTransport, OAuthAccount};
use ticca_core::session::Session;

/// Create horizontal space that fills available width (iced 0.14 helper)
fn horizontal_space() -> Space {
    Space::new().width(Length::Fill)
}

/// Provider authentication status
#[derive(Debug, Clone, Default)]
pub struct ProviderAuthStatus {
    pub claude: bool,
    pub gemini: bool,
    pub chatgpt: bool,
}

#[derive(Debug, Clone)]
pub struct McpServerFormState {
    pub editing_id: Option<String>,
    pub name: String,
    pub transport: McpTransport,
    pub command: String,
    pub args_json: String,
    pub env_json: String,
    pub endpoint_url: String,
    pub is_enabled: bool,
}

impl Default for McpServerFormState {
    fn default() -> Self {
        Self {
            editing_id: None,
            name: String::new(),
            transport: McpTransport::Stdio,
            command: String::new(),
            args_json: "[]".to_string(),
            env_json: "{}".to_string(),
            endpoint_url: String::new(),
            is_enabled: true,
        }
    }
}

/// Render the settings view
#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    theme: AppTheme,
    available_models: &'a [String],
    default_model: Option<&'a str>,
    agent_pinned_models: &'a HashMap<AgentType, String>,
    is_loading_models: bool,
    auth_status: &ProviderAuthStatus,
    claude_accounts: &'a [OAuthAccount],
    gemini_accounts: &'a [OAuthAccount],
    chatgpt_accounts: &'a [OAuthAccount],
    yolo_mode_enabled: bool,
    expert_mode_enabled: bool,
    recent_sessions: &'a [Session],
    active_tab: SettingsTab,
    mcp_servers: &'a [McpServer],
    mcp_form: &'a McpServerFormState,
    mcp_import_json: &'a text_editor::Content,
    agent_mcp_server_ids: &'a HashMap<AgentType, Vec<String>>,
) -> Element<'a, Message> {
    let header = row![
        button(row![icon(icons::ARROW_BACK).size(16), text(" Back").size(14),].spacing(4))
            .on_press(Message::Settings(settings::Msg::CloseSettings))
            .style(styles::secondary_button)
            .padding([8, 12]),
        text("Settings").size(24),
    ]
    .spacing(20)
    .padding(10)
    .align_y(iced::Alignment::Center);

    let effective_tab = if !expert_mode_enabled
        && matches!(active_tab, SettingsTab::Models | SettingsTab::Agents)
    {
        SettingsTab::Accounts
    } else {
        active_tab
    };

    let tabs = build_tabs(effective_tab, expert_mode_enabled);

    // Theme selector with all available themes
    let theme_options: Vec<AppTheme> = vec![
        AppTheme::Dark,
        AppTheme::Light,
        AppTheme::Zinc,
        AppTheme::Dracula,
        AppTheme::Nord,
        AppTheme::CatppuccinMocha,
        AppTheme::CatppuccinLatte,
        AppTheme::TokyoNight,
        AppTheme::OneDark,
        AppTheme::GruvboxDark,
        AppTheme::GruvboxLight,
    ];

    let theme_icon = if theme.is_dark() {
        icons::DARK_MODE
    } else {
        icons::LIGHT_MODE
    };

    let appearance: Element<Message> = container(
        column![
            text("Appearance").size(18),
            row![
                icon(theme_icon).size(16),
                text("Theme:").size(14).width(Length::Fixed(80.0)),
                pick_list(theme_options, Some(theme), |t| {
                    Message::Settings(settings::Msg::SetTheme(t))
                })
                .width(Length::Fixed(200.0)),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
            row![
                icon(icons::TUNE).size(16),
                text("Expert mode:").size(14).width(Length::Fixed(80.0)),
                checkbox(expert_mode_enabled)
                    .on_toggle(|v| Message::Settings(settings::Msg::SetExpertMode(v))),
                text(if expert_mode_enabled { "On" } else { "Off" }).size(12),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(15),
    )
    .padding(20)
    .style(styles::card_container)
    .into();

    let model_settings = build_model_settings_section(
        available_models,
        default_model,
        agent_pinned_models,
        is_loading_models,
    );
    let tools_section = build_tools_section(yolo_mode_enabled);
    let sessions_section = build_sessions_section(recent_sessions);
    let accounts_section = build_accounts_section(
        auth_status,
        claude_accounts,
        gemini_accounts,
        chatgpt_accounts,
        expert_mode_enabled,
    );
    let mcp_servers_section =
        build_mcp_servers_section(mcp_servers, mcp_form, mcp_import_json, theme);
    let agents_section = build_agent_mcp_section(mcp_servers, agent_mcp_server_ids);

    let content = match effective_tab {
        SettingsTab::Accounts => accounts_section,
        SettingsTab::Models => model_settings,
        SettingsTab::Agents => agents_section,
        SettingsTab::McpServers => mcp_servers_section,
        SettingsTab::Tools => tools_section,
        SettingsTab::Appearance => appearance,
        SettingsTab::Sessions => sessions_section,
    };

    scrollable(column![header, tabs, content,].spacing(16).padding(10)).into()
}

/// Build the model settings section with default model and agent pinning
fn build_model_settings_section<'a>(
    available_models: &'a [String],
    default_model: Option<&'a str>,
    agent_pinned_models: &'a HashMap<AgentType, String>,
    is_loading_models: bool,
) -> Element<'a, Message> {
    // Create options for pick_list with "Use Default" option for agent pinning
    let model_options: Vec<String> = available_models.to_vec();

    // Header with refresh button
    let header_row = row![
        text("Model Settings").size(18),
        horizontal_space(),
        button(
            row![
                icon(if is_loading_models {
                    icons::HOURGLASS_EMPTY
                } else {
                    icons::REFRESH
                })
                .size(16),
                text(if is_loading_models {
                    " Loading..."
                } else {
                    " Refresh"
                })
                .size(14),
            ]
            .spacing(4)
        )
        .on_press_maybe(if is_loading_models {
            None
        } else {
            Some(Message::Settings(settings::Msg::RefreshModels))
        })
        .style(styles::secondary_button)
        .padding([6, 10]),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    // Default model selector
    let default_model_row = if available_models.is_empty() {
        row![
            text("Default Model:").size(14).width(Length::Fixed(140.0)),
            text("No models available - authenticate with Claude first").size(14),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
    } else {
        let selected_default = default_model.map(|s| s.to_string());
        row![
            text("Default Model:").size(14).width(Length::Fixed(140.0)),
            pick_list(model_options.clone(), selected_default, |model| {
                Message::Settings(settings::Msg::SetDefaultModel(model))
            })
            .placeholder("Select default model...")
            .width(Length::Fixed(300.0)),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
    };

    // Agent model pinning section
    let agent_pinning_header = text("Agent Model Pinning").size(16);
    let agent_pinning_desc =
        text("Pin specific models to agents. If not pinned, the default model is used.").size(12);

    // Build agent pinning rows
    let coding_pinned = agent_pinned_models.get(&AgentType::Coding).cloned();
    let planning_pinned = agent_pinned_models.get(&AgentType::Planning).cloned();

    // Create options with "Use Default" at the start
    let agent_model_options: Vec<ModelOption> = std::iter::once(ModelOption::UseDefault)
        .chain(
            available_models
                .iter()
                .map(|m| ModelOption::Model(m.clone())),
        )
        .collect();

    let coding_row = if available_models.is_empty() {
        row![
            icon(icons::CODE).size(16),
            text("Coding Agent:").size(14).width(Length::Fixed(120.0)),
            text("No models available").size(14),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
    } else {
        let selected = coding_pinned
            .map(ModelOption::Model)
            .unwrap_or(ModelOption::UseDefault);
        row![
            icon(icons::CODE).size(16),
            text("Coding Agent:").size(14).width(Length::Fixed(120.0)),
            pick_list(agent_model_options.clone(), Some(selected), move |opt| {
                match opt {
                    ModelOption::UseDefault => {
                        Message::Settings(settings::Msg::SetAgentModel(AgentType::Coding, None))
                    }
                    ModelOption::Model(m) => {
                        Message::Settings(settings::Msg::SetAgentModel(AgentType::Coding, Some(m)))
                    }
                }
            })
            .width(Length::Fixed(300.0)),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
    };

    let planning_row = if available_models.is_empty() {
        row![
            icon(icons::CHECKLIST).size(16),
            text("Planning Agent:").size(14).width(Length::Fixed(120.0)),
            text("No models available").size(14),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
    } else {
        let selected = planning_pinned
            .map(ModelOption::Model)
            .unwrap_or(ModelOption::UseDefault);
        row![
            icon(icons::CHECKLIST).size(16),
            text("Planning Agent:").size(14).width(Length::Fixed(120.0)),
            pick_list(agent_model_options.clone(), Some(selected), move |opt| {
                match opt {
                    ModelOption::UseDefault => {
                        Message::Settings(settings::Msg::SetAgentModel(AgentType::Planning, None))
                    }
                    ModelOption::Model(m) => Message::Settings(settings::Msg::SetAgentModel(
                        AgentType::Planning,
                        Some(m),
                    )),
                }
            })
            .width(Length::Fixed(300.0)),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
    };

    // Custom horizontal rule using a styled container
    let rule = container(text(""))
        .width(Length::Fill)
        .height(1)
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(Color::from_rgba(0.5, 0.5, 0.5, 0.3).into()),
            border: Border::default(),
            ..Default::default()
        });

    container(
        column![
            header_row,
            default_model_row,
            rule,
            agent_pinning_header,
            agent_pinning_desc,
            coding_row,
            planning_row,
        ]
        .spacing(12),
    )
    .padding(20)
    .style(styles::card_container)
    .into()
}

fn build_tabs(active: SettingsTab, expert_mode_enabled: bool) -> Element<'static, Message> {
    let tab_button = |tab: SettingsTab, label: &str, tab_icon| {
        let style = if tab == active {
            styles::primary_button
        } else {
            styles::secondary_button
        };

        button(
            row![
                icon(tab_icon).size(14),
                text(format!(" {}", label)).size(13),
            ]
            .spacing(4),
        )
        .on_press(Message::Settings(settings::Msg::SwitchSettingsTab(tab)))
        .style(style)
        .padding([6, 10])
    };

    let mut buttons: Vec<Element<'static, Message>> = Vec::new();
    buttons.push(tab_button(SettingsTab::Accounts, "Accounts", icons::KEY).into());
    if expert_mode_enabled {
        buttons.push(tab_button(SettingsTab::Models, "Models", icons::TUNE).into());
        buttons.push(tab_button(SettingsTab::Agents, "Agents", icons::SMART_TOY).into());
    }
    buttons.push(
        tab_button(
            SettingsTab::McpServers,
            "MCP Servers",
            icons::INTEGRATION_INSTRUCTIONS,
        )
        .into(),
    );
    buttons.push(tab_button(SettingsTab::Tools, "Tools & Safety", icons::SECURITY).into());
    buttons.push(tab_button(SettingsTab::Appearance, "Appearance", icons::BRIGHTNESS_6).into());
    buttons.push(tab_button(SettingsTab::Sessions, "Sessions", icons::FOLDER_OPEN).into());

    row(buttons).spacing(8).into()
}

fn build_mcp_servers_section<'a>(
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

fn build_agent_mcp_section<'a>(
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

    let agents = [AgentType::Coding, AgentType::Planning];

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
                row(agents.iter().map(|agent| {
                    container(text(agent.display_name()).size(13))
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
            for agent in agents {
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

fn build_accounts_section(
    auth_status: &ProviderAuthStatus,
    claude_accounts: &[OAuthAccount],
    gemini_accounts: &[OAuthAccount],
    chatgpt_accounts: &[OAuthAccount],
    expert_mode_enabled: bool,
) -> Element<'static, Message> {
    let oauth_button = |provider: OAuthProvider, label: &str, is_authenticated: bool| {
        let auth_icon = if is_authenticated {
            icons::CHECK_CIRCLE
        } else {
            icons::VPN_KEY
        };
        let style_fn = if is_authenticated {
            styles::success_button
        } else {
            styles::secondary_button
        };
        button(
            row![
                icon(auth_icon).size(16),
                text(format!(" Add {}", label)).size(14),
            ]
            .spacing(4),
        )
        .on_press(Message::Settings(settings::Msg::StartOAuth(provider)))
        .style(style_fn)
        .padding([8, 12])
    };

    let accounts_section = |label: &str, accounts: &[OAuthAccount]| -> Element<Message> {
        let label = label.to_string();
        let rows: Vec<Element<Message>> = if accounts.is_empty() {
            vec![text("No accounts yet.").size(13).into()]
        } else {
            accounts
                .iter()
                .cloned()
                .map(|account| {
                    let status = if !account.is_active {
                        "inactive"
                    } else if account.is_expired() {
                        "expired"
                    } else if account.is_cooling() {
                        "cooldown"
                    } else {
                        "ready"
                    };

                    let label_text = account.label.clone().unwrap_or_else(|| {
                        format!("{}…", account.id.chars().take(8).collect::<String>())
                    });

                    let detail = account
                        .cooldown_until
                        .clone()
                        .map(|until| {
                            format!(
                                "priority {} • {} • cooldown until {}",
                                account.priority, status, until
                            )
                        })
                        .unwrap_or_else(|| format!("priority {} • {}", account.priority, status));

                    container(
                        row![
                            column![text(label_text).size(14), text(detail).size(11),]
                                .spacing(2)
                                .width(Length::Fill),
                            row![
                                button(
                                    row![
                                        icon(if account.is_active {
                                            icons::CHECK_CIRCLE
                                        } else {
                                            icons::CANCEL
                                        })
                                        .size(14),
                                        text(if account.is_active {
                                            " Active"
                                        } else {
                                            " Disabled"
                                        })
                                        .size(12),
                                    ]
                                    .spacing(4)
                                )
                                .on_press(Message::Settings(
                                    settings::Msg::ToggleOAuthAccountActive {
                                        account_id: account.id.clone(),
                                        is_active: !account.is_active,
                                    }
                                ))
                                .style(styles::secondary_button)
                                .padding([6, 10]),
                                button(
                                    row![
                                        icon(icons::ARROW_BACK).size(14),
                                        text(" Priority -").size(12),
                                    ]
                                    .spacing(4)
                                )
                                .on_press(Message::Settings(
                                    settings::Msg::AdjustOAuthAccountPriority {
                                        account_id: account.id.clone(),
                                        delta: -1,
                                    }
                                ))
                                .style(styles::secondary_button)
                                .padding([6, 10]),
                                button(
                                    row![
                                        icon(icons::ARROW_FORWARD).size(14),
                                        text(" Priority +").size(12),
                                    ]
                                    .spacing(4)
                                )
                                .on_press(Message::Settings(
                                    settings::Msg::AdjustOAuthAccountPriority {
                                        account_id: account.id.clone(),
                                        delta: 1,
                                    }
                                ))
                                .style(styles::secondary_button)
                                .padding([6, 10]),
                                button(
                                    row![
                                        icon(icons::REFRESH).size(14),
                                        text(" Reset cooldown").size(12),
                                    ]
                                    .spacing(4)
                                )
                                .on_press(Message::Settings(settings::Msg::ResetOAuthCooldown(
                                    account.id.clone(),
                                )))
                                .style(styles::secondary_button)
                                .padding([6, 10]),
                                button(
                                    row![icon(icons::DELETE).size(14), text(" Remove").size(12),]
                                        .spacing(4)
                                )
                                .on_press(Message::Settings(settings::Msg::RemoveOAuthAccount(
                                    account.id,
                                )))
                                .style(styles::secondary_button)
                                .padding([6, 10]),
                            ]
                            .spacing(6)
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

        column![text(label).size(16), Column::with_children(rows).spacing(4),]
            .spacing(8)
            .into()
    };

    let mut children: Vec<Element<Message>> = Vec::new();
    children.push(text("Accounts").size(18).into());
    children.push(
        row![
            oauth_button(OAuthProvider::Claude, "Claude", auth_status.claude),
            oauth_button(OAuthProvider::Gemini, "Gemini", auth_status.gemini),
            oauth_button(OAuthProvider::ChatGpt, "ChatGPT", auth_status.chatgpt),
        ]
        .spacing(8)
        .into(),
    );

    if expert_mode_enabled {
        children.push(accounts_section("Claude Accounts", claude_accounts));
        children.push(accounts_section("Gemini Accounts", gemini_accounts));
        children.push(accounts_section("ChatGPT Accounts", chatgpt_accounts));
    }

    container(Column::with_children(children).spacing(12))
        .padding(20)
        .style(styles::card_container)
        .into()
}

/// Option type for agent model picker
#[derive(Debug, Clone, PartialEq, Eq)]
enum ModelOption {
    UseDefault,
    Model(String),
}

impl std::fmt::Display for ModelOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelOption::UseDefault => write!(f, "Use Default"),
            ModelOption::Model(name) => write!(f, "{}", name),
        }
    }
}

fn build_tools_section(yolo_mode_enabled: bool) -> Element<'static, Message> {
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

    container(
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
    .style(styles::card_container)
    .into()
}

/// Build the recent sessions section
fn build_sessions_section<'a>(sessions: &'a [Session]) -> Element<'a, Message> {
    let session_list: Vec<Element<'a, Message>> = if sessions.is_empty() {
        vec![text("No saved sessions yet.").size(14).into()]
    } else {
        sessions
            .iter()
            .cloned()
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
