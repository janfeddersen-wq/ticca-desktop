//! Registry service for model and provider lookups
//!
//! Provides the main API for looking up model metadata,
//! context windows, and provider information.
//!
//! # Usage
//!
//! ```rust,ignore
//! use ticca_core::registry::RegistryService;
//!
//! // Get context window for a model
//! let ctx = RegistryService::get_context_window("gpt-4o");
//!
//! // Find a model by ID (supports multiple formats)
//! if let Some(model) = RegistryService::find_model("claude-3.5-sonnet") {
//!     println!("Context: {}", model.context_window);
//! }
//!
//! // Get all models for a provider
//! let anthropic_models = RegistryService::models_for_provider("anthropic");
//! ```

use super::{
    generated_context_overrides, generated_context_windows, generated_models, generated_providers,
    static_models, static_providers, AuthType, ModelDefinition, ProviderDefinition,
};
use std::collections::HashMap;
use std::sync::LazyLock;

/// Merge models from generated (API) and static (fallback) sources.
/// Generated data takes precedence for duplicates.
fn merged_models() -> Vec<ModelDefinition> {
    let mut seen = std::collections::HashSet::new();
    let mut models = Vec::new();

    // Add generated models first (they take precedence)
    for model in generated_models() {
        if seen.insert(model.id.clone()) {
            models.push(model);
        }
    }

    // Add static models that aren't already present
    for model in static_models() {
        if seen.insert(model.id.clone()) {
            models.push(model);
        }
    }

    models
}

/// Merge providers from generated (API) and static (fallback) sources.
/// Generated data takes precedence for duplicates.
fn merged_providers() -> Vec<ProviderDefinition> {
    let mut seen = std::collections::HashSet::new();
    let mut providers = Vec::new();

    // Add generated providers first (they take precedence)
    for provider in generated_providers() {
        if seen.insert(provider.id.clone()) {
            providers.push(provider);
        }
    }

    // Add static providers that aren't already present
    for provider in static_providers() {
        if seen.insert(provider.id.clone()) {
            providers.push(provider);
        }
    }

    providers
}

/// Pre-built lookup table for fast model access by ID
static MODEL_LOOKUP: LazyLock<HashMap<String, ModelDefinition>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for model in merged_models() {
        // Index by canonical ID (provider:model)
        map.insert(model.canonical_id(), model.clone());
        // Also index by just the model ID for convenience
        map.insert(model.id.clone(), model.clone());
        // Index by normalized ID (dots to dashes, lowercase)
        let normalized = model.id.to_lowercase().replace('.', "-");
        if normalized != model.id.to_lowercase() {
            map.insert(normalized, model);
        }
    }
    map
});

/// Pre-built lookup table for fast provider access by ID
static PROVIDER_LOOKUP: LazyLock<HashMap<String, ProviderDefinition>> = LazyLock::new(|| {
    merged_providers()
        .into_iter()
        .map(|p| (p.id.clone(), p))
        .collect()
});

/// Static storage for all models (for returning slices)
static ALL_MODELS: LazyLock<Vec<ModelDefinition>> = LazyLock::new(merged_models);

/// Static storage for all providers (for returning slices)
static ALL_PROVIDERS: LazyLock<Vec<ProviderDefinition>> = LazyLock::new(merged_providers);

/// Context window lookup from build-time data
static CONTEXT_WINDOWS: LazyLock<HashMap<&'static str, u64>> =
    LazyLock::new(generated_context_windows);

/// Context window overrides (provider:model or just model -> tokens)
/// These take precedence over registry data for specific configurations
static CONTEXT_OVERRIDES: LazyLock<HashMap<&'static str, u64>> =
    LazyLock::new(generated_context_overrides);

/// Default context window when model is unknown (100k is reasonable)
pub const DEFAULT_CONTEXT_WINDOW: u64 = 100_000;

/// Registry service for model and provider lookups.
///
/// This is the main entry point for all model metadata queries.
/// It replaces scattered hardcoded values throughout the codebase
/// with a single authoritative source.
///
/// All methods are static - no instantiation required.
pub struct RegistryService;

