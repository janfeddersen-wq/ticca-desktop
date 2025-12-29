//! ChatGPT OAuth Provider
//!
//! Wraps serdesAI's ChatGptOAuthModel with ticca-specific configuration.

use serdes_ai_models::chatgpt_oauth::ChatGptOAuthModel;

use super::common::{ProviderConfig, ProviderResult};

/// Default model
const DEFAULT_MODEL: &str = "chatgpt-4o-codex";

/// ChatGPT OAuth client configuration
#[derive(Debug, Clone)]
pub struct ChatGptConfig {
    /// OAuth access token
    pub access_token: String,
    /// Model to use
    pub model: String,
    /// Maximum tokens for response
    pub max_tokens: u32,
    /// Temperature for sampling
    pub temperature: f32,
}

impl ChatGptConfig {
    /// Create a new config with the given OAuth token
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            model: DEFAULT_MODEL.to_string(),
            max_tokens: 16384,
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

/// ChatGPT OAuth client wrapper around serdesAI's ChatGptOAuthModel
pub struct ChatGptOAuthClient {
    /// The underlying serdesAI model
    inner: ChatGptOAuthModel,
    /// Configuration
    config: ChatGptConfig,
}

impl ChatGptOAuthClient {
    /// Create a new ChatGPT OAuth client
    pub fn new(access_token: impl Into<String>) -> ProviderResult<Self> {
        Self::with_config(ChatGptConfig::new(access_token))
    }

    /// Create a new ChatGPT OAuth client with custom configuration
    pub fn with_config(config: ChatGptConfig) -> ProviderResult<Self> {
        let inner = ChatGptOAuthModel::new(&config.model, &config.access_token);
        Ok(Self { inner, config })
    }

    /// Get the underlying serdesAI model
    pub fn model(&self) -> &ChatGptOAuthModel {
        &self.inner
    }

    /// Get a mutable reference to the underlying model
    pub fn model_mut(&mut self) -> &mut ChatGptOAuthModel {
        &mut self.inner
    }

    /// Consume self and return the underlying model
    pub fn into_model(self) -> ChatGptOAuthModel {
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

impl From<ProviderConfig> for ChatGptConfig {
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

/// Check if a model name is a ChatGPT/Codex/GPT model
pub fn is_chatgpt_model(model_name: &str) -> bool {
    model_name.starts_with("chatgpt-") 
        || model_name.starts_with("gpt-") 
        || model_name.contains("codex")
        || model_name.starts_with("o1")
        || model_name.starts_with("o3")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chatgpt_config_builder() {
        let config = ChatGptConfig::new("test_token")
            .with_model("chatgpt-o3-codex")
            .with_max_tokens(8192)
            .with_temperature(0.5);

        assert_eq!(config.access_token, "test_token");
        assert_eq!(config.model, "chatgpt-o3-codex");
        assert_eq!(config.max_tokens, 8192);
        assert_eq!(config.temperature, 0.5);
    }

    #[test]
    fn test_is_chatgpt_model() {
        // ChatGPT-prefixed models
        assert!(is_chatgpt_model("chatgpt-4o-codex"));
        assert!(is_chatgpt_model("chatgpt-o1-codex"));
        assert!(is_chatgpt_model("chatgpt-o3-codex"));
        // GPT-prefixed models (OpenAI standard names)
        assert!(is_chatgpt_model("gpt-4o"));
        assert!(is_chatgpt_model("gpt-4o-mini"));
        assert!(is_chatgpt_model("gpt-3.5-turbo"));
        // o1/o3 reasoning models
        assert!(is_chatgpt_model("o1-preview"));
        assert!(is_chatgpt_model("o3-mini"));
        // Non-ChatGPT models
        assert!(!is_chatgpt_model("claude-sonnet-4"));
        assert!(!is_chatgpt_model("gemini-2.0-flash"));
    }
}
