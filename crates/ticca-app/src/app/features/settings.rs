use iced::Element;

use crate::messages::{settings, Message};
use crate::views::config;
use crate::views::config::ProviderAuthStatus;
use crate::messages::SettingsTab;
use ticca_core::config::{ConfigService, setting_keys};
use ticca_core::config::models::providers;
use ticca_core::llm::auth;
use ticca_core::llm::ProviderId;
use ticca_core::config::OAuthAccount;
use ticca_core::session::{Session, SessionService};

use super::super::{effects::Effect, TiccaApp, View};

pub(in crate::app) struct SettingsState {
    pub(in crate::app) settings_tab: SettingsTab,
    pub(in crate::app) provider_auth_status: ProviderAuthStatus,
    pub(in crate::app) accounts_claude: Vec<OAuthAccount>,
    pub(in crate::app) accounts_gemini: Vec<OAuthAccount>,
    pub(in crate::app) accounts_chatgpt: Vec<OAuthAccount>,
    pub(in crate::app) recent_sessions: Vec<Session>,
}

impl SettingsState {
    pub(in crate::app) fn new(provider_auth_status: ProviderAuthStatus) -> Self {
        Self {
            settings_tab: SettingsTab::Accounts,
            provider_auth_status,
            accounts_claude: Vec::new(),
            accounts_gemini: Vec::new(),
            accounts_chatgpt: Vec::new(),
            recent_sessions: Vec::new(),
        }
    }

    fn refresh_accounts(&mut self) {
        self.accounts_claude =
            ConfigService::list_oauth_accounts_pruned(providers::CLAUDE).unwrap_or_default();
        self.accounts_gemini =
            ConfigService::list_oauth_accounts_pruned(providers::GEMINI).unwrap_or_default();
        self.accounts_chatgpt =
            ConfigService::list_oauth_accounts_pruned(providers::CHATGPT).unwrap_or_default();
    }

    fn refresh_sessions(&mut self) {
        self.recent_sessions = SessionService::list_recent(10).unwrap_or_default();
    }
}

pub(in crate::app) fn check_provider_auth_status() -> ProviderAuthStatus {
    ProviderAuthStatus {
        claude: auth::has_valid_account(ticca_core::config::models::providers::CLAUDE),
        gemini: auth::has_valid_account(ticca_core::config::models::providers::GEMINI),
        chatgpt: auth::has_valid_account(ticca_core::config::models::providers::CHATGPT),
    }
}

