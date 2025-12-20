//! Model discovery and aggregation across providers

use crate::llm::ClaudeClient;
use crate::llm::auth;
use crate::llm::provider_registry::ProviderId;
use ticca_oauth::{ChatGptOAuth, GeminiOAuth};

pub struct ModelService;

impl ModelService {
    pub async fn fetch_all() -> Result<Vec<String>, String> {
        let mut all_models: Vec<String> = Vec::new();

        if let Some(token) = auth::select_token(crate::config::models::providers::CLAUDE) {
            let client = ClaudeClient::new(token.access_token);
            if let Ok(models) = client.fetch_latest_models().await {
                all_models.extend(models);
            }
        }

        if let Some(token) = auth::select_token(crate::config::models::providers::GEMINI) {
            let oauth = GeminiOAuth::new();
            match oauth.fetch_models(&token.access_token, None).await {
                Ok(models) => all_models.extend(models.into_iter().map(|m| m.name)),
                Err(e) => {
                    tracing::warn!("Could not fetch Gemini models: {}", e);
                    all_models.extend(vec![
                        "gemini-2.0-flash-exp".to_string(),
                        "gemini-1.5-pro".to_string(),
                        "gemini-1.5-flash".to_string(),
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
                Ok(models) => all_models.extend(models.into_iter().map(|m| m.id)),
                Err(e) => {
                    tracing::warn!("Could not fetch ChatGPT models: {}", e);
                    all_models.extend(vec![
                        "gpt-4o".to_string(),
                        "gpt-4o-mini".to_string(),
                        "o1-preview".to_string(),
                    ]);
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
        }
    }
}
