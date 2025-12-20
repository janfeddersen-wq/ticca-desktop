//! Configuration data models

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    pub model_type: String, // "anthropic", "openai", "gemini", etc.
    pub endpoint_url: Option<String>,
    pub context_length: i64,
    pub is_default: bool,
    pub config_json: Option<String>, // Additional config as JSON
    pub created_at: Option<String>,
}

impl ModelConfig {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        model_type: impl Into<String>,
    ) -> Self {
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
    pub provider: String, // "claude", "gemini", "chatgpt"
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub extra_json: Option<String>, // Provider-specific data (project_id, etc.)
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

    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }

    pub fn with_extra(mut self, extra: impl Into<String>) -> Self {
        self.extra_json = Some(extra.into());
        self
    }

    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        let Some(expires_at) = &self.expires_at else {
            return false;
        };

        match chrono::DateTime::parse_from_rfc3339(expires_at) {
            Ok(expires) => expires < Utc::now(),
            Err(_) => true,
        }
    }

    pub fn has_refresh_token(&self) -> bool {
        self.refresh_token
            .as_deref()
            .is_some_and(|token| !token.trim().is_empty())
    }
}

/// OAuth account storage (multi-account)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthAccount {
    pub id: String,
    pub provider: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub extra_json: Option<String>,
    pub label: Option<String>,
    pub is_active: bool,
    pub priority: i64,
    pub cooldown_until: Option<String>,
    pub last_error: Option<String>,
    pub last_429_at: Option<String>,
    pub last_used_at: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl OAuthAccount {
    pub fn new(
        id: impl Into<String>,
        provider: impl Into<String>,
        access_token: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            provider: provider.into(),
            access_token: access_token.into(),
            refresh_token: None,
            expires_at: None,
            token_type: Some("Bearer".to_string()),
            scope: None,
            extra_json: None,
            label: None,
            is_active: true,
            priority: 0,
            cooldown_until: None,
            last_error: None,
            last_429_at: None,
            last_used_at: None,
            created_at: None,
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

    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }

    pub fn with_extra(mut self, extra: impl Into<String>) -> Self {
        self.extra_json = Some(extra.into());
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_priority(mut self, priority: i64) -> Self {
        self.priority = priority;
        self
    }

    pub fn is_expired(&self) -> bool {
        let Some(expires_at) = &self.expires_at else {
            return false;
        };

        match chrono::DateTime::parse_from_rfc3339(expires_at) {
            Ok(expires) => expires < Utc::now(),
            Err(_) => true,
        }
    }

    pub fn is_cooling(&self) -> bool {
        if let Some(cooldown_until) = &self.cooldown_until
            && let Ok(until) = chrono::DateTime::parse_from_rfc3339(cooldown_until)
        {
            return until > Utc::now();
        }
        false
    }

    pub fn has_refresh_token(&self) -> bool {
        self.refresh_token
            .as_deref()
            .is_some_and(|token| !token.trim().is_empty())
    }
}

/// MCP transport type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTransport {
    Stdio,
    StreamableHttp,
}

impl McpTransport {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stdio => "stdio",
            Self::StreamableHttp => "streamable_http",
        }
    }
}

impl std::fmt::Display for McpTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for McpTransport {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "stdio" => Ok(Self::Stdio),
            "streamable_http" | "streamable-http" | "http" => Ok(Self::StreamableHttp),
            _ => Err(()),
        }
    }
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub id: String,
    pub name: String,
    pub transport: McpTransport,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub endpoint_url: Option<String>,
    pub is_enabled: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl McpServer {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            transport: McpTransport::Stdio,
            command: None,
            args: Vec::new(),
            env: BTreeMap::new(),
            endpoint_url: None,
            is_enabled: true,
            created_at: None,
            updated_at: None,
        }
    }

    pub fn with_stdio_command(mut self, command: impl Into<String>) -> Self {
        self.transport = McpTransport::Stdio;
        self.command = Some(command.into());
        self
    }

    pub fn with_streamable_http(mut self, url: impl Into<String>) -> Self {
        self.transport = McpTransport::StreamableHttp;
        self.endpoint_url = Some(url.into());
        self
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    pub fn with_env(mut self, env: BTreeMap<String, String>) -> Self {
        self.env = env;
        self
    }

    pub fn set_enabled(mut self, enabled: bool) -> Self {
        self.is_enabled = enabled;
        self
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
