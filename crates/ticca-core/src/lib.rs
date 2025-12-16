#![deny(clippy::unwrap_used)]
#![warn(missing_docs)]

//! Ticca Core - Integration layer for Ticca Desktop
//!
//! This crate is the central integration layer that ties together all
//! components of Ticca Desktop:
//!
//! - **Configuration** (`ticca-config`): Application settings and preferences
//! - **Database** (`ticca-db`): Conversation and message persistence
//! - **Bridge** (`ticca-bridge`): Python interoperability for AI agents
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                        ticca-ui                             │
//! │                    (GPUI application)                       │
//! └────────────────────────────┬────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                       ticca-core                            │
//! │  ┌──────────────┐  ┌─────────────────┐  ┌───────────────┐  │
//! │  │  AppState    │  │ SessionManager  │  │    Error      │  │
//! │  └──────┬───────┘  └────────┬────────┘  └───────────────┘  │
//! │         │                   │                               │
//! └─────────┼───────────────────┼───────────────────────────────┘
//!           │                   │
//!           ▼                   ▼
//! ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
//! │   ticca-config  │  │    ticca-db     │  │  ticca-bridge   │
//! │  (settings)     │  │  (persistence)  │  │  (Python/AI)    │
//! └─────────────────┘  └─────────────────┘  └─────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ```ignore
//! use ticca_core::AppState;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize the application
//!     let state = AppState::initialize(None).await?;
//!     
//!     // Create a new conversation
//!     let conv_id = state.create_conversation(Some("My Chat")).await?;
//!     
//!     // Send a message and get a response
//!     let response = state.send_message(&conv_id, "Hello!", None).await?;
//!     println!("AI: {}", response.content);
//!     
//!     // Graceful shutdown
//!     state.shutdown().await?;
//!     Ok(())
//! }
//! ```
//!
//! # Streaming Responses
//!
//! For real-time UI updates, use streaming:
//!
//! ```ignore
//! let (mut receiver, handle) = state
//!     .send_message_streaming(&conv_id, "Tell me a story")
//!     .await?;
//!
//! while let Some(chunk) = receiver.recv().await {
//!     match chunk {
//!         StreamChunk::TextDelta { content } => print!("{}", content),
//!         StreamChunk::Done { response } => println!("\n\nDone!"),
//!         _ => {}
//!     }
//! }
//!
//! let final_response = handle.await??;
//! ```

pub mod agent;
pub mod app_state;
pub mod error;
pub mod session;
pub mod session_manager;

// Re-exports for convenient access
pub use app_state::AppState;
pub use error::CoreError;
pub use session::{Message, MessageRole, Session, SessionId};
pub use session_manager::{SessionInfo, SessionManager, SessionStats};

// Re-export commonly used types from dependencies
pub use ticca_bridge::{
    AgentController, AgentInfo, AgentRequest, AgentResponse, FinishReason, StreamCallback,
    StreamChunk, StreamReceiver, StreamSender, TokenUsage,
};
pub use ticca_config::AppConfig;
pub use ticca_db::{Conversation, Database, DbMessage};

/// Result type alias using CoreError
pub type Result<T> = std::result::Result<T, CoreError>;

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_result_alias() {
        fn returns_result() -> Result<i32> {
            Ok(42)
        }
        assert_eq!(returns_result().expect("should return 42"), 42);
    }
}
