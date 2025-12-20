//! Common OAuth utilities and types

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OAuthError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Token exchange failed: {0}")]
    TokenExchangeFailed(String),

    #[error("Token refresh failed: {0}")]
    TokenRefreshFailed(String),

    #[error("Authorization failed: {0}")]
    AuthorizationFailed(String),

    #[error("Callback server error: {0}")]
    CallbackServerError(String),

    #[error("Timeout waiting for authorization")]
    Timeout,

    #[error("User cancelled authorization")]
    Cancelled,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

pub type OAuthResult<T> = Result<T, OAuthError>;

/// OAuth configuration for a provider
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub client_id: String,
    pub auth_url: String,
    pub token_url: String,
    pub api_base_url: String,
    pub scope: String,
    pub redirect_host: String,
    pub redirect_path: String,
    pub callback_port_range: (u16, u16),
    pub callback_timeout_secs: u64,
}

impl OAuthConfig {
    /// Build the redirect URI for a given port
    pub fn redirect_uri(&self, port: u16) -> String {
        format!(
            "http://{}:{}/{}",
            self.redirect_host,
            port,
            self.redirect_path.trim_start_matches('/')
        )
    }
}

/// Token response from OAuth provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_in: Option<u64>,
    #[serde(default)]
    pub token_type: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    /// Provider-specific extra fields (flattened)
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

impl TokenResponse {
    /// Calculate expiration timestamp from expires_in
    pub fn expires_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.expires_in
            .map(|secs| chrono::Utc::now() + chrono::Duration::seconds(secs as i64))
    }

    /// Convert to RFC3339 expiration string
    pub fn expires_at_rfc3339(&self) -> Option<String> {
        self.expires_at().map(|dt| dt.to_rfc3339())
    }

    /// Get id_token if present (used by OpenAI/ChatGPT)
    pub fn id_token(&self) -> Option<&str> {
        self.extra.get("id_token").and_then(|v| v.as_str())
    }
}

/// State for tracking an OAuth flow in progress
#[derive(Debug, Clone)]
pub struct OAuthFlowState {
    pub state: String,
    pub code_verifier: String,
    pub code_challenge: String,
    pub redirect_uri: Option<String>,
    pub created_at: std::time::Instant,
}

impl OAuthFlowState {
    pub fn new(state: String, code_verifier: String, code_challenge: String) -> Self {
        Self {
            state,
            code_verifier,
            code_challenge,
            redirect_uri: None,
            created_at: std::time::Instant::now(),
        }
    }

    pub fn with_redirect_uri(mut self, uri: String) -> Self {
        self.redirect_uri = Some(uri);
        self
    }

    /// Check if the flow has timed out
    pub fn is_expired(&self, timeout_secs: u64) -> bool {
        self.created_at.elapsed().as_secs() > timeout_secs
    }
}

/// Provider identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Claude,
    Gemini,
    ChatGpt,
}

impl Provider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Provider::Claude => "claude",
            Provider::Gemini => "gemini",
            Provider::ChatGpt => "chatgpt",
        }
    }
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
