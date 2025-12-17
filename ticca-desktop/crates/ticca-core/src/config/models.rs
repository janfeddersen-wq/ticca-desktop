//! Configuration data models

use serde::{Deserialize, Serialize};
use chrono::Utc;

/// A key-value setting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
    pub updated_at: Option<String>,
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub name: String,
    pub model_type: String,  // "anthropic", "openai", "gemini", etc.
    pub endpoint_url: Option<String>,
    pub context_length: i64,
    pub is_default: bool,
    pub config_json: Option<String>,  // Additional config as JSON
    pub created_at: Option<String>,
}

impl ModelConfig {
    pub fn new(id: impl Into<String>, name: impl Into<String>, model_type: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            model_type: model_type.into(),
            endpoint_url: None,
            context_length: 128000,
            is_default: false,
            config_json: None,
            created_at: None,
        }
    }
    
    pub fn with_endpoint(mut self, url: impl Into<String>) -> Self {
        self.endpoint_url = Some(url.into());
        self
    }
    
    pub fn with_context_length(mut self, length: i64) -> Self {
        self.context_length = length;
        self
    }
    
    pub fn as_default(mut self) -> Self {
        self.is_default = true;
        self
    }
}

/// OAuth token storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    pub provider: String,  // "claude", "gemini", "chatgpt"
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub extra_json: Option<String>,  // Provider-specific data (project_id, etc.)
    pub updated_at: Option<String>,
}

impl OAuthToken {
    pub fn new(provider: impl Into<String>, access_token: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            access_token: access_token.into(),
            refresh_token: None,
            expires_at: None,
            token_type: Some("Bearer".to_string()),
            scope: None,
            extra_json: None,
            updated_at: None,
        }
    }
    
    pub fn with_refresh_token(mut self, token: impl Into<String>) -> Self {
        self.refresh_token = Some(token.into());
        self
    }
    
    pub fn with_expires_at(mut self, expires: impl Into<String>) -> Self {
        self.expires_at = Some(expires.into());
        self
    }
    
    pub fn with_extra(mut self, extra: impl Into<String>) -> Self {
        self.extra_json = Some(extra.into());
        self
    }
    
    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = &self.expires_at {
            if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(expires_at) {
                return expires < Utc::now();
            }
        }
        false
    }
}

/// Well-known setting keys
pub mod setting_keys {
    pub const THEME: &str = "theme";
    pub const DEFAULT_MODEL: &str = "default_model";
    pub const ALLOW_RECURSION: &str = "allow_recursion";
}

/// OAuth provider identifiers
pub mod providers {
    pub const CLAUDE: &str = "claude";
    pub const GEMINI: &str = "gemini";
    pub const CHATGPT: &str = "chatgpt";
}
