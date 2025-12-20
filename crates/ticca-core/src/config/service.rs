use anyhow::Result;
use uuid::Uuid;

use ticca_oauth::TokenResponse;

use crate::config::database::ConfigDatabase;
use crate::config::models::{OAuthAccount, OAuthToken};
use crate::config::TypedSettings;
use crate::config::models::providers;

pub struct ConfigService;

#[derive(Debug, Clone)]
pub struct SettingsSnapshot {
    pub settings: TypedSettings,
    pub agent_pinned_models: std::collections::HashMap<String, String>,
}

impl ConfigService {
    pub fn load_settings_snapshot() -> Result<SettingsSnapshot> {
        let db = ConfigDatabase::open()?;
        let settings = TypedSettings::load(&db);
        let agent_pinned_models = db.get_all_agent_pinned_models().unwrap_or_default();
        Ok(SettingsSnapshot {
            settings,
            agent_pinned_models,
        })
    }

    pub fn set_setting(key: &str, value: &str) -> Result<()> {
        let db = ConfigDatabase::open()?;
        db.set_setting(key, value)
    }

    pub fn set_agent_pinned_model(agent_type: &str, model_name: Option<&str>) -> Result<()> {
        let db = ConfigDatabase::open()?;
        match model_name {
            Some(model) => db.set_agent_pinned_model(agent_type, model),
            None => db.clear_agent_pinned_model(agent_type),
        }
    }

    pub fn delete_oauth_account(account_id: &str) -> Result<bool> {
        let db = ConfigDatabase::open()?;
        db.delete_oauth_account(account_id)
    }

    pub fn set_oauth_account_active(account_id: &str, is_active: bool) -> Result<bool> {
        let db = ConfigDatabase::open()?;
        db.set_oauth_account_active(account_id, is_active)
    }

    pub fn clear_oauth_account_cooldown(account_id: &str) -> Result<bool> {
        let db = ConfigDatabase::open()?;
        db.clear_oauth_account_cooldown(account_id)
    }

    pub fn adjust_oauth_account_priority(account_id: &str, delta: i64) -> Result<bool> {
        let db = ConfigDatabase::open()?;
        let Some(account) = db.get_oauth_account(account_id)? else {
            return Ok(false);
        };
        let new_priority = account.priority.saturating_add(delta);
        db.set_oauth_account_priority(account_id, new_priority)
    }

    pub fn list_oauth_accounts_pruned(provider: &str) -> Result<Vec<OAuthAccount>> {
        let db = ConfigDatabase::open()?;
        db.list_oauth_accounts_pruned(Some(provider))
    }

    pub fn persist_token_response(provider: &str, token_response: TokenResponse) -> Result<()> {
        let db = ConfigDatabase::open()?;

        let mut token = OAuthToken::new(provider, &token_response.access_token);
        if let Some(refresh_token) = token_response
            .refresh_token
            .clone()
            .filter(|t| !t.trim().is_empty())
        {
            token = token.with_refresh_token(refresh_token);
        }
        if let Some(expires_at) = token_response.expires_at_rfc3339() {
            token = token.with_expires_at(expires_at);
        }
        if let Some(scope) = token_response.scope.clone() {
            token = token.with_scope(scope);
        }
        if provider == providers::CHATGPT
            && let Some(id_token) = token_response.id_token()
        {
            let extra = serde_json::json!({ "id_token": id_token }).to_string();
            token = token.with_extra(extra);
        }
        let _ = db.upsert_oauth_token(&token);

        let mut account =
            OAuthAccount::new(Uuid::new_v4().to_string(), provider, &token_response.access_token);
        if let Some(refresh_token) = token_response
            .refresh_token
            .clone()
            .filter(|t| !t.trim().is_empty())
        {
            account = account.with_refresh_token(refresh_token);
        }
        if let Some(expires_at) = token_response.expires_at_rfc3339() {
            account = account.with_expires_at(expires_at);
        }
        if let Some(scope) = token_response.scope.clone() {
            account = account.with_scope(scope);
        }
        if provider == providers::CHATGPT
            && let Some(id_token) = token_response.id_token()
        {
            let extra = serde_json::json!({ "id_token": id_token }).to_string();
            account = account.with_extra(extra);
        }
        db.upsert_oauth_account(&account)?;

        Ok(())
    }
}
