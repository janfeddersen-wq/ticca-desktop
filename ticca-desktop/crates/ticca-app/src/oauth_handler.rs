//! OAuth authentication handlers
//!
//! Handles OAuth flows for various LLM providers.

use ticca_core::config::ConfigDatabase;
use ticca_core::OAuthToken;
use ticca_oauth::{ClaudeOAuth, ChatGptOAuth, GeminiOAuth};

use crate::messages::OAuthProvider;

/// Start the OAuth flow for a provider
/// Returns Ok(()) on success, Err(error_message) on failure
pub async fn start_oauth(provider: OAuthProvider) -> Result<(), String> {
    // Move the blocking OAuth flow to a dedicated blocking thread
    tokio::task::spawn_blocking(move || {
        match provider {
            OAuthProvider::Claude => {
                let oauth = ClaudeOAuth::new();
                match oauth.authorize() {
                    Ok(token_response) => {
                        // Save token to database
                        if let Ok(db) = ConfigDatabase::open() {
                            let expires_at_str = token_response.expires_at()
                                .map(|t| t.to_rfc3339())
                                .unwrap_or_default();
                            let refresh = token_response.refresh_token
                                .clone()
                                .unwrap_or_default();
                            let token = OAuthToken::new("claude", &token_response.access_token)
                                .with_refresh_token(refresh)
                                .with_expires_at(expires_at_str);
                            let _ = db.upsert_oauth_token(&token);
                        }
                        Ok(())
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            OAuthProvider::Gemini => {
                let oauth = GeminiOAuth::new();
                match oauth.authorize() {
                    Ok(token_response) => {
                        if let Ok(db) = ConfigDatabase::open() {
                            let expires_at_str = token_response.expires_at()
                                .map(|t| t.to_rfc3339())
                                .unwrap_or_default();
                            let refresh = token_response.refresh_token
                                .clone()
                                .unwrap_or_default();
                            let token = OAuthToken::new("gemini", &token_response.access_token)
                                .with_refresh_token(refresh)
                                .with_expires_at(expires_at_str);
                            let _ = db.upsert_oauth_token(&token);
                        }
                        Ok(())
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            OAuthProvider::ChatGpt => {
                let oauth = ChatGptOAuth::new();
                match oauth.authorize() {
                    Ok(token_response) => {
                        if let Ok(db) = ConfigDatabase::open() {
                            let expires_at_str = token_response.expires_at()
                                .map(|t| t.to_rfc3339())
                                .unwrap_or_default();
                            let refresh = token_response.refresh_token
                                .clone()
                                .unwrap_or_default();
                            // Store id_token in extra_json for later use (needed for ChatGPT API)
                            let extra = token_response.id_token()
                                .map(|id_token| serde_json::json!({"id_token": id_token}).to_string());
                            let mut token = OAuthToken::new("chatgpt", &token_response.access_token)
                                .with_refresh_token(refresh)
                                .with_expires_at(expires_at_str);
                            if let Some(extra_json) = extra {
                                token = token.with_extra(extra_json);
                            }
                            let _ = db.upsert_oauth_token(&token);
                        }
                        Ok(())
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
        }
    })
    .await
    .map_err(|e| format!("OAuth task panicked: {}", e))?
}
