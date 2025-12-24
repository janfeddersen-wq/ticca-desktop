//! Model discovery and aggregation across providers

use crate::config::ApiKeyProvider;
use crate::llm::ClaudeClient;
use crate::llm::auth;
use crate::llm::provider_registry::ProviderId;
use serde::Deserialize;
use ticca_oauth::{ChatGptOAuth, GeminiOAuth};

pub struct ModelService;

/// Response from OpenAI-compatible /models endpoint
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelInfo>,
}

#[derive(Debug, Deserialize)]
struct ModelInfo {
    id: String,
}

impl ModelService {
    /// Fetch models from an OpenAI-compatible API endpoint
    async fn fetch_openai_compatible_models(
        base_url: &str,
        api_key: &str,
        provider_name: &str,
    ) -> Result<Vec<String>, String> {
        let client = reqwest::Client::new();
        let url = format!("{}/models", base_url.trim_end_matches('/'));

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| format!("Failed to fetch {} models: {}", provider_name, e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Failed to fetch {} models: HTTP {}",
                provider_name,
                response.status()
            ));
        }

        let models_response: ModelsResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse {} models response: {}", provider_name, e))?;

        // Filter to only include text/chat models (exclude embedding, audio, etc.)
        let models: Vec<String> = models_response
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
            .map(|m| format!("{} - {}", m.id, provider_name))
            .collect();

        Ok(models)
    }

    pub async fn fetch_all() -> Result<Vec<String>, String> {
        let mut all_models: Vec<String> = Vec::new();

        // Fetch from OAuth providers
        if let Some(token) = auth::select_token(crate::config::models::providers::CLAUDE) {
            let client = ClaudeClient::new(token.access_token);
            if let Ok(models) = client.fetch_latest_models().await {
                // Add provider suffix for OAuth Claude models
                all_models.extend(models.into_iter().map(|m| format!("{} - Claude (OAuth)", m)));
            }
        }

        if let Some(token) = auth::select_token(crate::config::models::providers::GEMINI) {
            let oauth = GeminiOAuth::new();
            match oauth.fetch_models(&token.access_token, None).await {
                Ok(models) => {
                    all_models.extend(
                        models
                            .into_iter()
                            .map(|m| format!("{} - Gemini (OAuth)", m.name)),
                    );
                }
                Err(e) => {
                    tracing::warn!("Could not fetch Gemini models: {}", e);
                    all_models.extend(vec![
                        "gemini-2.0-flash-exp - Gemini (OAuth)".to_string(),
                        "gemini-1.5-pro - Gemini (OAuth)".to_string(),
                        "gemini-1.5-flash - Gemini (OAuth)".to_string(),
                    ]);
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
                    all_models.extend(
                        models
                            .into_iter()
                            .map(|m| format!("{} - ChatGPT (OAuth)", m.id)),
                    );
                }
                Err(e) => {
                    tracing::warn!("Could not fetch ChatGPT models: {}", e);
                    all_models.extend(vec![
                        "gpt-4o - ChatGPT (OAuth)".to_string(),
                        "gpt-4o-mini - ChatGPT (OAuth)".to_string(),
                        "o1-preview - ChatGPT (OAuth)".to_string(),
                    ]);
                }
            }
        }

        // Fetch from API key providers
        for provider in ApiKeyProvider::ALL {
            if let Some(api_key_token) = auth::select_api_key(provider.id()) {
                let provider_name = provider.display_name();

                // Skip providers with non-standard APIs
                if !provider.is_openai_compatible() {
                    // For Anthropic, use their specific API
                    if *provider == ApiKeyProvider::Anthropic {
                        // Anthropic doesn't have a /models endpoint, use known models
                        all_models.extend(vec![
                            format!("claude-sonnet-4-20250514 - {}", provider_name),
                            format!("claude-3-5-sonnet-20241022 - {}", provider_name),
                            format!("claude-3-5-haiku-20241022 - {}", provider_name),
                            format!("claude-3-opus-20240229 - {}", provider_name),
                        ]);
                    }
                    continue;
                }

                match Self::fetch_openai_compatible_models(
                    provider.base_url(),
                    &api_key_token.api_key,
                    provider_name,
                )
                .await
                {
                    Ok(models) => {
                        tracing::info!(
                            "Fetched {} models from {}",
                            models.len(),
                            provider_name
                        );
                        all_models.extend(models);
                    }
                    Err(e) => {
                        tracing::warn!("Could not fetch {} models: {}", provider_name, e);
                    }
                }
            }
        }

        if all_models.is_empty() {
            Err("No models found from any provider".to_string())
        } else {
            Ok(all_models)
        }
    }

    pub async fn fetch_for(provider: ProviderId) -> Result<Vec<String>, String> {
        match provider {
            ProviderId::Claude => {
                let token = auth::select_token(crate::config::models::providers::CLAUDE)
                    .ok_or_else(|| "Claude authentication required".to_string())?;
                let client = ClaudeClient::new(token.access_token);
                client
                    .fetch_latest_models()
                    .await
                    .map_err(|e| e.to_string())
            }
            ProviderId::Gemini => {
                let token = auth::select_token(crate::config::models::providers::GEMINI)
                    .ok_or_else(|| "Gemini authentication required".to_string())?;
                let oauth = GeminiOAuth::new();
                match oauth.fetch_models(&token.access_token, None).await {
                    Ok(models) => Ok(models.into_iter().map(|m| m.name).collect()),
                    Err(e) => {
                        tracing::warn!("Could not fetch Gemini models ({}), using defaults", e);
                        Ok(vec![
                            "gemini-2.0-flash-exp".to_string(),
                            "gemini-1.5-pro".to_string(),
                            "gemini-1.5-flash".to_string(),
                            "gemini-1.0-pro".to_string(),
                        ])
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
                    Ok(models) => Ok(models.into_iter().map(|m| m.id).collect()),
                    Err(e) => {
                        tracing::warn!("Could not fetch ChatGPT models ({}), using defaults", e);
                        Ok(vec![
                            "gpt-4o".to_string(),
                            "gpt-4o-mini".to_string(),
                            "gpt-4-turbo".to_string(),
                            "gpt-4".to_string(),
                            "gpt-3.5-turbo".to_string(),
                            "o1-preview".to_string(),
                            "o1-mini".to_string(),
                        ])
                    }
                }
            }
            ProviderId::ApiKey(api_provider) => {
                let api_key_token = auth::select_api_key(api_provider.id())
                    .ok_or_else(|| format!("{} API key required", api_provider.display_name()))?;

                if api_provider.is_openai_compatible() {
                    Self::fetch_openai_compatible_models(
                        api_provider.base_url(),
                        &api_key_token.api_key,
                        api_provider.display_name(),
                    )
                    .await
                } else if api_provider == ApiKeyProvider::Anthropic {
                    Ok(vec![
                        format!("claude-sonnet-4-20250514 - {}", api_provider.display_name()),
                        format!("claude-3-5-sonnet-20241022 - {}", api_provider.display_name()),
                        format!("claude-3-5-haiku-20241022 - {}", api_provider.display_name()),
                        format!("claude-3-opus-20240229 - {}", api_provider.display_name()),
                    ])
                } else {
                    Err(format!("{} does not support model listing", api_provider.display_name()))
                }
            }
        }
    }
}
