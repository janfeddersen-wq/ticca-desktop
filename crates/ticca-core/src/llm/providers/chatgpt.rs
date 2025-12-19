//! ChatGPT/Codex OAuth Provider
//!
//! Wraps rig's OpenAI client with ChatGPT OAuth authentication support.
//! Uses the ChatGPT backend API with special requirements:
//!
//! - Base URL: `https://chatgpt.com/backend-api/codex`
//! - Headers:
//!   - `Authorization: Bearer {oauth_token}`
//!   - `ChatGPT-Account-Id: {account_id}` (extracted from JWT id_token)
//!   - `originator: codex_cli_rs`
//! - Additional request body params:
//!   - `store: false`
//!   - `codex_mode: true`
//!   - `parallel_tool_calls: false`

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use http::{HeaderMap, HeaderName, HeaderValue};
use rig::client::CompletionClient;
use rig::providers::openai;
use serde_json::json;
use uuid::Uuid;

use super::common::{OAuthProviderError, ProviderConfig, ProviderResult};
use super::http_wrapper::CodexHttpClient;

/// ChatGPT/Codex API constants
const CHATGPT_BACKEND_API: &str = "https://chatgpt.com/backend-api/codex";
const DEFAULT_MODEL: &str = "gpt-4o";

/// Type alias for the ChatGPT client with our custom Codex HTTP client
/// Uses CodexHttpClient which modifies requests for Codex backend compatibility
pub type ChatGptClient = openai::Client<CodexHttpClient>;

/// ChatGPT OAuth client configuration
#[derive(Debug, Clone)]
pub struct ChatGptConfig {
    /// OAuth access token
    pub access_token: String,

    /// Account ID extracted from id_token JWT
    pub account_id: String,

    /// Model to use
    pub model: String,

    /// Maximum tokens for response
    pub max_tokens: u32,

    /// Temperature for sampling
    pub temperature: f32,
}

impl ChatGptConfig {
    /// Create a new config with access token and account_id
    pub fn new(access_token: impl Into<String>, account_id: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            account_id: account_id.into(),
            model: DEFAULT_MODEL.to_string(),
            max_tokens: 8192,
            temperature: 0.7,
        }
    }

    /// Create config from access token and id_token
    /// Extracts account_id from the JWT id_token
    pub fn from_tokens(
        access_token: impl Into<String>,
        id_token: &str,
    ) -> ProviderResult<Self> {
        let account_id = extract_account_id_from_jwt(id_token).ok_or_else(|| {
            OAuthProviderError::AuthError(
                "Failed to extract account_id from id_token".to_string(),
            )
        })?;

        Ok(Self::new(access_token, account_id))
    }

    /// Set the model to use
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }
}

/// ChatGPT OAuth client wrapper
///
/// This wraps rig's OpenAI client, configured for the ChatGPT backend API
/// with OAuth authentication.
///
/// # Usage
///
/// ```ignore
/// use rig::agent::AgentBuilder;
/// use rig::client::CompletionClient;
///
/// let client = ChatGptOAuthClient::new("oauth_token", "account_id")?;
/// let model = client.completion_model("gpt-4o");
/// let agent = AgentBuilder::new(model)
///     .preamble("You are a helpful assistant")
///     .additional_params(ChatGptOAuthClient::codex_params())
///     .build();
/// ```
pub struct ChatGptOAuthClient {
    /// The underlying rig OpenAI client with custom HTTP client
    inner: ChatGptClient,

    /// Configuration
    config: ChatGptConfig,
}

impl ChatGptOAuthClient {
    /// Create a new ChatGPT OAuth client
    pub fn new(
        access_token: impl Into<String>,
        account_id: impl Into<String>,
    ) -> ProviderResult<Self> {
        Self::with_config(ChatGptConfig::new(access_token, account_id))
    }

    /// Create a new ChatGPT OAuth client from access_token and id_token
    pub fn from_tokens(
        access_token: impl Into<String>,
        id_token: &str,
    ) -> ProviderResult<Self> {
        Self::with_config(ChatGptConfig::from_tokens(access_token, id_token)?)
    }

