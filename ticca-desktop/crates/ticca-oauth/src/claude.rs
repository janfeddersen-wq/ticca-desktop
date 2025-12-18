//! Claude Code OAuth implementation
//!
//! This implements the OAuth flow for Claude Code (Anthropic).
//! Reference: https://docs.anthropic.com/en/docs/claude-code

use crate::common::{OAuthConfig, OAuthError, OAuthResult, TokenResponse, OAuthFlowState};
use crate::pkce::create_pkce_state;
use crate::callback_server::{find_available_port, wait_for_callback, build_redirect_uri};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use url::Url;

/// Claude OAuth configuration
const CLAUDE_AUTH_URL: &str = "https://claude.ai/oauth/authorize";
const CLAUDE_TOKEN_URL: &str = "https://console.anthropic.com/v1/oauth/token";
const CLAUDE_API_URL: &str = "https://api.anthropic.com";
const CLAUDE_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const CLAUDE_SCOPE: &str = "org:create_api_key user:profile user:inference";
const CLAUDE_REDIRECT_PATH: &str = "callback";
const DEFAULT_PORT_RANGE: (u16, u16) = (8765, 8795);
const DEFAULT_TIMEOUT_SECS: u64 = 180; // 3 minutes for OAuth flow

/// Model information returned by Claude API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeModel {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub context_window: Option<u64>,
    #[serde(default)]
    pub max_output_tokens: Option<u64>,
}

/// Claude OAuth client
pub struct ClaudeOAuth {
    config: OAuthConfig,
    client: Client,
}

impl Default for ClaudeOAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeOAuth {
    /// Create a new Claude OAuth client with default configuration
    pub fn new() -> Self {
        Self::with_config(OAuthConfig {
            client_id: CLAUDE_CLIENT_ID.to_string(),
            auth_url: CLAUDE_AUTH_URL.to_string(),
            token_url: CLAUDE_TOKEN_URL.to_string(),
            api_base_url: CLAUDE_API_URL.to_string(),
            scope: CLAUDE_SCOPE.to_string(),
            redirect_host: "localhost".to_string(),
            redirect_path: CLAUDE_REDIRECT_PATH.to_string(),
            callback_port_range: DEFAULT_PORT_RANGE,
            callback_timeout_secs: DEFAULT_TIMEOUT_SECS,
        })
    }
    
    /// Create a Claude OAuth client with custom configuration
    pub fn with_config(config: OAuthConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }
    
    /// Build the authorization URL
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
            .append_pair("code_challenge_method", "S256");
        
