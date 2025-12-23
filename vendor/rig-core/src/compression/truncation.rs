//! Simple truncation-based context compression.
//!
//! This strategy removes the oldest messages first until the token budget is met.
//! It's the simplest compression approach but may lose important context.

use crate::completion::Message;

use super::estimator::estimate_messages_tokens;
use super::traits::{CompressionError, ContextCompressor};

/// A simple compressor that truncates older messages to fit within token limits.
///
/// This is the most basic compression strategy - it simply removes messages
/// from the beginning of the conversation until the token budget is satisfied.
///
/// # Example
/// ```ignore
/// use rig::compression::TruncationCompressor;
///
/// let compressor = TruncationCompressor::new();
/// let compressed = compressor.compress(messages, 4096)?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct TruncationCompressor {
    /// Minimum number of messages to preserve (from the end).
    min_preserve: usize,
}

impl TruncationCompressor {
    /// Create a new truncation compressor with default settings.
    pub fn new() -> Self {
        Self { min_preserve: 1 }
    }

    /// Set the minimum number of messages to preserve from the end.
    ///
    /// This ensures at least N messages are kept even if they exceed the budget.
    pub fn with_min_preserve(mut self, count: usize) -> Self {
        self.min_preserve = count;
        self
    }
}

impl ContextCompressor for TruncationCompressor {
    fn compress(
        &self,
        messages: Vec<Message>,
        max_tokens: usize,
    ) -> Result<Vec<Message>, CompressionError> {
        if messages.is_empty() {
            return Ok(messages);
        }

        // If already within budget, return as-is
        if estimate_messages_tokens(&messages) <= max_tokens {
            return Ok(messages);
        }

        // Start removing from the beginning until we fit
        let mut start_idx = 0;
        let max_start = messages.len().saturating_sub(self.min_preserve);

        while start_idx < max_start {
            let remaining = &messages[start_idx..];
            if estimate_messages_tokens(remaining) <= max_tokens {
                break;
            }
            start_idx += 1;
        }

        Ok(messages.into_iter().skip(start_idx).collect())
    }

    fn estimate_tokens(&self, messages: &[Message]) -> usize {
        estimate_messages_tokens(messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncation_empty_messages() {
        let compressor = TruncationCompressor::new();
        let result = compressor.compress(vec![], 100).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_truncation_within_budget() {
        let compressor = TruncationCompressor::new();
        let messages = vec![Message::user("Hello")];
        let result = compressor.compress(messages.clone(), 1000).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_truncation_exceeds_budget() {
        let compressor = TruncationCompressor::new();
        let messages = vec![
            Message::user("First message that is quite long to use some tokens"),
            Message::assistant("Second message also with content"),
            Message::user("Third message"),
        ];

        // Very small budget should truncate older messages
        let result = compressor.compress(messages, 50).unwrap();
        // Should keep at least min_preserve (1) message
        assert!(!result.is_empty());
        assert!(result.len() <= 3);
    }
}
