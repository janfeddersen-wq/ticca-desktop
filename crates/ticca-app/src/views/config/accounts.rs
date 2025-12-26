//! Accounts section view - OAuth and API key management.

use std::collections::HashMap;

use iced::widget::{Column, button, column, container, row, text, text_input};
use iced::{Border, Color, Element, Length};

use crate::material_icons::{icon, icons};
use crate::messages::{Message, OAuthProvider, settings};
use crate::theme::styles;

use ticca_core::RegistryService;
use ticca_core::config::{ApiKeyAccount, OAuthAccount};

use super::types::{AccountsSectionParams, horizontal_space};

/// Build the accounts section with OAuth and API key providers
pub(super) fn build_accounts_section<'a>(
    params: AccountsSectionParams<'a>,
) -> Element<'a, Message> {
    let AccountsSectionParams {
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
    } = params;

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

    if ui_mode.shows_expert_ui() {
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
        add_provider_search,
        add_provider_expanded,
    );

    column![oauth_card, api_key_section].spacing(16).into()
}

/// Build the API key providers section
fn build_api_key_providers_section<'a>(
    api_key_accounts: &'a HashMap<String, Vec<ApiKeyAccount>>,
    form_provider: Option<&'a str>,
    form_value: &'a str,
    form_label: &'a str,
    search_text: &'a str,
    search_expanded: bool,
) -> Element<'a, Message> {
    // Get all API key providers from registry
    let all_providers = RegistryService::api_key_providers();

    // Providers with configured accounts (sorted by name)
    let mut configured_providers: Vec<_> = all_providers
        .iter()
        .filter(|p| api_key_accounts.contains_key(&p.id))
        .collect();
    configured_providers.sort_by(|a, b| a.name.cmp(&b.name));

    // Providers without configured accounts (for the search dropdown)
    let unconfigured_providers: Vec<_> = all_providers
        .iter()
        .filter(|p| !api_key_accounts.contains_key(&p.id))
        .collect();

    // Build the "Add Provider" search section
    let add_provider_section = {
        let search_input = text_input("Search providers to add...", search_text)
            .on_input(|v| Message::Settings(settings::Msg::AddProviderSearchChanged(v)))
            .width(Length::Fill);

        // Filter unconfigured providers by search text
        let search_lower = search_text.to_lowercase();
        let filtered_providers: Vec<_> = unconfigured_providers
            .iter()
            .filter(|p| {
                search_text.is_empty()
                    || p.name.to_lowercase().contains(&search_lower)
                    || p.id.to_lowercase().contains(&search_lower)
            })
            .take(8) // Limit suggestions
            .collect();

        // Build suggestion list if expanded and has results
        let suggestions: Element<'a, Message> =
            if search_expanded && !search_text.is_empty() && !filtered_providers.is_empty() {
                let suggestion_buttons: Vec<Element<'a, Message>> = filtered_providers
                    .into_iter()
                    .map(|provider| {
                        let provider_id = provider.id.clone();
                        button(
                            row![
                                text(&provider.name).size(13),
                                horizontal_space(),
                                text(&provider.id).size(11).style(|_theme: &iced::Theme| {
                                    iced::widget::text::Style {
                                        color: Some(Color::from_rgb8(120, 120, 120)),
                                    }
                                }),
                            ]
                            .width(Length::Fill),
                        )
                        .on_press(Message::Settings(settings::Msg::SelectProviderToAdd(
                            provider_id,
                        )))
                        .style(styles::secondary_button)
                        .padding([8, 12])
                        .width(Length::Fill)
                        .into()
                    })
                    .collect();

                container(Column::with_children(suggestion_buttons).spacing(2))
                    .padding(8)
                    .style(|theme: &iced::Theme| container::Style {
                        background: Some(theme.extended_palette().background.weak.color.into()),
                        border: Border {
                            color: Color::from_rgb8(60, 60, 60),
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    })
                    .into()
            } else if search_expanded && !search_text.is_empty() && filtered_providers.is_empty() {
                text("No matching providers found")
                    .size(12)
                    .style(|_theme: &iced::Theme| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(120, 120, 120)),
                    })
                    .into()
            } else {
                column![].into()
            };

        column![
            row![icon(icons::ADD).size(16), text(" Add Provider").size(14),].spacing(4),
            search_input,
            suggestions,
        ]
        .spacing(8)
    };

    // Build form section if a provider is selected for adding
    let form_section: Option<Element<'a, Message>> = form_provider.and_then(|provider_id| {
        let provider = RegistryService::find_provider(provider_id)?;
        Some(
            container(
                column![
                    row![
                        text(format!("Add API Key for {}", provider.name)).size(14),
                        horizontal_space(),
                        button(
                            row![icon(icons::CLOSE).size(14), text(" Cancel").size(12),].spacing(4)
                        )
                        .on_press(Message::Settings(settings::Msg::CancelAddApiKey))
                        .style(styles::secondary_button)
                        .padding([6, 10]),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
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
                            .on_input(|v| Message::Settings(settings::Msg::ApiKeyLabelFormChanged(
                                v
                            )))
                            .width(Length::Fill),
                    ]
                    .spacing(10)
                    .align_y(iced::Alignment::Center),
                    row![
                        horizontal_space(),
                        button(
                            row![icon(icons::SAVE).size(14), text(" Save").size(12),].spacing(4)
                        )
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
    });

    // Build provider cards for configured providers
    let mut provider_cards: Vec<Element<'a, Message>> = Vec::new();

    for provider in configured_providers {
        let accounts = api_key_accounts
            .get(&provider.id)
            .cloned()
            .unwrap_or_default();
        let is_form_open = form_provider == Some(provider.id.as_str());

        // Provider header with Add button
        let add_button = if is_form_open {
            button(row![icon(icons::CLOSE).size(14), text(" Cancel").size(12),].spacing(4))
                .on_press(Message::Settings(settings::Msg::CancelAddApiKey))
                .style(styles::secondary_button)
                .padding([6, 10])
        } else {
            let provider_id = provider.id.clone();
            button(row![icon(icons::ADD).size(14), text(" Add").size(12),].spacing(4))
                .on_press(Message::Settings(settings::Msg::StartAddApiKeyById(
                    provider_id,
                )))
                .style(styles::success_button)
                .padding([6, 10])
        };

        let header = row![
            text(&provider.name).size(16),
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
                                .on_input(|v| Message::Settings(settings::Msg::ApiKeyFormChanged(
                                    v
                                )))
                                .width(Length::Fill)
                                .secure(true),
                        ]
                        .spacing(10)
                        .align_y(iced::Alignment::Center),
                        row![
                            text("Label:").size(13).width(Length::Fixed(80.0)),
                            text_input("Optional label...", form_label)
                                .on_input(|v| Message::Settings(
                                    settings::Msg::ApiKeyLabelFormChanged(v)
                                ))
                                .width(Length::Fill),
                        ]
                        .spacing(10)
                        .align_y(iced::Alignment::Center),
                        row![
                            horizontal_space(),
                            button(
                                row![icon(icons::SAVE).size(14), text(" Save").size(12),]
                                    .spacing(4)
                            )
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
                            button(
                                icon(if account.is_active {
                                    icons::CHECK_CIRCLE
                                } else {
                                    icons::CANCEL
                                })
                                .size(14)
                            )
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

        let accounts_list: Element<'a, Message> =
            Column::with_children(account_rows).spacing(4).into();

        // Combine header, form, and accounts list
        let mut card_children: Vec<Element<'a, Message>> = vec![header.into()];
        if let Some(form) = form_element {
            card_children.push(form);
        }
        card_children.push(accounts_list);

        let provider_card: Element<'a, Message> =
            container(Column::with_children(card_children).spacing(10))
                .padding(12)
                .style(styles::card_container)
                .into();

        provider_cards.push(provider_card);
    }

    // Build the main content
    let mut content_children: Vec<Element<'a, Message>> = vec![
        text("API Key Providers").size(18).into(),
        text("Configure API keys for direct API access. Multiple keys per provider enable automatic failover on rate limits.")
            .size(12)
            .style(|_theme: &iced::Theme| iced::widget::text::Style {
                color: Some(Color::from_rgb8(120, 120, 120)),
            })
            .into(),
        add_provider_section.into(),
    ];

    // Add form section if adding a new provider (not already configured)
    if let Some(form) = form_section {
        // Only show this form if the provider is not already in configured list
        if let Some(provider_id) = form_provider
            && !api_key_accounts.contains_key(provider_id)
        {
            content_children.push(form);
        }
    }

    // Add configured provider cards
    if !provider_cards.is_empty() {
        content_children.push(
            column![
                text("Configured Providers")
                    .size(14)
                    .style(|_theme: &iced::Theme| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(150, 150, 150)),
                    }),
            ]
            .into(),
        );
        content_children.push(Column::with_children(provider_cards).spacing(10).into());
    }

    container(Column::with_children(content_children).spacing(12))
        .padding(20)
        .style(styles::card_container)
        .into()
}
