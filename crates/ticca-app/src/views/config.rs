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

use ticca_core::external_tools::{
    ExternalToolId, FUSE_DOCS_URL, get_all_tool_definitions, is_fuse_available,
};

use ticca_core::agents::AgentType;
use ticca_core::config::{ApiKeyAccount, ApiKeyProvider, CompressionSettings, CompressionStrategy, McpServer, McpTransport, OAuthAccount};
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
    api_key_accounts: &'a HashMap<ApiKeyProvider, Vec<ApiKeyAccount>>,
    api_key_form_provider: Option<ApiKeyProvider>,
    api_key_form_value: &'a str,
    api_key_form_label: &'a str,
    yolo_mode_enabled: bool,
    expert_mode_enabled: bool,
    recent_sessions: &'a [Session],
    active_tab: SettingsTab,
    mcp_servers: &'a [McpServer],
    mcp_form: &'a McpServerFormState,
    mcp_import_json: &'a text_editor::Content,
    agent_mcp_server_ids: &'a HashMap<AgentType, Vec<String>>,
    external_tools: &'a HashMap<ExternalToolId, settings::ToolStatusInfo>,
    compression: &'a CompressionSettings,
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
    let tools_section = build_tools_section(yolo_mode_enabled, external_tools, compression);
    let sessions_section = build_sessions_section(recent_sessions);
    let accounts_section = build_accounts_section(
        auth_status,
        claude_accounts,
        gemini_accounts,
        chatgpt_accounts,
        api_key_accounts,
        api_key_form_provider,
        api_key_form_value,
        api_key_form_label,
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

fn build_accounts_section<'a>(
    auth_status: &ProviderAuthStatus,
    claude_accounts: &[OAuthAccount],
    gemini_accounts: &[OAuthAccount],
    chatgpt_accounts: &[OAuthAccount],
    api_key_accounts: &'a HashMap<ApiKeyProvider, Vec<ApiKeyAccount>>,
    api_key_form_provider: Option<ApiKeyProvider>,
    api_key_form_value: &'a str,
    api_key_form_label: &'a str,
    expert_mode_enabled: bool,
) -> Element<'a, Message> {
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
        children.push(accounts_section("Claude (OAuth)", claude_accounts));
        children.push(accounts_section("Gemini (OAuth)", gemini_accounts));
        children.push(accounts_section("ChatGPT (OAuth)", chatgpt_accounts));
    }

    let oauth_card: Element<'a, Message> = container(Column::with_children(children).spacing(12))
        .padding(20)
        .style(styles::card_container)
        .into();

    // Build API key providers section
    let api_key_section = build_api_key_providers_section(
        api_key_accounts,
        api_key_form_provider,
        api_key_form_value,
        api_key_form_label,
    );

    column![oauth_card, api_key_section].spacing(16).into()
}

