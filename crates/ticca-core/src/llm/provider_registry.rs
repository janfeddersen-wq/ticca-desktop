//! Provider registry and model routing
//!
//! This module provides:
//! - `ModelId`: Canonical `provider:model_id` format for model identification
//! - `ProviderId`: Enumeration of available providers (OAuth and API key based)
//! - `ProviderRegistry`: Static utilities for provider info and model resolution

use crate::config::models::providers;
use crate::llm::providers::is_chatgpt_model;
use crate::registry::RegistryService;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Canonical model identifier in `provider:model_id` format.
///
/// Examples:
/// - `claude:claude-sonnet-4-20250514`
/// - `chatgpt:gpt-4o`
/// - `gemini:gemini-2.0-flash-exp`
/// - `openai:gpt-4o` (API key provider)
/// - `anthropic:claude-3-5-sonnet-20241022` (API key provider)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId {
    /// The provider identifier (e.g., "claude", "openai", "groq")
    pub provider: String,
    /// The model identifier within that provider (e.g., "gpt-4o", "claude-sonnet-4")
    pub model: String,
}

impl ModelId {
    /// Create a new ModelId from provider and model strings
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
        }
    }

    /// Create a ModelId from a ProviderId and model string
    pub fn from_provider(provider: ProviderId, model: impl Into<String>) -> Self {
        Self {
            provider: provider.as_str().to_string(),
            model: model.into(),
        }
    }

    /// Get the canonical string representation (`provider:model_id`)
    pub fn canonical(&self) -> String {
        format!("{}:{}", self.provider, self.model)
    }

    /// Parse a canonical model ID string.
    /// Supports both `provider:model` format and legacy `model - Provider` format.
    pub fn parse(s: &str) -> Option<Self> {
        // Try canonical format first: "provider:model_id"
        if let Some((provider, model)) = s.split_once(':') {
            let provider = provider.trim();
            let model = model.trim();
            if !provider.is_empty() && !model.is_empty() {
                return Some(Self::new(provider, model));
            }
        }

        // Fall back to legacy format: "model_id - Provider Name"
        if let Some((model, suffix)) = s.rsplit_once(" - ") {
            let model = model.trim();
            let suffix = suffix.trim();
            if !model.is_empty() && !suffix.is_empty() {
                // Map legacy suffix to provider ID
                let provider = Self::legacy_suffix_to_provider(suffix);
                return Some(Self::new(&provider, model));
            }
        }

        None
    }

    /// Map legacy provider suffix to provider ID string
    fn legacy_suffix_to_provider(suffix: &str) -> String {
        // OAuth providers
        if suffix.contains("Claude") && suffix.contains("OAuth") {
            return providers::CLAUDE.to_string();
        }
        if suffix.contains("Gemini") && suffix.contains("OAuth") {
            return providers::GEMINI.to_string();
        }
        if suffix.contains("ChatGPT") && suffix.contains("OAuth") {
            return providers::CHATGPT.to_string();
        }

        // API key providers - match by display name from registry
        for provider in RegistryService::api_key_providers() {
            if suffix == provider.name {
                return provider.id.clone();
            }
        }

        // Unknown - default to the suffix lowercased
        // This is a fallback for unknown providers
        "unknown".to_string()
    }

    /// Get the display name for UI (e.g., "gpt-4o (OpenAI)")
    pub fn display_name(&self) -> String {
        let provider_display = self.provider_display_name();
        format!("{} ({})", self.model, provider_display)
    }

    /// Get a human-readable provider display name
    fn provider_display_name(&self) -> String {
        // OAuth providers
        match self.provider.as_str() {
            providers::CLAUDE => "Claude".to_string(),
            providers::GEMINI => "Gemini".to_string(),
            providers::CHATGPT => "ChatGPT".to_string(),
            _ => {
                // Check API key providers from registry
                if let Some(provider_def) = RegistryService::find_provider(&self.provider) {
                    provider_def.name.clone()
                } else {
                    self.provider.clone()
                }
            }
        }
    }

    /// Resolve the ProviderId for this model
    pub fn resolve_provider(&self) -> ProviderId {
        match self.provider.as_str() {
            providers::CLAUDE => ProviderId::Claude,
            providers::GEMINI => ProviderId::Gemini,
            providers::CHATGPT => ProviderId::ChatGpt,
            _ => {
                // Check if this is a known API key provider from registry
                if RegistryService::find_provider(&self.provider).is_some() {
                    ProviderId::ApiKey(self.provider.clone())
                } else {
                    // Fallback: try to infer from model name patterns
                    if is_chatgpt_model(&self.model) {
                        ProviderId::ChatGpt
                    } else {
                        // Default to Claude for unknown
                        ProviderId::Claude
                    }
                }
            }
        }
    }
}

