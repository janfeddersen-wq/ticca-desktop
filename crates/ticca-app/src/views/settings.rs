//! Settings View - Application settings panel
//!
//! Provides tabs for managing accounts, models, MCP servers, sessions, tools, and preferences.

use gpui::{
    div, prelude::FluentBuilder as _, px, AnyElement, Context, Entity, FontWeight,
    InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    Icon, IconName,
    input::{Input, InputState},
    select::{Select, SelectItem},
    tab::{Tab, TabBar},
    v_flex, ActiveTheme, Disableable as _, Sizable as _,
};

// =============================================================================
// Provider Select Item for searchable dropdown
// =============================================================================

/// Item for the provider select dropdown
#[derive(Debug, Clone)]
pub struct ProviderSelectItem {
    pub id: String,
    pub name: String,
}

impl SelectItem for ProviderSelectItem {
    type Value = String;

    fn title(&self) -> SharedString {
        SharedString::from(self.name.clone())
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }

    fn matches(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        self.name.to_lowercase().contains(&query_lower)
            || self.id.to_lowercase().contains(&query_lower)
    }
}

// =============================================================================
// Model Select Item for searchable dropdown
// =============================================================================

/// Item for the model select dropdown
#[derive(Debug, Clone)]
pub struct ModelSelectItem {
    /// Canonical model ID (e.g., "anthropic:claude-sonnet-4-20250514")
    pub id: String,
    /// Display name (e.g., "Claude Sonnet 4")
    pub display_name: String,
    /// Provider name (e.g., "Anthropic")
    pub provider: String,
}

impl SelectItem for ModelSelectItem {
    type Value = String;

    fn title(&self) -> SharedString {
        SharedString::from(format!("{} ({})", self.display_name, self.provider))
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }

    fn matches(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        self.display_name.to_lowercase().contains(&query_lower)
            || self.provider.to_lowercase().contains(&query_lower)
            || self.id.to_lowercase().contains(&query_lower)
    }
}

use crate::actions::*;
use crate::app::TiccaApp;
use crate::theme::ALL_THEMES;

// =============================================================================
// Main Settings View
// =============================================================================

/// Render the complete settings view
pub fn render_settings_view(
    app: &TiccaApp,
    window: &mut Window,
    cx: &mut Context<TiccaApp>,
) -> AnyElement {
    let theme = cx.theme();

    v_flex()
        .size_full()
        .bg(theme.background)
        .child(render_settings_header(cx))
        .child(render_settings_tabs(app, cx))
        .child(render_tab_content(app, window, cx))
        .into_any_element()
}

// =============================================================================
// Header
// =============================================================================

/// Render the settings header with title and close button
fn render_settings_header(cx: &mut Context<TiccaApp>) -> impl IntoElement {
    let theme = cx.theme();

    h_flex()
        .w_full()
        .h(px(52.))
        .px_4()
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(theme.border)
        .bg(theme.title_bar)
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .child("Settings"),
        )
        .child(
            Button::new("close-settings")
                .icon(IconName::Close)
                .ghost()
                .tooltip("Close Settings (Escape)")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.current_view = crate::actions::View::Chat;
                    cx.notify();
                })),
        )
}

// =============================================================================
// Tab Bar
// =============================================================================

/// Render the settings tab bar
fn render_settings_tabs(app: &TiccaApp, cx: &mut Context<TiccaApp>) -> impl IntoElement {
    let theme = cx.theme();
    let current_tab = app.settings.current_tab;

    // Map current tab to index
    let selected_index = match current_tab {
        SettingsTab::Accounts => 0,
        SettingsTab::Models => 1,
        SettingsTab::Agents => 2,
        SettingsTab::McpServers => 3,
        SettingsTab::Sessions => 4,
        SettingsTab::Tools => 5,
        SettingsTab::Appearance => 6,
    };

    div()
        .w_full()
        .px_4()
        .py_2()
        .border_b_1()
        .border_color(theme.border)
        .child(
            TabBar::new("settings-tabs")
                .underline()
                .selected_index(selected_index)
                .on_click(cx.listener(|this, idx: &usize, window, cx| {
                    let new_tab = match idx {
                        0 => SettingsTab::Accounts,
                        1 => SettingsTab::Models,
                        2 => SettingsTab::Agents,
                        3 => SettingsTab::McpServers,
                        4 => SettingsTab::Sessions,
                        5 => SettingsTab::Tools,
                        6 => SettingsTab::Appearance,
                        _ => SettingsTab::Accounts,
                    };
                    this.settings.current_tab = new_tab;
                    
                    // Refresh data when switching tabs
                    match new_tab {
                        SettingsTab::Accounts => {
                            this.settings.refresh_provider_select(window, cx);
                        }
                        SettingsTab::Models => {
                            this.settings.refresh_model_select(window, cx);
                        }
                        _ => {}
                    }
                    cx.notify();
                }))
                .child(Tab::new().label("Accounts").icon(IconName::User))
                .child(Tab::new().label("Models").icon(IconName::Bot))
                .child(Tab::new().label("Agents").icon(IconName::Cpu))
                .child(Tab::new().label("MCP").icon(IconName::Network))
                .child(Tab::new().label("Sessions").icon(IconName::Inbox))
                .child(Tab::new().label("Tools").icon(IconName::Settings2))
                .child(Tab::new().label("Appearance").icon(IconName::Palette)),
        )
}

// =============================================================================
// Tab Content
// =============================================================================

/// Render the content for the currently selected tab
fn render_tab_content(
    app: &TiccaApp,
    _window: &mut Window,
    cx: &mut Context<TiccaApp>,
) -> impl IntoElement {
    let tab = app.settings.current_tab;

    div()
        .id("settings-content")
        .flex_1()
        .overflow_y_scroll()
        .p_4()
        .child(match tab {
            SettingsTab::Accounts => render_accounts_tab(app, cx).into_any_element(),
            SettingsTab::Models => render_models_tab(app, cx).into_any_element(),
            SettingsTab::Agents => render_agents_tab(app, cx).into_any_element(),
            SettingsTab::McpServers => render_mcp_tab(app, cx).into_any_element(),
            SettingsTab::Sessions => render_sessions_tab(app, cx).into_any_element(),
            SettingsTab::Tools => render_tools_tab(app, cx).into_any_element(),
            SettingsTab::Appearance => render_appearance_tab(app, cx).into_any_element(),
        })
}

