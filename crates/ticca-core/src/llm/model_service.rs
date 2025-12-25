//! Model discovery, aggregation, and persistence across providers
//!
//! This module provides:
//! - Model discovery from all configured providers (OAuth and API key)
//! - Persistence of discovered models to config.db
//! - Filtering of models based on available providers
//! - Context length tracking for each model

use crate::config::models::DiscoveredModel;
use crate::config::ConfigDatabase;
use crate::llm::ClaudeClient;
use crate::llm::auth;
use crate::llm::provider_registry::{ModelId, ProviderId};
use crate::registry::RegistryService;
use serde::Deserialize;
use ticca_oauth::{ChatGptOAuth, GeminiOAuth};

pub struct ModelService;

/// Response from OpenAI-compatible /models endpoint
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<OpenAIModelInfo>,
}

#[derive(Debug, Deserialize)]
struct OpenAIModelInfo {
    id: String,
    #[serde(default)]
    context_window: Option<i64>,
}

impl ModelService {
    /// Fetch models from an OpenAI-compatible API endpoint
    async fn fetch_openai_compatible_models(
        base_url: &str,
        api_key: &str,
        provider_id: &str,
    ) -> Result<Vec<DiscoveredModel>, String> {
        let client = reqwest::Client::new();
        let url = format!("{}/models", base_url.trim_end_matches('/'));

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| format!("Failed to fetch models: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch models: HTTP {}", response.status()));
        }

        let models_response: ModelsResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse models response: {}", e))?;

        // Filter to only include text/chat models (exclude embedding, audio, etc.)
        let models: Vec<DiscoveredModel> = models_response
            .data
            .into_iter()
            .filter(|m| {
                let id = m.id.to_lowercase();
                // Include chat/text models, exclude embeddings, whisper, tts, dall-e, etc.
                !id.contains("embed")
                    && !id.contains("whisper")
                    && !id.contains("tts")
                    && !id.contains("dall-e")
                    && !id.contains("moderation")
            })
            .map(|m| {
                let context_length = m.context_window
                    .unwrap_or_else(|| crate::compression::get_model_context_window(&m.id) as i64);
                DiscoveredModel::new(provider_id, &m.id)
                    .with_context_length(context_length)
            })
            .collect();

        Ok(models)
    }

    /// Fetch all models from all available providers and persist to database
    pub async fn fetch_all() -> Result<Vec<String>, String> {
        let models = Self::fetch_all_discovered().await?;
        Ok(models.into_iter().map(|m| m.canonical_id).collect())
    }

    /// Fetch all models from all available providers as DiscoveredModel
    pub async fn fetch_all_discovered() -> Result<Vec<DiscoveredModel>, String> {
        let mut all_models: Vec<DiscoveredModel> = Vec::new();

        // Fetch from OAuth providers
        if let Some(token) = auth::select_token(crate::config::models::providers::CLAUDE) {
            let client = ClaudeClient::new(token.access_token);
            if let Ok(models) = client.fetch_latest_models().await {
                let discovered: Vec<DiscoveredModel> = models
                    .into_iter()
                    .map(|m| {
                        let context_length = crate::compression::get_model_context_window(&m) as i64;
                        DiscoveredModel::new("claude", &m).with_context_length(context_length)
                    })
                    .collect();
                all_models.extend(discovered);
            }
        }

        if let Some(token) = auth::select_token(crate::config::models::providers::GEMINI) {
            let oauth = GeminiOAuth::new();
            match oauth.fetch_models(&token.access_token, None).await {
                Ok(models) => {
                    let discovered: Vec<DiscoveredModel> = models
                        .into_iter()
                        .map(|m| {
                            let context_length = m.input_token_limit.unwrap_or(
                                crate::compression::get_model_context_window(&m.name)
                            ) as i64;
                            DiscoveredModel::new("gemini", &m.name)
                                .with_context_length(context_length)
                        })
                        .collect();
                    all_models.extend(discovered);
                }
                Err(e) => {
                    tracing::warn!("Could not fetch Gemini models: {}", e);
                    // Use default fallback models
                    all_models.extend(Self::default_gemini_models());
                }
            }
        }

        if let Some(token) = auth::select_token(crate::config::models::providers::CHATGPT) {
            let oauth = ChatGptOAuth::new();
            match oauth
                .fetch_models(&token.access_token, token.id_token.as_deref())
                .await
            {
                Ok(models) => {
                    let discovered: Vec<DiscoveredModel> = models
                        .into_iter()
                        .map(|m| {
                            let context_length = crate::compression::get_model_context_window(&m.id) as i64;
                            DiscoveredModel::new("chatgpt", &m.id)
                                .with_context_length(context_length)
                        })
                        .collect();
                    all_models.extend(discovered);
                }
                Err(e) => {
                    tracing::warn!("Could not fetch ChatGPT models: {}", e);
                    all_models.extend(Self::default_chatgpt_models());
                }
            }
        }

        // Fetch from API key providers (using registry)
        for provider in RegistryService::api_key_providers() {
            let has_key = auth::select_api_key(&provider.id);
            tracing::debug!(
                "Checking provider {}: has_api_key={}",
                provider.id,
                has_key.is_some()
            );
            if let Some(api_key_token) = has_key {
                // Skip providers with non-standard APIs
                if !provider.is_openai_compatible {
                    // For Anthropic, use known models
                    if provider.id == "anthropic" {
                        all_models.extend(Self::default_anthropic_models(&provider.id));
                    }
                    continue;
                }

                match Self::fetch_openai_compatible_models(
                    &provider.api_base_url,
                    &api_key_token.api_key,
                    &provider.id,
                )
                .await
                {
                    Ok(models) => {
                        tracing::info!(
                            "Fetched {} models from {}",
                            models.len(),
                            provider.name
                        );
                        all_models.extend(models);
                    }
                    Err(e) => {
                        tracing::warn!("Could not fetch {} models: {}", provider.name, e);
                    }
                }
            }
        }

        if all_models.is_empty() {
            return Err("No models found from any provider".to_string());
        }

        // Persist to database
        if let Err(e) = Self::persist_models(&all_models) {
            tracing::warn!("Failed to persist discovered models: {}", e);
        }

        Ok(all_models)
    }

    /// Fetch models for a specific provider
    pub async fn fetch_for(provider: ProviderId) -> Result<Vec<String>, String> {
        let models = Self::fetch_for_discovered(provider).await?;
        Ok(models.into_iter().map(|m| m.canonical_id).collect())
    }

    /// Fetch models for a specific provider as DiscoveredModel
    pub async fn fetch_for_discovered(provider: ProviderId) -> Result<Vec<DiscoveredModel>, String> {
        let models = match provider {
            ProviderId::Claude => {
                let token = auth::select_token(crate::config::models::providers::CLAUDE)
                    .ok_or_else(|| "Claude authentication required".to_string())?;
                let client = ClaudeClient::new(token.access_token);
                let model_names = client.fetch_latest_models().await.map_err(|e| e.to_string())?;
                model_names
                    .into_iter()
                    .map(|m| {
                        let context_length = crate::compression::get_model_context_window(&m) as i64;
                        DiscoveredModel::new("claude", &m).with_context_length(context_length)
                    })
                    .collect()
            }
            ProviderId::Gemini => {
                let token = auth::select_token(crate::config::models::providers::GEMINI)
                    .ok_or_else(|| "Gemini authentication required".to_string())?;
                let oauth = GeminiOAuth::new();
                match oauth.fetch_models(&token.access_token, None).await {
                    Ok(models) => models
                        .into_iter()
                        .map(|m| {
                            let context_length = m.input_token_limit.unwrap_or(
                                crate::compression::get_model_context_window(&m.name)
                            ) as i64;
                            DiscoveredModel::new("gemini", &m.name)
                                .with_context_length(context_length)
                        })
                        .collect(),
                    Err(e) => {
                        tracing::warn!("Could not fetch Gemini models ({}), using defaults", e);
                        Self::default_gemini_models()
                    }
                }
            }
            ProviderId::ChatGpt => {
                let token = auth::select_token(crate::config::models::providers::CHATGPT)
                    .ok_or_else(|| "ChatGPT authentication required".to_string())?;
                let oauth = ChatGptOAuth::new();
                match oauth
                    .fetch_models(&token.access_token, token.id_token.as_deref())
                    .await
                {
                    Ok(models) => models
                        .into_iter()
                        .map(|m| {
                            let context_length = crate::compression::get_model_context_window(&m.id) as i64;
                            DiscoveredModel::new("chatgpt", &m.id)
                                .with_context_length(context_length)
                        })
                        .collect(),
                    Err(e) => {
                        tracing::warn!("Could not fetch ChatGPT models ({}), using defaults", e);
                        Self::default_chatgpt_models()
                    }
                }
            }
            ProviderId::ApiKey(provider_id) => {
                let api_key_token = auth::select_api_key(&provider_id)
                    .ok_or_else(|| format!("{} API key required", provider_id))?;

                // Look up provider info from registry
                let provider_def = RegistryService::find_provider(&provider_id)
                    .ok_or_else(|| format!("Unknown provider: {}", provider_id))?;

                if provider_def.is_openai_compatible {
                    Self::fetch_openai_compatible_models(
                        &provider_def.api_base_url,
                        &api_key_token.api_key,
                        &provider_id,
                    )
                    .await?
                } else if provider_id == "anthropic" {
                    Self::default_anthropic_models(&provider_id)
                } else {
                    return Err(format!("{} does not support model listing", provider_def.name));
                }
            }
        };

        // Persist to database
        if let Err(e) = Self::persist_models(&models) {
            tracing::warn!("Failed to persist discovered models: {}", e);
        }

        Ok(models)
    }

    /// Get cached models from the database, filtered by available providers
    pub fn get_cached_models() -> Result<Vec<DiscoveredModel>, String> {
        let db = ConfigDatabase::open().map_err(|e| e.to_string())?;
        let all_models = db.list_discovered_models(None).map_err(|e| e.to_string())?;

        // Filter by available providers
        let available = auth::available_provider_ids();
        let filtered: Vec<DiscoveredModel> = all_models
            .into_iter()
            .filter(|m| available.contains(&m.provider))
            .collect();

        Ok(filtered)
    }

    /// Get cached canonical model IDs, filtered by available providers
    pub fn get_cached_model_ids() -> Result<Vec<String>, String> {
        let models = Self::get_cached_models()?;
        Ok(models.into_iter().map(|m| m.canonical_id).collect())
    }

    /// Look up context length for a model (from cache or fallback)
    pub fn get_context_length(model_id: &str) -> i64 {
        // Try to get from database first
        if let Ok(db) = ConfigDatabase::open() {
            // Handle both canonical format and just model name
            let canonical_id = if model_id.contains(':') {
                model_id.to_string()
            } else if let Some(parsed) = ModelId::parse(model_id) {
                parsed.canonical()
            } else {
                model_id.to_string()
            };

            if let Ok(Some(length)) = db.get_model_context_length(&canonical_id) {
                return length;
            }
        }

        // Fallback to hardcoded values
        crate::compression::get_model_context_window(model_id) as i64
    }

    /// Persist models to the database
    fn persist_models(models: &[DiscoveredModel]) -> Result<(), String> {
        let db = ConfigDatabase::open().map_err(|e| e.to_string())?;
        db.upsert_discovered_models_batch(models).map_err(|e| e.to_string())?;
        tracing::debug!("Persisted {} discovered models to database", models.len());
        Ok(())
    }

    // Default fallback models

    fn default_gemini_models() -> Vec<DiscoveredModel> {
        vec![
            DiscoveredModel::new("gemini", "gemini-2.0-flash-exp").with_context_length(1_000_000),
            DiscoveredModel::new("gemini", "gemini-1.5-pro").with_context_length(1_000_000),
            DiscoveredModel::new("gemini", "gemini-1.5-flash").with_context_length(1_000_000),
        ]
    }

    fn default_chatgpt_models() -> Vec<DiscoveredModel> {
        // ChatGPT OAuth Codex models - curated to GPT-5.x Codex models only
        // Limited to 270k context for Plus subscription
        vec![
            DiscoveredModel::new("chatgpt", "gpt-5.1-codex-max").with_context_length(270_000),
            DiscoveredModel::new("chatgpt", "gpt-5.1-codex").with_context_length(270_000),
            DiscoveredModel::new("chatgpt", "gpt-5.1-codex-mini").with_context_length(270_000),
            DiscoveredModel::new("chatgpt", "gpt-5.2-codex").with_context_length(270_000),
            DiscoveredModel::new("chatgpt", "gpt-5.2").with_context_length(270_000),
        ]
    }

    fn default_anthropic_models(provider_id: &str) -> Vec<DiscoveredModel> {
        vec![
            DiscoveredModel::new(provider_id, "claude-sonnet-4-20250514").with_context_length(200_000),
            DiscoveredModel::new(provider_id, "claude-3-5-sonnet-20241022").with_context_length(200_000),
            DiscoveredModel::new(provider_id, "claude-3-5-haiku-20241022").with_context_length(200_000),
            DiscoveredModel::new(provider_id, "claude-3-opus-20240229").with_context_length(200_000),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_id_parsing() {
        // Canonical format
        let id = ModelId::parse("openai:gpt-4o").unwrap();
        assert_eq!(id.provider, "openai");
        assert_eq!(id.model, "gpt-4o");

        // Legacy format
        let id = ModelId::parse("gpt-4o - OpenAI").unwrap();
        assert_eq!(id.provider, "openai");
        assert_eq!(id.model, "gpt-4o");
    }

    #[test]
    fn test_discovered_model_canonical_id() {
        let model = DiscoveredModel::new("openai", "gpt-4o");
        assert_eq!(model.canonical_id, "openai:gpt-4o");
        assert_eq!(model.provider, "openai");
        assert_eq!(model.model_id, "gpt-4o");
    }

}
