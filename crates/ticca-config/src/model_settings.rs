//! Per-model configuration settings
//!
//! Provides model-specific configuration with:
//! - Model name sanitization for config keys
//! - Per-model temperature, sampling, and reasoning settings
//! - Automatic max_tokens calculation based on context length

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::settings::{ReasoningEffort, Verbosity};

/// Sanitize a model name for use as a configuration key.
///
/// Replaces `.`, `-`, and `/` with `_` and converts to lowercase.
///
/// # Examples
///
/// ```
/// use ticca_config::model_settings::sanitize_model_name;
///
/// assert_eq!(sanitize_model_name("gpt-4-turbo"), "gpt_4_turbo");
/// assert_eq!(sanitize_model_name("claude-3.5-sonnet"), "claude_3_5_sonnet");
/// assert_eq!(sanitize_model_name("meta/llama-3.1-70b"), "meta_llama_3_1_70b");
/// assert_eq!(sanitize_model_name("GPT-4"), "gpt_4");
/// ```
pub fn sanitize_model_name(name: &str) -> String {
    name.to_lowercase().replace(['.', '-', '/'], "_")
}

/// Calculate the recommended max_tokens for a model based on its context length.
///
/// Formula: max(2048, min(context_length * 0.15, 65536))
///
/// This ensures:
/// - At least 2048 tokens for small responses
/// - At most 65536 tokens (reasonable upper bound)
/// - 15% of context for output by default
///
/// # Examples
///
/// ```
/// use ticca_config::model_settings::calculate_max_tokens;
///
/// assert_eq!(calculate_max_tokens(8192), 2048);     // 8192 * 0.15 = 1228, clamped to 2048
/// assert_eq!(calculate_max_tokens(128000), 19200);  // 128000 * 0.15 = 19200
/// assert_eq!(calculate_max_tokens(1000000), 65536); // 1000000 * 0.15 = 150000, clamped to 65536
/// ```
pub fn calculate_max_tokens(context_length: u32) -> u32 {
    let calculated = (context_length as f64 * 0.15) as u32;
    calculated.clamp(2048, 65536)
}

/// Per-model configuration settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ModelSettings {
    /// Override temperature for this model (0.0-2.0).
    pub temperature: Option<f64>,

    /// Random seed for reproducible outputs.
    pub seed: Option<u64>,

    /// Top-p (nucleus) sampling parameter.
    pub top_p: Option<f64>,

    /// Reasoning effort level (for models that support it).
    pub reasoning_effort: Option<ReasoningEffort>,

    /// Response verbosity level.
    pub verbosity: Option<Verbosity>,

    /// Enable extended thinking mode (for models that support it).
    pub extended_thinking: Option<bool>,

    /// Token budget for extended thinking.
    pub budget_tokens: Option<u32>,

    /// Maximum tokens for response.
    /// If not set, calculated automatically from context_length.
    pub max_tokens: Option<u32>,

    /// Model's context window size (used for auto max_tokens calculation).
    pub context_length: Option<u32>,
}

impl ModelSettings {
    /// Create new empty model settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the effective max_tokens, calculating from context_length if needed.
    pub fn effective_max_tokens(&self) -> Option<u32> {
        self.max_tokens.or_else(|| {
            self.context_length.map(calculate_max_tokens)
        })
    }

    /// Merge another ModelSettings into this one.
    /// Values from `other` take precedence where set.
    pub fn merge(&mut self, other: &ModelSettings) {
        if other.temperature.is_some() {
            self.temperature = other.temperature;
        }
        if other.seed.is_some() {
            self.seed = other.seed;
        }
        if other.top_p.is_some() {
            self.top_p = other.top_p;
        }
        if other.reasoning_effort.is_some() {
            self.reasoning_effort = other.reasoning_effort;
        }
        if other.verbosity.is_some() {
            self.verbosity = other.verbosity;
        }
        if other.extended_thinking.is_some() {
            self.extended_thinking = other.extended_thinking;
        }
        if other.budget_tokens.is_some() {
            self.budget_tokens = other.budget_tokens;
        }
        if other.max_tokens.is_some() {
            self.max_tokens = other.max_tokens;
        }
        if other.context_length.is_some() {
            self.context_length = other.context_length;
        }
    }
}

/// Builder pattern for ModelSettings.
impl ModelSettings {
    pub fn with_temperature(mut self, temp: f64) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    pub fn with_top_p(mut self, top_p: f64) -> Self {
        self.top_p = Some(top_p);
        self
    }

    pub fn with_reasoning_effort(mut self, effort: ReasoningEffort) -> Self {
        self.reasoning_effort = Some(effort);
        self
    }

