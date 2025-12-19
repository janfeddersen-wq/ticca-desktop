//! Gemini (Google) OAuth implementation
//!
//! This implements the OAuth flow for Google's Gemini Code Assist API.
//! Reference: Google Cloud OAuth 2.0
//! Uses the same OAuth credentials as gemini-cli (Desktop app type - public client secret)
//!
//! # ⚠️ IMPORTANT: API Compatibility Limitation
//!
//! The gemini-cli OAuth client ID is ONLY registered for these scopes:
//! - `cloud-platform` (for Cloud Code Assist API)
//! - `userinfo.email`
//! - `userinfo.profile`
//!
//! This means tokens obtained via this OAuth flow can ONLY be used with:
//! - **Google Cloud Code Assist API** (`cloudcode-pa.googleapis.com`)
//!
//! These tokens CANNOT be used with:
//! - **Generative Language API** (`generativelanguage.googleapis.com`) - requires `generative-language` scope
//!   which is NOT registered for the gemini-cli OAuth client ID
//!
//! ## For users who want to use Gemini:
//!
//! 1. **Option A (Recommended)**: Use a Gemini API key from https://aistudio.google.com/
//!    - Works with the standard Generative Language API
//!    - No OAuth required
//!
//! 2. **Option B**: Use Claude or ChatGPT which have full OAuth support
//!
//! 3. **Option C (Future)**: We could implement Cloud Code Assist API support,
//!    but this would require significant changes to the LLM client layer.
//!
//! ## Why this limitation exists:
//!
//! Google's OAuth client registration controls which scopes a client can request.
//! The gemini-cli client ID was registered for Cloud Code Assist, not the
//! Generative Language API. We cannot add new scopes without Google's approval.

use crate::common::{OAuthConfig, OAuthError, OAuthResult, TokenResponse, OAuthFlowState};
use crate::pkce::create_pkce_state;
use crate::callback_server::{find_available_port, wait_for_callback, build_redirect_uri};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use url::Url;

/// Google OAuth configuration
const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GEMINI_API_URL: &str = "https://cloudcode-pa.googleapis.com";

/// OAuth credentials from gemini-cli (Desktop app type - public client secret is safe to embed)
/// Reference: https://github.com/google-gemini/gemini-cli
const GEMINI_CLIENT_ID: &str = "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com";
const GEMINI_CLIENT_SECRET: &str = "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";

/// Scopes required for Gemini API access
///
/// ⚠️ CRITICAL: These scopes MUST match exactly what the gemini-cli OAuth client ID
/// is registered for. Adding additional scopes will fail with "access_denied" errors.
///
/// The gemini-cli client is registered for:
/// - `cloud-platform` - Access to Cloud Code Assist API
/// - `userinfo.email` - User email for identification
/// - `userinfo.profile` - Basic user profile info
///
/// NOT available (not registered):
/// - `generative-language` - Required for generativelanguage.googleapis.com API
///
/// This means OAuth tokens from this flow can ONLY be used with:
/// - Cloud Code Assist API (`cloudcode-pa.googleapis.com`)
///
/// For the standard Generative Language API, users must use an API key instead.
const GEMINI_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile";
const GEMINI_REDIRECT_PATH: &str = "callback";
const DEFAULT_PORT_RANGE: (u16, u16) = (52501, 52600);
const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// Hardcoded Gemini models for fallback when API fetch fails
/// Models based on rig-core gemini completion constants and Google's public API
const GEMINI_MODELS: &[&str] = &[
    // Gemini 2.5 series (latest)
    "gemini-2.5-pro-preview-06-05",
    "gemini-2.5-flash-preview-05-20",
    "gemini-2.5-flash",
    // Gemini 2.0 series
    "gemini-2.0-flash",
    "gemini-2.0-flash-lite",
    // Gemini 1.5 series
    "gemini-1.5-pro",
    "gemini-1.5-flash",
    "gemini-1.5-flash-8b",
    // Legacy
    "gemini-pro",
];

/// Model information for Gemini
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiModel {
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "inputTokenLimit")]
    pub input_token_limit: Option<u64>,
    #[serde(rename = "outputTokenLimit")]
    pub output_token_limit: Option<u64>,
}

/// User info from Google
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleUserInfo {
    pub sub: String,  // User ID
    pub email: Option<String>,
    pub name: Option<String>,
    pub picture: Option<String>,
}

/// Gemini OAuth client
pub struct GeminiOAuth {
    config: OAuthConfig,
    client: Client,
}

impl Default for GeminiOAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl GeminiOAuth {
    /// Create a new Gemini OAuth client with default configuration
    pub fn new() -> Self {
        Self::with_config(OAuthConfig {
            client_id: GEMINI_CLIENT_ID.to_string(),
            auth_url: GOOGLE_AUTH_URL.to_string(),
            token_url: GOOGLE_TOKEN_URL.to_string(),
            api_base_url: GEMINI_API_URL.to_string(),
            scope: GEMINI_SCOPE.to_string(),
            redirect_host: "localhost".to_string(),
            redirect_path: GEMINI_REDIRECT_PATH.to_string(),
            callback_port_range: DEFAULT_PORT_RANGE,
            callback_timeout_secs: DEFAULT_TIMEOUT_SECS,
        })
    }