// =============================================================================
// Accounts Tab
// =============================================================================

/// Render the accounts/providers tab
fn render_accounts_tab(app: &TiccaApp, cx: &mut Context<TiccaApp>) -> impl IntoElement {
    let auth = app.settings.provider_auth_status.clone();
    let has_claude = !app.settings.accounts_claude.is_empty();
    let has_gemini = !app.settings.accounts_gemini.is_empty();
    let has_chatgpt = !app.settings.accounts_chatgpt.is_empty();
    
    // Clone data we need for display
    let claude_accounts = app.settings.accounts_claude.clone();
    let gemini_accounts = app.settings.accounts_gemini.clone();
    let chatgpt_accounts = app.settings.accounts_chatgpt.clone();
    let api_key_accounts = app.settings.api_key_accounts.clone();
    let api_key_form_provider = app.settings.api_key_form_provider.clone();
    
    // Clone input entities for closures
    let api_key_input = app.settings.api_key_input.clone();
    let api_key_label_input = app.settings.api_key_label_input.clone();
    
    let theme = cx.theme();

    v_flex()
        .gap_6()
        // OAuth Providers Section
        .child(
            v_flex()
                .gap_2()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("OAuth Providers"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("Connect AI providers using OAuth. Click to add accounts."),
                ),
        )
        .child(
            h_flex()
                .gap_2()
                .child(
                    Button::new("connect-claude")
                        .label("Add Claude")
                        .icon(if auth.claude { IconName::Check } else { IconName::Plus })
                        .when(auth.claude, |btn| btn.success())
                        .when(!auth.claude, |btn| btn.outline())
                        .on_click(cx.listener(|this, _, _, cx| {
                            start_oauth_flow("claude", this, cx);
                        })),
                )
                .child(
                    Button::new("connect-gemini")
                        .label("Add Gemini")
                        .icon(if auth.gemini { IconName::Check } else { IconName::Plus })
                        .when(auth.gemini, |btn| btn.success())
                        .when(!auth.gemini, |btn| btn.outline())
                        .on_click(cx.listener(|this, _, _, cx| {
                            start_oauth_flow("gemini", this, cx);
                        })),
                )
                .child(
                    Button::new("connect-chatgpt")
                        .label("Add ChatGPT")
                        .icon(if auth.chatgpt { IconName::Check } else { IconName::Plus })
                        .when(auth.chatgpt, |btn| btn.success())
                        .when(!auth.chatgpt, |btn| btn.outline())
                        .on_click(cx.listener(|this, _, _, cx| {
                            start_oauth_flow("chatgpt", this, cx);
                        })),
                ),
        )
        // OAuth Account Lists (if any) - simplified display
        .when(has_claude || has_gemini || has_chatgpt, |el| {
            el.child(
                v_flex()
                    .gap_2()
                    .p_3()
                    .rounded_md()
                    .bg(theme.muted)
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .child("Connected Accounts"),
                    )
                    .when(has_claude, |el| {
                        el.child(
                            div()
                                .text_sm()
                                .child(format!("Claude: {} account(s)", claude_accounts.len())),
                        )
                    })
                    .when(has_gemini, |el| {
                        el.child(
                            div()
                                .text_sm()
                                .child(format!("Gemini: {} account(s)", gemini_accounts.len())),
                        )
                    })
                    .when(has_chatgpt, |el| {
                        el.child(
                            div()
                                .text_sm()
                                .child(format!("ChatGPT: {} account(s)", chatgpt_accounts.len())),
                        )
                    }),
            )
        })
        // API Key Providers Section
        .child(
            v_flex()
                .gap_2()
                .mt_4()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("API Key Providers"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("Add API keys for providers like OpenRouter, Together, Ollama, etc."),
                ),
        )
        // Render API key section with form
        .child(render_api_key_form_section(
            app,
            &api_key_accounts,
            api_key_form_provider.as_deref(),
            &api_key_input,
            &api_key_label_input,
            cx,
        ))
}

