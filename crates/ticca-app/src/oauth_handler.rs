//! OAuth authentication handlers
//!
//! Handles OAuth flows for various LLM providers.

use ticca_core::config::ConfigService;
use ticca_core::config::models::providers;
use ticca_oauth::{ChatGptOAuth, ClaudeOAuth, GeminiOAuth};

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
                        let _ =
                            ConfigService::persist_token_response(providers::CLAUDE, token_response);
                        Ok(())
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            OAuthProvider::Gemini => {
                let oauth = GeminiOAuth::new();
                match oauth.authorize() {
                    Ok(token_response) => {
                        let _ =
                            ConfigService::persist_token_response(providers::GEMINI, token_response);
                        Ok(())
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            OAuthProvider::ChatGpt => {
                let oauth = ChatGptOAuth::new();
                match oauth.authorize() {
                    Ok(token_response) => {
                        let _ =
                            ConfigService::persist_token_response(providers::CHATGPT, token_response);
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
