//! Provider resolution and client creation.
//!
//! Resolves model names to serdesAI model instances.

use crate::config::models::providers;
use crate::llm::auth::{self, ApiKeyToken, AuthToken};
use crate::llm::ProviderRegistry;
use crate::llm::ProviderId;
use crate::registry::RegistryService;
use serdes_ai_models::claude_code_oauth::ClaudeCodeOAuthModel;
use serdes_ai_models::chatgpt_oauth::ChatGptOAuthModel;
use serdes_ai_models::openai::OpenAIChatModel;

/// Resolved provider with its model and metadata.
pub enum ResolvedProvider {
    Claude {
        model: ClaudeCodeOAuthModel,
        model_id: String,
        token: AuthToken,
    },
    ChatGpt {
        model: ChatGptOAuthModel,
        model_id: String,
        token: AuthToken,
    },
    /// OpenAI-compatible API key providers (Groq, Cerebras, OpenRouter, etc.)
    OpenAICompatible {
        model: OpenAIChatModel,
        model_id: String,
        provider_id: String,
        api_key_token: ApiKeyToken,
    },
}

impl ResolvedProvider {
    /// Get the account ID for cooldown marking.
    pub fn account_id(&self) -> Option<&str> {
        match self {
            Self::Claude { token, .. } => Some(&token.account_id),
            Self::ChatGpt { token, .. } => Some(&token.account_id),
            Self::OpenAICompatible { api_key_token, .. } => Some(&api_key_token.account_id),
        }
    }

    /// Get a display label for the provider.
    pub fn label(&self) -> &str {
        match self {
            Self::Claude { .. } => "Claude",
            Self::ChatGpt { .. } => "ChatGPT",
            Self::OpenAICompatible { provider_id, .. } => provider_id.as_str(),
        }
    }

    /// Get the model ID.
    pub fn model_id(&self) -> &str {
        match self {
            Self::Claude { model_id, .. } => model_id,
            Self::ChatGpt { model_id, .. } => model_id,
            Self::OpenAICompatible { model_id, .. } => model_id,
        }
    }
    
    /// Check if this is an API key provider (for cooldown marking).
    pub fn is_api_key_provider(&self) -> bool {
        matches!(self, Self::OpenAICompatible { .. })
    }
}

/// Resolve and create a provider for the given model name.
pub fn resolve_provider(model_name: &str) -> Result<ResolvedProvider, String> {
    let model_id = ProviderRegistry::extract_model_id(model_name);

    match ProviderRegistry::resolve_provider(model_name) {
        ProviderId::Claude => {
            let token = auth::select_token(providers::CLAUDE).ok_or_else(|| {
                "Claude authentication required. Please authenticate in Settings.".to_string()
            })?;

            let model = ClaudeCodeOAuthModel::new(model_id, &token.access_token);

            Ok(ResolvedProvider::Claude {
                model,
                model_id: model_id.to_string(),
                token,
            })
        }
        ProviderId::ChatGpt => {
            let token = auth::select_token(providers::CHATGPT).ok_or_else(|| {
                "ChatGPT authentication required. Please authenticate in Settings.".to_string()
            })?;
            
            let model = ChatGptOAuthModel::new(model_id, &token.access_token);

            Ok(ResolvedProvider::ChatGpt {
                model,
                model_id: model_id.to_string(),
                token,
            })
        }
        ProviderId::Gemini => {
            Err("Gemini OAuth is deprecated and no longer available.".to_string())
        }
        ProviderId::ApiKey(provider_id) => {
            // Get API key for this provider
            let api_key_token = auth::select_api_key(&provider_id).ok_or_else(|| {
                format!(
                    "{} API key required. Add one in Settings → Accounts.",
                    provider_id
                )
            })?;
            
            // Get provider definition from registry
            let provider_def = RegistryService::find_provider(&provider_id).ok_or_else(|| {
                format!("Unknown provider: {}", provider_id)
            })?;
            
            // Verify this provider supports API key auth
            if !provider_def.supports_api_key() {
                return Err(format!(
                    "Provider '{}' does not support API key authentication",
                    provider_id
                ));
            }
            
            // For now, all API key providers use OpenAI-compatible format
            // TODO: Add Anthropic API support when needed
            if !provider_def.is_openai_compatible {
                return Err(format!(
                    "Provider '{}' uses a custom API format not yet supported",
                    provider_id
                ));
            }
            
            // Create OpenAI-compatible model with custom base URL
            let model = OpenAIChatModel::new(model_id, &api_key_token.api_key)
                .with_base_url(&provider_def.api_base_url);
            
            tracing::info!(
                "Resolved API key provider: {} with model {} (base_url: {})",
                provider_id,
                model_id,
                provider_def.api_base_url
            );
            
            Ok(ResolvedProvider::OpenAICompatible {
                model,
                model_id: model_id.to_string(),
                provider_id,
                api_key_token,
            })
        }
    }
}

/// Fetch the best available model from the Claude API.
pub async fn fetch_best_model(auth_token: &AuthToken) -> Result<String, String> {
    let client = crate::llm::ClaudeClient::new(auth_token.access_token.to_string());
    let models = client
        .fetch_latest_models()
        .await
        .map_err(|e| format!("Failed to fetch models: {}", e))?;

    // Prefer sonnet, then opus, then haiku
    let preferred_order = ["sonnet", "opus", "haiku"];

    for family in preferred_order {
        if let Some(model) = models.iter().find(|m| m.contains(family)) {
            return Ok(model.clone());
        }
    }

    models
        .into_iter()
        .next()
        .ok_or_else(|| "No models available from Claude API".to_string())
}

/// Resolve the model name, falling back to auto-detection if not specified.
pub async fn resolve_model_name(model_name: Option<String>) -> Result<String, String> {
    match model_name {
        Some(name) => {
            tracing::info!("Using configured model: {}", name);
            Ok(name)
        }
        None => {
            let claude_token = auth::select_token(providers::CLAUDE).ok_or_else(|| {
                "Claude authentication required to auto-select a model".to_string()
            })?;
            match fetch_best_model(&claude_token).await {
                Ok(name) => {
                    tracing::info!("Using auto-detected model: {}", name);
                    Ok(name)
                }
                Err(e) => Err(e),
            }
        }
    }
}
