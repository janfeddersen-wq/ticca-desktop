//! Settings/Configuration view

use iced::widget::{button, column, container, pick_list, row, scrollable, text, Column};
use iced::{Element, Length};
use std::collections::HashMap;

use crate::icons::{self, icon};
use crate::messages::{Message, OAuthProvider};
use crate::theme::{styles, AppTheme};

use ticca_core::agents::AgentType;
use ticca_core::session::SessionDatabase;

/// Render the settings view
pub fn view<'a>(
    theme: AppTheme,
    available_models: &'a [String],
    default_model: Option<&'a str>,
    agent_pinned_models: &'a HashMap<AgentType, String>,
    is_loading_models: bool,
) -> Element<'a, Message> {
    let is_dark = matches!(theme, AppTheme::Dark);
    let is_light = matches!(theme, AppTheme::Light);
    let is_zinc = matches!(theme, AppTheme::Zinc);

    let header = row![
        button(
            row![
                icon(icons::ARROW_LEFT).size(14),
                text(" Back").size(14),
            ]
            .spacing(4)
        )
        .on_press(Message::CloseSettings)
        .style(styles::secondary_button)
        .padding([8, 12]),
        text("Settings").size(24),
    ]
    .spacing(20)
    .padding(10)
    .align_y(iced::Alignment::Center);

    let appearance = container(
        column![
            text("Appearance").size(18),
            row![
                text("Theme:").size(14),
                button(
                    row![
                        icon(icons::MOON_FILL).size(14),
                        text(" Dark").size(14),
                    ]
                    .spacing(4)
                )
                .on_press(Message::SetTheme(AppTheme::Dark))
                .style(move |t, status| styles::tab_button(t, status, is_dark))
                .padding([8, 12]),
                button(
                    row![
                        icon(icons::SUN_FILL).size(14),
                        text(" Light").size(14),
                    ]
                    .spacing(4)
                )
                .on_press(Message::SetTheme(AppTheme::Light))
                .style(move |t, status| styles::tab_button(t, status, is_light))
                .padding([8, 12]),
                button(
                    row![
                        icon(icons::CIRCLE_HALF).size(14),
                        text(" Zinc").size(14),
                    ]
                    .spacing(4)
                )
                .on_press(Message::SetTheme(AppTheme::Zinc))
                .style(move |t, status| styles::tab_button(t, status, is_zinc))
                .padding([8, 12]),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(15)
    )
    .padding(20)
    .style(styles::card_container);

    let oauth_settings = container(
        column![
            text("Authentication").size(18),
            row![
                button(
                    row![
                        icon(icons::KEY_FILL).size(14),
                        text(" Claude").size(14),
                    ]
                    .spacing(4)
                )
                .on_press(Message::StartOAuth(OAuthProvider::Claude))
                .style(styles::secondary_button)
                .padding([8, 12]),
                button(
                    row![
                        icon(icons::KEY_FILL).size(14),
                        text(" Gemini").size(14),
                    ]
                    .spacing(4)
                )
                .on_press(Message::StartOAuth(OAuthProvider::Gemini))
                .style(styles::secondary_button)
                .padding([8, 12]),
                button(
                    row![
                        icon(icons::KEY_FILL).size(14),
                        text(" ChatGPT").size(14),
                    ]
                    .spacing(4)
                )
                .on_press(Message::StartOAuth(OAuthProvider::ChatGpt))
                .style(styles::secondary_button)
                .padding([8, 12]),
            ]
            .spacing(8),
        ]
        .spacing(15)
    )
    .padding(20)
    .style(styles::card_container);

    // Model selection section
    let model_settings = build_model_settings_section(
        available_models,
        default_model,
        agent_pinned_models,
        is_loading_models,
    );

    // Load recent sessions
    let sessions_section = build_sessions_section();

    scrollable(
        column![
            header,
            appearance,
            oauth_settings,
            model_settings,
            sessions_section,
        ]
        .spacing(16)
        .padding(10)
    )
    .into()
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
        iced::widget::horizontal_space(),
        button(
            row![
                icon(if is_loading_models { icons::HOURGLASS_SPLIT } else { icons::ARROW_CLOCKWISE }).size(14),
                text(if is_loading_models { " Loading..." } else { " Refresh" }).size(14),
            ]
            .spacing(4)
        )
        .on_press_maybe(if is_loading_models { None } else { Some(Message::RefreshModels) })
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
            pick_list(
                model_options.clone(),
                selected_default,
                |model| Message::SetDefaultModel(model)
            )
            .placeholder("Select default model...")
            .width(Length::Fixed(300.0)),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
    };

    // Agent model pinning section
    let agent_pinning_header = text("Agent Model Pinning").size(16);
    let agent_pinning_desc = text("Pin specific models to agents. If not pinned, the default model is used.").size(12);

    // Build agent pinning rows
    let coding_pinned = agent_pinned_models.get(&AgentType::Coding).cloned();
    let planning_pinned = agent_pinned_models.get(&AgentType::Planning).cloned();

    // Create options with "Use Default" at the start
    let agent_model_options: Vec<ModelOption> = std::iter::once(ModelOption::UseDefault)
        .chain(available_models.iter().map(|m| ModelOption::Model(m.clone())))
        .collect();

    let coding_row = if available_models.is_empty() {
        row![
            icon(icons::CODE_SLASH).size(14),
            text("Coding Agent:").size(14).width(Length::Fixed(120.0)),
            text("No models available").size(14),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
    } else {
        let selected = coding_pinned.map(ModelOption::Model).unwrap_or(ModelOption::UseDefault);
        row![
            icon(icons::CODE_SLASH).size(14),
            text("Coding Agent:").size(14).width(Length::Fixed(120.0)),
            pick_list(
                agent_model_options.clone(),
                Some(selected),
                move |opt| {
                    match opt {
                        ModelOption::UseDefault => Message::SetAgentModel(AgentType::Coding, None),
                        ModelOption::Model(m) => Message::SetAgentModel(AgentType::Coding, Some(m)),
                    }
                }
            )
            .width(Length::Fixed(300.0)),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
    };

    let planning_row = if available_models.is_empty() {
        row![
            icon(icons::LIST_CHECK).size(14),
            text("Planning Agent:").size(14).width(Length::Fixed(120.0)),
            text("No models available").size(14),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
    } else {
        let selected = planning_pinned.map(ModelOption::Model).unwrap_or(ModelOption::UseDefault);
        row![
            icon(icons::LIST_CHECK).size(14),
            text("Planning Agent:").size(14).width(Length::Fixed(120.0)),
            pick_list(
                agent_model_options.clone(),
                Some(selected),
                move |opt| {
                    match opt {
                        ModelOption::UseDefault => Message::SetAgentModel(AgentType::Planning, None),
                        ModelOption::Model(m) => Message::SetAgentModel(AgentType::Planning, Some(m)),
                    }
                }
            )
            .width(Length::Fixed(300.0)),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
    };

    container(
        column![
            header_row,
            default_model_row,
            iced::widget::horizontal_rule(1),
            agent_pinning_header,
            agent_pinning_desc,
            coding_row,
            planning_row,
        ]
        .spacing(12)
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

/// Build the recent sessions section
fn build_sessions_section<'a>() -> Element<'a, Message> {
    let sessions = SessionDatabase::open()
        .ok()
        .and_then(|db| db.list_sessions().ok())
        .unwrap_or_default();
    
    let session_list: Vec<Element<'a, Message>> = if sessions.is_empty() {
        vec![text("No saved sessions yet.").size(14).into()]
    } else {
        sessions.into_iter()
            .take(10) // Show last 10 sessions
            .map(|session| {
                let session_id = session.id.clone();
                let session_name = session.name.clone();
                let agent_icon = if session.agent_type == "coding" {
                    icons::CODE_SLASH
                } else {
                    icons::LIST_CHECK
                };
                
                let updated = session.updated_at
                    .as_ref()
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.format("%m/%d %H:%M").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                
                let info_text = format!("{} messages • {}", session.message_count, updated);
                
                container(
                    row![
                        icon(agent_icon).size(14),
                        column![
                            text(session_name).size(14),
                            text(info_text).size(11),
                        ]
                        .spacing(2)
                        .width(Length::Fill),
                        button(
                            row![
                                icon(icons::FOLDER_OPEN).size(12),
                                text(" Load").size(12),
                            ]
                            .spacing(4)
                        )
                        .on_press(Message::LoadSession(session_id))
                        .style(styles::secondary_button)
                        .padding([6, 10]),
                    ]
                    .spacing(10)
                    .align_y(iced::Alignment::Center)
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
        .spacing(15)
    )
    .padding(20)
    .style(styles::card_container)
    .into()
}
