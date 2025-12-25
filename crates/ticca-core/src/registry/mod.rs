//! Model and Provider Registry
//!
//! Provides authoritative model metadata including context windows,
//! capabilities, and provider information.
//!
//! This registry is the single source of truth for model information,
//! replacing scattered hardcoded values throughout the codebase.
//!
//! # Data Sources
//!
//! - **Static**: Bundled at build time from models.dev API
//! - **Discovered**: Runtime discovery from provider /models endpoints
//! - **User**: Custom user-defined models
//!
//! # Usage
//!
//! ```rust,ignore
//! use ticca_core::registry::{RegistryService, ModelDefinition};
//!
//! // Get context window for a model (the main use case!)
//! let ctx = RegistryService::get_context_window("gpt-4o");
//!
//! // Find a specific model
//! if let Some(model) = RegistryService::find_model("claude-sonnet-4") {
//!     println!("Context window: {}", model.context_window);
//! }
//!
//! // Get all models for a provider
//! let anthropic_models = RegistryService::models_for_provider("anthropic");
//! ```

mod service;
mod static_data;

// Include build-time generated data from models.dev API
// This provides generated_providers(), generated_models(), and generated_context_windows()
include!(concat!(env!("OUT_DIR"), "/registry_generated.rs"));

pub use service::{RegistryService, DEFAULT_CONTEXT_WINDOW};
pub use static_data::{static_models, static_providers};

use serde::{Deserialize, Serialize};

/// How a provider authenticates API requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    /// OAuth 2.0 / PKCE flow
    OAuth,
    /// Static API key (env var or user-provided)
    #[default]
    ApiKey,
    /// Supports both OAuth and API key
    Both,
}

/// Provider definition from static registry.
///
/// Represents an LLM provider like Anthropic, OpenAI, Google, etc.
/// Contains metadata about API endpoints, authentication, and documentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderDefinition {
    /// Unique identifier (e.g., "anthropic", "openai", "google")
    pub id: String,
    /// Human-readable name (e.g., "Anthropic", "OpenAI")
    pub name: String,
    /// Base URL for API requests
    pub api_base_url: String,
    /// Environment variable names for API keys
    pub env_vars: Vec<String>,
    /// Authentication method(s) supported
    pub auth_type: AuthType,
    /// Whether the provider uses OpenAI-compatible API format
    pub is_openai_compatible: bool,
    /// Link to provider documentation
    pub doc_url: Option<String>,
}

impl ProviderDefinition {
    /// Check if this provider supports API key authentication.
    pub fn supports_api_key(&self) -> bool {
        matches!(self.auth_type, AuthType::ApiKey | AuthType::Both)
    }

    /// Check if this provider supports OAuth authentication.
    pub fn supports_oauth(&self) -> bool {
        matches!(self.auth_type, AuthType::OAuth | AuthType::Both)
    }
}

/// Model capabilities - what features the model supports.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelCapabilities {
    /// Supports function/tool calling
    pub tool_call: bool,
    /// Supports image/vision input
    pub vision: bool,
    /// Has extended reasoning/chain-of-thought
    pub reasoning: bool,
    /// Supports streaming responses
    pub streaming: bool,
    /// Supports JSON mode output
    pub json_mode: bool,
}

impl ModelCapabilities {
    /// Create capabilities for a full-featured model.
    pub fn full_featured() -> Self {
        Self {
            tool_call: true,
            vision: true,
            reasoning: true,
            streaming: true,
            json_mode: true,
        }
    }

    /// Create capabilities for a basic text-only model.
    pub fn basic() -> Self {
        Self {
            tool_call: false,
            vision: false,
            reasoning: false,
            streaming: true,
            json_mode: false,
        }
    }
}

/// Model definition with all metadata.
///
/// This is the core type representing a model in the registry.
/// Contains everything needed to work with a model: identification,
/// context limits, capabilities, and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDefinition {
    /// Model identifier as used in API calls (e.g., "claude-sonnet-4-20250514")
    pub id: String,
    /// Provider identifier (e.g., "anthropic")
    pub provider_id: String,
    /// Human-readable display name (e.g., "Claude Sonnet 4")
    pub name: String,
    /// Model family grouping (e.g., "claude-4", "gpt-4")
    pub family: Option<String>,
    /// Maximum input context window in tokens - THE KEY DATA POINT
    pub context_window: u64,
    /// Maximum output tokens (if limited)
    pub max_output_tokens: Option<u64>,
    /// Model capabilities
    pub capabilities: ModelCapabilities,
    /// Knowledge cutoff date (e.g., "2025-03")
    pub knowledge_cutoff: Option<String>,
    /// Model release date (e.g., "2025-05-14")
    pub release_date: Option<String>,
}