    /// Create with custom configuration
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
            .append_pair("code_challenge_method", "S256")
            .append_pair("access_type", "offline")  // Request refresh token
            .append_pair("prompt", "consent");  // Force consent to get refresh token
        
        Ok(url.to_string())
    }
    
    /// Start the OAuth flow
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
    
    /// Complete the full OAuth flow
    pub fn authorize(&self) -> OAuthResult<TokenResponse> {
        let (flow_state, port) = self.start_flow()?;
        let auth_url = self.build_auth_url(&flow_state)?;
        
        tracing::info!("Opening browser for Google OAuth...");
        
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
            ("client_secret", GEMINI_CLIENT_SECRET),
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
        
        tracing::info!("Successfully obtained Google access token");
        
        Ok(token_response)
    }
    
    /// Refresh an access token
    pub fn refresh_token(&self, refresh_token: &str) -> OAuthResult<TokenResponse> {
        let client = reqwest::blocking::Client::new();
        
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.config.client_id),
            ("client_secret", GEMINI_CLIENT_SECRET),
        ];
        
        tracing::debug!("Refreshing Google access token");
        
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
        
        tracing::info!("Successfully refreshed Google access token");
        
        Ok(token_response)
    }
    
    /// Get user info from Google
    pub async fn get_user_info(&self, access_token: &str) -> OAuthResult<GoogleUserInfo> {
        let url = "https://www.googleapis.com/oauth2/v3/userinfo";
        
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
        
        let user_info: GoogleUserInfo = response.json().await
            .map_err(|e| OAuthError::InvalidResponse(format!("Failed to parse user info: {}", e)))?;
        
        Ok(user_info)
    }
    
    /// Fetch available Gemini models
    /// 
    /// Tries to fetch models from the Code Assist API first, falls back to hardcoded
    /// models if the API is unavailable or returns an error.
    pub async fn fetch_models(&self, access_token: &str, project_id: Option<&str>) -> OAuthResult<Vec<GeminiModel>> {
        // Try Code Assist API if project_id is provided
        if let Some(project_id) = project_id {
            let url = format!(
                "{}/v1/projects/{}/locations/global/codeAssistModels",
                self.config.api_base_url, project_id
            );
            
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await
                .map_err(OAuthError::HttpError)?;
            
            if response.status().is_success() {
                #[derive(Deserialize)]
                struct ModelsResponse {
                    #[serde(default)]
                    models: Vec<GeminiModel>,
                }
                
                if let Ok(models_response) = response.json::<ModelsResponse>().await {
                    if !models_response.models.is_empty() {
                        tracing::info!("Fetched {} models from Code Assist API", models_response.models.len());
                        return Ok(models_response.models);
                    }
                }
            } else {
                let status = response.status();
                tracing::warn!("Code Assist API returned {}, using fallback models", status);
            }
        }
        
        // Fallback to hardcoded models
        tracing::info!("Using hardcoded Gemini models");
        let models: Vec<GeminiModel> = GEMINI_MODELS
            .iter()
            .map(|name| GeminiModel {
                name: name.to_string(),
                display_name: Some(name.to_string()),
                description: None,
                input_token_limit: None,
                output_token_limit: None,
            })
            .collect();
        
        Ok(models)
    }
    
    /// Validate an access token
    pub async fn validate_token(&self, access_token: &str) -> OAuthResult<bool> {
        let url = "https://www.googleapis.com/oauth2/v3/tokeninfo";
        
        let response = self.client
            .get(url)
            .query(&[("access_token", access_token)])
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
        let oauth = GeminiOAuth::new();
        let (flow_state, port) = oauth.start_flow().unwrap();

        assert!(!flow_state.state.is_empty());
        assert!(!flow_state.code_verifier.is_empty());
        assert!(flow_state.redirect_uri.is_some());
        assert!(port >= DEFAULT_PORT_RANGE.0 && port <= DEFAULT_PORT_RANGE.1);
    }

    #[test]
    fn test_build_auth_url() {
        let oauth = GeminiOAuth::new();
        let (flow_state, _) = oauth.start_flow().unwrap();
        let auth_url = oauth.build_auth_url(&flow_state).unwrap();

        assert!(auth_url.contains(GOOGLE_AUTH_URL));
        assert!(auth_url.contains("response_type=code"));
        assert!(auth_url.contains(&format!("client_id={}", GEMINI_CLIENT_ID)));
        assert!(auth_url.contains("access_type=offline"));
        assert!(auth_url.contains("prompt=consent"));
    }
}
