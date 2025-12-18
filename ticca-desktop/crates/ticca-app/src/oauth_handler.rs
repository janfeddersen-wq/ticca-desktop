//! OAuth authentication handlers
//!
//! Handles OAuth flows for various LLM providers.

use ticca_core::config::ConfigDatabase;
use ticca_core::OAuthToken;
use ticca_oauth::{ClaudeOAuth, ChatGptOAuth};

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
                // Gemini requires user's own credentials
                Err("Gemini OAuth requires setting GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET environment variables. Please set these and restart.".to_string())
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
                            let token = OAuthToken::new("chatgpt", &token_response.access_token)
                                .with_refresh_token(refresh)
                                .with_expires_at(expires_at_str);
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