/// API key providers section with form
fn render_api_key_form_section(
    app: &TiccaApp,
    api_key_accounts: &std::collections::HashMap<String, Vec<ticca_core::config::ApiKeyAccount>>,
    form_provider: Option<&str>,
    api_key_input: &Entity<InputState>,
    api_key_label_input: &Entity<InputState>,
    cx: &mut Context<TiccaApp>,
) -> impl IntoElement {
    let theme = cx.theme();
    
    // Get all API key providers from registry
    let all_providers = ticca_core::RegistryService::api_key_providers();
    
    // Separate configured providers (ones with at least one account)
    let configured: Vec<_> = all_providers.iter()
        .filter(|p| api_key_accounts.contains_key(&p.id))
        .collect();
    
    // Check if there are unconfigured providers available
    let has_unconfigured = all_providers.len() > configured.len();
    
    // Get the provider select entity (items are refreshed via refresh_provider_select)
    let provider_select = app.settings.provider_select.clone();
    
    // Clone for closures
    let api_key_input = api_key_input.clone();
    let api_key_label_input = api_key_label_input.clone();
    
    // Extract colors for use in closures
    let primary_color = theme.primary;
    let secondary_bg = theme.secondary;
    
    v_flex()
        .gap_4()
        // =====================================================================
        // Configured Providers Section - Show individual accounts with delete
        // =====================================================================
        .when(!configured.is_empty(), |el| {
            el.child(
                v_flex()
                    .gap_3()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .child("Configured API Keys"),
                    )
                    .children(configured.iter().map(|provider| {
                        let accounts = api_key_accounts.get(&provider.id)
                            .cloned()
                            .unwrap_or_default();
                        let provider_name = provider.name.clone();
                        let provider_id = provider.id.clone();
                        
                        v_flex()
                            .gap_2()
                            .p_3()
                            .rounded_md()
                            .bg(theme.muted)
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(provider_name),
                                    )
                                    .child({
                                        let provider_id = provider_id.clone();
                                        Button::new(SharedString::from(format!("add-more-{}", provider_id)))
                                            .icon(IconName::Plus)
                                            .ghost()
                                            .xsmall()
                                            .tooltip("Add another key")
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.settings.api_key_form_provider = Some(provider_id.clone());
                                                cx.notify();
                                            }))
                                    }),
                            )
                            .children(accounts.iter().map(|account| {
                                let account_id = account.id.clone();
                                let label = account.label.clone()
                                    .unwrap_or_else(|| "Unnamed".to_string());
                                // Mask the API key for display
                                let masked_key = mask_api_key(&account.api_key);
                                
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .py_1()
                                    .child(
                                        div()
                                            .flex_1()
                                            .child(
                                                h_flex()
                                                    .gap_2()
                                                    .items_center()
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .child(label),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(theme.muted_foreground)
                                                            .child(masked_key),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        Button::new(SharedString::from(format!("delete-api-key-{}", account_id)))
                                            .icon(IconName::Delete)
                                            .ghost()
                                            .xsmall()
                                            .tooltip("Delete this API key")
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                use ticca_core::config::ConfigService;
                                                match ConfigService::delete_api_key_account(&account_id) {
                                                    Ok(_) => {
                                                        tracing::info!("Deleted API key: {}", account_id);
                                                        this.settings.refresh_api_key_accounts();
                                                        this.settings.refresh_provider_select(window, cx);
                                                    }
                                                    Err(e) => {
                                                        tracing::error!("Failed to delete API key: {}", e);
                                                    }
                                                }
                                                cx.notify();
                                            })),
                                    )
                            }))
                    }))
            )
        })
        // =====================================================================
        // Add Provider Section - Searchable dropdown for unconfigured providers
        // =====================================================================
        .when(has_unconfigured, |el| {
            el.child(
                v_flex()
                    .gap_2()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(Icon::new(IconName::Plus).size_4())
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child("Add Provider"),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("Search and select a provider to add an API key:"),
                    )
                    // Searchable provider dropdown
                    .child(
                        Select::new(&provider_select)
                            .placeholder("Search providers...")
                            .small()
                            .w(px(280.))
                    )
            )
        })
        // =====================================================================
        // API Key Form - Shows when a provider is selected
        // =====================================================================
        .when(form_provider.is_some(), |el| {
            let provider_str = form_provider.unwrap();
            let provider_id = provider_str.to_string();
            let provider_name = ticca_core::RegistryService::find_provider(provider_str)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "Provider".to_string());
            
            el.child(
                v_flex()
                    .gap_3()
                    .p_4()
                    .rounded_md()
                    .border_1()
                    .border_color(primary_color)
                    .bg(secondary_bg)
                    .child(
                        h_flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(format!("Add API Key for {}", provider_name)),
                            )
                            .child(
                                Button::new("cancel-api-form")
                                    .icon(IconName::Close)
                                    .ghost()
                                    .xsmall()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.settings.api_key_form_provider = None;
                                        cx.notify();
                                    })),
                            ),
                    )
                    // API Key input
                    .child(
                        v_flex()
                            .gap_1()
                            .child(div().text_xs().child("API Key"))
                            .child(
                                Input::new(&api_key_input)
                                    .small()
                                    .cleanable(true),
                            ),
                    )
                    // Label input
                    .child(
                        v_flex()
                            .gap_1()
                            .child(div().text_xs().child("Label (optional)"))
                            .child(
                                Input::new(&api_key_label_input)
                                    .small()
                                    .cleanable(true),
                            ),
                    )
                    // Actions
                    .child(
                        h_flex()
                            .gap_2()
                            .child({
                                let provider_id = provider_id.clone();
                                Button::new("save-api-key")
                                    .label("Save Key")
                                    .primary()
                                    .small()
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        use ticca_core::config::ConfigService;
                                        use ticca_core::config::ApiKeyAccount;
                                        
                                        // Read values from input states
                                        let key = this.settings.api_key_input.read(cx).value().trim().to_string();
                                        let label = this.settings.api_key_label_input.read(cx).value().trim().to_string();
                                        
                                        if key.is_empty() {
                                            tracing::error!("API key cannot be empty");
                                            return;
                                        }
                                        
                                        let mut account = ApiKeyAccount::new(
                                            uuid::Uuid::new_v4().to_string(),
                                            provider_id.clone(),
                                            key,
                                        );
                                        
                                        if !label.is_empty() {
                                            account.label = Some(label);
                                        }
                                        
                                        match ConfigService::upsert_api_key_account(&account) {
                                            Ok(_) => {
                                                tracing::info!("API key saved for {}", provider_id);
                                                this.settings.api_key_form_provider = None;
                                                // Clear inputs
                                                this.settings.api_key_input.update(cx, |state, cx| state.set_value("", window, cx));
                                                this.settings.api_key_label_input.update(cx, |state, cx| state.set_value("", window, cx));
                                                this.settings.refresh_api_key_accounts();
                                                this.settings.refresh_provider_select(window, cx);
                                            }
                                            Err(e) => {
                                                tracing::error!("Failed to save API key: {}", e);
                                            }
                                        }
                                        cx.notify();
                                    }))
                            })
                            .child(
                                Button::new("cancel-key")
                                    .label("Cancel")
                                    .ghost()
                                    .small()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.settings.api_key_form_provider = None;
                                        cx.notify();
                                    })),
                            ),
                    )
            )
        })
}

/// Mask an API key for display (show first 4 and last 4 chars)
fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        "••••••••".to_string()
    } else {
        format!("{}••••{}", &key[..4], &key[key.len()-4..])
    }
}

#[allow(dead_code)]
/// Render a list of OAuth accounts for a provider
fn render_oauth_account_list(
    title: &str,
    accounts: &[ticca_core::config::OAuthAccount],
    cx: &mut Context<TiccaApp>,
) -> impl IntoElement {
    let theme = cx.theme();
    
    v_flex()
        .gap_2()
        .p_3()
        .rounded_md()
        .bg(theme.muted)
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .child(title.to_string()),
        )
        .children(accounts.iter().map(|account| {
            let account_id = account.id.clone();
            let label = account.label.clone()
                .unwrap_or_else(|| format!("{}...", &account.id[..8.min(account.id.len())]));
            let status = if !account.is_active {
                "inactive"
            } else if account.is_expired() {
                "expired"
            } else if account.is_cooling() {
                "cooldown"
            } else {
                "ready"
            };
            
            h_flex()
                .gap_2()
                .items_center()
                .py_1()
                .child(
                    div()
                        .flex_1()
                        .child(
                            h_flex()
                                .gap_2()
                                .child(div().text_sm().child(label))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(format!("priority {} • {}", account.priority, status)),
                                ),
                        ),
                )
                .child(
                    Button::new(format!("remove-{}", &account_id))
                        .icon(IconName::Delete)
                        .ghost()
                        .xsmall()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let _ = ticca_core::config::ConfigService::delete_oauth_account(&account_id);
                            this.settings.refresh_accounts();
                            cx.notify();
                        })),
                )
        }))
}