/// Build the API key providers section
fn build_api_key_providers_section<'a>(
    api_key_accounts: &'a HashMap<ApiKeyProvider, Vec<ApiKeyAccount>>,
    form_provider: Option<ApiKeyProvider>,
    form_value: &'a str,
    form_label: &'a str,
) -> Element<'a, Message> {
    // Build provider cards
    let mut provider_cards: Vec<Element<'a, Message>> = Vec::new();

    for provider in ApiKeyProvider::ALL {
        let accounts = api_key_accounts.get(provider).cloned().unwrap_or_default();
        let has_accounts = !accounts.is_empty();
        let is_form_open = form_provider == Some(*provider);

        // Provider header with Add button
        let add_button = if is_form_open {
            button(row![icon(icons::CLOSE).size(14), text(" Cancel").size(12),].spacing(4))
                .on_press(Message::Settings(settings::Msg::CancelAddApiKey))
                .style(styles::secondary_button)
                .padding([6, 10])
        } else {
            button(row![icon(icons::ADD).size(14), text(" Add").size(12),].spacing(4))
                .on_press(Message::Settings(settings::Msg::StartAddApiKey(*provider)))
                .style(if has_accounts {
                    styles::success_button
                } else {
                    styles::secondary_button
                })
                .padding([6, 10])
        };

        let header = row![
            text(provider.display_name()).size(16),
            horizontal_space(),
            add_button,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        // Form (if open for this provider)
        let form_element: Option<Element<'a, Message>> = if is_form_open {
            Some(
                container(
                    column![
                        row![
                            text("API Key:").size(13).width(Length::Fixed(80.0)),
                            text_input("Enter API key...", form_value)
                                .on_input(|v| Message::Settings(settings::Msg::ApiKeyFormChanged(v)))
                                .width(Length::Fill)
                                .secure(true),
                        ]
                        .spacing(10)
                        .align_y(iced::Alignment::Center),
                        row![
                            text("Label:").size(13).width(Length::Fixed(80.0)),
                            text_input("Optional label...", form_label)
                                .on_input(|v| Message::Settings(settings::Msg::ApiKeyLabelFormChanged(v)))
                                .width(Length::Fill),
                        ]
                        .spacing(10)
                        .align_y(iced::Alignment::Center),
                        row![
                            horizontal_space(),
                            button(row![icon(icons::SAVE).size(14), text(" Save").size(12),].spacing(4))
                                .on_press(Message::Settings(settings::Msg::SaveApiKey))
                                .style(styles::primary_button)
                                .padding([6, 10]),
                        ]
                        .spacing(8),
                    ]
                    .spacing(10),
                )
                .padding(12)
                .style(styles::card_container)
                .into(),
            )
        } else {
            None
        };

        // Account list
        let account_rows: Vec<Element<'a, Message>> = accounts
            .into_iter()
            .map(|account| {
                let status = if !account.is_active {
                    "inactive"
                } else if account.is_cooling() {
                    "cooldown"
                } else {
                    "ready"
                };

                let label_text = account
                    .label
                    .clone()
                    .unwrap_or_else(|| account.masked_key());

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
                        column![text(label_text).size(13), text(detail).size(10),]
                            .spacing(2)
                            .width(Length::Fill),
                        row![
                            button(icon(if account.is_active {
                                icons::CHECK_CIRCLE
                            } else {
                                icons::CANCEL
                            }).size(14))
                            .on_press(Message::Settings(
                                settings::Msg::ToggleApiKeyAccountActive {
                                    account_id: account.id.clone(),
                                    is_active: !account.is_active,
                                }
                            ))
                            .style(styles::secondary_button)
                            .padding([4, 6]),
                            button(icon(icons::ARROW_BACK).size(14))
                                .on_press(Message::Settings(
                                    settings::Msg::AdjustApiKeyAccountPriority {
                                        account_id: account.id.clone(),
                                        delta: -1,
                                    }
                                ))
                                .style(styles::secondary_button)
                                .padding([4, 6]),
                            button(icon(icons::ARROW_FORWARD).size(14))
                                .on_press(Message::Settings(
                                    settings::Msg::AdjustApiKeyAccountPriority {
                                        account_id: account.id.clone(),
                                        delta: 1,
                                    }
                                ))
                                .style(styles::secondary_button)
                                .padding([4, 6]),
                            button(icon(icons::REFRESH).size(14))
                                .on_press(Message::Settings(settings::Msg::ResetApiKeyCooldown(
                                    account.id.clone(),
                                )))
                                .style(styles::secondary_button)
                                .padding([4, 6]),
                            button(icon(icons::DELETE).size(14))
                                .on_press(Message::Settings(settings::Msg::RemoveApiKeyAccount(
                                    account.id,
                                )))
                                .style(styles::danger_icon_button)
                                .padding([4, 6]),
                        ]
                        .spacing(4)
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                )
                .padding(6)
                .width(Length::Fill)
                .into()
            })
            .collect();

        let accounts_list: Element<'a, Message> = if account_rows.is_empty() {
            text("No API keys configured.").size(12).into()
        } else {
            Column::with_children(account_rows).spacing(4).into()
        };

        // Combine header, form, and accounts list
        let mut card_children: Vec<Element<'a, Message>> = vec![header.into()];
        if let Some(form) = form_element {
            card_children.push(form);
        }
        card_children.push(accounts_list);

        let provider_card: Element<'a, Message> = container(
            Column::with_children(card_children).spacing(10),
        )
        .padding(12)
        .style(styles::card_container)
        .into();

        provider_cards.push(provider_card);
    }

    container(
        column![
            text("API Key Providers").size(18),
            text("Configure API keys for direct API access. Multiple keys per provider enable automatic failover on rate limits.")
                .size(12)
                .style(|_theme: &iced::Theme| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(120, 120, 120)),
                }),
            Column::with_children(provider_cards).spacing(10),
        ]
        .spacing(12),
    )
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

fn build_tools_section<'a>(
    yolo_mode_enabled: bool,
    external_tools: &'a HashMap<ExternalToolId, settings::ToolStatusInfo>,
    compression: &'a CompressionSettings,
) -> Element<'a, Message> {
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

    let yolo_section = container(
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
    .style(styles::card_container);

    // Build compression settings section
    let compression_section = build_compression_section(compression);

    // Build external tools section
    let external_tools_section = build_external_tools_section(external_tools);

    column![yolo_section, compression_section, external_tools_section,]
        .spacing(16)
        .into()
}

