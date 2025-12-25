//! OAuth account selection and cooldown handling

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::{ApiKeyAccount, OAuthAccount};
use crate::config::models::providers as provider_names;
use crate::config::{AccountRotationPolicy, ConfigDatabase, TypedSettings};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub account_id: String,
    pub provider: String,
    pub access_token: String,
    pub id_token: Option<String>,
    pub expires_at: Option<String>,
}

fn parse_time(value: &Option<String>) -> Option<DateTime<Utc>> {
    value
        .as_ref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

fn extract_id_token(extra_json: &Option<String>) -> Option<String> {
    extra_json.as_ref().and_then(|json_str| {
        serde_json::from_str::<serde_json::Value>(json_str)
            .ok()
            .and_then(|v| {
                v.get("id_token")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
            })
    })
}

pub fn list_accounts(provider: &str) -> Vec<OAuthAccount> {
    let db = match ConfigDatabase::open() {
        Ok(db) => db,
        Err(_) => return Vec::new(),
    };

    db.list_oauth_accounts_pruned(Some(provider))
        .unwrap_or_default()
}

pub fn select_token(provider: &str) -> Option<AuthToken> {
    let accounts = list_accounts(provider);

    let mut eligible: Vec<OAuthAccount> = accounts
        .into_iter()
        .filter(|a| a.is_active)
        .filter(|a| !a.is_expired())
        .filter(|a| !a.is_cooling())
        .collect();

    let rotation_policy = ConfigDatabase::open()
        .ok()
        .map(|db| TypedSettings::load(&db).account_rotation_policy)
        .unwrap_or(crate::config::defaults::ACCOUNT_ROTATION_POLICY);

    eligible.sort_by(|a, b| match rotation_policy {
        AccountRotationPolicy::PriorityThenLeastRecentlyUsed => {
            let priority_cmp = b.priority.cmp(&a.priority);
            if priority_cmp != std::cmp::Ordering::Equal {
                return priority_cmp;
            }
            let a_used = parse_time(&a.last_used_at);
            let b_used = parse_time(&b.last_used_at);
            a_used.cmp(&b_used)
        }
        AccountRotationPolicy::PriorityOnly => b.priority.cmp(&a.priority),
        AccountRotationPolicy::LeastRecentlyUsed => {
            let a_used = parse_time(&a.last_used_at);
            let b_used = parse_time(&b.last_used_at);
            a_used.cmp(&b_used)
        }
    });

    let account = eligible.into_iter().next()?;
    if let Ok(db) = ConfigDatabase::open() {
        let _ = db.set_oauth_account_last_used(&account.id, Utc::now().to_rfc3339());
    }

    Some(AuthToken {
        account_id: account.id,
        provider: account.provider,
        access_token: account.access_token,
        id_token: extract_id_token(&account.extra_json),
        expires_at: account.expires_at,
    })
}

pub fn has_valid_account(provider: &str) -> bool {
    select_token(provider).is_some()
}

pub fn has_any_valid_account() -> bool {
    has_valid_account(provider_names::CLAUDE)
        || has_valid_account(provider_names::GEMINI)
        || has_valid_account(provider_names::CHATGPT)
        || has_any_valid_api_key()
}

/// Check if any API key provider has a valid key configured
pub fn has_any_valid_api_key() -> bool {
    use crate::registry::RegistryService;
    RegistryService::api_key_providers()
        .iter()
        .any(|p| has_valid_api_key(&p.id))
}

/// Get a list of provider IDs that have valid authentication
pub fn available_provider_ids() -> Vec<String> {
    use crate::registry::RegistryService;

    let mut ids = Vec::new();

    // Check OAuth providers
    if has_valid_account(provider_names::CLAUDE) {
        ids.push(provider_names::CLAUDE.to_string());
    }
    if has_valid_account(provider_names::GEMINI) {
        ids.push(provider_names::GEMINI.to_string());
    }
    if has_valid_account(provider_names::CHATGPT) {
        ids.push(provider_names::CHATGPT.to_string());
    }

    // Check API key providers (using registry)
    for provider in RegistryService::api_key_providers() {
        if has_valid_api_key(&provider.id) {
            ids.push(provider.id.clone());
        }
    }

    ids
}

pub fn mark_cooldown(account_id: &str, reason: &str, cooldown_secs: i64) -> bool {
    let db = match ConfigDatabase::open() {
        Ok(db) => db,
        Err(_) => return false,
    };

    let cooldown_until = Utc::now() + chrono::Duration::seconds(cooldown_secs);
    let now_str = Utc::now().to_rfc3339();
    db.set_oauth_account_cooldown(
        account_id,
        Some(cooldown_until.to_rfc3339()),
        Some(reason.to_string()),
        Some(now_str),
    )
    .unwrap_or(false)
}

// ============================================================================
// API Key Account Selection
// ============================================================================

/// API key token for direct API access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyToken {
    pub account_id: String,
    pub provider: String,
    pub api_key: String,
}