// =============================================================================
// Models Tab
// =============================================================================

/// Render the models tab
fn render_models_tab(app: &TiccaApp, cx: &mut Context<TiccaApp>) -> impl IntoElement {
    let theme = cx.theme();
    let available_models = app.settings.available_models.clone();
    let default_model = app.settings.default_model.clone();
    let is_loading = app.settings.is_loading_models;
    let model_select = app.settings.model_select.clone();
    let model_count = available_models.len();

    v_flex()
        .gap_6()
        // Header with title and refresh button
        .child(
            h_flex()
                .items_center()
                .justify_between()
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Model Settings"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("Configure default models for AI tasks."),
                        ),
                )
                .child(
                    Button::new("refresh-models")
                        .icon(if is_loading { IconName::Loader } else { IconName::Redo })
                        .label(if is_loading { "Fetching..." } else { "Refresh Models" })
                        .small()
                        .outline()
                        .disabled(is_loading)
                        .on_click(cx.listener(|this, _, window, cx| {
                            if this.settings.is_loading_models {
                                return;
                            }
                            
                            tracing::info!("Refresh models clicked - starting async fetch");
                            this.settings.is_loading_models = true;
                            cx.notify();
                            
                            // Spawn async task to fetch models
                            cx.spawn(async move |this: gpui::WeakEntity<TiccaApp>, cx| {
                                match ticca_core::llm::ModelService::fetch_all().await {
                                    Ok(models) => {
                                        tracing::info!("Fetched {} models", models.len());
                                        let _ = this.update(cx, |this, cx| {
                                            this.settings.available_models = models;
                                            this.settings.is_loading_models = false;
                                            // Reload from database to get full model info
                                            this.settings.load_cached_models();
                                            // Note: model_select will be refreshed on next settings open
                                            // or when Models tab is clicked
                                            cx.notify();
                                        });
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to fetch models: {}", e);
                                        let _ = this.update(cx, |this, cx| {
                                            this.settings.is_loading_models = false;
                                            cx.notify();
                                        });
                                    }
                                }
                            }).detach();
                        })),
                ),
        )
        // Default model selection with searchable dropdown
        .child(
            v_flex()
                .gap_3()
                .p_4()
                .rounded_md()
                .bg(theme.muted)
                .child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .child(Icon::new(IconName::Bot).size_4())
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .child("Default Model"),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child("This model will be used for new conversations."),
                )
                .when(model_count == 0, |el| {
                    el.child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .p_3()
                            .rounded_md()
                            .border_1()
                            .border_color(theme.border)
                            .child(Icon::new(IconName::Info).size_4().text_color(theme.muted_foreground))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("No models available. Connect a provider in the Accounts tab, then click 'Refresh Models'."),
                            ),
                    )
                })
                .when(model_count > 0, |el| {
                    el.child(
                        h_flex()
                            .gap_3()
                            .items_center()
                            .child(
                                Select::new(&model_select)
                                    .placeholder("Search and select a model...")
                                    .small()
                                    .w(px(400.))
                            )
                            .when(default_model.is_some(), |el| {
                                el.child(
                                    Button::new("clear-default")
                                        .icon(IconName::Close)
                                        .ghost()
                                        .xsmall()
                                        .tooltip("Clear default model")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            use ticca_core::config::{ConfigService, setting_keys};
                                            this.settings.default_model = None;
                                            let _ = ConfigService::delete_setting(setting_keys::DEFAULT_MODEL);
                                            cx.notify();
                                        }))
                                )
                            }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(format!("{} models available from connected providers", model_count)),
                    )
                }),
        )
        // Quick model info section
        .when(default_model.is_some(), |el| {
            let model_id = default_model.as_ref().unwrap();
            let parsed = ticca_core::llm::ModelId::parse(model_id);
            let display_name = parsed.as_ref()
                .map(|m| m.display_name())
                .unwrap_or_else(|| model_id.clone());
            let provider = parsed.as_ref()
                .map(|m| m.provider.to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            
            el.child(
                v_flex()
                    .gap_2()
                    .p_4()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.primary.opacity(0.3))
                    .bg(theme.primary.opacity(0.05))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(Icon::new(IconName::Check).size_4().text_color(theme.primary))
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child("Current Default"),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_4()
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(div().text_xs().text_color(theme.muted_foreground).child("Model"))
                                    .child(div().text_sm().child(display_name)),
                            )
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(div().text_xs().text_color(theme.muted_foreground).child("Provider"))
                                    .child(div().text_sm().child(provider)),
                            ),
                    ),
            )
        })
}
// =============================================================================
// Agents Tab
// =============================================================================

