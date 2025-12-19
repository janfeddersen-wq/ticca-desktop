//! Gemini OAuth Provider
//!
//! Wraps rig's Gemini client with Google OAuth authentication support.
//! Uses Bearer token authentication (same pattern as Claude and ChatGPT).
//!
//! - Base URL: `https://generativelanguage.googleapis.com`
//! - Headers:
//!   - `Authorization: Bearer {oauth_token}`

use http::{HeaderMap, HeaderValue};
use rig::providers::gemini;

use super::common::{OAuthProviderError, ProviderConfig, ProviderResult};
use super::http_wrapper::OAuthHttpClient;

/// Gemini API constants
const GEMINI_API_URL: &str = "https://generativelanguage.googleapis.com";
const DEFAULT_MODEL: &str = "gemini-2.0-flash";

/// Type alias for the Gemini client with our custom HTTP client
pub type GeminiClient = gemini::Client<OAuthHttpClient>;

/// Type alias for the Gemini completion model with our custom HTTP client
/// Note: We use the concrete type because rig's Gemini Capabilities implementation
/// doesn't propagate the HTTP client type parameter (unlike Anthropic/OpenAI).
pub type GeminiCompletionModel = gemini::completion::CompletionModel<OAuthHttpClient>;

/// Gemini OAuth client configuration
#[derive(Debug, Clone)]
pub struct GeminiConfig {
    /// OAuth access token
    pub access_token: String,

    /// Optional project ID for Cloud API
    pub project_id: Option<String>,

    /// Model to use
    pub model: String,

    /// Maximum tokens for response
    pub max_tokens: u32,

    /// Temperature for sampling
    pub temperature: f32,
}

impl GeminiConfig {
    /// Create a new config with the given OAuth token
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            project_id: None,
            model: DEFAULT_MODEL.to_string(),
            max_tokens: 8192,
            temperature: 0.7,
        }
    }

    /// Set the project ID for Cloud API
    pub fn with_project_id(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
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

/// Gemini OAuth client wrapper
///
/// This wraps rig's Gemini client with OAuth authentication.
/// Uses the same OAuthHttpClient pattern as Claude and ChatGPT for consistency.
///
/// # Usage
///
/// ```ignore
/// use rig::agent::AgentBuilder;
/// use rig::client::CompletionClient;
///
/// let client = GeminiOAuthClient::new("oauth_token")?;
/// let model = client.completion_model("gemini-2.0-flash");
/// let agent = AgentBuilder::new(model)
///     .preamble("You are a helpful assistant")
///     .build();
/// ```
pub struct GeminiOAuthClient {
    /// The underlying rig Gemini client with custom HTTP client
    inner: GeminiClient,

    /// Configuration
    config: GeminiConfig,
}

impl GeminiOAuthClient {
    /// Create a new Gemini OAuth client
    pub fn new(access_token: impl Into<String>) -> ProviderResult<Self> {
        Self::with_config(GeminiConfig::new(access_token))
    }

    /// Create a new Gemini OAuth client with custom configuration
    pub fn with_config(config: GeminiConfig) -> ProviderResult<Self> {
        let headers = build_oauth_headers(&config.access_token)?;
        let http_client = OAuthHttpClient::new(headers);

        // Build the rig Gemini client with custom HTTP client
        // The api_key is a placeholder - our OAuthHttpClient adds the real Authorization header
        // Note: rig's Gemini client adds ?key={api_key} to URLs, but Google's API
        // will use the Bearer token from our Authorization header instead
        let inner: GeminiClient = gemini::Client::<OAuthHttpClient>::builder()
            .http_client(http_client)
            .api_key("oauth-placeholder") // Will be overridden by our headers
            .base_url(GEMINI_API_URL)
            .build()
            .map_err(|e| OAuthProviderError::ConfigError(e.to_string()))?;

        Ok(Self { inner, config })
    }

    /// Get the underlying rig Gemini client
    pub fn client(&self) -> &GeminiClient {
        &self.inner
    }

    /// Get a completion model for the given model name
    ///
    /// Returns a model that can be used with `AgentBuilder::new(model)`.
    ///
    /// Note: We construct the completion model directly because rig's Gemini
    /// Capabilities implementation doesn't propagate the HTTP client type parameter.
    /// This is a workaround until rig-core fixes this upstream.
    pub fn completion_model(&self, model: &str) -> GeminiCompletionModel {
        gemini::completion::CompletionModel::new(self.inner.clone(), model)
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

    /// Get the project ID if set
    pub fn project_id(&self) -> Option<&str> {
        self.config.project_id.as_deref()
    }
}

/// Build the OAuth headers for Gemini API
///
/// These headers are required for Gemini OAuth to work properly.
fn build_oauth_headers(access_token: &str) -> ProviderResult<HeaderMap> {
    let mut headers = HeaderMap::new();

    // Authorization header with Bearer token (overrides the ?key= query param)
    headers.insert(
        http::header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", access_token))
            .map_err(|e| OAuthProviderError::ConfigError(format!("Invalid access token: {}", e)))?,
    );

    tracing::debug!("Built OAuth headers for Gemini (token: {}... chars)",
        std::cmp::min(access_token.len(), 8));

    Ok(headers)
}

/// Check if a model name is a Gemini model
pub fn is_gemini_model(model_name: &str) -> bool {
    model_name.starts_with("gemini-") || model_name.contains("gemini")
}

impl From<ProviderConfig> for GeminiConfig {
    fn from(config: ProviderConfig) -> Self {
        Self {
            access_token: String::new(), // Must be set separately
            project_id: None,
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
    fn test_gemini_config_builder() {
        let config = GeminiConfig::new("test_token")
            .with_model("gemini-2.0-flash-lite")
            .with_project_id("my-project")
            .with_max_tokens(4096)
            .with_temperature(0.5);

        assert_eq!(config.access_token, "test_token");
        assert_eq!(config.model, "gemini-2.0-flash-lite");
        assert_eq!(config.project_id, Some("my-project".to_string()));
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.temperature, 0.5);
    }

    #[test]
    fn test_build_oauth_headers() {
        let headers = build_oauth_headers("test_token").unwrap();

        assert!(headers.get(http::header::AUTHORIZATION).is_some());
        assert_eq!(
            headers.get(http::header::AUTHORIZATION).unwrap(),
            "Bearer test_token"
        );
    }

    #[test]
    fn test_is_gemini_model() {
        assert!(is_gemini_model("gemini-2.0-flash"));
        assert!(is_gemini_model("gemini-2.0-flash-lite"));
        assert!(is_gemini_model("gemini-pro"));
        assert!(!is_gemini_model("gpt-4o"));
        assert!(!is_gemini_model("claude-sonnet-4"));
    }
}
