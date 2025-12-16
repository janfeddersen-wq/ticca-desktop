//! API key management for AI providers
//!
//! Loads API keys from environment variables with support for:
//! - Multiple providers (OpenAI, Anthropic, Gemini, etc.)
//! - Custom endpoints with $ENV_VAR expansion
//! - Graceful handling of missing keys (warning, not error)

use std::collections::HashMap;
use std::env;
use tracing::warn;

/// Known API key environment variable names.
pub const OPENAI_API_KEY: &str = "OPENAI_API_KEY";
pub const ANTHROPIC_API_KEY: &str = "ANTHROPIC_API_KEY";
pub const GEMINI_API_KEY: &str = "GEMINI_API_KEY";
pub const CEREBRAS_API_KEY: &str = "CEREBRAS_API_KEY";
pub const OPENROUTER_API_KEY: &str = "OPENROUTER_API_KEY";
pub const ZAI_API_KEY: &str = "ZAI_API_KEY";
pub const AZURE_OPENAI_API_KEY: &str = "AZURE_OPENAI_API_KEY";
pub const AZURE_OPENAI_ENDPOINT: &str = "AZURE_OPENAI_ENDPOINT";
pub const CLAUDE_CODE_ACCESS_TOKEN: &str = "CLAUDE_CODE_ACCESS_TOKEN";

/// All known API key environment variables.
pub const ALL_API_KEYS: &[&str] = &[
    OPENAI_API_KEY,
    ANTHROPIC_API_KEY,
    GEMINI_API_KEY,
    CEREBRAS_API_KEY,
    OPENROUTER_API_KEY,
    ZAI_API_KEY,
    AZURE_OPENAI_API_KEY,
    AZURE_OPENAI_ENDPOINT,
    CLAUDE_CODE_ACCESS_TOKEN,
];

/// Provider identification for routing requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provider {
    OpenAi,
    Anthropic,
    Gemini,
    Cerebras,
    OpenRouter,
    Zai,
    AzureOpenAi,
    ClaudeCode,
}

impl Provider {
    /// Get the environment variable name for this provider's API key.
    pub fn env_var_name(&self) -> &'static str {
        match self {
            Self::OpenAi => OPENAI_API_KEY,
            Self::Anthropic => ANTHROPIC_API_KEY,
            Self::Gemini => GEMINI_API_KEY,
            Self::Cerebras => CEREBRAS_API_KEY,
            Self::OpenRouter => OPENROUTER_API_KEY,
            Self::Zai => ZAI_API_KEY,
            Self::AzureOpenAi => AZURE_OPENAI_API_KEY,
            Self::ClaudeCode => CLAUDE_CODE_ACCESS_TOKEN,
        }
    }

    /// All providers.
    pub fn all() -> &'static [Provider] {
        &[
            Self::OpenAi,
            Self::Anthropic,
            Self::Gemini,
            Self::Cerebras,
            Self::OpenRouter,
            Self::Zai,
            Self::AzureOpenAi,
            Self::ClaudeCode,
        ]
    }
}

/// Container for loaded API keys.
#[derive(Debug, Clone, Default)]
pub struct ApiKeys {
    keys: HashMap<String, String>,
}

impl ApiKeys {
    /// Create a new ApiKeys instance by loading from environment variables.
    ///
    /// Missing keys generate warnings but do not cause errors.
    pub fn from_env() -> Self {
        let mut keys = HashMap::new();

        for &var_name in ALL_API_KEYS {
            match env::var(var_name) {
                Ok(value) if !value.is_empty() => {
                    keys.insert(var_name.to_string(), value);
                }
                Ok(_) => {
                    warn!("Environment variable {} is set but empty", var_name);
                }
                Err(_) => {
                    // Missing keys are fine - user may not need all providers
                }
            }
        }

        Self { keys }
    }

    /// Get an API key by environment variable name.
    pub fn get(&self, key_name: &str) -> Option<&str> {
        self.keys.get(key_name).map(|s| s.as_str())
    }

