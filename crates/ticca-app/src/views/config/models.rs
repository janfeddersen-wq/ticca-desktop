//! Model settings section view.

use std::collections::HashMap;

use iced::widget::{Column, button, container, pick_list, row, text};
use iced::{Border, Color, Element, Length};

use crate::material_icons::{icon, icons};
use crate::messages::{Message, settings};
use crate::theme::styles;

use ticca_core::agents::{AgentRegistry, AgentType};

use super::types::{DisplayModel, ModelOption, horizontal_space};

/// Build the model settings section with default model and agent pinning
pub(super) fn build_model_settings_section<'a>(
    available_models: &'a [String],
    default_model: Option<&'a str>,
    agent_pinned_models: &'a HashMap<AgentType, String>,
    is_loading_models: bool,
) -> Element<'a, Message> {
    // Create options for pick_list with DisplayModel wrapper for nice formatting
    let model_options: Vec<DisplayModel> = available_models
        .iter()
        .map(|m| DisplayModel(m.clone()))
        .collect();

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
            text("No models available - authenticate with a provider first").size(14),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
    } else {
        let selected_default = default_model.map(|s| DisplayModel(s.to_string()));
        row![
            text("Default Model:").size(14).width(Length::Fixed(140.0)),
            pick_list(model_options.clone(), selected_default, |model| {
                Message::Settings(settings::Msg::SetDefaultModel(
                    model.canonical_id().to_string(),
                ))
            })
            .placeholder("Select default model...")
            .width(Length::Fill),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
    };

    // Agent model pinning section
    let agent_pinning_header = text("Agent Model Pinning").size(16);
    let agent_pinning_desc =
        text("Pin specific models to agents. If not pinned, the default model is used.").size(12);

    // Create options with "Use Default" at the start
    let agent_model_options: Vec<ModelOption> = std::iter::once(ModelOption::UseDefault)
        .chain(
            available_models
                .iter()
                .map(|m| ModelOption::Model(DisplayModel(m.clone()))),
        )
        .collect();

    // Build agent pinning rows dynamically for all agents
    let agent_pinning_rows: Vec<Element<'a, Message>> = AgentRegistry::all()
        .iter()
        .map(|&agent_type| {
            let metadata = AgentRegistry::get(agent_type);
            let pinned = agent_pinned_models.get(&agent_type).cloned();

            if available_models.is_empty() {
                row![
                    icon(metadata.icon).size(16),
                    text(format!("{}:", metadata.display_name))
                        .size(14)
                        .width(Length::Fixed(140.0)),
                    text("No models available").size(14),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center)
                .into()
            } else {
                let selected = pinned
                    .map(|m| ModelOption::Model(DisplayModel(m)))
                    .unwrap_or(ModelOption::UseDefault);
                let options = agent_model_options.clone();
                row![
                    icon(metadata.icon).size(16),
                    text(format!("{}:", metadata.display_name))
                        .size(14)
                        .width(Length::Fixed(140.0)),
                    pick_list(options, Some(selected), move |opt| {
                        match opt {
                            ModelOption::UseDefault => {
                                Message::Settings(settings::Msg::SetAgentModel(agent_type, None))
                            }
                            ModelOption::Model(m) => {
                                Message::Settings(settings::Msg::SetAgentModel(
                                    agent_type,
                                    Some(m.canonical_id().to_string()),
                                ))
                            }
                        }
                    })
                    .width(Length::Fill),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center)
                .into()
            }
        })
        .collect();

    // Custom horizontal rule using a styled container
    let rule = container(text(""))
        .width(Length::Fill)
        .height(1)
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(Color::from_rgba(0.5, 0.5, 0.5, 0.3).into()),
            border: Border::default(),
            ..Default::default()
        });

    // Build the column with dynamic agent rows
    let mut content = Column::new()
        .push(header_row)
        .push(default_model_row)
        .push(rule)
        .push(agent_pinning_header)
        .push(agent_pinning_desc)
        .spacing(12);

    for row in agent_pinning_rows {
        content = content.push(row);
    }

    container(content)
        .padding(20)
        .style(styles::card_container)
        .into()
}
