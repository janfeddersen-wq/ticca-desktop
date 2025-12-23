//! Sliding window context compression.
//!
//! This strategy maintains a sliding window of recent messages while optionally
//! preserving the system prompt and initial context.

use crate::completion::Message;

use super::estimator::estimate_messages_tokens;
use super::traits::{CompressionError, ContextCompressor};

/// A compressor that maintains a sliding window of recent messages.
///
/// Unlike simple truncation, this strategy can preserve important context
/// like system prompts while maintaining a rolling window of conversation.
///
/// # Example
/// ```ignore
/// use rig::compression::SlidingWindowCompressor;
///
/// let compressor = SlidingWindowCompressor::new()
///     .with_preserve_first(1);  // Keep system prompt
/// let compressed = compressor.compress(messages, 4096)?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct SlidingWindowCompressor {
    /// Number of messages to preserve from the start (e.g., system prompt).
    preserve_first: usize,
    /// Minimum number of recent messages to keep.
    min_recent: usize,
}

impl SlidingWindowCompressor {
    /// Create a new sliding window compressor with default settings.
    pub fn new() -> Self {
        Self {
            preserve_first: 0,
            min_recent: 2,
        }
    }

    /// Set the number of messages to preserve from the start.
    ///
    /// Use this to keep system prompts or initial context.
    pub fn with_preserve_first(mut self, count: usize) -> Self {
        self.preserve_first = count;
        self
    }

    /// Set the minimum number of recent messages to keep.
    pub fn with_min_recent(mut self, count: usize) -> Self {
        self.min_recent = count;
        self
    }
}

impl ContextCompressor for SlidingWindowCompressor {
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

        let total = messages.len();

        // Determine which messages we must keep
        let must_keep_start = self.preserve_first.min(total);
        let must_keep_end = self.min_recent.min(total.saturating_sub(must_keep_start));

        // Split into: preserved_start | middle (can be trimmed) | preserved_end
        let preserved_start: Vec<_> = messages.iter().take(must_keep_start).cloned().collect();
        let preserved_end: Vec<_> = messages
            .iter()
            .skip(total.saturating_sub(must_keep_end))
            .cloned()
            .collect();

        // Check if just the preserved messages fit
        let preserved_tokens =
            estimate_messages_tokens(&preserved_start) + estimate_messages_tokens(&preserved_end);

        if preserved_tokens > max_tokens {
            // Even preserved messages don't fit - just return preserved_end
            return Ok(preserved_end);
        }

        // Try to fit as many middle messages as possible (from the end of middle section)
        let middle_start = must_keep_start;
        let middle_end = total.saturating_sub(must_keep_end);

        if middle_start >= middle_end {
            // No middle section
            let mut result = preserved_start;
            result.extend(preserved_end);
            return Ok(result);
        }

        let remaining_budget = max_tokens.saturating_sub(preserved_tokens);
        let mut middle_window_start = middle_start;

        // Find how many middle messages we can keep from the end
        for start in middle_start..middle_end {
            let middle_slice: Vec<_> = messages[start..middle_end].to_vec();
            if estimate_messages_tokens(&middle_slice) <= remaining_budget {
                middle_window_start = start;
                break;
            }
        }

        // Build final result
        let mut result = preserved_start;
        result.extend(messages[middle_window_start..middle_end].iter().cloned());
        result.extend(preserved_end);

        Ok(result)
    }

    fn estimate_tokens(&self, messages: &[Message]) -> usize {
        estimate_messages_tokens(messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sliding_window_empty_messages() {
        let compressor = SlidingWindowCompressor::new();
        let result = compressor.compress(vec![], 100).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_sliding_window_within_budget() {
        let compressor = SlidingWindowCompressor::new();
        let messages = vec![Message::user("Hello"), Message::assistant("Hi there!")];
        let result = compressor.compress(messages.clone(), 1000).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_sliding_window_preserves_first() {
        let compressor = SlidingWindowCompressor::new()
            .with_preserve_first(1)
            .with_min_recent(1);

        let messages = vec![
            Message::user("System prompt - this should be preserved"),
            Message::assistant("Message 2"),
            Message::user("Message 3"),
            Message::assistant("Message 4"),
            Message::user("Message 5 - most recent"),
        ];

        // Small budget should keep first and last
        let result = compressor.compress(messages, 100).unwrap();
        assert!(!result.is_empty());
    }
}