/// Build the context compression settings section
fn build_compression_section(compression: &CompressionSettings) -> Element<'_, Message> {
    // Compression enabled toggle
    let enabled_row = row![
        icon(icons::COMPRESS).size(16),
        text("Context Compression:").size(14).width(Length::Fixed(140.0)),
        checkbox(compression.enabled)
            .on_toggle(|v| Message::Settings(settings::Msg::SetCompressionEnabled(v))),
        text(if compression.enabled { "Enabled" } else { "Disabled" }).size(12),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    // Threshold slider
    let threshold_row = row![
        text("Threshold:").size(14).width(Length::Fixed(140.0)),
        iced::widget::slider(50..=95, compression.threshold_percent, |v| {
            Message::Settings(settings::Msg::SetCompressionThreshold(v))
        })
        .width(Length::Fixed(200.0)),
        text(format!("{}%", compression.threshold_percent)).size(12),
        text("of context window").size(11).style(|_theme: &iced::Theme| iced::widget::text::Style {
            color: Some(Color::from_rgb8(120, 120, 120)),
        }),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    // Strategy picker
    let strategy_options = vec![
        CompressionStrategy::Truncation,
        CompressionStrategy::Summarizing,
    ];
    let strategy_row = row![
        text("Strategy:").size(14).width(Length::Fixed(140.0)),
        pick_list(strategy_options, Some(compression.strategy), |s| {
            Message::Settings(settings::Msg::SetCompressionStrategy(s))
        })
        .width(Length::Fixed(180.0)),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    // Strategy description
    let strategy_desc = match compression.strategy {
        CompressionStrategy::Truncation => {
            "Keeps system prompt + recent messages (by token count)"
        }
        CompressionStrategy::Summarizing => {
            "Generates a summary of removed context (uses LLM tokens)"
        }
    };
    let strategy_desc_row = row![
        Space::new().width(Length::Fixed(140.0)),
        text(strategy_desc).size(11).style(|_theme: &iced::Theme| iced::widget::text::Style {
            color: Some(Color::from_rgb8(120, 120, 120)),
        }),
    ]
    .spacing(10);

    // Preserve first messages
    let preserve_first_row = row![
        text("Preserve First:").size(14).width(Length::Fixed(140.0)),
        iced::widget::slider(0..=5, compression.preserve_first, |v| {
            Message::Settings(settings::Msg::SetCompressionPreserveFirst(v))
        })
        .width(Length::Fixed(120.0)),
        text(format!("{} messages", compression.preserve_first)).size(12),
        text("(system prompt)").size(11).style(|_theme: &iced::Theme| iced::widget::text::Style {
            color: Some(Color::from_rgb8(120, 120, 120)),
        }),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    // Protected tokens for recent messages
    // Range: 10k to 100k tokens, step by 5k
    let protected_tokens_k = compression.protected_tokens / 1000;
    let preserve_recent_row = row![
        text("Protected Tokens:").size(14).width(Length::Fixed(140.0)),
        iced::widget::slider(10..=100, protected_tokens_k, |v| {
            Message::Settings(settings::Msg::SetCompressionProtectedTokens(v * 1000))
        })
        .width(Length::Fixed(120.0)),
        text(format!("{}k tokens", protected_tokens_k)).size(12),
        text("(recent context budget)").size(11).style(|_theme: &iced::Theme| iced::widget::text::Style {
            color: Some(Color::from_rgb8(120, 120, 120)),
        }),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    container(
        column![
            text("Context Compression").size(18),
            text("Automatically compress context when approaching token limits.").size(12).style(|_theme: &iced::Theme| iced::widget::text::Style {
                color: Some(Color::from_rgb8(120, 120, 120)),
            }),
            enabled_row,
            threshold_row,
            strategy_row,
            strategy_desc_row,
            preserve_first_row,
            preserve_recent_row,
        ]
        .spacing(12),
    )
    .padding(20)
    .style(styles::card_container)
    .into()
}

/// Build the external tools management section
fn build_external_tools_section<'a>(
    external_tools: &'a HashMap<ExternalToolId, settings::ToolStatusInfo>,
) -> Element<'a, Message> {
    let header = row![
        text("External Tools").size(18),
        horizontal_space(),
        button(row![icon(icons::REFRESH).size(14), text(" Refresh").size(12),].spacing(4))
            .on_press(Message::Settings(settings::Msg::RefreshExternalTools))
            .style(styles::secondary_button)
            .padding([6, 10]),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    let description = text(
        "Download optional tools for advanced features like document conversion and JavaScript execution.",
    )
    .size(12)
    .style(|_theme: &iced::Theme| iced::widget::text::Style {
        color: Some(Color::from_rgb8(120, 120, 120)),
    });

    // Check if LibreOffice needs FUSE warning (installed but FUSE 2 missing)
    let libreoffice_installed = external_tools
        .get(&ExternalToolId::LibreOffice)
        .map(|s| s.is_installed)
        .unwrap_or(false);
    let needs_fuse_warning = libreoffice_installed && !is_fuse_available();

    // Build tool cards
    let tool_cards: Vec<Element<'a, Message>> =
        get_all_tool_definitions()
            .iter()
            .map(|def| {
                let tool_id = def.id;
                let status = external_tools.get(&tool_id);

                let status_text = if let Some(status) = status {
                    if status.is_installing {
                        format!("Installing... {}%", status.install_progress)
                    } else if status.is_installed {
                        format!("Installed v{}", status.version.as_deref().unwrap_or("?"))
                    } else if !status.is_supported {
                        "Unsupported Platform".to_string()
                    } else {
                        "Not Installed".to_string()
                    }
                } else {
                    "Unknown".to_string()
                };

                let is_installed = status.map(|s| s.is_installed).unwrap_or(false);
                let is_installing = status.map(|s| s.is_installing).unwrap_or(false);
                let is_supported = status.map(|s| s.is_supported).unwrap_or(true);

                let action_button: Element<'a, Message> = if is_installing {
                    text(format!(
                        "{}%",
                        status.map(|s| s.install_progress).unwrap_or(0)
                    ))
                    .size(12)
                    .into()
                } else if is_installed {
                    button(text("Uninstall").size(12))
                        .on_press(Message::Settings(settings::Msg::UninstallExternalTool(
                            tool_id,
                        )))
                        .style(styles::secondary_button)
                        .padding([4, 8])
                        .into()
                } else if is_supported {
                    button(text("Install").size(12))
                        .on_press(Message::Settings(settings::Msg::InstallExternalTool(
                            tool_id,
                        )))
                        .style(styles::primary_button)
                        .padding([4, 8])
                        .into()
                } else {
                    text("N/A").size(12).into()
                };

                let required_by = def.required_by.join(", ");

                container(
                    row![
                        column![
                            text(def.display_name).size(14),
                            text(def.description)
                                .size(11)
                                .style(|_theme: &iced::Theme| iced::widget::text::Style {
                                    color: Some(Color::from_rgb8(120, 120, 120)),
                                }),
                            text(format!(
                                "~{} MB • Required by: {}",
                                def.size_mb, required_by
                            ))
                            .size(10)
                            .style(|_theme: &iced::Theme| iced::widget::text::Style {
                                color: Some(Color::from_rgb8(100, 100, 100)),
                            }),
                        ]
                        .spacing(2)
                        .width(Length::Fill),
                        column![text(status_text).size(11), action_button,]
                            .spacing(4)
                            .align_x(iced::Alignment::End),
                    ]
                    .spacing(10)
                    .align_y(iced::Alignment::Center),
                )
                .padding(10)
                .width(Length::Fill)
                .into()
            })
            .collect();

    // FUSE warning banner (only shown when LibreOffice is installed but FUSE 2 is missing)
    let fuse_warning: Option<Element<'a, Message>> = if needs_fuse_warning {
        Some(
            container(
                column![
                    row![
                        icon(icons::WARNING).size(16),
                        text("LibreOffice requires FUSE 2").size(13),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                    text("AppImages need FUSE 2 (libfuse.so.2) to run. FUSE 3 is not compatible.")
                        .size(11)
                        .style(|_theme: &iced::Theme| iced::widget::text::Style {
                            color: Some(Color::from_rgb8(180, 140, 80)),
                        }),
                    text("Install: Arch: pacman -S fuse2 | Debian/Ubuntu: apt install libfuse2")
                        .size(10)
                        .style(|_theme: &iced::Theme| iced::widget::text::Style {
                            color: Some(Color::from_rgb8(150, 120, 70)),
                        }),
                    button(text("View installation guide →").size(10).style(
                        |_theme: &iced::Theme| iced::widget::text::Style {
                            color: Some(Color::from_rgb8(100, 160, 255)),
                        }
                    ))
                    .on_press(Message::Settings(settings::Msg::OpenUrl(
                        FUSE_DOCS_URL.to_string()
                    )))
                    .style(|_theme: &iced::Theme, _status| button::Style {
                        background: None,
                        text_color: Color::from_rgb8(100, 160, 255),
                        ..Default::default()
                    })
                    .padding(0),
                ]
                .spacing(4),
            )
            .padding(12)
            .width(Length::Fill)
            .style(|_theme: &iced::Theme| container::Style {
                background: Some(Color::from_rgb8(60, 50, 30).into()),
                border: Border {
                    color: Color::from_rgb8(180, 140, 50),
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            })
            .into(),
        )
    } else {
        None
    };

    let mut content = column![header, description,].spacing(12);

    if let Some(warning) = fuse_warning {
        content = content.push(warning);
    }

    content = content.push(Column::with_children(tool_cards).spacing(8));

    container(content)
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