/// Render the agents tab
/// Render the pinned model section for a single agent
fn render_agent_pinned_model_section(
    agent_type: ticca_core::agents::AgentType,
    pinned_model: Option<&str>,
    available_models: &[String],
    primary_color: gpui::Hsla,
    muted_fg: gpui::Hsla,
    cx: &mut Context<TiccaApp>,
) -> impl IntoElement {
    let has_models = !available_models.is_empty();
    let agent = agent_type;
    
    // Get display info for pinned model
    let pinned_display = pinned_model.map(|id| {
        ticca_core::llm::ModelId::parse(id)
            .map(|m| (m.display_name(), m.provider.to_string()))
            .unwrap_or_else(|| (id.to_string(), "Unknown".to_string()))
    });
    
    v_flex()
        .gap_2()
        .child(
            h_flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .child("Pinned Model"),
                )
                .when(pinned_model.is_some(), |el| {
                    el.child(
                        Button::new(SharedString::from(format!("clear-pinned-{}", agent.as_str())))
                            .icon(IconName::Close)
                            .ghost()
                            .xsmall()
                            .tooltip("Clear pinned model (use default)")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                use ticca_core::config::ConfigService;
                                let _ = ConfigService::set_agent_pinned_model(agent.as_str(), None);
                                this.settings.agent_pinned_models.remove(&agent);
                                cx.notify();
                            }))
                    )
                }),
        )
        .when(pinned_model.is_some(), |el| {
            let (name, provider) = pinned_display.clone().unwrap();
            el.child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(primary_color.opacity(0.3))
                    .bg(primary_color.opacity(0.05))
                    .child(Icon::new(IconName::Check).size_4().text_color(primary_color))
                    .child(
                        div()
                            .text_xs()
                            .child(format!("{} ({})", name, provider)),
                    )
            )
        })
        .when(pinned_model.is_none(), |el| {
            el.child(
                div()
                    .text_xs()
                    .text_color(muted_fg)
                    .child("Using default model"),
            )
        })
        // Model selection buttons (show top 5 popular models)
        .when(has_models, |el| {
            // Show up to 5 models as quick-select buttons
            let models_to_show: Vec<_> = available_models.iter()
                .take(5)
                .cloned()
                .collect();
            
            el.child(
                h_flex()
                    .flex_wrap()
                    .gap_1()
                    .children(models_to_show.iter().map(|model_id| {
                        let id = model_id.clone();
                        let is_pinned = pinned_model == Some(model_id.as_str());
                        let display = ticca_core::llm::ModelId::parse(model_id)
                            .map(|m| m.model.clone())
                            .unwrap_or_else(|| model_id.clone());
                        // Truncate long names
                        let short_display = if display.len() > 20 {
                            format!("{}...", &display[..17])
                        } else {
                            display
                        };
                        
                        Button::new(SharedString::from(format!("pin-{}-{}", agent.as_str(), &id)))
                            .label(short_display)
                            .xsmall()
                            .when(is_pinned, |b| b.primary())
                            .when(!is_pinned, |b| b.ghost())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                use ticca_core::config::ConfigService;
                                let _ = ConfigService::set_agent_pinned_model(agent.as_str(), Some(&id));
                                this.settings.agent_pinned_models.insert(agent, id.clone());
                                cx.notify();
                            }))
                    }))
                    .when(available_models.len() > 5, |el| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(muted_fg)
                                .child(format!("...+{} more", available_models.len() - 5)),
                        )
                    })
            )
        })
        .when(!has_models, |el| {
            el.child(
                div()
                    .text_xs()
                    .text_color(muted_fg)
                    .child("No models available. Refresh models in the Models tab."),
            )
        })
}

fn render_agents_tab(app: &TiccaApp, cx: &mut Context<TiccaApp>) -> impl IntoElement {
    use ticca_core::agents::{AgentRegistry, AgentType};
    
    let theme = cx.theme();
    // Extract colors before the closures to avoid borrow issues
    let muted_bg = theme.muted;
    let muted_fg = theme.muted_foreground;
    let primary_color = theme.primary;
    
    let mcp_servers = app.settings.mcp_servers.clone();
    let agent_mcp_ids = app.settings.agent_mcp_server_ids.clone();
    let agent_pinned_models = app.settings.agent_pinned_models.clone();
    let available_models = app.settings.available_models.clone();

    v_flex()
        .gap_6()
        .child(
            v_flex()
                .gap_2()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Agent Configuration"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(muted_fg)
                        .child("Configure agent behavior, MCP servers, and tool permissions."),
                ),
        )
        // List all agents
        .children(AgentRegistry::all().iter().map(|&agent_type| {
            let metadata = AgentRegistry::get(agent_type);
            let agent_mcp = agent_mcp_ids.get(&agent_type).cloned().unwrap_or_default();
            let mcp_list = mcp_servers.clone();
            let pinned_model = agent_pinned_models.get(&agent_type).cloned();
            let models_for_agent = available_models.clone();
            
            let icon = match metadata.icon {
                "code" => IconName::SquareTerminal,
                "assignment" => IconName::BookOpen,
                "build" => IconName::Settings,
                "folder-open" => IconName::FolderOpen,
                _ => IconName::Bot,
            };
            
            v_flex()
                .gap_3()
                .p_4()
                .rounded_md()
                .bg(muted_bg)
                .child(
                    h_flex()
                        .gap_3()
                        .items_center()
                        .child(Icon::new(icon).size_5())
                        .child(
                            v_flex()
                                .flex_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(metadata.display_name.to_string()),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(muted_fg)
                                        .child(metadata.description.to_string()),
                                ),
                        ),
                )
                // Pinned Model section
                .child(
                    render_agent_pinned_model_section(
                        agent_type,
                        pinned_model.as_deref(),
                        &models_for_agent,
                        primary_color,
                        muted_fg,
                        cx,
                    )
                )
                // MCP Servers section
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::MEDIUM)
                                .child("MCP Servers"),
                        )
                        .when(mcp_list.is_empty(), |el| {
                            el.child(
                                div()
                                    .text_xs()
                                    .text_color(muted_fg)
                                    .child("No MCP servers configured. Add servers in the MCP tab."),
                            )
                        })
                        .when(!mcp_list.is_empty(), |el| {
                            el.children(mcp_list.iter().map(|server| {
                                let server_id = server.id.clone();
                                let server_id_toggle = server_id.clone();
                                let is_enabled = agent_mcp.contains(&server_id);
                                let agent = agent_type;
                                
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        Button::new(format!("mcp-{}-{}", metadata.id, &server_id_toggle))
                                            .icon(if is_enabled { IconName::Check } else { IconName::Plus })
                                            .xsmall()
                                            .when(is_enabled, |b| b.success())
                                            .when(!is_enabled, |b| b.ghost())
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                use ticca_core::config::ConfigService;
                                                let current = this.settings.agent_mcp_server_ids
                                                    .entry(agent)
                                                    .or_insert_with(Vec::new);
                                                
                                                if current.contains(&server_id_toggle) {
                                                    current.retain(|id| id != &server_id_toggle);
                                                } else {
                                                    current.push(server_id_toggle.clone());
                                                }
                                                
                                                // Save to config
                                                let _ = ConfigService::set_agent_mcp_server_ids(
                                                    agent.as_str(),
                                                    current,
                                                );
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .child(server.name.clone()),
                                    )
                                    .into_any_element()
                            }))
                        }),
                )
                // Tool Policy info
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::MEDIUM)
                                .child("Tool Policy:"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted_fg)
                                .child(format!("{:?}", metadata.tool_policy)),
                        ),
                )
                .into_any_element()
        }))
}

// =============================================================================
// MCP Servers Tab
// =============================================================================

