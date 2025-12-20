//! OAuth authentication handlers
//!
//! Handles OAuth flows for various LLM providers.

use ticca_core::OAuthToken;
use ticca_core::config::ConfigDatabase;
use ticca_core::config::OAuthAccount;
use ticca_oauth::{ChatGptOAuth, ClaudeOAuth, GeminiOAuth};
use uuid::Uuid;

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
                            let mut token = OAuthToken::new("claude", &token_response.access_token);
                            if let Some(refresh_token) = token_response
                                .refresh_token
                                .clone()
                                .filter(|token| !token.trim().is_empty())
                            {
                                token = token.with_refresh_token(refresh_token);
                            }
                            if let Some(expires_at) = token_response.expires_at_rfc3339() {
                                token = token.with_expires_at(expires_at);
                            }
                            if let Some(scope) = token_response.scope.clone() {
                                token = token.with_scope(scope);
                            }
                            let _ = db.upsert_oauth_token(&token);

                            let mut account = OAuthAccount::new(
                                Uuid::new_v4().to_string(),
                                "claude",
                                &token_response.access_token,
                            );
                            if let Some(refresh_token) = token_response
                                .refresh_token
                                .clone()
                                .filter(|token| !token.trim().is_empty())
                            {
                                account = account.with_refresh_token(refresh_token);
                            }
                            if let Some(expires_at) = token_response.expires_at_rfc3339() {
                                account = account.with_expires_at(expires_at);
                            }
                            if let Some(scope) = token_response.scope.clone() {
                                account = account.with_scope(scope);
                            }
                            let _ = db.upsert_oauth_account(&account);
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
                            let mut token = OAuthToken::new("gemini", &token_response.access_token);
                            if let Some(refresh_token) = token_response
                                .refresh_token
                                .clone()
                                .filter(|token| !token.trim().is_empty())
                            {
                                token = token.with_refresh_token(refresh_token);
                            }
                            if let Some(expires_at) = token_response.expires_at_rfc3339() {
                                token = token.with_expires_at(expires_at);
                            }
                            if let Some(scope) = token_response.scope.clone() {
                                token = token.with_scope(scope);
                            }
                            let _ = db.upsert_oauth_token(&token);

                            let mut account = OAuthAccount::new(
                                Uuid::new_v4().to_string(),
                                "gemini",
                                &token_response.access_token,
                            );
                            if let Some(refresh_token) = token_response
                                .refresh_token
                                .clone()
                                .filter(|token| !token.trim().is_empty())
                            {
                                account = account.with_refresh_token(refresh_token);
                            }
                            if let Some(expires_at) = token_response.expires_at_rfc3339() {
                                account = account.with_expires_at(expires_at);
                            }
                            if let Some(scope) = token_response.scope.clone() {
                                account = account.with_scope(scope);
                            }
                            let _ = db.upsert_oauth_account(&account);
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
                            // Store id_token in extra_json for later use (needed for ChatGPT API)
                            let extra = token_response.id_token().map(|id_token| {
                                serde_json::json!({"id_token": id_token}).to_string()
                            });
                            let mut token =
                                OAuthToken::new("chatgpt", &token_response.access_token);
                            if let Some(refresh_token) = token_response
                                .refresh_token
                                .clone()
                                .filter(|token| !token.trim().is_empty())
                            {
                                token = token.with_refresh_token(refresh_token);
                            }
                            if let Some(expires_at) = token_response.expires_at_rfc3339() {
                                token = token.with_expires_at(expires_at);
                            }
                            if let Some(scope) = token_response.scope.clone() {
                                token = token.with_scope(scope);
                            }
                            if let Some(extra_json) = extra {
                                token = token.with_extra(extra_json);
                            }
                            let _ = db.upsert_oauth_token(&token);

                            let mut account = OAuthAccount::new(
                                Uuid::new_v4().to_string(),
                                "chatgpt",
                                &token_response.access_token,
                            );
                            if let Some(refresh_token) = token_response
                                .refresh_token
                                .clone()
                                .filter(|token| !token.trim().is_empty())
                            {
                                account = account.with_refresh_token(refresh_token);
                            }
                            if let Some(expires_at) = token_response.expires_at_rfc3339() {
                                account = account.with_expires_at(expires_at);
                            }
                            if let Some(scope) = token_response.scope.clone() {
                                account = account.with_scope(scope);
                            }
                            if let Some(extra_json) = token_response.id_token() {
                                let extra_json =
                                    serde_json::json!({"id_token": extra_json}).to_string();
                                account = account.with_extra(extra_json);
                            }
                            let _ = db.upsert_oauth_account(&account);
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
