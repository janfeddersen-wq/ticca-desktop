//! Settings/Configuration view

use iced::widget::{Column, Space, button, column, container, pick_list, row, scrollable, text};
use iced::{Border, Color, Element, Length};

use std::collections::HashMap;

use crate::material_icons::{icon, icons};
use crate::messages::{chat, settings, Message, OAuthProvider, SettingsTab};
use crate::theme::{AppTheme, styles};

use ticca_core::agents::AgentType;
use ticca_core::config::OAuthAccount;
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
    recent_sessions: &'a [Session],
    active_tab: SettingsTab,
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

    let tabs = build_tabs(active_tab);

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
    let accounts_section =
        build_accounts_section(auth_status, claude_accounts, gemini_accounts, chatgpt_accounts);

    let content = match active_tab {
        SettingsTab::Accounts => accounts_section,
        SettingsTab::Models => model_settings,
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
                    ModelOption::Model(m) => Message::Settings(settings::Msg::SetAgentModel(
                        AgentType::Coding,
                        Some(m),
                    )),
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

fn build_tabs(active: SettingsTab) -> Element<'static, Message> {
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

    row![
        tab_button(SettingsTab::Accounts, "Accounts", icons::KEY),
        tab_button(SettingsTab::Models, "Models", icons::TUNE),
        tab_button(SettingsTab::Tools, "Tools & Safety", icons::SECURITY),
        tab_button(SettingsTab::Appearance, "Appearance", icons::BRIGHTNESS_6),
        tab_button(SettingsTab::Sessions, "Sessions", icons::FOLDER_OPEN),
    ]
    .spacing(8)
    .into()
}

fn build_accounts_section(
    auth_status: &ProviderAuthStatus,
    claude_accounts: &[OAuthAccount],
    gemini_accounts: &[OAuthAccount],
    chatgpt_accounts: &[OAuthAccount],
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
                                .on_press(Message::Settings(settings::Msg::ToggleOAuthAccountActive {
                                    account_id: account.id.clone(),
                                    is_active: !account.is_active,
                                }))
                                .style(styles::secondary_button)
                                .padding([6, 10]),
                                button(
                                    row![
                                        icon(icons::ARROW_BACK).size(14),
                                        text(" Priority -").size(12),
                                    ]
                                    .spacing(4)
                                )
                                .on_press(Message::Settings(settings::Msg::AdjustOAuthAccountPriority {
                                    account_id: account.id.clone(),
                                    delta: -1,
                                }))
                                .style(styles::secondary_button)
                                .padding([6, 10]),
                                button(
                                    row![
                                        icon(icons::ARROW_FORWARD).size(14),
                                        text(" Priority +").size(12),
                                    ]
                                    .spacing(4)
                                )
                                .on_press(Message::Settings(settings::Msg::AdjustOAuthAccountPriority {
                                    account_id: account.id.clone(),
                                    delta: 1,
                                }))
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

    container(
        column![
            text("Accounts").size(18),
            row![
                oauth_button(OAuthProvider::Claude, "Claude", auth_status.claude),
                oauth_button(OAuthProvider::Gemini, "Gemini", auth_status.gemini),
                oauth_button(OAuthProvider::ChatGpt, "ChatGPT", auth_status.chatgpt),
            ]
            .spacing(8),
            accounts_section("Claude Accounts", claude_accounts),
            accounts_section("Gemini Accounts", gemini_accounts),
            accounts_section("ChatGPT Accounts", chatgpt_accounts),
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

fn build_tools_section(yolo_mode_enabled: bool) -> Element<'static, Message> {
    let status_label = if yolo_mode_enabled {
        "On (no prompts)"
    } else {
        "Off (ask first)"
    };

    let status_button = if yolo_mode_enabled {
        button(text(status_label).size(12))
            .on_press(Message::Settings(settings::Msg::SetYoloMode(!yolo_mode_enabled)))
            .style(styles::success_button)
            .padding([6, 10])
    } else {
        button(text(status_label).size(12))
            .on_press(Message::Settings(settings::Msg::SetYoloMode(!yolo_mode_enabled)))
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
