//! Provider resolution and client creation.
//!
//! Consolidates provider-specific logic that was previously duplicated
//! in multiple places.

use crate::config::models::providers;
use crate::llm::auth::{self, AuthToken};
use crate::llm::providers::{
    ChatGptOAuthClient, GeminiCodeAssistRigClient, OpenAICompatibleApiClient,
};
use crate::llm::{ClaudeOAuthClient, ProviderId, ProviderRegistry};
use crate::registry::RegistryService;

/// Resolved provider with its client and metadata.
pub enum ResolvedProvider {
    Claude {
        client: ClaudeOAuthClient,
        model_id: String,
        token: AuthToken,
    },
    ChatGpt {
        client: ChatGptOAuthClient,
        model_id: String,
        token: AuthToken,
    },
    Gemini {
        client: GeminiCodeAssistRigClient,
        model_id: String,
        token: AuthToken,
    },
    ApiKey {
        client: OpenAICompatibleApiClient,
        model_id: String,
        provider_name: String,
    },
}

impl ResolvedProvider {
    /// Get the account ID for cooldown marking (OAuth providers only).
    pub fn account_id(&self) -> Option<&str> {
        match self {
            Self::Claude { token, .. } => Some(&token.account_id),
            Self::ChatGpt { token, .. } => Some(&token.account_id),
            Self::Gemini { token, .. } => Some(&token.account_id),
            Self::ApiKey { .. } => None,
        }
    }

    /// Get a display label for the provider.
    pub fn label(&self) -> &str {
        match self {
            Self::Claude { .. } => "Claude",
            Self::ChatGpt { .. } => "ChatGPT",
            Self::Gemini { .. } => "Gemini",
            Self::ApiKey { provider_name, .. } => provider_name,
        }
    }
}

/// Resolve and create a provider client for the given model name.
pub fn resolve_provider(model_name: &str) -> Result<ResolvedProvider, String> {
    let model_id = ProviderRegistry::extract_model_id(model_name);

    match ProviderRegistry::resolve_provider(model_name) {
        ProviderId::Claude => {
            let token = auth::select_token(providers::CLAUDE).ok_or_else(|| {
                "Claude authentication required. Please authenticate in Settings.".to_string()
            })?;

            let client = ClaudeOAuthClient::new(token.access_token.clone())
                .map_err(|e| format!("Failed to create Claude client: {}", e))?;

            Ok(ResolvedProvider::Claude {
                client,
                model_id: model_id.to_string(),
                token,
            })
        }
        ProviderId::ChatGpt => {
            let token = auth::select_token(providers::CHATGPT).ok_or_else(|| {
                "ChatGPT authentication required. Please authenticate in Settings.".to_string()
            })?;
            let id_token = token.id_token.clone().ok_or_else(|| {
                "ChatGPT id_token not found. Please re-authenticate in Settings.".to_string()
            })?;

            let client = ChatGptOAuthClient::from_tokens(&token.access_token, &id_token)
                .map_err(|e| format!("Failed to create ChatGPT client: {}", e))?;

            Ok(ResolvedProvider::ChatGpt {
                client,
                model_id: model_id.to_string(),
                token,
            })
        }
        ProviderId::Gemini => {
            let token = auth::select_token(providers::GEMINI).ok_or_else(|| {
                "Gemini authentication required. Please authenticate in Settings.".to_string()
            })?;

            let client = GeminiCodeAssistRigClient::new(token.access_token.clone());

            Ok(ResolvedProvider::Gemini {
                client,
                model_id: model_id.to_string(),
                token,
            })
        }
        ProviderId::ApiKey(provider_id) => {
            let provider_def = RegistryService::find_provider(&provider_id)
                .ok_or_else(|| format!("Unknown provider: {}", provider_id))?;

            if !provider_def.is_openai_compatible {
                return Err(format!(
                    "{} API key provider requires special handling not yet implemented",
                    provider_def.name
                ));
            }

            let api_key_token = auth::select_api_key(&provider_id).ok_or_else(|| {
                format!(
                    "{} API key required. Please add an API key in Settings.",
                    provider_def.name
                )
            })?;

            let client = OpenAICompatibleApiClient::new(&provider_id, &api_key_token.api_key)?;

            Ok(ResolvedProvider::ApiKey {
                client,
                model_id: model_id.to_string(),
                provider_name: provider_def.name.clone(),
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