    /// Get an API key for a specific provider.
    pub fn get_for_provider(&self, provider: Provider) -> Option<&str> {
        self.get(provider.env_var_name())
    }

    /// Check if a specific provider has an API key configured.
    pub fn has_provider(&self, provider: Provider) -> bool {
        self.get_for_provider(provider).is_some()
    }

    /// Get the Azure OpenAI endpoint if configured.
    pub fn azure_endpoint(&self) -> Option<&str> {
        self.get(AZURE_OPENAI_ENDPOINT)
    }

    /// List all configured providers.
    pub fn configured_providers(&self) -> Vec<Provider> {
        Provider::all()
            .iter()
            .filter(|p| self.has_provider(**p))
            .copied()
            .collect()
    }

    /// Check if any API keys are configured.
    pub fn has_any(&self) -> bool {
        !self.keys.is_empty()
    }

    /// Log warnings for missing keys that are commonly expected.
    pub fn warn_missing_common(&self) {
        let common = [Provider::OpenAi, Provider::Anthropic];
        for provider in common {
            if !self.has_provider(provider) {
                warn!(
                    "No API key found for {:?}. Set {} to enable.",
                    provider,
                    provider.env_var_name()
                );
            }
        }
    }
}

/// Expand environment variables in a string.
///
/// Replaces `${VAR_NAME}` with the value of the environment variable.
/// If the variable is not set, the placeholder is left unchanged.
///
/// # Examples
///
/// ```
/// use ticca_config::api_keys::expand_env_vars;
///
/// std::env::set_var("MY_VAR", "hello");
/// assert_eq!(expand_env_vars("prefix_$MY_VAR_suffix"), "prefix_$MY_VAR_suffix"); // No match (no braces)
/// assert_eq!(expand_env_vars("prefix_${MY_VAR}_suffix"), "prefix_hello_suffix");
/// ```
pub fn expand_env_vars(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' && chars.peek() == Some(&'{') {
            chars.next(); // consume '{'

            // Collect variable name until '}'
            let mut var_name = String::new();
            let mut found_close = false;

            for ch in chars.by_ref() {
                if ch == '}' {
                    found_close = true;
                    break;
                }
                var_name.push(ch);
            }

            if found_close {
                // Try to expand, or keep original if not set
                match env::var(&var_name) {
                    Ok(value) => result.push_str(&value),
                    Err(_) => {
                        result.push_str("${");
                        result.push_str(&var_name);
                        result.push('}');
                    }
                }
            } else {
                // Unclosed brace, keep as-is
                result.push_str("${");
                result.push_str(&var_name);
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Parse a custom endpoint URL, expanding any environment variables.
pub fn parse_endpoint(endpoint: &str) -> String {
    expand_env_vars(endpoint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_env_var_names() {
        assert_eq!(Provider::OpenAi.env_var_name(), "OPENAI_API_KEY");
        assert_eq!(Provider::Anthropic.env_var_name(), "ANTHROPIC_API_KEY");
        assert_eq!(Provider::AzureOpenAi.env_var_name(), "AZURE_OPENAI_API_KEY");
    }

    #[test]
    fn test_expand_env_vars_braced() {
        env::set_var("TEST_CONFIG_VAR", "test_value");
        assert_eq!(
            expand_env_vars("https://api.example.com/${TEST_CONFIG_VAR}/v1"),
            "https://api.example.com/test_value/v1"
        );
        env::remove_var("TEST_CONFIG_VAR");
    }

    #[test]
    fn test_expand_env_vars_missing() {
        let result = expand_env_vars("https://api.example.com/${NONEXISTENT_VAR}/v1");
        assert_eq!(result, "https://api.example.com/${NONEXISTENT_VAR}/v1");
    }

    #[test]
    fn test_api_keys_from_env() {
        // This test just ensures the function doesn't panic
        let keys = ApiKeys::from_env();
        // We can't assert much without controlling the environment
        let _ = keys.has_any();
    }

    #[test]
    fn test_configured_providers() {
        let keys = ApiKeys::default();
        assert!(keys.configured_providers().is_empty());
    }
}
