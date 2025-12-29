//! Claude OAuth Provider
//!
//! Wraps serdesAI's ClaudeCodeOAuthModel with ticca-specific configuration.

use serdes_ai_models::claude_code_oauth::ClaudeCodeOAuthModel;

use super::common::{ProviderConfig, ProviderResult};

/// Claude API constants
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

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

/// Claude OAuth client wrapper around serdesAI's ClaudeCodeOAuthModel
pub struct ClaudeOAuthClient {
    /// The underlying serdesAI model
    inner: ClaudeCodeOAuthModel,
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
        let inner = ClaudeCodeOAuthModel::new(&config.model, &config.access_token);
        Ok(Self { inner, config })
    }

    /// Get the underlying serdesAI model
    pub fn model(&self) -> &ClaudeCodeOAuthModel {
        &self.inner
    }

    /// Get a mutable reference to the underlying model
    pub fn model_mut(&mut self) -> &mut ClaudeCodeOAuthModel {
        &mut self.inner
    }

    /// Consume self and return the underlying model
    pub fn into_model(self) -> ClaudeCodeOAuthModel {
        self.inner
    }

    /// Get the configured model name
    pub fn model_name(&self) -> &str {
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

/// Check if a model name is a Claude model
pub fn is_claude_model(model_name: &str) -> bool {
    model_name.starts_with("claude-") || model_name.contains("claude")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_config_builder() {
        let config = ClaudeConfig::new("test_token")
            .with_model("claude-3-5-sonnet-20241022")
            .with_max_tokens(4096)
            .with_temperature(0.5);

        assert_eq!(config.access_token, "test_token");
        assert_eq!(config.model, "claude-3-5-sonnet-20241022");
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.temperature, 0.5);
    }

    #[test]
    fn test_is_claude_model() {
        assert!(is_claude_model("claude-sonnet-4-20250514"));
        assert!(is_claude_model("claude-3-5-sonnet-20241022"));
        assert!(is_claude_model("claude-3-opus-20240229"));
        assert!(!is_claude_model("gpt-4o"));
        assert!(!is_claude_model("gemini-2.0-flash"));
    }
}
