//! Context compression module for managing LLM context window limits.
//!
//! This module provides pluggable compression strategies that can reduce
//! chat history to fit within token budgets while preserving conversation
//! coherence and tool call/result integrity.

mod estimator;
mod traits;
mod truncation;
mod sliding_window;

pub use estimator::{estimate_tokens, estimate_message_tokens, estimate_messages_tokens};
pub use traits::{ContextCompressor, CompressionError};
pub use truncation::TruncationCompressor;
pub use sliding_window::SlidingWindowCompressor;