impl ModelDefinition {
    /// Get the canonical ID in `provider:model` format.
    ///
    /// This format is used throughout ticca for unambiguous model identification.
    ///
    /// # Example
    /// ```rust,ignore
    /// let model = ModelDefinition { id: "gpt-4o".into(), provider_id: "openai".into(), .. };
    /// assert_eq!(model.canonical_id(), "openai:gpt-4o");
    /// ```
    pub fn canonical_id(&self) -> String {
        format!("{}:{}", self.provider_id, self.id)
    }

    /// Check if this model supports tool/function calling.
    pub fn supports_tools(&self) -> bool {
        self.capabilities.tool_call
    }

    /// Check if this model supports vision/image input.
    pub fn supports_vision(&self) -> bool {
        self.capabilities.vision
    }

    /// Get the effective output token limit.
    ///
    /// Returns the max_output_tokens if set, otherwise estimates
    /// a reasonable default based on context window.
    pub fn effective_max_output(&self) -> u64 {
        self.max_output_tokens.unwrap_or_else(|| {
            // Reasonable default: ~25% of context window, capped at 16k
            (self.context_window / 4).min(16_384)
        })
    }
}

/// Source of model data - how we know about this model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelSource {
    /// Bundled at build time from models.dev
    #[default]
    Static,
    /// Discovered from provider's /models API at runtime
    Discovered,
    /// User-configured custom model
    User,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_id() {
        let model = ModelDefinition {
            id: "gpt-4o".to_string(),
            provider_id: "openai".to_string(),
            name: "GPT-4o".to_string(),
            family: Some("gpt-4".to_string()),
            context_window: 128_000,
            max_output_tokens: Some(16_384),
            capabilities: ModelCapabilities::full_featured(),
            knowledge_cutoff: None,
            release_date: None,
        };

        assert_eq!(model.canonical_id(), "openai:gpt-4o");
    }

    #[test]
    fn test_provider_auth_support() {
        let provider = ProviderDefinition {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            api_base_url: "https://api.anthropic.com/v1".to_string(),
            env_vars: vec!["ANTHROPIC_API_KEY".to_string()],
            auth_type: AuthType::Both,
            is_openai_compatible: false,
            doc_url: None,
        };

        assert!(provider.supports_api_key());
        assert!(provider.supports_oauth());
    }

    #[test]
    fn test_effective_max_output() {
        // Model with explicit max_output_tokens
        let model_with_limit = ModelDefinition {
            id: "test".to_string(),
            provider_id: "test".to_string(),
            name: "Test".to_string(),
            family: None,
            context_window: 200_000,
            max_output_tokens: Some(64_000),
            capabilities: ModelCapabilities::default(),
            knowledge_cutoff: None,
            release_date: None,
        };
        assert_eq!(model_with_limit.effective_max_output(), 64_000);

        // Model without explicit limit - should get estimated value
        let model_no_limit = ModelDefinition {
            id: "test".to_string(),
            provider_id: "test".to_string(),
            name: "Test".to_string(),
            family: None,
            context_window: 32_000,
            max_output_tokens: None,
            capabilities: ModelCapabilities::default(),
            knowledge_cutoff: None,
            release_date: None,
        };
        // 32k / 4 = 8k, which is less than 16k cap
        assert_eq!(model_no_limit.effective_max_output(), 8_000);
    }

    #[test]
    fn test_model_capabilities_presets() {
        let full = ModelCapabilities::full_featured();
        assert!(full.tool_call);
        assert!(full.vision);
        assert!(full.reasoning);
        assert!(full.streaming);
        assert!(full.json_mode);

        let basic = ModelCapabilities::basic();
        assert!(!basic.tool_call);
        assert!(!basic.vision);
        assert!(!basic.reasoning);
        assert!(basic.streaming); // Even basic models usually stream
        assert!(!basic.json_mode);
    }
}
