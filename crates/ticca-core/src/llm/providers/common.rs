//! Common types and utilities for OAuth providers
//!
//! Shared infrastructure for all OAuth-based LLM provider implementations.

use thiserror::Error;

/// Errors that can occur with OAuth providers
#[derive(Debug, Error)]
pub enum OAuthProviderError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("Stream error: {0}")]
    StreamError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Configuration for an OAuth provider
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Base URL for the API
    pub base_url: String,

    /// Default model to use
    pub default_model: String,

    /// Maximum tokens for responses
    pub max_tokens: u32,

    /// Temperature for sampling
    pub temperature: f32,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            default_model: String::new(),
            max_tokens: 8192,
            temperature: 0.7,
        }
    }
}

/// Result type for OAuth provider operations
pub type ProviderResult<T> = Result<T, OAuthProviderError>;

/// Extract model ID from a full model path
///
/// Handles formats like:
/// - `claude-sonnet-4-20250514` -> as-is
/// - `models/gemini-2.0-flash` -> `gemini-2.0-flash`
pub fn normalize_model_id(model: &str) -> String {
    if let Some(stripped) = model.strip_prefix("models/") {
        stripped.to_string()
    } else {
        model.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_model_id() {
        assert_eq!(
            normalize_model_id("claude-sonnet-4-20250514"),
            "claude-sonnet-4-20250514"
        );
        assert_eq!(
            normalize_model_id("models/gemini-2.0-flash"),
            "gemini-2.0-flash"
        );
        assert_eq!(normalize_model_id("gpt-4o"), "gpt-4o");
    }
}
