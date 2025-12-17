//! Gemini (Google) OAuth implementation
//!
//! This implements the OAuth flow for Google's Gemini Code Assist API.
//! Reference: Google Cloud OAuth 2.0

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

// Note: In production, you'd need to register your own OAuth client
// Users need to configure their own client_id and client_secret

/// Scopes required for Gemini Code Assist
const GEMINI_SCOPE: &str = "openid email profile https://www.googleapis.com/auth/cloud-platform https://www.googleapis.com/auth/cloudcodeassist";
const GEMINI_REDIRECT_PATH: &str = "callback";
const DEFAULT_PORT_RANGE: (u16, u16) = (52501, 52600);
const DEFAULT_TIMEOUT_SECS: u64 = 300;

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
    client_secret: String,
    client: Client,
}

impl GeminiOAuth {
    /// Create a new Gemini OAuth client
    /// 
    /// # Arguments
    /// * `client_id` - Google OAuth client ID
    /// * `client_secret` - Google OAuth client secret
    pub fn new(client_id: &str, client_secret: &str) -> Self {
        Self::with_config(
            OAuthConfig {
                client_id: client_id.to_string(),
                auth_url: GOOGLE_AUTH_URL.to_string(),
                token_url: GOOGLE_TOKEN_URL.to_string(),
                api_base_url: GEMINI_API_URL.to_string(),
                scope: GEMINI_SCOPE.to_string(),
                redirect_host: "localhost".to_string(),
                redirect_path: GEMINI_REDIRECT_PATH.to_string(),
                callback_port_range: DEFAULT_PORT_RANGE,
                callback_timeout_secs: DEFAULT_TIMEOUT_SECS,
            },
            client_secret,
        )
    }
    
    /// Create with custom configuration
    pub fn with_config(config: OAuthConfig, client_secret: &str) -> Self {
        Self {
            config,
            client_secret: client_secret.to_string(),
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
        if self.config.client_id.is_empty() {
            return Err(OAuthError::InvalidResponse(
                "Google OAuth client_id not configured. Please set up OAuth credentials.".into()
            ));
        }
        
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
            ("client_secret", &self.client_secret),
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
            ("client_secret", &self.client_secret),
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
    pub async fn fetch_models(&self, access_token: &str, project_id: &str) -> OAuthResult<Vec<GeminiModel>> {
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
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(OAuthError::InvalidResponse(format!(
                "Failed to fetch models: HTTP {}: {}", status, body
            )));
        }
        
        #[derive(Deserialize)]
        struct ModelsResponse {
            #[serde(default)]
            models: Vec<GeminiModel>,
        }
        
        let models_response: ModelsResponse = response.json().await
            .map_err(|e| OAuthError::InvalidResponse(format!("Failed to parse models response: {}", e)))?;
        
        Ok(models_response.models)
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
    fn test_start_flow_with_credentials() {
        let oauth = GeminiOAuth::new("test_client_id", "test_secret");
        let (flow_state, port) = oauth.start_flow().unwrap();
        
        assert!(!flow_state.state.is_empty());
        assert!(!flow_state.code_verifier.is_empty());
        assert!(flow_state.redirect_uri.is_some());
        assert!(port >= DEFAULT_PORT_RANGE.0 && port <= DEFAULT_PORT_RANGE.1);
    }
    
    #[test]
    fn test_build_auth_url() {
        let oauth = GeminiOAuth::new("test_client_id", "test_secret");
        let (flow_state, _) = oauth.start_flow().unwrap();
        let auth_url = oauth.build_auth_url(&flow_state).unwrap();
        
        assert!(auth_url.contains(GOOGLE_AUTH_URL));
        assert!(auth_url.contains("response_type=code"));
        assert!(auth_url.contains("client_id=test_client_id"));
        assert!(auth_url.contains("access_type=offline"));
        assert!(auth_url.contains("prompt=consent"));
    }
    
    #[test]
    fn test_authorize_without_credentials() {
        let oauth = GeminiOAuth::new("", "");
        let result = oauth.authorize();
        assert!(result.is_err());
    }
}