impl fmt::Display for ModelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.canonical())
    }
}

impl FromStr for ModelId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("Invalid model ID format: {}", s))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProviderId {
    Claude,
    Gemini,
    ChatGpt,
    /// API key-based provider, identified by provider ID string from registry
    ApiKey(String),
}

impl ProviderId {
    /// Get the provider ID as a string slice
    pub fn as_str(&self) -> &str {
        match self {
            ProviderId::Claude => providers::CLAUDE,
            ProviderId::Gemini => providers::GEMINI,
            ProviderId::ChatGpt => providers::CHATGPT,
            ProviderId::ApiKey(id) => id,
        }
    }

    /// Get a human-readable display name for the provider
    pub fn display_name(&self) -> String {
        match self {
            ProviderId::Claude => "Claude".to_string(),
            ProviderId::Gemini => "Gemini".to_string(),
            ProviderId::ChatGpt => "ChatGPT".to_string(),
            ProviderId::ApiKey(id) => {
                // Look up display name from registry
                if let Some(provider_def) = RegistryService::find_provider(id) {
                    provider_def.name.clone()
                } else {
                    id.clone()
                }
            }
        }
    }

    pub fn is_oauth(&self) -> bool {
        matches!(
            self,
            ProviderId::Claude | ProviderId::Gemini | ProviderId::ChatGpt
        )
    }
}

#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    pub supports_tools: bool,
    pub supports_images: bool,
    pub supports_reasoning: bool,
    pub supports_streaming: bool,
    pub requires_id_token: bool,
}

#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub id: ProviderId,
    pub display_name: String,
    pub capabilities: ProviderCapabilities,
}

pub struct ProviderRegistry;

impl ProviderRegistry {
    /// Extract the actual model ID from a model string.
    ///
    /// Supports both canonical format (`provider:model`) and legacy format (`model - Provider`).
    /// Returns just the model ID portion.
    pub fn extract_model_id(model_name: &str) -> &str {
        // Try canonical format first: "provider:model_id"
        if let Some((_provider, model)) = model_name.split_once(':') {
            return model.trim();
        }

        // Fall back to legacy format: "model_id - Provider Name"
        if let Some(idx) = model_name.rfind(" - ") {
            return model_name[..idx].trim();
        }

        // No recognized format - return as-is
        model_name
    }

    /// Parse a model string into a ModelId.
    ///
    /// Supports both canonical format (`provider:model`) and legacy format (`model - Provider`).
    pub fn parse_model_id(model_name: &str) -> Option<ModelId> {
        ModelId::parse(model_name)
    }

    /// Resolve the provider for a model string.
    ///
    /// Supports both canonical format (`provider:model`) and legacy format (`model - Provider`).
    pub fn resolve_provider(model_name: &str) -> ProviderId {
        // Try to parse as ModelId first
        if let Some(model_id) = ModelId::parse(model_name) {
            return model_id.resolve_provider();
        }

        // Fall back to model name pattern matching
        if is_chatgpt_model(model_name) {
            ProviderId::ChatGpt
        } else {
            ProviderId::Claude
        }
    }

    /// Extract the provider suffix from a legacy display name like "model-id - Provider Name"
    #[deprecated(note = "Use ModelId::parse() instead")]
    pub fn extract_provider_suffix(model_name: &str) -> Option<&str> {
        if let Some(idx) = model_name.rfind(" - ") {
            Some(model_name[idx + 3..].trim())
        } else {
            None
        }
    }