    pub fn with_verbosity(mut self, verbosity: Verbosity) -> Self {
        self.verbosity = Some(verbosity);
        self
    }

    pub fn with_extended_thinking(mut self, enabled: bool) -> Self {
        self.extended_thinking = Some(enabled);
        self
    }

    pub fn with_budget_tokens(mut self, tokens: u32) -> Self {
        self.budget_tokens = Some(tokens);
        self
    }

    pub fn with_max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    pub fn with_context_length(mut self, length: u32) -> Self {
        self.context_length = Some(length);
        self
    }
}

/// Registry of per-model settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ModelSettingsRegistry {
    /// Model-specific settings, keyed by sanitized model name.
    models: HashMap<String, ModelSettings>,
}

impl ModelSettingsRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get settings for a model by name.
    /// The name is automatically sanitized.
    pub fn get(&self, model_name: &str) -> Option<&ModelSettings> {
        let key = sanitize_model_name(model_name);
        self.models.get(&key)
    }

    /// Get mutable settings for a model by name.
    /// Creates default settings if not present.
    pub fn get_or_insert(&mut self, model_name: &str) -> &mut ModelSettings {
        let key = sanitize_model_name(model_name);
        self.models.entry(key).or_default()
    }

    /// Set settings for a model.
    pub fn set(&mut self, model_name: &str, settings: ModelSettings) {
        let key = sanitize_model_name(model_name);
        self.models.insert(key, settings);
    }

    /// Remove settings for a model.
    pub fn remove(&mut self, model_name: &str) -> Option<ModelSettings> {
        let key = sanitize_model_name(model_name);
        self.models.remove(&key)
    }

    /// List all configured model names (sanitized).
    pub fn model_names(&self) -> impl Iterator<Item = &str> {
        self.models.keys().map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_model_name() {
        assert_eq!(sanitize_model_name("gpt-4-turbo"), "gpt_4_turbo");
        assert_eq!(sanitize_model_name("claude-3.5-sonnet"), "claude_3_5_sonnet");
        assert_eq!(sanitize_model_name("meta/llama-3.1-70b"), "meta_llama_3_1_70b");
        assert_eq!(sanitize_model_name("GPT-4"), "gpt_4");
        assert_eq!(sanitize_model_name("already_sanitized"), "already_sanitized");
    }

    #[test]
    fn test_calculate_max_tokens() {
        // Small context: clamp to minimum
        assert_eq!(calculate_max_tokens(1000), 2048);
        assert_eq!(calculate_max_tokens(8192), 2048); // 8192 * 0.15 = 1228

        // Medium context: use 15%
        assert_eq!(calculate_max_tokens(32000), 4800); // 32000 * 0.15 = 4800
        assert_eq!(calculate_max_tokens(128000), 19200); // 128000 * 0.15 = 19200

        // Large context: clamp to maximum
        assert_eq!(calculate_max_tokens(500000), 65536); // 500000 * 0.15 = 75000
        assert_eq!(calculate_max_tokens(1000000), 65536);
    }

    #[test]
    fn test_model_settings_merge() {
        let mut base = ModelSettings::new()
            .with_temperature(0.7)
            .with_seed(42);

        let override_settings = ModelSettings::new()
            .with_temperature(0.9)
            .with_top_p(0.95);

        base.merge(&override_settings);

        assert_eq!(base.temperature, Some(0.9)); // Overridden
        assert_eq!(base.seed, Some(42)); // Kept
        assert_eq!(base.top_p, Some(0.95)); // Added
    }

    #[test]
    fn test_effective_max_tokens() {
        // Explicit max_tokens takes precedence
        let settings = ModelSettings::new()
            .with_max_tokens(4096)
            .with_context_length(128000);
        assert_eq!(settings.effective_max_tokens(), Some(4096));

        // Falls back to calculated from context_length
        let settings = ModelSettings::new()
            .with_context_length(128000);
        assert_eq!(settings.effective_max_tokens(), Some(19200));

        // None if neither is set
        let settings = ModelSettings::new();
        assert_eq!(settings.effective_max_tokens(), None);
    }

    #[test]
    fn test_registry() {
        let mut registry = ModelSettingsRegistry::new();

        // Set with unsanitized name
        registry.set("gpt-4-turbo", ModelSettings::new().with_temperature(0.7));

        // Get with same name
        assert!(registry.get("gpt-4-turbo").is_some());
        assert_eq!(registry.get("gpt-4-turbo").and_then(|s| s.temperature), Some(0.7));

        // Get with sanitized name also works
        assert!(registry.get("gpt_4_turbo").is_some());
    }
}
