//! Claude OAuth Provider
//!
//! Wraps rig's Anthropic client with OAuth authentication support.
//! Uses custom headers required by Claude Code OAuth:
//!
//! - `Authorization: Bearer {oauth_token}`
//! - `anthropic-version: 2023-06-01`
//! - `anthropic-beta: oauth-2025-04-20,interleaved-thinking-2025-05-14`
//! - `x-app: cli`
//! - `User-Agent: claude-cli/2.0.61 (external, cli)`

use http::{HeaderMap, HeaderName, HeaderValue};
use rig::client::CompletionClient;
use rig::providers::anthropic;

use super::common::{OAuthProviderError, ProviderConfig, ProviderResult};
use super::http_wrapper::OAuthHttpClient;

/// Claude API constants
const CLAUDE_BASE_URL: &str = "https://api.anthropic.com";
const ANTHROPIC_BETA: &str = "oauth-2025-04-20,interleaved-thinking-2025-05-14";
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
const USER_AGENT: &str = "claude-cli/2.0.61 (external, cli)";

/// Type alias for the Claude client with our custom HTTP client
pub type ClaudeClient = anthropic::Client<OAuthHttpClient>;

/// Claude OAuth client configuration
#[derive(Debug, Clone)]
pub struct ClaudeConfig {
    /// OAuth access token
    pub access_token: String,

    /// Model to use
    pub model: String,

    /// Maximum tokens for response
    pub max_tokens: u32,

    /// Temperature for sampling
    pub temperature: f32,
}

impl ClaudeConfig {
    /// Create a new config with the given OAuth token
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            model: DEFAULT_MODEL.to_string(),
            max_tokens: 8192,
            temperature: 0.7,
        }
    }

    /// Set the model to use
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }
}

/// Claude OAuth client wrapper
///
/// This wraps rig's Anthropic client, adding the necessary OAuth headers.
///
/// # Usage
///
/// ```ignore
/// use rig::agent::AgentBuilder;
/// use rig::client::CompletionClient;
///
/// let client = ClaudeOAuthClient::new("oauth_token")?;
/// let model = client.completion_model("claude-sonnet-4-20250514");
/// let agent = AgentBuilder::new(model)
///     .preamble("You are a helpful assistant")
///     .build();
/// ```
pub struct ClaudeOAuthClient {
    /// The underlying rig Anthropic client with custom HTTP client
    inner: ClaudeClient,

    /// Configuration
    config: ClaudeConfig,
}

impl ClaudeOAuthClient {
    /// Create a new Claude OAuth client
    pub fn new(access_token: impl Into<String>) -> ProviderResult<Self> {
        Self::with_config(ClaudeConfig::new(access_token))
    }

    /// Create a new Claude OAuth client with custom configuration
    pub fn with_config(config: ClaudeConfig) -> ProviderResult<Self> {
        let headers = build_oauth_headers(&config.access_token)?;
        let http_client = OAuthHttpClient::new(headers);

        // Build the rig Anthropic client with custom HTTP client
        // The api_key is a placeholder - our OAuthHttpClient adds the real Authorization header
        // We need to set the http_client BEFORE calling build() to get the right types
        let inner: ClaudeClient = anthropic::Client::<OAuthHttpClient>::builder()
            .http_client(http_client)
            .api_key("oauth-placeholder") // Will be overridden by our headers
            .base_url(CLAUDE_BASE_URL)
            .anthropic_beta(ANTHROPIC_BETA)
            .build()
            .map_err(|e| OAuthProviderError::ConfigError(e.to_string()))?;

        Ok(Self { inner, config })
    }

    /// Get the underlying rig Anthropic client
    ///
    /// Use this to access rig's client methods.
    pub fn client(&self) -> &ClaudeClient {
        &self.inner
    }

    /// Get a completion model for the given model name
    ///
    /// Returns a model that can be used with `AgentBuilder::new(model)`.
    /// Prompt caching is enabled by default to reduce latency and costs.
    pub fn completion_model(
        &self,
        model: &str,
    ) -> <ClaudeClient as CompletionClient>::CompletionModel {
        self.inner.completion_model(model).with_prompt_caching()
    }

    /// Get the configured model name
    pub fn model(&self) -> &str {
        &self.config.model
    }

    /// Get the configured max tokens
    pub fn max_tokens(&self) -> u32 {
        self.config.max_tokens
    }

    /// Get the configured temperature
    pub fn temperature(&self) -> f32 {
        self.config.temperature
    }
}

/// Build the OAuth headers for Claude API
///
/// These headers are required for Claude Code OAuth to work properly.
fn build_oauth_headers(access_token: &str) -> ProviderResult<HeaderMap> {
    let mut headers = HeaderMap::new();

    // Authorization header with Bearer token (overrides x-api-key)
    headers.insert(
        http::header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", access_token))
            .map_err(|e| OAuthProviderError::ConfigError(format!("Invalid access token: {}", e)))?,
    );

    // Anthropic version (required for API)
    headers.insert(
        HeaderName::from_static("anthropic-version"),
        HeaderValue::from_static("2023-06-01"),
    );

    // Anthropic beta features (required for OAuth and interleaved thinking)
    headers.insert(
        HeaderName::from_static("anthropic-beta"),
        HeaderValue::from_static(ANTHROPIC_BETA),
    );

    // App identifier
    headers.insert(
        HeaderName::from_static("x-app"),
        HeaderValue::from_static("cli"),
    );

    // User agent
    headers.insert(
        http::header::USER_AGENT,
        HeaderValue::from_static(USER_AGENT),
    );

    tracing::debug!(
        "Built OAuth headers for Claude (token: {}... chars)",
        std::cmp::min(access_token.len(), 8)
    );

    Ok(headers)
}

impl From<ProviderConfig> for ClaudeConfig {
    fn from(config: ProviderConfig) -> Self {
        Self {
            access_token: String::new(), // Must be set separately
            model: if config.default_model.is_empty() {
                DEFAULT_MODEL.to_string()
            } else {
                config.default_model
            },
            max_tokens: config.max_tokens,
            temperature: config.temperature,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_config_builder() {
        let config = ClaudeConfig::new("test_token")
            .with_model("claude-3-haiku")
            .with_max_tokens(4096)
            .with_temperature(0.5);

        assert_eq!(config.access_token, "test_token");
        assert_eq!(config.model, "claude-3-haiku");
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.temperature, 0.5);
    }

    #[test]
    fn test_build_oauth_headers() {
        let headers = build_oauth_headers("test_token").unwrap();

        assert!(headers.get(http::header::AUTHORIZATION).is_some());
        assert_eq!(
            headers.get(http::header::AUTHORIZATION).unwrap(),
            "Bearer test_token"
        );
    }
}