    /// Create a new ChatGPT OAuth client with custom configuration
    pub fn with_config(config: ChatGptConfig) -> ProviderResult<Self> {
        let headers = build_chatgpt_headers(&config.access_token, &config.account_id)?;
        
        // Use CodexHttpClient which modifies request bodies for Codex compatibility:
        // - Removes unsupported fields (max_output_tokens)
        // - Adds required fields (store: false)
        let http_client = CodexHttpClient::new(headers);

        // Build the rig OpenAI client with custom HTTP client and base URL
        // Rig's default OpenAI client uses the Responses API (/responses endpoint)
        let inner: ChatGptClient = openai::Client::<CodexHttpClient>::builder()
            .http_client(http_client)
            .api_key("oauth-placeholder") // Will be overridden by our headers
            .base_url(CHATGPT_BACKEND_API)
            .build()
            .map_err(|e| OAuthProviderError::ConfigError(e.to_string()))?;

        Ok(Self { inner, config })
    }

    /// Get the underlying rig OpenAI client
    pub fn client(&self) -> &ChatGptClient {
        &self.inner
    }

    /// Get a completion model for the given model name
    ///
    /// Returns a model that can be used with `AgentBuilder::new(model)`.
    pub fn completion_model(
        &self,
        model: &str,
    ) -> <ChatGptClient as CompletionClient>::CompletionModel {
        self.inner.completion_model(model)
    }

    /// Get the Codex-specific additional parameters
    ///
    /// These must be added to requests for the ChatGPT backend:
    /// - `store: false` - Don't store conversations
    /// - `parallel_tool_calls: false` - Disable parallel tool calls
    ///
    /// Note: These are now automatically added by CodexHttpClient,
    /// but this method is kept for manual use if needed.
    pub fn codex_params() -> serde_json::Value {
        json!({
            "store": false,
            "parallel_tool_calls": false
        })
    }

    /// Get the configured model name
    pub fn model(&self) -> &str {
        &self.config.model
    }

    /// Get the configured max tokens
    pub fn max_tokens(&self) -> u32 {
        self.config.max_tokens
    }

    /// Get the configured temperature
    pub fn temperature(&self) -> f32 {
        self.config.temperature
    }

    /// Get the account ID
    pub fn account_id(&self) -> &str {
        &self.config.account_id
    }
}

/// Codex CLI version to report in headers
const CODEX_VERSION: &str = "0.75.0";