    pub fn info(provider: ProviderId) -> ProviderInfo {
        match &provider {
            ProviderId::Claude => ProviderInfo {
                id: provider,
                display_name: "Claude".to_string(),
                capabilities: ProviderCapabilities {
                    supports_tools: true,
                    supports_images: true,
                    supports_reasoning: true,
                    supports_streaming: true,
                    requires_id_token: false,
                },
            },
            ProviderId::Gemini => ProviderInfo {
                id: provider,
                display_name: "Gemini".to_string(),
                capabilities: ProviderCapabilities {
                    supports_tools: true,
                    supports_images: true,
                    supports_reasoning: true,
                    supports_streaming: true,
                    requires_id_token: false,
                },
            },
            ProviderId::ChatGpt => ProviderInfo {
                id: provider,
                display_name: "ChatGPT".to_string(),
                capabilities: ProviderCapabilities {
                    supports_tools: true,
                    supports_images: true,
                    supports_reasoning: true,
                    supports_streaming: true,
                    requires_id_token: true,
                },
            },
            ProviderId::ApiKey(provider_id) => {
                // Look up provider info from registry
                let (display_name, is_openai_compatible) =
                    if let Some(provider_def) = RegistryService::find_provider(provider_id) {
                        (provider_def.name.clone(), provider_def.is_openai_compatible)
                    } else {
                        (provider_id.clone(), true) // Default to OpenAI compatible
                    };

                ProviderInfo {
                    id: provider,
                    display_name,
                    capabilities: ProviderCapabilities {
                        supports_tools: true,
                        supports_images: is_openai_compatible,
                        supports_reasoning: true,
                        supports_streaming: true,
                        requires_id_token: false,
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // ModelId parsing tests
    // ==========================================================================

    #[test]
    fn test_model_id_parse_canonical_format() {
        // Basic canonical format
        let id = ModelId::parse("claude:claude-sonnet-4-20250514").unwrap();
        assert_eq!(id.provider, "claude");
        assert_eq!(id.model, "claude-sonnet-4-20250514");

        let id = ModelId::parse("openai:gpt-4o").unwrap();
        assert_eq!(id.provider, "openai");
        assert_eq!(id.model, "gpt-4o");

        let id = ModelId::parse("gemini:gemini-2.0-flash-exp").unwrap();
        assert_eq!(id.provider, "gemini");
        assert_eq!(id.model, "gemini-2.0-flash-exp");
    }

    #[test]
    fn test_model_id_parse_legacy_format() {
        // OAuth providers (legacy format)
        let id = ModelId::parse("claude-sonnet-4-20250514 - Claude (OAuth)").unwrap();
        assert_eq!(id.provider, "claude");
        assert_eq!(id.model, "claude-sonnet-4-20250514");

        let id = ModelId::parse("gpt-4o - ChatGPT (OAuth)").unwrap();
        assert_eq!(id.provider, "chatgpt");
        assert_eq!(id.model, "gpt-4o");

        let id = ModelId::parse("gemini-2.0-flash-exp - Gemini (OAuth)").unwrap();
        assert_eq!(id.provider, "gemini");
        assert_eq!(id.model, "gemini-2.0-flash-exp");

        // API key providers (legacy format)
        let id = ModelId::parse("gpt-4o - OpenAI").unwrap();
        assert_eq!(id.provider, "openai");
        assert_eq!(id.model, "gpt-4o");

        let id = ModelId::parse("llama-3.1-70b - Groq").unwrap();
        assert_eq!(id.provider, "groq");
        assert_eq!(id.model, "llama-3.1-70b");
    }

    #[test]
    fn test_model_id_canonical_string() {
        let id = ModelId::new("claude", "claude-sonnet-4-20250514");
        assert_eq!(id.canonical(), "claude:claude-sonnet-4-20250514");
        assert_eq!(id.to_string(), "claude:claude-sonnet-4-20250514");
    }

    #[test]
    fn test_model_id_display_name() {
        let id = ModelId::new("claude", "claude-sonnet-4-20250514");
        assert_eq!(id.display_name(), "claude-sonnet-4-20250514 (Claude)");

        let id = ModelId::new("openai", "gpt-4o");
        assert_eq!(id.display_name(), "gpt-4o (OpenAI)");
    }

    #[test]
    fn test_model_id_resolve_provider() {
        let id = ModelId::new("claude", "claude-sonnet-4");
        assert_eq!(id.resolve_provider(), ProviderId::Claude);

        let id = ModelId::new("chatgpt", "gpt-4o");
        assert_eq!(id.resolve_provider(), ProviderId::ChatGpt);

        let id = ModelId::new("gemini", "gemini-2.0-flash");
        assert_eq!(id.resolve_provider(), ProviderId::Gemini);

        let id = ModelId::new("openai", "gpt-4o");
        assert_eq!(
            id.resolve_provider(),
            ProviderId::ApiKey("openai".to_string())
        );

        let id = ModelId::new("groq", "llama-3.1-70b");
        assert_eq!(
            id.resolve_provider(),
            ProviderId::ApiKey("groq".to_string())
        );
    }

    #[test]
    fn test_model_id_from_str() {
        let id: ModelId = "claude:claude-sonnet-4".parse().unwrap();
        assert_eq!(id.provider, "claude");
        assert_eq!(id.model, "claude-sonnet-4");

        // Invalid format should fail
        let result: Result<ModelId, _> = "".parse();
        assert!(result.is_err());
    }

    // ==========================================================================
    // ProviderRegistry tests
    // ==========================================================================

    #[test]
    fn test_extract_model_id_canonical() {
        assert_eq!(
            ProviderRegistry::extract_model_id("claude:claude-sonnet-4-20250514"),
            "claude-sonnet-4-20250514"
        );
        assert_eq!(
            ProviderRegistry::extract_model_id("openai:gpt-4o"),
            "gpt-4o"
        );
    }

    #[test]
    fn test_extract_model_id_legacy() {
        assert_eq!(
            ProviderRegistry::extract_model_id("gpt-4o - OpenAI"),
            "gpt-4o"
        );
        assert_eq!(
            ProviderRegistry::extract_model_id("claude-sonnet-4 - Claude (OAuth)"),
            "claude-sonnet-4"
        );
    }

    #[test]
    fn test_extract_model_id_plain() {
        // Plain model names without provider info
        assert_eq!(ProviderRegistry::extract_model_id("gpt-4o"), "gpt-4o");
        assert_eq!(
            ProviderRegistry::extract_model_id("claude-sonnet-4"),
            "claude-sonnet-4"
        );
    }

    #[test]
    fn resolves_chatgpt_models() {
        // Canonical format
        assert_eq!(
            ProviderRegistry::resolve_provider("chatgpt:gpt-4o"),
            ProviderId::ChatGpt
        );
        // Legacy format
        assert_eq!(
            ProviderRegistry::resolve_provider("gpt-4o - ChatGPT (OAuth)"),
            ProviderId::ChatGpt
        );
        // Plain model name (pattern matching fallback)
        assert_eq!(
            ProviderRegistry::resolve_provider("gpt-4o"),
            ProviderId::ChatGpt
        );
        assert_eq!(
            ProviderRegistry::resolve_provider("chatgpt-4o-latest"),
            ProviderId::ChatGpt
        );
    }

    #[test]
    fn resolves_gemini_as_legacy_provider() {
        // Canonical format - still recognized as Gemini provider
        assert_eq!(
            ProviderRegistry::resolve_provider("gemini:gemini-2.0-flash"),
            ProviderId::Gemini
        );
        // Plain model name now defaults to Claude since Gemini OAuth is deprecated
        assert_eq!(
            ProviderRegistry::resolve_provider("gemini-2.0-flash"),
            ProviderId::Claude
        );
    }

    #[test]
    fn resolves_claude_as_default() {
        // Canonical format
        assert_eq!(
            ProviderRegistry::resolve_provider("claude:claude-3-5-sonnet"),
            ProviderId::Claude
        );
        // Plain model name
        assert_eq!(
            ProviderRegistry::resolve_provider("claude-3-5-sonnet"),
            ProviderId::Claude
        );
        // Unknown model defaults to Claude
        assert_eq!(
            ProviderRegistry::resolve_provider("unknown-model"),
            ProviderId::Claude
        );
    }

    #[test]
    fn resolves_api_key_providers() {
        // Canonical format
        assert_eq!(
            ProviderRegistry::resolve_provider("openai:gpt-4o"),
            ProviderId::ApiKey("openai".to_string())
        );
        assert_eq!(
            ProviderRegistry::resolve_provider("groq:llama-3.1-70b"),
            ProviderId::ApiKey("groq".to_string())
        );
        // Legacy format
        assert_eq!(
            ProviderRegistry::resolve_provider("gpt-4o - OpenAI"),
            ProviderId::ApiKey("openai".to_string())
        );
    }
}
