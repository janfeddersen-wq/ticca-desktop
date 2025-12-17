//! Ticca OAuth - OAuth implementations for various LLM providers
//!
//! This crate provides OAuth authentication flows for:
//! - **Claude Code** (Anthropic) - Uses public PKCE flow
//! - **Gemini** (Google) - Requires OAuth client credentials
//! - **ChatGPT** (OpenAI) - Uses public PKCE flow
//!
//! ## Example
//!
//! ```rust,no_run
//! use ticca_oauth::claude::ClaudeOAuth;
//!
//! // Start OAuth flow for Claude
//! let oauth = ClaudeOAuth::new();
//! let token_response = oauth.authorize().expect("OAuth failed");
//! println!("Access token: {}", token_response.access_token);
//! ```

pub mod common;
pub mod pkce;
pub mod callback_server;
pub mod claude;
pub mod gemini;
pub mod chatgpt;

// Re-export commonly used types
pub use common::{
    OAuthConfig,
    OAuthError,
    OAuthResult,
    OAuthFlowState,
    TokenResponse,
    Provider,
};

pub use pkce::create_pkce_state;
pub use callback_server::{CallbackResult, find_available_port, wait_for_callback, build_redirect_uri};
pub use claude::ClaudeOAuth;
pub use gemini::GeminiOAuth;
pub use chatgpt::ChatGptOAuth;
