//! Context compression service for managing LLM context window limits.
//!
//! This module provides:
//! - Model context window lookup (via [`crate::registry::RegistryService`])
//! - Compression strategies:
//!   - Truncation: Token-based LIFO (like code_puppy) - keeps system prompt + recent messages
//!   - Summarizing: LLM-based summarization of removed context
//!
//! Context window sizes are sourced from the model registry, which provides
//! authoritative metadata for all known models.
//!
//! Token estimation is handled by rig-core's compression module (uses chars/3.4 ratio).

use std::sync::Arc;

use rig::compression::SummarizingCompressor;
use rig::completion::{Message, Prompt};

use crate::config::{CompressionSettings, CompressionStrategy};

// Re-export rig's context estimation for use throughout ticca
pub use rig::compression::{estimate_tokens, CompressionError, ContextEstimate};

/// Get the context window size for a model (in tokens).
///
/// Delegates to [`crate::registry::RegistryService`] for authoritative lookup.
/// Returns a reasonable default (100k) for unknown models.
///
/// # Examples
///
/// ```rust,ignore
/// use ticca_core::compression::get_model_context_window;
///
/// assert_eq!(get_model_context_window("gpt-4o"), 128_000);
/// assert_eq!(get_model_context_window("claude-sonnet-4-20250514"), 200_000);
/// assert_eq!(get_model_context_window("gemini-2.0-flash"), 1_000_000);
/// ```
#[inline]
pub fn get_model_context_window(model_id: &str) -> u64 {
    crate::registry::RegistryService::get_context_window(model_id)
}

/// Create a comprehensive context estimate including all components.
///
/// This is the main function to call before each LLM request. It calculates
/// tokens for the system prompt, tool definitions, and all messages.
///
/// # Arguments
/// * `system_prompt` - The system prompt/preamble text
/// * `tool_definitions` - Tool definitions to serialize and estimate
/// * `messages` - All conversation messages
/// * `model_id` - Model identifier for context window lookup (supports canonical `provider:model` format)
/// * `api_context_window` - Optional API-provided context window (overrides lookup)
pub fn create_context_estimate(
    system_prompt: &str,
    tool_definitions: &[crate::tools::ToolDefinition],
    messages: &[Message],
    model_id: &str,
    api_context_window: Option<u64>,
) -> ContextEstimate {
    // Serialize tool definitions to JSON for estimation
    let tool_definitions_json = if tool_definitions.is_empty() {
        String::new()
    } else {
        serde_json::to_string(tool_definitions).unwrap_or_default()
    };

    // Use API-provided context window if available, otherwise look up from registry or fallback
    let context_window = api_context_window.unwrap_or_else(|| {
        // Try to get from model service (database cache) first
        let context_length = crate::llm::ModelService::get_context_length(model_id);
        context_length as u64
    });

    ContextEstimate::new(system_prompt, &tool_definitions_json, messages, context_window)
}

/// Check if compression is needed based on a context estimate.
pub fn needs_compression(estimate: &ContextEstimate, settings: &CompressionSettings) -> bool {
    settings.enabled && estimate.needs_compression(settings.threshold_percent)
}

/// Legacy compression check - use create_context_estimate for full context estimation.
#[deprecated(note = "Use create_context_estimate for comprehensive estimation including system prompt and tools")]
pub fn check_compression_needed(
    messages: &[Message],
    model_id: &str,
    _settings: &CompressionSettings,
    api_context_window: Option<u64>,
) -> ContextEstimate {
    // Create estimate with empty system prompt and tools for backwards compatibility
    create_context_estimate("", &[], messages, model_id, api_context_window)
}

/// Compress messages using the configured strategy (sync version).
///
/// For the summarizing strategy, this falls back to truncation
/// since summarization requires async. Use `compress_messages_async` for full support.
pub fn compress_messages(
    messages: Vec<Message>,
    settings: &CompressionSettings,
    _target_tokens: usize,
) -> Vec<Message> {
    if messages.is_empty() {
        return messages;
    }

    // Both strategies use the same truncation logic for sync calls.
    // Summarizing uses async for full LLM-based compression.
    truncate_messages_by_tokens(
        messages,
        settings.preserve_first as usize,
        settings.protected_tokens as usize,
    )
}

/// Token-based truncation similar to code_puppy's implementation.
///
/// This strategy:
/// 1. Always preserves the first N messages (typically system prompt)
/// 2. Scans messages from most recent backwards
/// 3. Keeps messages until protected_tokens limit is exceeded
/// 4. Returns: preserved_first + recent messages within token budget
///
/// This is a LIFO approach that prioritizes recent context.
pub fn truncate_messages_by_tokens(
    messages: Vec<Message>,
    preserve_first: usize,
    protected_tokens: usize,
) -> Vec<Message> {
    if messages.is_empty() {
        return messages;
    }

    let total = messages.len();
    let preserve_start = preserve_first.min(total);

    // Always keep the first N messages (system prompt, etc.)
    let mut result: Vec<Message> = messages.iter().take(preserve_start).cloned().collect();

    // If there's nothing after the preserved first, return as-is
    if preserve_start >= total {
        return result;
    }

    // Build a stack of recent messages (LIFO) until we exceed protected_tokens
    let remaining_messages = &messages[preserve_start..];
    let mut stack: Vec<Message> = Vec::new();
    let mut accumulated_tokens: usize = 0;

    // Scan from most recent backwards
    for msg in remaining_messages.iter().rev() {
        let msg_tokens = rig::compression::estimate_message_tokens(msg);
        if accumulated_tokens + msg_tokens > protected_tokens {
            // This message would exceed the budget, stop here
            break;
        }
        accumulated_tokens += msg_tokens;
        stack.push(msg.clone());
    }

    // Pop from stack to restore chronological order
    while let Some(msg) = stack.pop() {
        result.push(msg);
    }

    tracing::debug!(
        "Truncation: kept {} of {} messages ({} tokens in protected region)",
        result.len(),
        total,
        accumulated_tokens
    );

    result
}

