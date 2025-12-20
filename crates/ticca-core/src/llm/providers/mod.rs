//! Custom OAuth provider implementations for LLM APIs
//!
//! These providers wrap upstream rig's client implementations with OAuth support.
//! Instead of using API keys, they authenticate using OAuth tokens obtained via
//! the ticca-oauth crate.
//!
//! # Supported Providers
//!
//! - **Claude**: Anthropic's Claude models via OAuth
//! - **ChatGPT**: OpenAI's GPT models via ChatGPT OAuth (Codex backend)
//! - **Gemini**: Google's Gemini models via Google OAuth

pub mod chatgpt;
pub mod claude;
pub mod common;
pub mod gemini;
pub mod gemini_code_assist;
pub mod http_wrapper;

pub use chatgpt::ChatGptOAuthClient;
pub use claude::ClaudeOAuthClient;
pub use common::{OAuthProviderError, ProviderConfig};
pub use gemini::GeminiOAuthClient;
pub use gemini_code_assist::{
    CodeAssistContent, GeminiCodeAssistClient, GeminiCodeAssistCompletionModel,
    GeminiCodeAssistRigClient,
};
pub use http_wrapper::{CodexHttpClient, OAuthHttpClient};