pub(in crate::app) fn update(
    app: &mut TiccaApp,
    message: settings::Msg,
) -> Vec<Effect> {
    let mut effects = Vec::new();

    match message {
        settings::Msg::OpenSettings => {
            app.current_view = View::Settings;
            app.settings.refresh_accounts();
            app.settings.refresh_sessions();
        }
        settings::Msg::CloseSettings => {
            app.current_view = View::Chat;
        }
        settings::Msg::SwitchSettingsTab(tab) => {
            app.settings.settings_tab = tab;
            if tab == SettingsTab::Accounts {
                app.settings.refresh_accounts();
            }
            if tab == SettingsTab::Sessions {
                app.settings.refresh_sessions();
            }
        }
        settings::Msg::ThemeToggle => {
            app.theme = app.theme.next();
            let _ = ConfigService::set_setting(setting_keys::THEME, app.theme.as_str());
        }
        settings::Msg::SetTheme(theme) => {
            app.theme = theme;
            let _ = ConfigService::set_setting(setting_keys::THEME, app.theme.as_str());
        }
        settings::Msg::SetYoloMode(enabled) => {
            app.chat.yolo_mode_enabled = enabled;
            let value = if enabled { "true" } else { "false" };
            let _ = ConfigService::set_setting(setting_keys::YOLO_MODE, value);
        }
        settings::Msg::StartOAuth(provider) => {
            effects.push(Effect::StartOAuth(provider));
        }
        settings::Msg::OAuthComplete(provider, result) => match result {
            Ok(()) => {
                app.error_message = None;
                app.settings.provider_auth_status = check_provider_auth_status();
                app.settings.refresh_accounts();
                effects.push(Effect::RefreshModelsForProvider(match provider {
                    crate::messages::OAuthProvider::Claude => ProviderId::Claude,
                    crate::messages::OAuthProvider::Gemini => ProviderId::Gemini,
                    crate::messages::OAuthProvider::ChatGpt => ProviderId::ChatGpt,
                }));
            }
            Err(e) => {
                app.error_message = Some(e);
            }
        },
        settings::Msg::RemoveOAuthAccount(account_id) => {
            let _ = ConfigService::delete_oauth_account(&account_id);
            app.settings.provider_auth_status = check_provider_auth_status();
            app.settings.refresh_accounts();
        }
        settings::Msg::ToggleOAuthAccountActive {
            account_id,
            is_active,
        } => {
            let _ = ConfigService::set_oauth_account_active(&account_id, is_active);
            app.settings.provider_auth_status = check_provider_auth_status();
            app.settings.refresh_accounts();
        }
        settings::Msg::ResetOAuthCooldown(account_id) => {
            let _ = ConfigService::clear_oauth_account_cooldown(&account_id);
            app.settings.refresh_accounts();
        }
        settings::Msg::AdjustOAuthAccountPriority { account_id, delta } => {
            let _ = ConfigService::adjust_oauth_account_priority(&account_id, delta);
            app.settings.refresh_accounts();
        }
        settings::Msg::RefreshModels => {
            if app.chat.is_loading_models {
                return effects;
            }
            app.chat.is_loading_models = true;

            if !auth::has_any_valid_account() {
                app.chat.is_loading_models = false;
                return effects;
            }

            effects.push(Effect::RefreshModels);
        }
        settings::Msg::RefreshModelsForProvider(provider) => {
            if app.chat.is_loading_models {
                return effects;
            }
            app.chat.is_loading_models = true;

            let mapped = match provider {
                crate::messages::OAuthProvider::Claude => ProviderId::Claude,
                crate::messages::OAuthProvider::Gemini => ProviderId::Gemini,
                crate::messages::OAuthProvider::ChatGpt => ProviderId::ChatGpt,
            };
            effects.push(Effect::RefreshModelsForProvider(mapped));
        }
        settings::Msg::ModelsLoaded(result) => {
            app.chat.is_loading_models = false;
            match result {
                Ok(models) => {
                    tracing::info!("Loaded {} models: {:?}", models.len(), models);
                    app.chat.available_models = models;
                }
                Err(e) => {
                    tracing::error!("Failed to load models: {}", e);
                    app.error_message = Some(format!("Failed to load models: {}", e));
                }
            }
        }
        settings::Msg::SetDefaultModel(model_name) => {
            app.chat.default_model = Some(model_name.clone());
            let _ = ConfigService::set_setting(setting_keys::DEFAULT_MODEL, &model_name);
            tracing::info!("Set default model: {}", model_name);
        }
        settings::Msg::SetAgentModel(agent_type, model) => match &model {
            Some(model_name) => {
                app.chat
                    .agent_pinned_models
                    .insert(agent_type, model_name.clone());
                let _ = ConfigService::set_agent_pinned_model(agent_type.as_str(), Some(model_name));
                tracing::info!("Pinned {} to model: {}", agent_type.as_str(), model_name);
            }
            None => {
                app.chat.agent_pinned_models.remove(&agent_type);
                let _ = ConfigService::set_agent_pinned_model(agent_type.as_str(), None);
                tracing::info!("Cleared pinned model for {}", agent_type.as_str());
            }
        }
    }

    effects
}

pub(in crate::app) fn view(app: &TiccaApp) -> Element<'_, Message> {
    config::view(
        app.theme,
        &app.chat.available_models,
        app.chat.default_model.as_deref(),
        &app.chat.agent_pinned_models,
        app.chat.is_loading_models,
        &app.settings.provider_auth_status,
        &app.settings.accounts_claude,
        &app.settings.accounts_gemini,
        &app.settings.accounts_chatgpt,
        app.chat.yolo_mode_enabled,
        &app.settings.recent_sessions,
        app.settings.settings_tab,
    )
}