/// Build the OAuth headers for ChatGPT backend API
///
/// These headers are required for the ChatGPT Codex backend to work properly.
/// Reference: codex-rs and llxprt-code implementations, verified via mitmproxy
fn build_chatgpt_headers(access_token: &str, account_id: &str) -> ProviderResult<HeaderMap> {
    let mut headers = HeaderMap::new();

    // Content-Type header (explicitly set to ensure it's not modified)
    headers.insert(
        http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    // Authorization header with Bearer token
    headers.insert(
        http::header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", access_token))
            .map_err(|e| OAuthProviderError::ConfigError(format!("Invalid access token: {}", e)))?,
    );

    // ChatGPT Account ID header (case-sensitive!)
    headers.insert(
        HeaderName::from_static("chatgpt-account-id"),
        HeaderValue::from_str(account_id)
            .map_err(|e| OAuthProviderError::ConfigError(format!("Invalid account_id: {}", e)))?,
    );

    // Originator header (required by ChatGPT backend)
    headers.insert(
        HeaderName::from_static("originator"),
        HeaderValue::from_static("codex_cli_rs"),
    );

    // Version header (required by Codex API - discovered via mitmproxy)
    headers.insert(
        HeaderName::from_static("version"),
        HeaderValue::from_static(CODEX_VERSION),
    );

    // Conversation ID header (generate a UUID for each client instance)
    // This should ideally be passed from a session manager for multi-turn conversations
    let conversation_id = Uuid::new_v4().to_string();
    headers.insert(
        HeaderName::from_static("conversation_id"),
        HeaderValue::from_str(&conversation_id)
            .map_err(|e| OAuthProviderError::ConfigError(format!("Invalid conversation_id: {}", e)))?,
    );

    // Session ID header (can be same as conversation_id for now)
    headers.insert(
        HeaderName::from_static("session_id"),
        HeaderValue::from_str(&conversation_id)
            .map_err(|e| OAuthProviderError::ConfigError(format!("Invalid session_id: {}", e)))?,
    );

    // Accept SSE for streaming responses
    headers.insert(
        http::header::ACCEPT,
        HeaderValue::from_static("text/event-stream"),
    );

    // User-Agent (matches codex-cli-rs format)
    headers.insert(
        http::header::USER_AGENT,
        HeaderValue::from_str(&format!("codex_cli_rs/{} (Linux; x86_64)", CODEX_VERSION))
            .unwrap_or_else(|_| HeaderValue::from_static("codex_cli_rs/0.75.0 (Linux; x86_64)")),
    );

    tracing::debug!("Built OAuth headers for ChatGPT (token: {}... chars, conversation_id: {})",
        std::cmp::min(access_token.len(), 8), conversation_id);

    Ok(headers)
}

/// Extract account_id from ChatGPT id_token JWT
///
/// The account_id is in the claim: `https://api.openai.com/auth` -> `chatgpt_account_id`
pub fn extract_account_id_from_jwt(id_token: &str) -> Option<String> {
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
    claims
        .get("https://api.openai.com/auth")
        .and_then(|auth| auth.get("chatgpt_account_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Check if a model name is a GPT/Codex model
pub fn is_gpt_model(model_name: &str) -> bool {
    model_name.starts_with("gpt-")
        || model_name.starts_with("o1-")
        || model_name.starts_with("o3-")
        || model_name.starts_with("chatgpt-")
}

impl From<ProviderConfig> for ChatGptConfig {
    fn from(config: ProviderConfig) -> Self {
        Self {
            access_token: String::new(), // Must be set separately
            account_id: String::new(),   // Must be set separately
            model: if config.default_model.is_empty() {
                DEFAULT_MODEL.to_string()
            } else {
                config.default_model
            },
            max_tokens: config.max_tokens,
            temperature: config.temperature,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chatgpt_config_builder() {
        let config = ChatGptConfig::new("test_token", "account_123")
            .with_model("gpt-4o-mini")
            .with_max_tokens(4096)
            .with_temperature(0.5);

        assert_eq!(config.access_token, "test_token");
        assert_eq!(config.account_id, "account_123");
        assert_eq!(config.model, "gpt-4o-mini");
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.temperature, 0.5);
    }

    #[test]
    fn test_build_chatgpt_headers() {
        let headers = build_chatgpt_headers("test_token", "account_123").unwrap();

        assert_eq!(
            headers.get(http::header::AUTHORIZATION).unwrap(),
            "Bearer test_token"
        );
        assert_eq!(headers.get("chatgpt-account-id").unwrap(), "account_123");
        assert_eq!(headers.get("originator").unwrap(), "codex_cli_rs");
    }

    #[test]
    fn test_is_gpt_model() {
        assert!(is_gpt_model("gpt-4o"));
        assert!(is_gpt_model("gpt-4o-mini"));
        assert!(is_gpt_model("o1-preview"));
        assert!(is_gpt_model("o3-mini"));
        assert!(is_gpt_model("chatgpt-4o-latest"));
        assert!(!is_gpt_model("claude-sonnet-4"));
        assert!(!is_gpt_model("gemini-2.0-flash"));
    }

    #[test]
    fn test_codex_params() {
        let params = ChatGptOAuthClient::codex_params();

        assert_eq!(params["store"], false);
        assert_eq!(params["parallel_tool_calls"], false);
        // Note: codex_mode was removed - it's not a valid OpenAI Responses API parameter
    }
}