/// Render the MCP servers tab
fn render_mcp_tab(app: &TiccaApp, cx: &mut Context<TiccaApp>) -> impl IntoElement {
    use ticca_core::config::McpTransport;
    
    let theme = cx.theme();
    let servers = app.settings.mcp_servers.clone();
    let form_editing = app.settings.mcp_form_server_id.is_some();
    let _form_name = app.settings.mcp_form_name.clone();
    let form_transport = app.settings.mcp_form_transport;
    let _form_command = app.settings.mcp_form_command.clone();
    let _form_url = app.settings.mcp_form_endpoint_url.clone();
    
    // Clone input entities for use in render
    let mcp_name_input = app.settings.mcp_name_input.clone();
    let mcp_command_input = app.settings.mcp_command_input.clone();
    let mcp_url_input = app.settings.mcp_url_input.clone();

    v_flex()
        .gap_6()
        .child(
            v_flex()
                .gap_2()
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("MCP Servers"),
                        )
                        .child(
                            Button::new("add-mcp")
                                .icon(IconName::Plus)
                                .label("Add Server")
                                .small()
                                .primary()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.settings.clear_mcp_form(window, cx);
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("Configure MCP servers for extended tool capabilities."),
                ),
        )
        // Server list
        .when(servers.is_empty(), |el| {
            el.child(
                div()
                    .p_4()
                    .rounded_md()
                    .bg(theme.muted)
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("No MCP servers configured. Click 'Add Server' to get started."),
                    ),
            )
        })
        .when(!servers.is_empty(), |el| {
            el.child(
                v_flex()
                    .gap_2()
                    .children(servers.into_iter().map(|server| {
                        let server_id = server.id.clone();
                        let server_id_edit = server_id.clone();
                        let server_id_del = server_id.clone();
                        let server_for_edit = server.clone();
                        
                        let transport_label = match server.transport {
                            McpTransport::Stdio => "stdio",
                            McpTransport::StreamableHttp => "http",
                        };
                        let detail = match server.transport {
                            McpTransport::Stdio => server.command.clone().unwrap_or_default(),
                            McpTransport::StreamableHttp => server.endpoint_url.clone().unwrap_or_default(),
                        };
                        
                        h_flex()
                            .gap_3()
                            .p_3()
                            .rounded_md()
                            .bg(theme.muted)
                            .items_center()
                            .child(
                                Icon::new(if server.is_enabled { IconName::Check } else { IconName::CircleX })
                                    .size_4()
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_1()
                                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(server.name.clone()))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(theme.muted_foreground)
                                            .child(format!("{} • {}", transport_label, detail)),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        Button::new(format!("edit-mcp-{}", &server_id_edit))
                                            .icon(IconName::Settings)
                                            .ghost()
                                            .small()
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.settings.edit_mcp_server(&server_for_edit, window, cx);
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new(format!("del-mcp-{}", &server_id_del))
                                            .icon(IconName::Delete)
                                            .ghost()
                                            .small()
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                use ticca_core::config::ConfigService;
                                                let _ = ConfigService::delete_mcp_server(&server_id_del);
                                                this.settings.refresh_mcp();
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .into_any_element()
                    })),
            )
        })
        // Add/Edit form with real inputs
        .child(
            v_flex()
                .gap_3()
                .p_4()
                .rounded_md()
                .border_1()
                .border_color(theme.border)
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .child(if form_editing { "Edit Server" } else { "New Server" }),
                )
                // Name input
                .child(
                    v_flex()
                        .gap_1()
                        .child(div().text_xs().child("Name"))
                        .child(
                            Input::new(&mcp_name_input)
                                .small()
                                .cleanable(true),
                        ),
                )
                // Transport selector
                .child(
                    h_flex()
                        .gap_2()
                        .child(div().text_xs().child("Transport:"))
                        .child(
                            Button::new("transport-stdio")
                                .label("Stdio")
                                .xsmall()
                                .when(form_transport == McpTransport::Stdio, |b| b.primary())
                                .when(form_transport != McpTransport::Stdio, |b| b.ghost())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.settings.mcp_form_transport = McpTransport::Stdio;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("transport-http")
                                .label("HTTP")
                                .xsmall()
                                .when(form_transport == McpTransport::StreamableHttp, |b| b.primary())
                                .when(form_transport != McpTransport::StreamableHttp, |b| b.ghost())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.settings.mcp_form_transport = McpTransport::StreamableHttp;
                                    cx.notify();
                                })),
                        ),
                )
                // Command input (for Stdio)
                .when(form_transport == McpTransport::Stdio, |el| {
                    el.child(
                        v_flex()
                            .gap_1()
                            .child(div().text_xs().child("Command"))
                            .child(
                                Input::new(&mcp_command_input)
                                    .small()
                                    .cleanable(true),
                            ),
                    )
                })
                // URL input (for HTTP)
                .when(form_transport == McpTransport::StreamableHttp, |el| {
                    el.child(
                        v_flex()
                            .gap_1()
                            .child(div().text_xs().child("Endpoint URL"))
                            .child(
                                Input::new(&mcp_url_input)
                                    .small()
                                    .cleanable(true),
                            ),
                    )
                })
                // Actions
                .child(
                    h_flex()
                        .gap_2()
                        .mt_2()
                        .child(
                            Button::new("save-mcp")
                                .label("Save")
                                .primary()
                                .small()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    match this.settings.save_mcp_form(window, cx) {
                                        Ok(()) => tracing::info!("MCP server saved"),
                                        Err(e) => tracing::error!("Failed to save MCP: {}", e),
                                    }
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("cancel-mcp")
                                .label("Clear")
                                .ghost()
                                .small()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.settings.clear_mcp_form(window, cx);
                                    cx.notify();
                                })),
                        ),
                ),
        )
}

// =============================================================================
// Sessions Tab
// =============================================================================

