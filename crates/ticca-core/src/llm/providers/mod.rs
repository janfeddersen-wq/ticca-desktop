//! LLM provider implementations
//!
//! These providers wrap upstream rig's client implementations with OAuth support
//! or API key authentication.
//!
//! # Supported Providers
//!
//! ## OAuth Providers
//! - **Claude**: Anthropic's Claude models via OAuth
//! - **ChatGPT**: OpenAI's GPT models via ChatGPT OAuth (Codex backend)
//! - **Gemini**: Google's Gemini models via Google OAuth
//!
//! ## API Key Providers (OpenAI-Compatible)
//! - Groq, Mistral, Together AI, DeepSeek, Cerebras, and many more

pub mod chatgpt;
pub mod claude;
pub mod common;
pub mod gemini;
pub mod gemini_code_assist;
pub mod http_wrapper;
pub mod openai_compatible;

pub use chatgpt::ChatGptOAuthClient;
pub use claude::ClaudeOAuthClient;
pub use common::{OAuthProviderError, ProviderConfig};
pub use gemini::GeminiOAuthClient;
pub use gemini_code_assist::{
    CodeAssistContent, GeminiCodeAssistClient, GeminiCodeAssistCompletionModel,
    GeminiCodeAssistRigClient,
};
pub use http_wrapper::{CodexHttpClient, OAuthHttpClient};
pub use openai_compatible::OpenAICompatibleApiClient;
