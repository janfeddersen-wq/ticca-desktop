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

// ============================================================================
// API Key-based Providers
// ============================================================================

/// API key provider identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyProvider {
    // Direct API providers
    OpenAI,
    Anthropic,
    GoogleAI,
    Azure,
    // OpenAI-compatible providers
    Groq,
    Mistral,
    TogetherAI,
    DeepSeek,
    OpenRouter,
    XAI,
    Fireworks,
    Cerebras,
    Cohere,
    Perplexity,
    DeepInfra,
    HuggingFace,
    SiliconFlow,
    Nebius,
    Nvidia,
    SambaNova,
    Hyperbolic,
    Novita,
    AIHubMix,
    Synthetic,
}

impl ApiKeyProvider {
    /// All available API key providers
    pub const ALL: &'static [ApiKeyProvider] = &[
        ApiKeyProvider::OpenAI,
        ApiKeyProvider::Anthropic,
        ApiKeyProvider::GoogleAI,
        ApiKeyProvider::Azure,
        ApiKeyProvider::Groq,
        ApiKeyProvider::Mistral,
        ApiKeyProvider::TogetherAI,
        ApiKeyProvider::DeepSeek,
        ApiKeyProvider::OpenRouter,
        ApiKeyProvider::XAI,
        ApiKeyProvider::Fireworks,
        ApiKeyProvider::Cerebras,
        ApiKeyProvider::Cohere,
        ApiKeyProvider::Perplexity,
        ApiKeyProvider::DeepInfra,
        ApiKeyProvider::HuggingFace,
        ApiKeyProvider::SiliconFlow,
        ApiKeyProvider::Nebius,
        ApiKeyProvider::Nvidia,
        ApiKeyProvider::SambaNova,
        ApiKeyProvider::Hyperbolic,
        ApiKeyProvider::Novita,
        ApiKeyProvider::AIHubMix,
        ApiKeyProvider::Synthetic,
    ];

    /// Get the provider ID string (used in database)
    pub fn id(&self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
            Self::GoogleAI => "google_ai",
            Self::Azure => "azure",
            Self::Groq => "groq",
            Self::Mistral => "mistral",
            Self::TogetherAI => "together_ai",
            Self::DeepSeek => "deepseek",
            Self::OpenRouter => "openrouter",
            Self::XAI => "xai",
            Self::Fireworks => "fireworks",
            Self::Cerebras => "cerebras",
            Self::Cohere => "cohere",
            Self::Perplexity => "perplexity",
            Self::DeepInfra => "deepinfra",
            Self::HuggingFace => "huggingface",
            Self::SiliconFlow => "siliconflow",
            Self::Nebius => "nebius",
            Self::Nvidia => "nvidia",
            Self::SambaNova => "sambanova",
            Self::Hyperbolic => "hyperbolic",
            Self::Novita => "novita",
            Self::AIHubMix => "aihubmix",
            Self::Synthetic => "synthetic",
        }
    }

    /// Get the display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::OpenAI => "OpenAI",
            Self::Anthropic => "Anthropic",
            Self::GoogleAI => "Google AI",
            Self::Azure => "Azure OpenAI",
            Self::Groq => "Groq",
            Self::Mistral => "Mistral",
            Self::TogetherAI => "Together AI",
            Self::DeepSeek => "DeepSeek",
            Self::OpenRouter => "OpenRouter",
            Self::XAI => "xAI (Grok)",
            Self::Fireworks => "Fireworks",
            Self::Cerebras => "Cerebras",
            Self::Cohere => "Cohere",
            Self::Perplexity => "Perplexity",
            Self::DeepInfra => "DeepInfra",
            Self::HuggingFace => "HuggingFace",
            Self::SiliconFlow => "SiliconFlow",
            Self::Nebius => "Nebius",
            Self::Nvidia => "NVIDIA NIM",
            Self::SambaNova => "SambaNova",
            Self::Hyperbolic => "Hyperbolic",
            Self::Novita => "Novita AI",
            Self::AIHubMix => "AIHubMix",
            Self::Synthetic => "Synthetic",
        }
    }

    /// Get the base API URL for the provider
    pub fn base_url(&self) -> &'static str {
        match self {
            Self::OpenAI => "https://api.openai.com/v1",
            Self::Anthropic => "https://api.anthropic.com/v1",
            Self::GoogleAI => "https://generativelanguage.googleapis.com/v1beta",
            Self::Azure => "https://{resource}.openai.azure.com/openai/deployments/{deployment}",
            Self::Groq => "https://api.groq.com/openai/v1",
            Self::Mistral => "https://api.mistral.ai/v1",
            Self::TogetherAI => "https://api.together.xyz/v1",
            Self::DeepSeek => "https://api.deepseek.com/v1",
            Self::OpenRouter => "https://openrouter.ai/api/v1",
            Self::XAI => "https://api.x.ai/v1",
            Self::Fireworks => "https://api.fireworks.ai/inference/v1",
            Self::Cerebras => "https://api.cerebras.ai/v1",
            Self::Cohere => "https://api.cohere.com/compatibility/v1",
            Self::Perplexity => "https://api.perplexity.ai",
            Self::DeepInfra => "https://api.deepinfra.com/v1/openai",
            Self::HuggingFace => "https://api-inference.huggingface.co/models",
            Self::SiliconFlow => "https://api.siliconflow.cn/v1",
            Self::Nebius => "https://api.studio.nebius.ai/v1",
            Self::Nvidia => "https://integrate.api.nvidia.com/v1",
            Self::SambaNova => "https://api.sambanova.ai/v1",
            Self::Hyperbolic => "https://api.hyperbolic.xyz/v1",
            Self::Novita => "https://api.novita.ai/v3/openai",
            Self::AIHubMix => "https://aihubmix.com/v1",
            Self::Synthetic => "https://api.synthetic.dev/v1",
        }
    }

    /// Check if this provider uses OpenAI-compatible API
    pub fn is_openai_compatible(&self) -> bool {
        match self {
            Self::OpenAI => true,
            Self::Anthropic => false,
            Self::GoogleAI => false,
            Self::Azure => true,
            Self::Groq => true,
            Self::Mistral => true,
            Self::TogetherAI => true,
            Self::DeepSeek => true,
            Self::OpenRouter => true,
            Self::XAI => true,
            Self::Fireworks => true,
            Self::Cerebras => true,
            Self::Cohere => true,
            Self::Perplexity => true,
            Self::DeepInfra => true,
            Self::HuggingFace => false, // HuggingFace has its own API format
            Self::SiliconFlow => true,
            Self::Nebius => true,
            Self::Nvidia => true,
            Self::SambaNova => true,
            Self::Hyperbolic => true,
            Self::Novita => true,
            Self::AIHubMix => true,
            Self::Synthetic => true,
        }
    }

    /// Parse provider from ID string
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "openai" => Some(Self::OpenAI),
            "anthropic" => Some(Self::Anthropic),
            "google_ai" => Some(Self::GoogleAI),
            "azure" => Some(Self::Azure),
            "groq" => Some(Self::Groq),
            "mistral" => Some(Self::Mistral),
            "together_ai" => Some(Self::TogetherAI),
            "deepseek" => Some(Self::DeepSeek),
            "openrouter" => Some(Self::OpenRouter),
            "xai" => Some(Self::XAI),
            "fireworks" => Some(Self::Fireworks),
            "cerebras" => Some(Self::Cerebras),
            "cohere" => Some(Self::Cohere),
            "perplexity" => Some(Self::Perplexity),
            "deepinfra" => Some(Self::DeepInfra),
            "huggingface" => Some(Self::HuggingFace),
            "siliconflow" => Some(Self::SiliconFlow),
            "nebius" => Some(Self::Nebius),
            "nvidia" => Some(Self::Nvidia),
            "sambanova" => Some(Self::SambaNova),
            "hyperbolic" => Some(Self::Hyperbolic),
            "novita" => Some(Self::Novita),
            "aihubmix" => Some(Self::AIHubMix),
            "synthetic" => Some(Self::Synthetic),
            _ => None,
        }
    }
}

impl std::fmt::Display for ApiKeyProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// API key account storage (multi-key per provider)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyAccount {
    pub id: String,
    pub provider: String,
    pub api_key: String,
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

impl ApiKeyAccount {
    pub fn new(
        id: impl Into<String>,
        provider: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            provider: provider.into(),
            api_key: api_key.into(),
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

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_priority(mut self, priority: i64) -> Self {
        self.priority = priority;
        self
    }

    pub fn is_cooling(&self) -> bool {
        if let Some(cooldown_until) = &self.cooldown_until
            && let Ok(until) = chrono::DateTime::parse_from_rfc3339(cooldown_until)
        {
            return until > Utc::now();
        }
        false
    }

    /// Get a masked version of the API key for display (e.g., "sk-abc...xyz")
    pub fn masked_key(&self) -> String {
        let key = &self.api_key;
        if key.len() <= 8 {
            return "***".to_string();
        }
        format!("{}...{}", &key[..4], &key[key.len() - 4..])
    }
}