/// Render the sessions tab
fn render_sessions_tab(app: &TiccaApp, cx: &mut Context<TiccaApp>) -> impl IntoElement {
    let theme = cx.theme();
    let sessions = app.settings.recent_sessions.clone();

    v_flex()
        .gap_6()
        .child(
            v_flex()
                .gap_2()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Recent Sessions"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("Load previous conversations or start fresh."),
                ),
        )
        .when(sessions.is_empty(), |el| {
            el.child(
                div()
                    .p_4()
                    .rounded_md()
                    .bg(theme.muted)
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("No saved sessions yet. Start a conversation and it will appear here."),
                    ),
            )
        })
        .when(!sessions.is_empty(), |el| {
            el.child(
                v_flex()
                    .gap_2()
                    .children(sessions.into_iter().take(10).map(|session| {
                        let session_id = session.id.clone();
                        let session_id_del = session_id.clone();
                        let agent_icon = if session.agent_type == "coding" {
                            IconName::SquareTerminal
                        } else {
                            IconName::BookOpen
                        };
                        
                        let updated = session.updated_at
                            .as_ref()
                            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                            .map(|dt| dt.format("%m/%d %H:%M").to_string())
                            .unwrap_or_else(|| "Unknown".to_string());
                        
                        let info = format!("{} messages • {}", session.message_count, updated);
                        
                        h_flex()
                            .gap_3()
                            .p_3()
                            .rounded_md()
                            .bg(theme.muted)
                            .items_center()
                            .child(Icon::new(agent_icon).size_4())
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_1()
                                    .child(div().text_sm().child(session.name.clone()))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(theme.muted_foreground)
                                            .child(info),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        Button::new(format!("load-{}", &session_id))
                                            .icon(IconName::FolderOpen)
                                            .label("Load")
                                            .small()
                                            .outline()
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                tracing::info!("Load session: {}", session_id);
                                                // TODO: Load session into chat
                                                this.current_view = crate::actions::View::Chat;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new(format!("del-{}", &session_id_del))
                                            .icon(IconName::Delete)
                                            .ghost()
                                            .small()
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                // Delete session using database directly
                                                if let Ok(db) = ticca_core::session::SessionDatabase::open() {
                                                    let _ = db.delete_session(&session_id_del);
                                                }
                                                this.settings.refresh_sessions();
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .into_any_element()
                    })),
            )
        })
}

// =============================================================================
// Tools Tab
// =============================================================================

/// Render the tools tab
fn render_tools_tab(app: &TiccaApp, cx: &mut Context<TiccaApp>) -> impl IntoElement {
    use ticca_core::external_tools::get_all_tool_definitions;
    
    let theme = cx.theme();
    let yolo_mode = app.settings.yolo_mode;
    let external_tools = app.settings.external_tools.clone();

    v_flex()
        .gap_6()
        // YOLO Mode section
        .child(
            v_flex()
                .gap_3()
                .p_4()
                .rounded_md()
                .bg(theme.muted)
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Safety Settings"),
                )
                .child(
                    h_flex()
                        .gap_3()
                        .items_center()
                        .child(Icon::new(IconName::Check).size_4())
                        .child(
                            div()
                                .flex_1()
                                .child(
                                    v_flex()
                                        .child(div().text_sm().child("YOLO Mode"))
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(theme.muted_foreground)
                                                .child("When ON, tools execute without asking for approval."),
                                        ),
                                ),
                        )
                        .child(
                            Button::new("yolo-toggle")
                                .label(if yolo_mode { "ON (No prompts)" } else { "OFF (Ask first)" })
                                .when(yolo_mode, |b| b.success())
                                .when(!yolo_mode, |b| b.outline())
                                .small()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    use ticca_core::config::{ConfigService, setting_keys};
                                    this.settings.yolo_mode = !this.settings.yolo_mode;
                                    let val = if this.settings.yolo_mode { "true" } else { "false" };
                                    let _ = ConfigService::set_setting(setting_keys::YOLO_MODE, val);
                                    cx.notify();
                                })),
                        ),
                ),
        )
        // External Tools section
        .child(
            v_flex()
                .gap_3()
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("External Tools"),
                        )
                        .child(
                            Button::new("refresh-tools")
                                .icon(IconName::Redo)
                                .label("Refresh")
                                .small()
                                .ghost()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.settings.refresh_external_tools();
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("Optional tools for document conversion and advanced features."),
                )
                .children(get_all_tool_definitions().iter().map(|def| {
                    let tool_id = def.id;
                    let status = external_tools.get(&def.id).cloned().unwrap_or_default();
                    
                    let status_text = if status.is_installing {
                        format!("Installing... {}%", status.install_progress)
                    } else if status.is_installed {
                        format!("Installed v{}", status.version.as_deref().unwrap_or("?"))
                    } else if !status.is_supported {
                        "Unsupported Platform".to_string()
                    } else {
                        "Not Installed".to_string()
                    };
                    
                    let required_by = def.required_by.join(", ");
                    
                    h_flex()
                        .gap_3()
                        .p_3()
                        .rounded_md()
                        .bg(theme.muted)
                        .items_center()
                        .child(
                            v_flex()
                                .flex_1()
                                .gap_1()
                                .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(def.display_name.to_string()))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(def.description.to_string()),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(format!("~{} MB • Required by: {}", def.size_mb, required_by)),
                                ),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .items_end()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(if status.is_installed { theme.success } else { theme.muted_foreground })
                                        .child(status_text),
                                )
                                .child(
                                    if status.is_installing {
                                        div().text_xs().child(format!("{}%", status.install_progress)).into_any_element()
                                    } else if status.is_installed {
                                        Button::new(format!("uninstall-{:?}", tool_id))
                                            .label("Uninstall")
                                            .xsmall()
                                            .ghost()
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                start_tool_uninstall(tool_id, this, cx);
                                            }))
                                            .into_any_element()
                                    } else if status.is_supported {
                                        Button::new(format!("install-{:?}", tool_id))
                                            .label("Install")
                                            .xsmall()
                                            .primary()
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                start_tool_install(tool_id, this, cx);
                                            }))
                                            .into_any_element()
                                    } else {
                                        div().text_xs().child("N/A").into_any_element()
                                    }
                                ),
                        )
                        .into_any_element()
                })),
        )
}

// =============================================================================
// Appearance Tab (Theme Selection)
// =============================================================================

/// Render the appearance/preferences tab with theme selection
fn render_appearance_tab(app: &TiccaApp, cx: &mut Context<TiccaApp>) -> impl IntoElement {
    let theme = cx.theme();
    let current_theme = app.theme;

    v_flex()
        .gap_6()
        .child(
            v_flex()
                .gap_2()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Appearance"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("Customize the look and feel of the application."),
                ),
        )
        // Theme selection
        .child(
            v_flex()
                .gap_3()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .child("Theme"),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .children(ALL_THEMES.iter().map(|&t| {
                            let is_selected = t == current_theme;
                            let theme_name = t.theme_name();

                            Button::new(SharedString::from(format!("theme-{}", t.display_name())))
                                .label(t.display_name())
                                .small()
                                .when(is_selected, |btn| btn.primary())
                                .when(!is_selected, |btn| btn.outline())
                                .on_click(cx.listener(move |_, _, _, cx| {
                                    cx.dispatch_action(&SwitchTheme(theme_name.to_string()));
                                }))
                        })),
                ),
        )
        // Current theme info
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("Current theme:"),
                )
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .child(current_theme.display_name()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(if current_theme.is_dark() {
                            "(dark mode)"
                        } else {
                            "(light mode)"
                        }),
                ),
        )
}

