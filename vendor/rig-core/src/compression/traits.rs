//! Core traits for context compression strategies.

use crate::completion::Message;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompressionError {
    #[error("Token estimation failed: {0}")]
    EstimationFailed(String),
    #[error("Invalid message structure: {0}")]
    InvalidStructure(String),
    #[error("Compression failed: {0}")]
    CompressionFailed(String),
}

/// Trait for pluggable context compression strategies.
///
/// Implementations can use different strategies like simple truncation,
/// sliding windows, or more sophisticated approaches.
pub trait ContextCompressor: Send + Sync {
    /// Compress messages to fit within the token budget.
    ///
    /// Returns a new vector of messages that fits within `max_tokens`.
    /// The implementation should preserve message ordering and keep
    /// tool call/result pairs together.
    fn compress(
        &self,
        messages: Vec<Message>,
        max_tokens: usize,
    ) -> Result<Vec<Message>, CompressionError>;

    /// Estimate the token count for a sequence of messages.
    fn estimate_tokens(&self, messages: &[Message]) -> usize;

    /// Check if compression is needed for the given messages and budget.
    fn needs_compression(&self, messages: &[Message], max_tokens: usize) -> bool {
        self.estimate_tokens(messages) > max_tokens
    }
}
