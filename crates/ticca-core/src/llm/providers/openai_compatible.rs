//! OpenAI-Compatible API Key Provider
//!
//! Generic client for OpenAI-compatible APIs using API key authentication.
//! Supports providers like Groq, Together AI, Mistral, DeepSeek, Cerebras, etc.
//!
//! Uses the standard Chat Completions API (/chat/completions) which is supported
//! by most OpenAI-compatible providers, unlike the newer Responses API which
//! is OpenAI-specific.

use rig::client::{BearerAuth, CompletionClient};
use rig::providers::openai;

use crate::config::ApiKeyProvider;

/// Type alias for OpenAI-compatible client (using Chat Completions API)
pub type OpenAICompatibleClient = openai::CompletionsClient;

/// OpenAI-compatible API key client
///
/// This wraps rig's OpenAI client for use with OpenAI-compatible API providers.
/// Unlike OAuth clients, this uses standard API key authentication.
/// Uses the Chat Completions API (/chat/completions) for broad compatibility.
///
/// # Usage
///
/// ```ignore
/// use rig::agent::AgentBuilder;
/// use rig::client::CompletionClient;
///
/// let client = OpenAICompatibleApiClient::new(ApiKeyProvider::Cerebras, "sk-...")?;
/// let model = client.completion_model("llama3-70b");
/// let agent = AgentBuilder::new(model)
///     .preamble("You are a helpful assistant")
///     .build();
/// ```
pub struct OpenAICompatibleApiClient {
    /// The underlying rig OpenAI client (Chat Completions API)
    inner: OpenAICompatibleClient,

    /// The provider being used
    provider: ApiKeyProvider,
}

impl OpenAICompatibleApiClient {
    /// Create a new OpenAI-compatible API client
    pub fn new(provider: ApiKeyProvider, api_key: &str) -> Result<Self, String> {
        let base_url = provider.base_url();
        let auth: BearerAuth = api_key.into();

        tracing::info!(
            "Creating {} client with base_url: {} (using Chat Completions API)",
            provider.display_name(),
            base_url
        );

        // Build the base client, then switch to Chat Completions API
        // (the default is Responses API which is OpenAI-specific)
        let inner = openai::Client::builder()
            .api_key(auth)
            .base_url(base_url)
            .build()
            .map_err(|e| format!("Failed to create {} client: {}", provider.display_name(), e))?
            .completions_api();

        Ok(Self { inner, provider })
    }

    /// Get the underlying rig OpenAI client
    pub fn client(&self) -> &OpenAICompatibleClient {
        &self.inner
    }

    /// Get a completion model for the given model name
    ///
    /// Returns a model that can be used with `AgentBuilder::new(model)`.
    pub fn completion_model(
        &self,
        model: &str,
    ) -> <OpenAICompatibleClient as CompletionClient>::CompletionModel {
        self.inner.completion_model(model)
    }

    /// Get the provider
    pub fn provider(&self) -> ApiKeyProvider {
        self.provider
    }

    /// Get the provider display name
    pub fn provider_name(&self) -> &'static str {
        self.provider.display_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        // Test that client can be created (will fail network calls but struct should build)
        let result = OpenAICompatibleApiClient::new(ApiKeyProvider::Cerebras, "test-key");
        assert!(result.is_ok());

        let client = result.unwrap();
        assert_eq!(client.provider(), ApiKeyProvider::Cerebras);
        assert_eq!(client.provider_name(), "Cerebras");
    }
}