        Ok(url.to_string())
    }
    
    /// Start the OAuth flow and get a flow state
    pub fn start_flow(&self) -> OAuthResult<(OAuthFlowState, u16)> {
        let port = find_available_port(self.config.callback_port_range)?;
        let redirect_uri = build_redirect_uri(
            &self.config.redirect_host,
            port,
            &self.config.redirect_path,
        );
        
        let flow_state = create_pkce_state().with_redirect_uri(redirect_uri);
        
        Ok((flow_state, port))
    }
    
    /// Complete the full OAuth flow (opens browser, waits for callback)
    pub fn authorize(&self) -> OAuthResult<TokenResponse> {
        let (flow_state, port) = self.start_flow()?;
        let auth_url = self.build_auth_url(&flow_state)?;
        
        tracing::info!("Opening browser for Claude OAuth...");
        
        // Open the browser
        open::that(&auth_url)
            .map_err(|e| OAuthError::CallbackServerError(format!("Failed to open browser: {}", e)))?;
        
        // Wait for callback
        let callback = wait_for_callback(
            port,
            &self.config.redirect_path,
            &flow_state.state,
            Duration::from_secs(self.config.callback_timeout_secs),
        )?;
        
        // Exchange code for tokens (include state for verification)
        self.exchange_code_with_state(
            &callback.code,
            &flow_state.code_verifier,
            flow_state.redirect_uri.as_deref().unwrap(),
            Some(&flow_state.state),
        )
    }
    
    /// Exchange authorization code for tokens
    pub fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> OAuthResult<TokenResponse> {
        self.exchange_code_with_state(code, code_verifier, redirect_uri, None)
    }
    
    /// Exchange authorization code for tokens with optional state
    pub fn exchange_code_with_state(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
        state: Option<&str>,
    ) -> OAuthResult<TokenResponse> {
        // Use blocking client for simplicity in OAuth flow
        let client = reqwest::blocking::Client::new();
        
        // Build the JSON payload matching the Python implementation
        let mut payload = serde_json::json!({
            "grant_type": "authorization_code",
            "client_id": self.config.client_id,
            "code": code,
            "code_verifier": code_verifier,
            "redirect_uri": redirect_uri,
        });
        
        // Add state if provided
        if let Some(s) = state {
            payload["state"] = serde_json::Value::String(s.to_string());
        }
        
        tracing::debug!("Exchanging code for tokens at {}", self.config.token_url);
        tracing::debug!("Payload keys: {:?}", payload.as_object().map(|o| o.keys().collect::<Vec<_>>()));
        
        let response = client
            .post(&self.config.token_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("anthropic-beta", "oauth-2025-04-20")
            .json(&payload)
            .send()
            .map_err(OAuthError::HttpError)?;
        
        tracing::info!("Token exchange response: {}", response.status());
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            tracing::error!("Token exchange failed: {} - {}", status, body);
            return Err(OAuthError::TokenExchangeFailed(format!(
                "HTTP {}: {}", status, body
            )));
        }
        
        let token_response: TokenResponse = response.json()
            .map_err(|e| OAuthError::InvalidResponse(format!("Failed to parse token response: {}", e)))?;
        
        tracing::info!("Successfully obtained Claude access token");
        
        Ok(token_response)
    }
    
    /// Refresh an access token
    pub fn refresh_token(&self, refresh_token: &str) -> OAuthResult<TokenResponse> {
        let client = reqwest::blocking::Client::new();
        
        // Use JSON for refresh as well to match the token exchange
        let payload = serde_json::json!({
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
            "client_id": self.config.client_id,
        });
        
        tracing::debug!("Refreshing Claude access token");
        
        let response = client
            .post(&self.config.token_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("anthropic-beta", "oauth-2025-04-20")
            .json(&payload)
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
        
        tracing::info!("Successfully refreshed Claude access token");
        
        Ok(token_response)
    }
    
    /// Fetch available models using the access token
    pub async fn fetch_models(&self, access_token: &str) -> OAuthResult<Vec<ClaudeModel>> {
        let url = format!("{}/v1/models", self.config.api_base_url);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .header("anthropic-beta", "oauth-2025-04-20")
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
            .map_err(OAuthError::HttpError)?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(OAuthError::InvalidResponse(format!(
                "Failed to fetch models: HTTP {}: {}", status, body
            )));
        }
        
        #[derive(Deserialize)]
        struct ModelsResponse {
            data: Vec<ClaudeModel>,
        }
        
        let models_response: ModelsResponse = response.json().await
            .map_err(|e| OAuthError::InvalidResponse(format!("Failed to parse models response: {}", e)))?;
        
        Ok(models_response.data)
    }
    
    /// Validate an access token by making a simple API call
    pub async fn validate_token(&self, access_token: &str) -> OAuthResult<bool> {
        let url = format!("{}/v1/models", self.config.api_base_url);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .header("anthropic-beta", "oauth-2025-04-20")
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
            .map_err(OAuthError::HttpError)?;
        
        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_start_flow() {
        let oauth = ClaudeOAuth::new();
        let (flow_state, port) = oauth.start_flow().unwrap();
        
        assert!(!flow_state.state.is_empty());
        assert!(!flow_state.code_verifier.is_empty());
        assert!(!flow_state.code_challenge.is_empty());
        assert!(flow_state.redirect_uri.is_some());
        assert!(port >= DEFAULT_PORT_RANGE.0 && port <= DEFAULT_PORT_RANGE.1);
    }
    
    #[test]
    fn test_build_auth_url() {
        let oauth = ClaudeOAuth::new();
        let (flow_state, _) = oauth.start_flow().unwrap();
        let auth_url = oauth.build_auth_url(&flow_state).unwrap();
        
        assert!(auth_url.contains(CLAUDE_AUTH_URL));
        assert!(auth_url.contains("response_type=code"));
        assert!(auth_url.contains(&format!("client_id={}", CLAUDE_CLIENT_ID)));
        assert!(auth_url.contains("code_challenge="));
        assert!(auth_url.contains("code_challenge_method=S256"));
    }
}