/// Async compression using LLM-based summarization.
///
/// This function uses rig's SummarizingCompressor to:
/// 1. Identify messages that would be truncated
/// 2. Send those messages to an LLM to generate a "Continuity Briefing"
/// 3. Inject the briefing between preserved initial context and recent messages
///
/// The summarizer can be any model that implements the Prompt trait.
pub async fn compress_messages_async<P: Prompt + Send + Sync + 'static>(
    messages: Vec<Message>,
    settings: &CompressionSettings,
    target_tokens: usize,
    summarizer: Arc<P>,
) -> Result<Vec<Message>, CompressionError> {
    if messages.is_empty() {
        return Ok(messages);
    }

    match settings.strategy {
        CompressionStrategy::Truncation => {
            // For truncation, just use the sync version
            Ok(truncate_messages_by_tokens(
                messages,
                settings.preserve_first as usize,
                settings.protected_tokens as usize,
            ))
        }
        CompressionStrategy::Summarizing => {
            // Use rig's SummarizingCompressor for LLM-based summarization
            let compressor = SummarizingCompressor::from_arc(summarizer)
                .with_preserve_first(settings.preserve_first as usize)
                .with_preserve_recent(4) // Keep last 4 messages for context
                .with_max_summary_tokens(2000); // Reasonable summary size

            compressor.compress_async(messages, target_tokens).await
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
        // Claude 3.5 Sonnet has 200k context
        assert!(get_model_context_window("claude-3-5-sonnet-20241022") >= 200_000);
        // GPT-4o has 128k context
        assert_eq!(get_model_context_window("gpt-4o"), 128_000);
        // GPT-4 Turbo has 128k context
        assert_eq!(get_model_context_window("gpt-4-turbo"), 128_000);
        // Gemini 1.5 Pro has 1M+ context
        assert!(get_model_context_window("gemini-1.5-pro") >= 1_000_000);
        // Unknown models fall back to default
        assert_eq!(get_model_context_window("unknown-model"), 100_000);
    }

    #[test]
    fn test_context_estimate() {
        let settings = CompressionSettings {
            enabled: true,
            threshold_percent: 80,
            strategy: CompressionStrategy::Truncation,
            summarizer_model: None,
            preserve_first: 1,
            protected_tokens: 50_000,
        };

        let messages = vec![Message::user("Hello"), Message::assistant("Hi there!")];

        let estimate = create_context_estimate(
            "You are a helpful assistant.",
            &[],
            &messages,
            "claude-3-5-sonnet-20241022",
            None,
        );

        // Claude 3.5 Sonnet has 200k context
        assert!(estimate.context_window >= 200_000);
        // Threshold should be 80% of context window
        let expected_threshold = (estimate.context_window as f64 * 0.8) as u64;
        assert_eq!(estimate.threshold_tokens(80), expected_threshold);
        assert!(!needs_compression(&estimate, &settings)); // Small messages don't need compression

        // Verify components are calculated
        assert!(estimate.system_prompt_tokens > 0);
        assert!(estimate.messages_tokens > 0);
        assert_eq!(
            estimate.total_tokens,
            estimate.system_prompt_tokens + estimate.tool_definitions_tokens + estimate.messages_tokens
        );
    }

    #[test]
    fn test_truncate_messages_by_tokens() {
        // Create messages with longer content to test truncation
        let long_text = "This is a fairly long message that will use up quite a few tokens. ".repeat(20);
        let messages = vec![
            Message::user("System prompt - this should always be preserved"),
            Message::assistant(long_text.clone()),
            Message::user(long_text.clone()),
            Message::assistant(long_text.clone()),
            Message::user("Third user message - relatively short"),
            Message::assistant("Third response - most recent, also short"),
        ];

        // With a small protected_tokens limit, only recent short messages should be kept
        let result = truncate_messages_by_tokens(messages.clone(), 1, 200);

        // Should keep first message (system prompt) + some recent messages
        assert!(!result.is_empty());
        assert!(result.len() < messages.len(), "Expected truncation but got {} of {} messages", result.len(), messages.len());

        // With large protected_tokens, all should be kept
        let result_all = truncate_messages_by_tokens(messages.clone(), 1, 100_000);
        assert_eq!(result_all.len(), messages.len());
    }

    #[test]
    fn test_truncate_preserves_first() {
        let messages = vec![
            Message::user("First message"),
            Message::assistant("Second message"),
            Message::user("Third message"),
        ];

        // Even with 0 protected tokens, first message should be preserved
        let result = truncate_messages_by_tokens(messages, 1, 0);
        assert_eq!(result.len(), 1);
    }
}
