//! LLM integration module
//!
//! Provides clients for interacting with various LLM providers using OAuth tokens.

pub mod claude;

use crate::config::ConfigDatabase;
use crate::config::models::providers;

pub use claude::ClaudeClient;

/// Get a Claude client if we have valid credentials
pub fn get_claude_client() -> Option<ClaudeClient> {
    let db = ConfigDatabase::open().ok()?;
    let token = db.get_oauth_token(providers::CLAUDE).ok()??;

    // Check if token is expired
    if token.is_expired() {
        tracing::warn!("Claude OAuth token is expired, need to refresh or re-authenticate");
        return None;
    }

    Some(ClaudeClient::new(token.access_token))
}

/// Get a Claude client with a specific model
pub fn get_claude_client_with_model(model: &str) -> Option<ClaudeClient> {
    let db = ConfigDatabase::open().ok()?;
    let token = db.get_oauth_token(providers::CLAUDE).ok()??;

    // Check if token is expired
    if token.is_expired() {
        tracing::warn!("Claude OAuth token is expired, need to refresh or re-authenticate");
        return None;
    }

    Some(ClaudeClient::with_model(token.access_token, model))
}

/// Get a Claude client for a specific agent type (uses pinned model if set)
pub fn get_claude_client_for_agent(agent_type: &str) -> Option<ClaudeClient> {
    let db = ConfigDatabase::open().ok()?;
    let token = db.get_oauth_token(providers::CLAUDE).ok()??;

    // Check if token is expired
    if token.is_expired() {
        tracing::warn!("Claude OAuth token is expired, need to refresh or re-authenticate");
        return None;
    }

    // Check for pinned model
    let model = db.get_agent_pinned_model(agent_type).ok().flatten();

    match model {
        Some(m) => Some(ClaudeClient::with_model(token.access_token, m)),
        None => Some(ClaudeClient::new(token.access_token)),
    }
}

/// Check if we have valid Claude credentials
pub fn has_claude_credentials() -> bool {
    get_claude_client().is_some()
}
