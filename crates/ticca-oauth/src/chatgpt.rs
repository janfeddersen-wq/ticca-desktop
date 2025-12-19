//! ChatGPT (OpenAI) OAuth implementation
//!
//! This implements the OAuth flow for OpenAI's ChatGPT.
//! Reference: OpenAI OAuth documentation

use crate::common::{OAuthConfig, OAuthError, OAuthResult, TokenResponse, OAuthFlowState};
use crate::pkce::create_pkce_state;
use crate::callback_server::{wait_for_callback, build_redirect_uri};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::time::Duration;
use url::Url;

/// OpenAI OAuth configuration
const OPENAI_AUTH_URL: &str = "https://auth.openai.com/oauth/authorize";
const OPENAI_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const OPENAI_API_URL: &str = "https://api.openai.com";
/// ChatGPT backend API for models (requires ChatGPT-Account-Id header)
const CHATGPT_BACKEND_API: &str = "https://chatgpt.com/backend-api/codex";
const CODEX_CLIENT_VERSION: &str = "0.72.0";

/// Hardcoded Codex models for fallback when API fetch fails
/// Note: /models endpoint is behind Cloudflare bot protection, so we use fallback
/// Models based on llxprt-code and codex-rs
const CODEX_MODELS: &[&str] = &[
    "gpt-5.1-codex-max",
    "gpt-5.1-codex",
    "gpt-5.1-codex-mini",
    "gpt-5.2",
    "gpt-5.1",
    "gpt-4o",
    "gpt-4o-mini",
    "o1-preview",
    "o1-mini",
    "o3-mini",
];

// ChatGPT OAuth client ID (public, used by ChatGPT desktop apps)
const CHATGPT_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const CHATGPT_SCOPE: &str = "openid profile email offline_access";
const CHATGPT_REDIRECT_PATH: &str = "auth/callback";
const REQUIRED_PORT: u16 = 1455;
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Model information from OpenAI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIModel {
    pub id: String,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub created: Option<u64>,
    #[serde(default)]
    pub owned_by: Option<String>,
}

/// User info from OpenAI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIUserInfo {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub picture: Option<String>,
}

/// Model from ChatGPT backend API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatGPTModel {
    pub slug: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Extract account_id from JWT id_token