impl RegistryService {
    /// Get all statically defined models.
    ///
    /// Returns a slice of all models known at compile time.
    /// Runtime-discovered models are not included here.
    #[inline]
    pub fn all_models() -> &'static [ModelDefinition] {
        ALL_MODELS.as_slice()
    }

    /// Get all statically defined providers.
    ///
    /// Returns a slice of all providers known at compile time.
    #[inline]
    pub fn all_providers() -> &'static [ProviderDefinition] {
        ALL_PROVIDERS.as_slice()
    }

    /// Get all providers that support API key authentication.
    ///
    /// This excludes OAuth-only providers and returns providers
    /// with `AuthType::ApiKey` or `AuthType::Both`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// for provider in RegistryService::api_key_providers() {
    ///     println!("{}: {}", provider.name, provider.api_base_url);
    /// }
    /// ```
    pub fn api_key_providers() -> Vec<&'static ProviderDefinition> {
        ALL_PROVIDERS
            .iter()
            .filter(|p| matches!(p.auth_type, AuthType::ApiKey | AuthType::Both))
            .collect()
    }

    /// Look up a model by ID (canonical or just model name).
    ///
    /// Supports multiple formats:
    /// - Canonical: `"openai:gpt-4o"`
    /// - Model ID only: `"gpt-4o"`
    /// - With variations: `"claude-3.5-sonnet"` matches `"claude-3-5-sonnet"`
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // All of these work:
    /// RegistryService::find_model("gpt-4o");
    /// RegistryService::find_model("openai:gpt-4o");
    /// RegistryService::find_model("claude-3.5-sonnet");
    /// RegistryService::find_model("CLAUDE-3-5-SONNET"); // case-insensitive
    /// ```
    pub fn find_model(model_id: &str) -> Option<&'static ModelDefinition> {
        // Strip provider prefix if present (canonical format: "provider:model_id")
        let model_id = Self::strip_provider_prefix(model_id);

        // Try exact match first (fastest path)
        if let Some(model) = MODEL_LOOKUP.get(model_id) {
            return Some(model);
        }

        // Try lowercase match
        let lowercase = model_id.to_lowercase();
        if let Some(model) = MODEL_LOOKUP.get(&lowercase) {
            return Some(model);
        }

        // Try normalized match (replace . with -, lowercase)
        let normalized = lowercase.replace('.', "-");
        if let Some(model) = MODEL_LOOKUP.get(&normalized) {
            return Some(model);
        }

        // Try fuzzy match - find first model whose ID contains the search term
        // or the search term contains the model ID
        Self::fuzzy_find_model(model_id)
    }

    /// Strip provider prefix from canonical format model IDs.
    /// E.g., "claude:claude-sonnet-4-20250514" -> "claude-sonnet-4-20250514"
    #[inline]
    fn strip_provider_prefix(model_id: &str) -> &str {
        if let Some((_provider, model)) = model_id.split_once(':') {
            model
        } else {
            model_id
        }
    }

    /// Fuzzy find a model when exact/normalized matches fail.
    ///
    /// This handles cases like:
    /// - Partial matches: "sonnet" matches "claude-sonnet-4-20250514"
    /// - Substring matches: "gpt-4" matches "gpt-4o"
    fn fuzzy_find_model(search: &str) -> Option<&'static ModelDefinition> {
        let search_lower = search.to_lowercase();
        let search_normalized = search_lower.replace('.', "-");

        // Score models by match quality and return the best one
        ALL_MODELS
            .iter()
            .filter_map(|m| {
                let id_lower = m.id.to_lowercase();
                let id_normalized = id_lower.replace('.', "-");

                // Calculate a match score (higher = better)
                let score = if id_normalized == search_normalized {
                    100 // Exact match after normalization
                } else if id_normalized.contains(&search_normalized) {
                    // Search is substring of model ID
                    50 + (10 - (id_normalized.len() - search_normalized.len()).min(10)) as i32
                } else if search_normalized.contains(&id_normalized) {
                    // Model ID is substring of search
                    40
                } else {
                    return None;
                };

                Some((m, score))
            })
            .max_by_key(|(_, score)| *score)
            .map(|(m, _)| m)
    }

    /// Get the context window for a model.
    ///
    /// This is THE main function that replaces hardcoded lookups.
    /// Lookup priority:
    /// 1. Context overrides (provider:model_id or model_id)
    /// 2. Model registry lookup
    /// 3. Direct context window map
    /// 4. Default (100k)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// assert_eq!(RegistryService::get_context_window("claude-sonnet-4-20250514"), 200_000);
    /// assert_eq!(RegistryService::get_context_window("gpt-4o"), 128_000);
    /// assert_eq!(RegistryService::get_context_window("gemini-2.0-flash"), 1_000_000);
    /// // ChatGPT OAuth models are limited to 270k via overrides
    /// assert_eq!(RegistryService::get_context_window("chatgpt:gpt-4o"), 270_000);
    /// ```
    #[inline]
    pub fn get_context_window(model_id: &str) -> u64 {
        // First check overrides - this takes highest priority
        // Overrides can be "provider:model_id" or just "model_id"
        if let Some(&ctx) = CONTEXT_OVERRIDES.get(model_id) {
            return ctx;
        }

        // Check if there's a global override for just the model ID
        let model_only = Self::strip_provider_prefix(model_id);
        if model_only != model_id && let Some(&ctx) = CONTEXT_OVERRIDES.get(model_only) {
            return ctx;
        }

        // Then try to find the model in the registry
        if let Some(model) = Self::find_model(model_only) {
            return model.context_window;
        }

        // Fall back to the direct context window lookup (for models not in registry)
        if let Some(&ctx) = CONTEXT_WINDOWS.get(model_only) {
            return ctx;
        }

        // Try normalized lookup in context windows
        let normalized = model_only.to_lowercase().replace('.', "-");
        if let Some(&ctx) = CONTEXT_WINDOWS.get(normalized.as_str()) {
            return ctx;
        }

        DEFAULT_CONTEXT_WINDOW
    }

    /// Get the max output tokens for a model.
    ///
    /// Returns `None` if model is unknown or has no limit specified.
    #[inline]
    pub fn get_max_output_tokens(model_id: &str) -> Option<u64> {
        Self::find_model(model_id).and_then(|m| m.max_output_tokens)
    }

    /// Look up a provider by ID.
    ///
    /// Provider IDs are lowercase: "anthropic", "openai", "google", etc.
    #[inline]
    pub fn find_provider(provider_id: &str) -> Option<&'static ProviderDefinition> {
        PROVIDER_LOOKUP
            .get(provider_id)
            .or_else(|| PROVIDER_LOOKUP.get(&provider_id.to_lowercase()))
    }

    /// Get all models for a specific provider.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let claude_models = RegistryService::models_for_provider("anthropic");
    /// for model in claude_models {
    ///     println!("{}: {}k context", model.name, model.context_window / 1000);
    /// }
    /// ```
    pub fn models_for_provider(provider_id: &str) -> Vec<&'static ModelDefinition> {
        let provider_lower = provider_id.to_lowercase();
        ALL_MODELS
            .iter()
            .filter(|m| m.provider_id.to_lowercase() == provider_lower)
            .collect()
    }

    /// Check if a model ID matches a known model (for validation).
    #[inline]
    pub fn is_known_model(model_id: &str) -> bool {
        Self::find_model(model_id).is_some()
    }

    /// Check if a provider ID matches a known provider.
    #[inline]
    pub fn is_known_provider(provider_id: &str) -> bool {
        Self::find_provider(provider_id).is_some()
    }

    /// Get the provider for a model ID.
    ///
    /// Extracts provider from canonical ID or looks up from model definition.
    pub fn get_provider_for_model(model_id: &str) -> Option<&'static ProviderDefinition> {
        // Check if it's a canonical ID (provider:model)
        if let Some((provider_id, _)) = model_id.split_once(':') {
            return Self::find_provider(provider_id);
        }

        // Otherwise look up the model and get its provider
        Self::find_model(model_id).and_then(|m| Self::find_provider(&m.provider_id))
    }

    /// Parse a canonical model ID into (provider_id, model_id).
    ///
    /// Returns `None` if not in canonical format.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// assert_eq!(
    ///     RegistryService::parse_canonical_id("openai:gpt-4o"),
    ///     Some(("openai", "gpt-4o"))
    /// );
    /// assert_eq!(RegistryService::parse_canonical_id("gpt-4o"), None);
    /// ```
    pub fn parse_canonical_id(canonical_id: &str) -> Option<(&str, &str)> {
        canonical_id.split_once(':')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_models_not_empty() {
        let models = RegistryService::all_models();
        assert!(!models.is_empty());
        assert!(models.len() >= 10);
    }

    #[test]
    fn test_all_providers_not_empty() {
        let providers = RegistryService::all_providers();
        assert!(!providers.is_empty());
        assert!(providers.len() >= 5);
    }

    #[test]
    fn test_find_model_by_canonical_id() {
        let model = RegistryService::find_model("openai:gpt-4o");
        assert!(model.is_some());
        assert_eq!(model.unwrap().id, "gpt-4o");
    }

    #[test]
    fn test_find_model_by_id_only() {
        // gpt-4o exists in multiple providers in models.dev
        let model = RegistryService::find_model("gpt-4o");
        assert!(model.is_some());
        assert_eq!(model.unwrap().id, "gpt-4o");
    }

    #[test]
    fn test_find_model_case_insensitive() {
        let model = RegistryService::find_model("GPT-4O");
        assert!(model.is_some());
        assert_eq!(model.unwrap().id, "gpt-4o");
    }

    #[test]
    fn test_get_context_window_known_model() {
        // Claude models have ~200k context
        let ctx = RegistryService::get_context_window("claude-sonnet-4-20250514");
        assert!(ctx >= 200_000, "Claude Sonnet 4 should have at least 200k context");
    }

    #[test]
    fn test_get_context_window_gpt4o() {
        let ctx = RegistryService::get_context_window("gpt-4o");
        assert_eq!(ctx, 128_000);
    }

    #[test]
    fn test_get_context_window_gemini() {
        // Gemini 2.0 Flash has 1M+ context (1048576 tokens)
        let ctx = RegistryService::get_context_window("gemini-2.0-flash");
        assert!(ctx >= 1_000_000, "Gemini 2.0 Flash should have at least 1M context");
    }

    #[test]
    fn test_get_context_window_unknown_model() {
        let ctx = RegistryService::get_context_window("totally-unknown-model-xyz");
        assert_eq!(ctx, DEFAULT_CONTEXT_WINDOW);
    }

    #[test]
    fn test_get_context_window_canonical_format() {
        // Canonical format with provider prefix should work
        let ctx = RegistryService::get_context_window("anthropic:claude-sonnet-4-20250514");
        assert!(ctx >= 200_000, "Claude Sonnet 4 should have at least 200k context");

        let ctx = RegistryService::get_context_window("openai:gpt-4o");
        assert_eq!(ctx, 128_000);

        let ctx = RegistryService::get_context_window("google:gemini-2.0-flash");
        assert!(ctx >= 1_000_000, "Gemini should have at least 1M context");
    }

    #[test]
    fn test_find_model_canonical_format() {
        // Canonical format with provider prefix should work
        let model = RegistryService::find_model("openai:gpt-4o");
        assert!(model.is_some());
        assert_eq!(model.unwrap().id, "gpt-4o");
    }

    #[test]
    fn test_models_for_provider() {
        let anthropic_models = RegistryService::models_for_provider("anthropic");
        assert!(!anthropic_models.is_empty());
        assert!(anthropic_models.iter().all(|m| m.provider_id == "anthropic"));
    }

    #[test]
    fn test_models_for_provider_case_insensitive() {
        let models = RegistryService::models_for_provider("ANTHROPIC");
        assert!(!models.is_empty());
    }

    #[test]
    fn test_normalized_model_lookup_dot_notation() {
        // Should find claude-3-5-sonnet even with dot notation
        let model = RegistryService::find_model("claude-3.5-sonnet");
        assert!(model.is_some(), "Should find model with dot notation");
        assert!(
            model.unwrap().id.contains("claude-3-5-sonnet")
                || model.unwrap().id.contains("claude-3.5-sonnet"),
            "Should match a Claude 3.5 Sonnet model"
        );
    }

    #[test]
    fn test_find_provider() {
        let provider = RegistryService::find_provider("anthropic");
        assert!(provider.is_some());
        assert_eq!(provider.unwrap().name, "Anthropic");
    }

    #[test]
    fn test_find_provider_case_insensitive() {
        let provider = RegistryService::find_provider("OpenAI");
        assert!(provider.is_some());
    }

    #[test]
    fn test_is_known_model() {
        assert!(RegistryService::is_known_model("gpt-4o"));
        assert!(RegistryService::is_known_model("claude-sonnet-4-20250514"));
        assert!(!RegistryService::is_known_model("fake-model-123"));
    }

    #[test]
    fn test_is_known_provider() {
        assert!(RegistryService::is_known_provider("anthropic"));
        assert!(RegistryService::is_known_provider("openai"));
        assert!(!RegistryService::is_known_provider("fake-provider"));
    }

    #[test]
    fn test_get_provider_for_model_canonical() {
        let provider = RegistryService::get_provider_for_model("openai:gpt-4o");
        assert!(provider.is_some());
        assert_eq!(provider.unwrap().id, "openai");
    }

    #[test]
    fn test_get_provider_for_model_lookup() {
        // gpt-4o exists in multiple providers, just verify we get a valid one
        let provider = RegistryService::get_provider_for_model("gpt-4o");
        assert!(provider.is_some(), "gpt-4o should have a provider");
    }

    #[test]
    fn test_parse_canonical_id() {
        assert_eq!(
            RegistryService::parse_canonical_id("openai:gpt-4o"),
            Some(("openai", "gpt-4o"))
        );
        assert_eq!(
            RegistryService::parse_canonical_id("anthropic:claude-sonnet-4-20250514"),
            Some(("anthropic", "claude-sonnet-4-20250514"))
        );
        assert_eq!(RegistryService::parse_canonical_id("gpt-4o"), None);
    }

    #[test]
    fn test_get_max_output_tokens() {
        // Note: Value depends on whether static data or generated data is used
        // If generated from API cache (no max_output_tokens), defaults to 16384
        // If using static_data.rs, returns 64000
        let tokens = RegistryService::get_max_output_tokens("claude-sonnet-4-20250514");
        assert!(tokens.is_some(), "Should return some max_output_tokens");
        assert!(
            tokens.unwrap() >= 16_384,
            "max_output_tokens should be at least 16384"
        );

        let unknown = RegistryService::get_max_output_tokens("unknown-model");
        assert_eq!(unknown, None);
    }

    #[test]
    fn test_fuzzy_match_partial() {
        // "sonnet" should match some Claude Sonnet model
        let model = RegistryService::find_model("sonnet-4");
        assert!(model.is_some(), "Should fuzzy match 'sonnet-4'");
        assert!(
            model.unwrap().id.contains("sonnet"),
            "Matched model should contain 'sonnet'"
        );
    }

    #[test]
    fn test_context_overrides() {
        // ChatGPT OAuth Codex models are limited to 270k via overrides
        // (from data/context_overrides.json)
        let ctx = RegistryService::get_context_window("chatgpt:gpt-5.1-codex");
        assert_eq!(ctx, 270_000, "ChatGPT OAuth gpt-5.1-codex should be limited to 270k");

        let ctx = RegistryService::get_context_window("chatgpt:gpt-5.2");
        assert_eq!(ctx, 270_000, "ChatGPT OAuth gpt-5.2 should be limited to 270k");

        // Regular gpt-4o (not ChatGPT OAuth) should use the API value
        let ctx = RegistryService::get_context_window("openai:gpt-4o");
        assert_eq!(ctx, 128_000, "OpenAI gpt-4o should be 128k");

        // Just "gpt-4o" without provider prefix should use API value (no global override)
        let ctx = RegistryService::get_context_window("gpt-4o");
        assert_eq!(ctx, 128_000, "gpt-4o without prefix should be 128k");
    }
}