/// List all API key accounts for a provider
pub fn list_api_key_accounts(provider: &str) -> Vec<ApiKeyAccount> {
    let db = match ConfigDatabase::open() {
        Ok(db) => db,
        Err(_) => return Vec::new(),
    };

    db.list_api_key_accounts(Some(provider)).unwrap_or_default()
}

/// Select the best available API key for a provider
pub fn select_api_key(provider: &str) -> Option<ApiKeyToken> {
    let accounts = list_api_key_accounts(provider);

    let mut eligible: Vec<ApiKeyAccount> = accounts
        .into_iter()
        .filter(|a| a.is_active)
        .filter(|a| !a.is_cooling())
        .collect();

    let rotation_policy = ConfigDatabase::open()
        .ok()
        .map(|db| TypedSettings::load(&db).account_rotation_policy)
        .unwrap_or(crate::config::defaults::ACCOUNT_ROTATION_POLICY);

    eligible.sort_by(|a, b| match rotation_policy {
        AccountRotationPolicy::PriorityThenLeastRecentlyUsed => {
            let priority_cmp = b.priority.cmp(&a.priority);
            if priority_cmp != std::cmp::Ordering::Equal {
                return priority_cmp;
            }
            let a_used = parse_time(&a.last_used_at);
            let b_used = parse_time(&b.last_used_at);
            a_used.cmp(&b_used)
        }
        AccountRotationPolicy::PriorityOnly => b.priority.cmp(&a.priority),
        AccountRotationPolicy::LeastRecentlyUsed => {
            let a_used = parse_time(&a.last_used_at);
            let b_used = parse_time(&b.last_used_at);
            a_used.cmp(&b_used)
        }
    });

    let account = eligible.into_iter().next()?;
    if let Ok(db) = ConfigDatabase::open() {
        let _ = db.set_api_key_account_last_used(&account.id, Utc::now().to_rfc3339());
    }

    Some(ApiKeyToken {
        account_id: account.id,
        provider: account.provider,
        api_key: account.api_key,
    })
}

/// Check if a provider has a valid API key configured
pub fn has_valid_api_key(provider: &str) -> bool {
    select_api_key(provider).is_some()
}

/// Mark an API key account as rate-limited with cooldown
pub fn mark_api_key_cooldown(account_id: &str, reason: &str, cooldown_secs: i64) -> bool {
    let db = match ConfigDatabase::open() {
        Ok(db) => db,
        Err(_) => return false,
    };

    let cooldown_until = Utc::now() + chrono::Duration::seconds(cooldown_secs);
    let now_str = Utc::now().to_rfc3339();
    db.set_api_key_account_cooldown(
        account_id,
        Some(cooldown_until.to_rfc3339()),
        Some(reason.to_string()),
        Some(now_str),
    )
    .unwrap_or(false)
}