/// The account_id is in the claim: https://api.openai.com/auth -> chatgpt_account_id
fn extract_account_id_from_jwt(id_token: &str) -> Option<String> {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

    // JWT format: header.payload.signature
    let parts: Vec<&str> = id_token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }

    // Decode payload (second part)
    let payload = URL_SAFE_NO_PAD.decode(parts[1]).ok()?;
    let payload_str = String::from_utf8(payload).ok()?;

    // Parse JSON
    let claims: serde_json::Value = serde_json::from_str(&payload_str).ok()?;

    // Extract chatgpt_account_id from https://api.openai.com/auth claim
    claims.get("https://api.openai.com/auth")
        .and_then(|auth| auth.get("chatgpt_account_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// ChatGPT OAuth client
pub struct ChatGptOAuth {
    config: OAuthConfig,
    client: Client,
}

impl Default for ChatGptOAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatGptOAuth {
    /// Create a new ChatGPT OAuth client with default configuration
    pub fn new() -> Self {
        Self::with_config(
            OAuthConfig {
                client_id: CHATGPT_CLIENT_ID.to_string(),
                auth_url: OPENAI_AUTH_URL.to_string(),
                token_url: OPENAI_TOKEN_URL.to_string(),
                api_base_url: OPENAI_API_URL.to_string(),
                scope: CHATGPT_SCOPE.to_string(),
                redirect_host: "localhost".to_string(),
                redirect_path: CHATGPT_REDIRECT_PATH.to_string(),
                callback_port_range: (REQUIRED_PORT, REQUIRED_PORT), // Fixed port
                callback_timeout_secs: DEFAULT_TIMEOUT_SECS,
            },
        )
    }
    
    /// Create with custom configuration
    pub fn with_config(config: OAuthConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }
    
    /// Build the authorization URL (no audience parameter needed)
    pub fn build_auth_url(&self, flow_state: &OAuthFlowState) -> OAuthResult<String> {
        let redirect_uri = flow_state.redirect_uri.as_ref()
            .ok_or_else(|| OAuthError::InvalidResponse("redirect_uri not set in flow state".into()))?;
        
        let mut url = Url::parse(&self.config.auth_url)
            .map_err(|e| OAuthError::InvalidResponse(format!("Invalid auth URL: {}", e)))?;
        
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.config.client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("scope", &self.config.scope)
            .append_pair("state", &flow_state.state)
            .append_pair("code_challenge", &flow_state.code_challenge)
            .append_pair("code_challenge_method", "S256")
            // These parameters grant API access permissions for model listing
            .append_pair("id_token_add_organizations", "true")
            .append_pair("codex_cli_simplified_flow", "true")
            .append_pair("originator", "codex_cli_rs")
            .append_pair("theme", "dark");

        Ok(url.to_string())
    }
    
    /// Start the OAuth flow (uses fixed port 1455)
    pub fn start_flow(&self) -> OAuthResult<(OAuthFlowState, u16)> {
        // ChatGPT OAuth requires port 1455 specifically
        let port = REQUIRED_PORT;
        
        // Check if port 1455 is available
        if TcpListener::bind(("127.0.0.1", port)).is_err() {
            return Err(OAuthError::CallbackServerError(format!(
                "Port {} is not available. ChatGPT OAuth requires this specific port. \
                 Please close any application using port {} and try again.",
                port, port
            )));
        }
        
        let redirect_uri = build_redirect_uri(
            &self.config.redirect_host,
            port,
            &self.config.redirect_path,
        );
        
        let flow_state = create_pkce_state().with_redirect_uri(redirect_uri);
        
        Ok((flow_state, port))
    }
    
    /// Complete the full OAuth flow
    pub fn authorize(&self) -> OAuthResult<TokenResponse> {
        let (flow_state, port) = self.start_flow()?;
        let auth_url = self.build_auth_url(&flow_state)?;
        
        tracing::info!("Opening browser for OpenAI OAuth...");
        
        open::that(&auth_url)
            .map_err(|e| OAuthError::CallbackServerError(format!("Failed to open browser: {}", e)))?;
        
        let callback = wait_for_callback(
            port,
            &self.config.redirect_path,
            &flow_state.state,
            Duration::from_secs(self.config.callback_timeout_secs),
        )?;
        
        self.exchange_code(
            &callback.code,
            &flow_state.code_verifier,
            flow_state.redirect_uri.as_deref().unwrap(),
        )
    }
    
    /// Exchange authorization code for tokens
    pub fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> OAuthResult<TokenResponse> {
        let client = reqwest::blocking::Client::new();
        
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", &self.config.client_id),
            ("code_verifier", code_verifier),
        ];
        
        tracing::debug!("Exchanging code for tokens at {}", self.config.token_url);
        
        let response = client
            .post(&self.config.token_url)
            .form(&params)
            .send()
            .map_err(OAuthError::HttpError)?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(OAuthError::TokenExchangeFailed(format!(
                "HTTP {}: {}", status, body
            )));
        }
        
        let token_response: TokenResponse = response.json()
            .map_err(|e| OAuthError::InvalidResponse(format!("Failed to parse token response: {}", e)))?;
        
        tracing::info!("Successfully obtained OpenAI access token");
        
        Ok(token_response)
    }
    
    /// Refresh an access token
    pub fn refresh_token(&self, refresh_token: &str) -> OAuthResult<TokenResponse> {
        let client = reqwest::blocking::Client::new();
        
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.config.client_id),
        ];
        
        tracing::debug!("Refreshing OpenAI access token");
        
        let response = client
            .post(&self.config.token_url)
            .form(&params)
            .send()
            .map_err(OAuthError::HttpError)?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(OAuthError::TokenRefreshFailed(format!(
                "HTTP {}: {}", status, body
            )));
        }
        
        let token_response: TokenResponse = response.json()
            .map_err(|e| OAuthError::InvalidResponse(format!("Failed to parse refresh response: {}", e)))?;
        
        tracing::info!("Successfully refreshed OpenAI access token");
        
        Ok(token_response)
    }
    
    /// Fetch available models using the access token and id_token
    /// Uses ChatGPT backend API which requires ChatGPT-Account-ID header
    /// Note: The Codex /models endpoint is protected by Cloudflare bot detection
    /// which blocks automated requests. Therefore, we use hardcoded models as fallback.
    pub async fn fetch_models(&self, access_token: &str, id_token: Option<&str>) -> OAuthResult<Vec<OpenAIModel>> {
        // Try to extract account_id from id_token
        let account_id = id_token.and_then(extract_account_id_from_jwt);

        if let Some(ref account_id) = account_id {
            // Use ChatGPT backend API with account_id
            let url = format!("{}/models?client_version={}", CHATGPT_BACKEND_API, CODEX_CLIENT_VERSION);

            tracing::info!("Fetching models from ChatGPT backend API with account_id");

            let response = self.client
                .get(&url)
                .header("Authorization", format!("Bearer {}", access_token))
                .header("ChatGPT-Account-Id", account_id)
                .header("originator", "codex_cli_rs")
                .header("User-Agent", format!("codex_cli_rs/{} (Linux; x86_64)", CODEX_CLIENT_VERSION))
                .send()
                .await
                .map_err(OAuthError::HttpError)?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                tracing::warn!("ChatGPT backend API failed (Cloudflare may be blocking): {} - {}", status, body);
                // Fall through to hardcoded models
            } else {
                #[derive(Deserialize)]
                struct ChatGPTModelsResponse {
                    models: Vec<ChatGPTModelInfo>,
                }

                #[derive(Deserialize)]
                struct ChatGPTModelInfo {
                    slug: String,
                    #[allow(dead_code)]
                    #[serde(default)]
                    name: Option<String>,
                }

                if let Ok(models_response) = response.json::<ChatGPTModelsResponse>().await {
                    let models: Vec<OpenAIModel> = models_response.models
                        .into_iter()
                        .map(|m| OpenAIModel {
                            id: m.slug,
                            object: "model".to_string(),
                            created: None,
                            owned_by: Some("openai".to_string()),
                        })
                        .collect();

                    tracing::info!("Fetched {} models from ChatGPT backend API", models.len());
                    return Ok(models);
                }
            }
        }

        // Cloudflare blocks the /models endpoint, so use hardcoded models
        // This is the same approach used by llxprt-code
        tracing::info!("Using hardcoded Codex models (API blocked by Cloudflare)");
        let models: Vec<OpenAIModel> = CODEX_MODELS
            .iter()
            .map(|id| OpenAIModel {
                id: id.to_string(),
                object: "model".to_string(),
                created: None,
                owned_by: Some("openai".to_string()),
            })
            .collect();

        Ok(models)
    }
    
    /// Validate an access token by checking the models endpoint
    pub async fn validate_token(&self, access_token: &str) -> OAuthResult<bool> {
        let url = format!("{}/v1/models", self.config.api_base_url);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(OAuthError::HttpError)?;
        
        Ok(response.status().is_success())
    }
    
    /// Get user info (if available)
    pub async fn get_user_info(&self, access_token: &str) -> OAuthResult<OpenAIUserInfo> {
        // OpenAI doesn't have a standard userinfo endpoint for OAuth tokens,
        // but we can try the OIDC userinfo endpoint
        let url = "https://auth.openai.com/userinfo";
        
        let response = self.client
            .get(url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(OAuthError::HttpError)?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(OAuthError::InvalidResponse(format!(
                "Failed to get user info: HTTP {}: {}", status, body
            )));
        }
        
        let user_info: OpenAIUserInfo = response.json().await
            .map_err(|e| OAuthError::InvalidResponse(format!("Failed to parse user info: {}", e)))?;
        
        Ok(user_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_start_flow() {
        let oauth = ChatGptOAuth::new();
        // Note: This test may fail if port 1455 is in use
        match oauth.start_flow() {
            Ok((flow_state, port)) => {
                assert!(!flow_state.state.is_empty());
                assert!(!flow_state.code_verifier.is_empty());
                assert!(!flow_state.code_challenge.is_empty());
                assert!(flow_state.redirect_uri.is_some());
                assert_eq!(port, REQUIRED_PORT);
            }
            Err(OAuthError::CallbackServerError(_)) => {
                // Port 1455 is in use, which is expected in some environments
                // Skip this test in that case
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
    
    #[test]
    fn test_build_auth_url() {
        let oauth = ChatGptOAuth::new();
        // Create a flow state manually to avoid port binding issues
        let flow_state = create_pkce_state()
            .with_redirect_uri(format!("http://localhost:{}/auth/callback", REQUIRED_PORT));
        let auth_url = oauth.build_auth_url(&flow_state).unwrap();
        
        assert!(auth_url.contains(OPENAI_AUTH_URL));
        assert!(auth_url.contains("response_type=code"));
        assert!(auth_url.contains(&format!("client_id={}", CHATGPT_CLIENT_ID)));
        assert!(auth_url.contains("code_challenge="));
        assert!(auth_url.contains("code_challenge_method=S256"));
        // No audience parameter
        assert!(!auth_url.contains("audience="));
        // Correct redirect path
        assert!(auth_url.contains("auth%2Fcallback") || auth_url.contains("auth/callback"));
    }
}
