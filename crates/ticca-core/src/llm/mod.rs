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

pub mod auth;
pub mod claude;
pub mod model_service;
pub mod provider_registry;
pub mod providers;

use crate::config::ConfigDatabase;
use crate::config::models::providers as provider_names;
use crate::llm::auth as account_auth;

// Re-export legacy Claude client for model fetching
pub use claude::ClaudeClient;

// Re-export OAuth providers for convenience
pub use model_service::ModelService;
pub use provider_registry::{ProviderCapabilities, ProviderId, ProviderInfo, ProviderRegistry};
pub use providers::{ChatGptOAuthClient, ClaudeOAuthClient, GeminiOAuthClient, OAuthProviderError};

/// Get a Claude client if we have valid credentials
pub fn get_claude_client() -> Option<ClaudeClient> {
    let token = account_auth::select_token(provider_names::CLAUDE)?;
    Some(ClaudeClient::new(token.access_token))
}

/// Get a Claude client with a specific model
pub fn get_claude_client_with_model(model: &str) -> Option<ClaudeClient> {
    let token = account_auth::select_token(provider_names::CLAUDE)?;
    Some(ClaudeClient::with_model(token.access_token, model))
}

/// Get a Claude client for a specific agent type (uses pinned model if set)
pub fn get_claude_client_for_agent(agent_type: &str) -> Option<ClaudeClient> {
    let db = ConfigDatabase::open().ok()?;
    let token = account_auth::select_token(provider_names::CLAUDE)?;

    // Check for pinned model
    let model = db.get_agent_pinned_model(agent_type).ok().flatten();

    match model {
        Some(m) => Some(ClaudeClient::with_model(token.access_token, m)),
        None => Some(ClaudeClient::new(token.access_token)),
    }
}

/// Check if we have valid Claude credentials
pub fn has_claude_credentials() -> bool {
    account_auth::has_valid_account(provider_names::CLAUDE)
}

// =============================================================================
// OAuth Provider Helpers
// =============================================================================

/// Get a Claude OAuth client if we have valid credentials
pub fn get_claude_oauth_client() -> Option<ClaudeOAuthClient> {
    let token = account_auth::select_token(provider_names::CLAUDE)?;
    ClaudeOAuthClient::new(token.access_token).ok()
}

/// Get a ChatGPT OAuth client if we have valid credentials
pub fn get_chatgpt_oauth_client() -> Option<ChatGptOAuthClient> {
    let token = account_auth::select_token(provider_names::CHATGPT)?;
    let id_token = token.id_token?;
    ChatGptOAuthClient::from_tokens(token.access_token, &id_token).ok()
}

/// Get a Gemini OAuth client if we have valid credentials
pub fn get_gemini_oauth_client() -> Option<GeminiOAuthClient> {
    let token = account_auth::select_token(provider_names::GEMINI)?;
    GeminiOAuthClient::new(token.access_token).ok()
}
