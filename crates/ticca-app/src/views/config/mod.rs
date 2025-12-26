//! Settings/Configuration view
//!
//! This module is split into focused submodules:
//! - `types`: Common types and helpers
//! - `accounts`: OAuth and API key management
//! - `models`: Model settings and agent pinning
//! - `mcp`: MCP server configuration
//! - `tools`: Tools & safety settings
//! - `sessions`: Recent sessions list

mod accounts;
mod mcp;
mod models;
mod sessions;
mod tools;
mod types;

use std::collections::HashMap;

use iced::widget::{button, column, container, pick_list, row, scrollable, text, text_editor};
use iced::Element;

use crate::material_icons::{icon, icons};
use crate::messages::{Message, SettingsTab, settings};
use crate::theme::{AppTheme, styles};

use ticca_core::agents::AgentType;
use ticca_core::config::{CompressionSettings, McpServer, OAuthAccount, ApiKeyAccount, UiMode};
use ticca_core::external_tools::ExternalToolId;
use ticca_core::session::Session;

// Re-export public types
pub use types::{McpServerFormState, ProviderAuthStatus, AccountsSectionParams};

/// Render the settings view
#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    theme: AppTheme,
    available_models: &'a [String],
    default_model: Option<&'a str>,
    agent_pinned_models: &'a HashMap<AgentType, String>,
    is_loading_models: bool,
    auth_status: &'a ProviderAuthStatus,
    claude_accounts: &'a [OAuthAccount],
    gemini_accounts: &'a [OAuthAccount],
    chatgpt_accounts: &'a [OAuthAccount],
    api_key_accounts: &'a HashMap<String, Vec<ApiKeyAccount>>,
    api_key_form_provider: Option<&'a str>,
    api_key_form_value: &'a str,
    api_key_form_label: &'a str,
    add_provider_search: &'a str,
    add_provider_expanded: bool,
    yolo_mode_enabled: bool,
    ui_mode: UiMode,
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

    let effective_tab = if !ui_mode.shows_expert_ui()
        && matches!(active_tab, SettingsTab::Models | SettingsTab::Agents)
    {
        SettingsTab::Accounts
    } else {
        active_tab
    };

    let tabs = build_tabs(effective_tab, ui_mode.shows_expert_ui());

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
                text("Theme:").size(14).width(iced::Length::Fixed(80.0)),
                pick_list(theme_options, Some(theme), |t| {
                    Message::Settings(settings::Msg::SetTheme(t))
                })
                .width(iced::Length::Fixed(200.0)),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
            row![
                icon(icons::TUNE).size(16),
                text("UI Mode:").size(14).width(iced::Length::Fixed(80.0)),
                pick_list(UiMode::all(), Some(ui_mode), |mode| Message::Settings(
                    settings::Msg::SetUiMode(mode)
                ))
                .width(iced::Length::Fixed(100.0)),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(15),
    )
    .padding(20)
    .style(styles::card_container)
    .into();

    let model_settings = models::build_model_settings_section(
        available_models,
        default_model,
        agent_pinned_models,
        is_loading_models,
    );
    let tools_section = tools::build_tools_section(yolo_mode_enabled, external_tools, compression);
    let sessions_section = sessions::build_sessions_section(recent_sessions);
    let accounts_section = accounts::build_accounts_section(AccountsSectionParams {
        auth_status,
        claude_accounts,
        gemini_accounts,
        chatgpt_accounts,
        api_key_accounts,
        api_key_form_provider,
        api_key_form_value,
        api_key_form_label,
        add_provider_search,
        add_provider_expanded,
        ui_mode,
    });
    let mcp_servers_section =
        mcp::build_mcp_servers_section(mcp_servers, mcp_form, mcp_import_json, theme);
    let agents_section = mcp::build_agent_mcp_section(mcp_servers, agent_mcp_server_ids);

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

/// Build the settings tab bar
fn build_tabs(active: SettingsTab, shows_expert_ui: bool) -> Element<'static, Message> {
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
    if shows_expert_ui {
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