// =============================================================================
// Placeholder Component
// =============================================================================

/// Render a "coming soon" placeholder for unimplemented tabs
fn render_coming_soon_placeholder(
    title: &str,
    description: &str,
    cx: &mut Context<TiccaApp>,
) -> impl IntoElement {
    let theme = cx.theme();

    v_flex()
        .p_8()
        .rounded_lg()
        .border_1()
        .border_color(theme.border)
        .bg(theme.secondary)
        .items_center()
        .gap_4()
        .child(
            div()
                .text_3xl()
                .child("🛠️"),
        )
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::MEDIUM)
                .child(format!("{} - Coming Soon", title)),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .text_center()
                .max_w(px(400.))
                .child(description.to_string()),
        )
}

// =============================================================================
// OAuth Flow Helper
// =============================================================================

/// Start an OAuth flow for the specified provider
fn start_oauth_flow(provider: &str, app: &mut TiccaApp, cx: &mut Context<TiccaApp>) {
    use std::thread;
    
    tracing::info!("Starting OAuth flow for {}", provider);
    
    let provider_owned = provider.to_string();
    
    // Spawn in background thread (OAuth uses blocking I/O)
    thread::spawn(move || {
        let result = match provider_owned.as_str() {
            "claude" => {
                use ticca_oauth::ClaudeOAuth;
                let oauth = ClaudeOAuth::new();
                match oauth.start_flow() {
                    Ok((flow_state, port)) => {
                        match oauth.build_auth_url(&flow_state) {
                            Ok(auth_url) => {
                                let _ = open::that(&auth_url);
                                tracing::info!("Opened Claude OAuth URL, waiting on port {}", port);
                                Ok(())
                            }
                            Err(e) => Err(format!("Failed to build auth URL: {}", e)),
                        }
                    }
                    Err(e) => Err(format!("Failed to start Claude OAuth: {}", e)),
                }
            }
            "gemini" => {
                use ticca_oauth::GeminiOAuth;
                let oauth = GeminiOAuth::new();
                match oauth.start_flow() {
                    Ok((flow_state, port)) => {
                        match oauth.build_auth_url(&flow_state) {
                            Ok(auth_url) => {
                                let _ = open::that(&auth_url);
                                tracing::info!("Opened Gemini OAuth URL, waiting on port {}", port);
                                Ok(())
                            }
                            Err(e) => Err(format!("Failed to build auth URL: {}", e)),
                        }
                    }
                    Err(e) => Err(format!("Failed to start Gemini OAuth: {}", e)),
                }
            }
            "chatgpt" => {
                use ticca_oauth::ChatGptOAuth;
                let oauth = ChatGptOAuth::new();
                match oauth.start_flow() {
                    Ok((flow_state, port)) => {
                        match oauth.build_auth_url(&flow_state) {
                            Ok(auth_url) => {
                                let _ = open::that(&auth_url);
                                tracing::info!("Opened ChatGPT OAuth URL, waiting on port {}", port);
                                Ok(())
                            }
                            Err(e) => Err(format!("Failed to build auth URL: {}", e)),
                        }
                    }
                    Err(e) => Err(format!("Failed to start ChatGPT OAuth: {}", e)),
                }
            }
            _ => Err(format!("Unknown provider: {}", provider_owned)),
        };
        
        if let Err(e) = result {
            tracing::error!("{}", e);
        }
    });
    
    // Refresh accounts after a delay (the OAuth callback will update the DB)
    // Note: In production, we'd use proper async/callback pattern
    app.settings.refresh_accounts();
    cx.notify();
}

// =============================================================================
// External Tool Install/Uninstall
// =============================================================================

use ticca_core::external_tools::ExternalToolId;

/// Start installing an external tool
fn start_tool_install(tool_id: ExternalToolId, app: &mut TiccaApp, cx: &mut Context<TiccaApp>) {
    use std::thread;
    use crate::app::ExternalToolStatus;
    
    tracing::info!("Starting install for {:?}", tool_id);
    
    // Mark as installing
    app.settings.external_tools.insert(tool_id, ExternalToolStatus {
        is_installed: false,
        is_installing: true,
        install_progress: 0,
        is_supported: true,
        version: None,
    });
    cx.notify();
    
    // Spawn background install
    thread::spawn(move || {
        use ticca_core::external_tools::ExternalToolManager;
        
        let rt = tokio::runtime::Runtime::new().ok();
        if let Some(rt) = rt {
            rt.block_on(async {
                match ExternalToolManager::new() {
                    Ok(manager) => {
                        match manager.install(tool_id, |_progress| {}).await {
                            Ok(_) => tracing::info!("{:?} installed successfully", tool_id),
                            Err(e) => tracing::error!("Failed to install {:?}: {}", tool_id, e),
                        }
                    }
                    Err(e) => tracing::error!("Failed to create tool manager: {}", e),
                }
            });
        }
    });
}

/// Start uninstalling an external tool
fn start_tool_uninstall(tool_id: ExternalToolId, app: &mut TiccaApp, cx: &mut Context<TiccaApp>) {
    use std::thread;
    use crate::app::ExternalToolStatus;
    
    tracing::info!("Starting uninstall for {:?}", tool_id);
    
    // Mark as installing (uninstalling)
    app.settings.external_tools.insert(tool_id, ExternalToolStatus {
        is_installed: true,
        is_installing: true,
        install_progress: 0,
        is_supported: true,
        version: None,
    });
    cx.notify();
    
    // Spawn background uninstall
    thread::spawn(move || {
        use ticca_core::external_tools::ExternalToolManager;
        
        let rt = tokio::runtime::Runtime::new().ok();
        if let Some(rt) = rt {
            rt.block_on(async {
                match ExternalToolManager::new() {
                    Ok(manager) => {
                        match manager.uninstall(tool_id).await {
                            Ok(_) => tracing::info!("{:?} uninstalled successfully", tool_id),
                            Err(e) => tracing::error!("Failed to uninstall {:?}: {}", tool_id, e),
                        }
                    }
                    Err(e) => tracing::error!("Failed to create tool manager: {}", e),
                }
            });
        }
    });
}