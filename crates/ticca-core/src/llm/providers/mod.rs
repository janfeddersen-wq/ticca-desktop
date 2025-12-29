//! LLM provider implementations
//!
//! These providers wrap serdesAI models with ticca-specific configuration.
//!
//! # Supported Providers
//!
//! ## OAuth Providers
//! - **Claude**: Anthropic's Claude models via OAuth (wraps serdesAI's ClaudeCodeOAuthModel)
//! - **ChatGPT**: OpenAI's GPT models via ChatGPT OAuth (wraps serdesAI's ChatGptOAuthModel)

pub mod chatgpt;
pub mod claude;
pub mod common;

pub use chatgpt::{ChatGptConfig, ChatGptOAuthClient, is_chatgpt_model};
pub use claude::{ClaudeConfig, ClaudeOAuthClient, is_claude_model};
pub use common::{OAuthProviderError, ProviderConfig, ProviderResult};
