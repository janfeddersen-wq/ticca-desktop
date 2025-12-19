//! LLM integration module
//!
//! Provides clients for interacting with various LLM providers using OAuth tokens.
//!
//! # Architecture
//!
//! This module provides two layers:
//!
//! 1. **Legacy Claude Client** (`claude.rs`): Direct HTTP client for Claude API,
//!    used for model fetching and simple chat operations.
//!
//! 2. **OAuth Providers** (`providers/`): Rig-compatible OAuth wrappers that work
//!    with upstream rig's agent and streaming infrastructure. These are the preferred
//!    way to build agents with tool support.
//!
//! # Usage
//!
//! For simple model fetching:
//! ```ignore
//! let client = get_claude_client()?;
//! let models = client.fetch_latest_models().await?;
//! ```
//!
//! For building agents with tools:
//! ```ignore
//! use ticca_core::llm::providers::ClaudeOAuthClient;
//!
//! let client = ClaudeOAuthClient::new(oauth_token)?;
//! let agent = client.agent("claude-sonnet-4")
//!     .preamble("You are a helpful assistant")
//!     .tool(my_tool)
//!     .build();
//! ```

pub mod claude;
pub mod providers;

use crate::config::ConfigDatabase;
use crate::config::models::providers as provider_names;

// Re-export legacy Claude client for model fetching
pub use claude::ClaudeClient;

// Re-export OAuth providers for convenience
pub use providers::{
    ClaudeOAuthClient,
    ChatGptOAuthClient,
    GeminiOAuthClient,
    OAuthProviderError,
};

/// Get a Claude client if we have valid credentials
pub fn get_claude_client() -> Option<ClaudeClient> {
    let db = ConfigDatabase::open().ok()?;
    let token = db.get_oauth_token(provider_names::CLAUDE).ok()??;

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
    let token = db.get_oauth_token(provider_names::CLAUDE).ok()??;

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
    let token = db.get_oauth_token(provider_names::CLAUDE).ok()??;

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

// =============================================================================
// OAuth Provider Helpers
// =============================================================================

/// Get a Claude OAuth client if we have valid credentials
pub fn get_claude_oauth_client() -> Option<ClaudeOAuthClient> {
    let db = ConfigDatabase::open().ok()?;
    let token = db.get_oauth_token(provider_names::CLAUDE).ok()??;

    if token.is_expired() {
        tracing::warn!("Claude OAuth token is expired");
        return None;
    }

    ClaudeOAuthClient::new(token.access_token).ok()
}

/// Get a ChatGPT OAuth client if we have valid credentials
pub fn get_chatgpt_oauth_client() -> Option<ChatGptOAuthClient> {
    let db = ConfigDatabase::open().ok()?;
    let token = db.get_oauth_token(provider_names::CHATGPT).ok()??;

    if token.is_expired() {
        tracing::warn!("ChatGPT OAuth token is expired");
        return None;
    }

    // Extract id_token from extra_json
    let id_token = token.extra_json.as_ref().and_then(|json_str| {
        serde_json::from_str::<serde_json::Value>(json_str)
            .ok()
            .and_then(|v| v.get("id_token").and_then(|t| t.as_str()).map(|s| s.to_string()))
    })?;

    ChatGptOAuthClient::from_tokens(token.access_token, &id_token).ok()
}

/// Get a Gemini OAuth client if we have valid credentials
pub fn get_gemini_oauth_client() -> Option<GeminiOAuthClient> {
    let db = ConfigDatabase::open().ok()?;
    let token = db.get_oauth_token(provider_names::GEMINI).ok()??;

    if token.is_expired() {
        tracing::warn!("Gemini OAuth token is expired");
        return None;
    }

    GeminiOAuthClient::new(token.access_token).ok()
}
