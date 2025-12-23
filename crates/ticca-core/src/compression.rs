//! Context compression service for managing LLM context window limits.
//!
//! This module provides:
//! - Model context window lookup
//! - Token estimation for chat history
//! - Compression strategies (truncation, sliding window, summarizing)

use rig::compression::{
    estimate_messages_tokens, ContextCompressor, SlidingWindowCompressor, TruncationCompressor,
};
use rig::completion::Message;

use crate::config::{CompressionSettings, CompressionStrategy};

/// Known model context window sizes (in tokens).
/// These are fallback values when the API doesn't provide context_window.
pub fn get_model_context_window(model_id: &str) -> u64 {
    // Normalize the model ID for matching
    let model = model_id.to_lowercase();

    // Claude models
    if model.contains("claude-3-5-sonnet") || model.contains("claude-3.5-sonnet") {
        return 200_000;
    }
    if model.contains("claude-3-5-haiku") || model.contains("claude-3.5-haiku") {
        return 200_000;
    }
    if model.contains("claude-3-opus") || model.contains("claude-3.0-opus") {
        return 200_000;
    }
    if model.contains("claude-opus-4") || model.contains("claude-4-opus") {
        return 200_000;
    }
    if model.contains("claude-sonnet-4") || model.contains("claude-4-sonnet") {
        return 200_000;
    }
    if model.contains("claude") {
        return 200_000; // Default for other Claude models
    }

    // GPT models
    if model.contains("gpt-4o") {
        return 128_000;
    }
    if model.contains("gpt-4-turbo") {
        return 128_000;
    }
    if model.contains("gpt-4-32k") {
        return 32_768;
    }
    if model.contains("gpt-4") {
        return 8_192;
    }
    if model.contains("gpt-3.5-turbo-16k") {
        return 16_384;
    }
    if model.contains("gpt-3.5") {
        return 4_096;
    }
    if model.contains("o1-preview") || model.contains("o1-mini") {
        return 128_000;
    }
    if model.contains("o3") || model.contains("o4-mini") {
        return 200_000;
    }

    // Gemini models
    if model.contains("gemini-1.5-pro") || model.contains("gemini-1.5-flash") {
        return 1_000_000;
    }
    if model.contains("gemini-2") {
        return 1_000_000;
    }
    if model.contains("gemini") {
        return 128_000;
    }

    // Default fallback
    100_000
}

/// Result of compression check.
#[derive(Debug, Clone)]
pub struct CompressionCheck {
    /// Current estimated token count.
    pub current_tokens: usize,
    /// Maximum tokens before compression triggers.
    pub threshold_tokens: u64,
    /// Context window size for the model.
    pub context_window: u64,
    /// Whether compression is needed.
    pub needs_compression: bool,
    /// Percentage of context window used.
    pub usage_percent: u32,
}

/// Check if compression is needed for the given messages.
pub fn check_compression_needed(
    messages: &[Message],
    model_id: &str,
    settings: &CompressionSettings,
    api_context_window: Option<u64>,
) -> CompressionCheck {
    // Use API-provided context window if available, otherwise fall back to lookup
    let context_window = api_context_window.unwrap_or_else(|| get_model_context_window(model_id));

    let threshold_tokens = settings.token_threshold(context_window);
    let current_tokens = estimate_messages_tokens(messages);
    let usage_percent = ((current_tokens as u64 * 100) / context_window.max(1)) as u32;
    let needs_compression = settings.enabled && current_tokens as u64 > threshold_tokens;

    CompressionCheck {
        current_tokens,
        threshold_tokens,
        context_window,
        needs_compression,
        usage_percent,
    }
}

/// Compress messages using the configured strategy (sync version).
///
/// For the summarizing strategy, this falls back to sliding window
/// since summarization requires async. Use `compress_messages_async` for full support.
pub fn compress_messages(
    messages: Vec<Message>,
    settings: &CompressionSettings,
    target_tokens: usize,
) -> Vec<Message> {
    if messages.is_empty() {
        return messages;
    }

    match settings.strategy {
        CompressionStrategy::Truncation => {
            let compressor = TruncationCompressor::new()
                .with_min_preserve(settings.preserve_recent as usize);

            compressor
                .compress(messages.clone(), target_tokens)
                .unwrap_or(messages)
        }
        CompressionStrategy::SlidingWindow | CompressionStrategy::Summarizing => {
            // For sync, summarizing falls back to sliding window
            let compressor = SlidingWindowCompressor::new()
                .with_preserve_first(settings.preserve_first as usize)
                .with_min_recent(settings.preserve_recent as usize);

            compressor
                .compress(messages.clone(), target_tokens)
                .unwrap_or(messages)
        }
    }
}

/// Convert internal Message format to rig Message format.
pub fn chat_history_to_rig_messages(history: &[(crate::session::MessageRole, String)]) -> Vec<Message> {
    use crate::session::MessageRole;

    history
        .iter()
        .filter_map(|(role, content)| match role {
            MessageRole::User => Some(Message::user(content.clone())),
            MessageRole::Assistant => Some(Message::assistant(content.clone())),
            // System and Tool messages are not directly convertible to chat messages
            // They are typically handled separately (preamble, tool results)
            MessageRole::System | MessageRole::Tool => None,
        })
        .collect()
}

/// Convert rig Messages back to internal format.
pub fn rig_messages_to_chat_history(messages: Vec<Message>) -> Vec<(crate::session::MessageRole, String)> {
    use rig::completion::message::{AssistantContent, UserContent};

    messages
        .into_iter()
        .filter_map(|msg| {
            match msg {
                Message::User { content } => {
                    let text: String = content
                        .iter()
                        .filter_map(|c| {
                            if let UserContent::Text(t) = c {
                                Some(t.text.clone())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    if text.is_empty() {
                        None
                    } else {
                        Some((crate::session::MessageRole::User, text))
                    }
                }
                Message::Assistant { content, .. } => {
                    let text: String = content
                        .iter()
                        .filter_map(|c| {
                            if let AssistantContent::Text(t) = c {
                                Some(t.text.clone())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    if text.is_empty() {
                        None
                    } else {
                        Some((crate::session::MessageRole::Assistant, text))
                    }
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_model_context_window() {
        assert_eq!(get_model_context_window("claude-3-5-sonnet-20241022"), 200_000);
        assert_eq!(get_model_context_window("gpt-4o"), 128_000);
        assert_eq!(get_model_context_window("gpt-4-turbo"), 128_000);
        assert_eq!(get_model_context_window("gemini-1.5-pro"), 1_000_000);
        assert_eq!(get_model_context_window("unknown-model"), 100_000);
    }

    #[test]
    fn test_compression_check() {
        let settings = CompressionSettings {
            enabled: true,
            threshold_percent: 80,
            strategy: CompressionStrategy::SlidingWindow,
            summarizer_model: None,
            preserve_first: 1,
            preserve_recent: 4,
        };

        let messages = vec![Message::user("Hello"), Message::assistant("Hi there!")];

        let check = check_compression_needed(&messages, "claude-3-5-sonnet", &settings, None);

        assert_eq!(check.context_window, 200_000);
        assert_eq!(check.threshold_tokens, 160_000); // 80% of 200k
        assert!(!check.needs_compression); // Small messages don't need compression
    }
}
