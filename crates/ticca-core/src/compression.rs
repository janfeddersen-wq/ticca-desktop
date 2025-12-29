//! Context compression service for managing LLM context window limits.
//!
//! This module provides:
//! - Model context window lookup (via [`crate::registry::RegistryService`])
//! - Token estimation (chars / 3.4 ratio)
//! - Truncation-based compression (LIFO - keeps system prompt + recent messages)
//!
//! Context window sizes are sourced from the model registry, which provides
//! authoritative metadata for all known models.

use serdes_ai_core::{ModelRequest, ModelRequestPart, UserContent};

use crate::config::{CompressionSettings, CompressionStrategy};

/// Compression error type.
#[derive(Debug, thiserror::Error)]
pub enum CompressionError {
    /// Compression operation failed.
    #[error("Compression failed: {0}")]
    Failed(String),
}

/// Estimate tokens from text using chars/3.4 ratio.
///
/// This is a simple heuristic that works reasonably well for English text.
/// Different languages and content types may have different ratios.
#[inline]
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() as f64 / 3.4).ceil() as usize
}

/// Estimate tokens for a ModelRequest by serializing to JSON.
pub fn estimate_message_tokens(request: &ModelRequest) -> usize {
    serde_json::to_string(request)
        .map(|s| estimate_tokens(&s))
        .unwrap_or(0)
}

/// Estimate tokens for a single request part.
pub fn estimate_part_tokens(part: &ModelRequestPart) -> usize {
    serde_json::to_string(part)
        .map(|s| estimate_tokens(&s))
        .unwrap_or(0)
}

/// Context estimate for tracking token usage across all components.
#[derive(Debug, Clone)]
pub struct ContextEstimate {
    /// Tokens used by the system prompt.
    pub system_prompt_tokens: u64,
    /// Tokens used by tool definitions.
    pub tool_definitions_tokens: u64,
    /// Tokens used by conversation messages.
    pub messages_tokens: u64,
    /// Total tokens (sum of all components).
    pub total_tokens: u64,
    /// Model's context window size.
    pub context_window: u64,
}

impl ContextEstimate {
    /// Create a new context estimate from components.
    pub fn new(
        system_prompt: &str,
        tool_definitions_json: &str,
        messages: &[ModelRequest],
        context_window: u64,
    ) -> Self {
        let system_prompt_tokens = estimate_tokens(system_prompt) as u64;
        let tool_definitions_tokens = estimate_tokens(tool_definitions_json) as u64;
        let messages_tokens = messages
            .iter()
            .map(|m| estimate_message_tokens(m))
            .sum::<usize>() as u64;
        let total_tokens = system_prompt_tokens + tool_definitions_tokens + messages_tokens;

        Self {
            system_prompt_tokens,
            tool_definitions_tokens,
            messages_tokens,
            total_tokens,
            context_window,
        }
    }

    /// Create a context estimate from request parts (for single-request estimation).
    pub fn from_parts(
        system_prompt: &str,
        tool_definitions_json: &str,
        parts: &[ModelRequestPart],
        context_window: u64,
    ) -> Self {
        let system_prompt_tokens = estimate_tokens(system_prompt) as u64;
        let tool_definitions_tokens = estimate_tokens(tool_definitions_json) as u64;
        let messages_tokens = parts
            .iter()
            .map(|p| estimate_part_tokens(p))
            .sum::<usize>() as u64;
        let total_tokens = system_prompt_tokens + tool_definitions_tokens + messages_tokens;

        Self {
            system_prompt_tokens,
            tool_definitions_tokens,
            messages_tokens,
            total_tokens,
            context_window,
        }
    }

    /// Check if compression is needed based on threshold percentage.
    pub fn needs_compression(&self, threshold_percent: u32) -> bool {
        let threshold = self.threshold_tokens(threshold_percent);
        self.total_tokens > threshold
    }

    /// Calculate threshold tokens from percentage.
    pub fn threshold_tokens(&self, threshold_percent: u32) -> u64 {
        (self.context_window as f64 * threshold_percent as f64 / 100.0) as u64
    }

    /// Get available tokens (context_window - total_tokens).
    pub fn available_tokens(&self) -> u64 {
        self.context_window.saturating_sub(self.total_tokens)
    }

    /// Get usage percentage.
    pub fn usage_percent(&self) -> f64 {
        if self.context_window == 0 {
            100.0
        } else {
            (self.total_tokens as f64 / self.context_window as f64) * 100.0
        }
    }
}

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
/// * `parts` - All conversation message parts
/// * `model_id` - Model identifier for context window lookup
/// * `api_context_window` - Optional API-provided context window (overrides lookup)
pub fn create_context_estimate(
    system_prompt: &str,
    tool_definitions: &[crate::tools::ToolDefinition],
    parts: &[ModelRequestPart],
    model_id: &str,
    api_context_window: Option<u64>,
) -> ContextEstimate {
    // Serialize tool definitions to JSON for estimation
    let tool_definitions_json = if tool_definitions.is_empty() {
        String::new()
    } else {
        serde_json::to_string(tool_definitions).unwrap_or_default()
    };

    // Use API-provided context window if available, otherwise look up from registry
    let context_window = api_context_window.unwrap_or_else(|| {
        crate::llm::ModelService::get_context_length(model_id) as u64
    });

    ContextEstimate::from_parts(system_prompt, &tool_definitions_json, parts, context_window)
}

/// Check if compression is needed based on a context estimate.
pub fn needs_compression(estimate: &ContextEstimate, settings: &CompressionSettings) -> bool {
    settings.enabled && estimate.needs_compression(settings.threshold_percent)
}

/// Compress message parts using the configured strategy.
///
/// Currently only truncation is supported. Summarization can be added later.
pub fn compress_parts(
    parts: Vec<ModelRequestPart>,
    settings: &CompressionSettings,
    _target_tokens: usize,
) -> Vec<ModelRequestPart> {
    if parts.is_empty() {
        return parts;
    }

    // Both strategies use truncation for now
    // Summarizing can be added later with async support
    truncate_parts_by_tokens(
        parts,
        settings.preserve_first as usize,
        settings.protected_tokens as usize,
    )
}

/// Token-based truncation for message parts.
///
/// This strategy:
/// 1. Always preserves the first N parts (typically system prompt)
/// 2. Scans parts from most recent backwards
/// 3. Keeps parts until protected_tokens limit is exceeded
/// 4. Returns: preserved_first + recent parts within token budget
///
/// This is a LIFO approach that prioritizes recent context.
pub fn truncate_parts_by_tokens(
    parts: Vec<ModelRequestPart>,
    preserve_first: usize,
    protected_tokens: usize,
) -> Vec<ModelRequestPart> {
    if parts.is_empty() {
        return parts;
    }

    let total = parts.len();
    let preserve_start = preserve_first.min(total);

    // Always keep the first N parts (system prompt, etc.)
    let mut result: Vec<ModelRequestPart> = parts.iter().take(preserve_start).cloned().collect();

    // If there's nothing after the preserved first, return as-is
    if preserve_start >= total {
        return result;
    }

    // Build a stack of recent parts (LIFO) until we exceed protected_tokens
    let remaining_parts = &parts[preserve_start..];
    let mut stack: Vec<ModelRequestPart> = Vec::new();
    let mut accumulated_tokens: usize = 0;

    // Scan from most recent backwards
    for part in remaining_parts.iter().rev() {
        let part_tokens = estimate_part_tokens(part);
        if accumulated_tokens + part_tokens > protected_tokens {
            // This part would exceed the budget, stop here
            break;
        }
        accumulated_tokens += part_tokens;
        stack.push(part.clone());
    }

    // Pop from stack to restore chronological order
    while let Some(part) = stack.pop() {
        result.push(part);
    }

    tracing::debug!(
        "Truncation: kept {} of {} parts ({} tokens in protected region)",
        result.len(),
        total,
        accumulated_tokens
    );

    result
}

/// Convert internal chat history format to ModelRequest parts.
pub fn chat_history_to_model_requests(
    history: &[(crate::session::MessageRole, String)],
) -> Vec<ModelRequestPart> {
    use crate::session::MessageRole;
    use serdes_ai_core::{TextPart, UserPromptPart, ModelResponsePart, ModelResponse};

    history
        .iter()
        .filter_map(|(role, content)| match role {
            MessageRole::User => Some(ModelRequestPart::UserPrompt(
                UserPromptPart::new(UserContent::text(content.clone())),
            )),
            MessageRole::Assistant => {
                // Wrap assistant content in a ModelResponse
                let response = ModelResponse {
                    parts: vec![ModelResponsePart::Text(TextPart::new(content.clone()))],
                    ..Default::default()
                };
                Some(ModelRequestPart::ModelResponse(Box::new(response)))
            }
            // System and Tool messages are handled separately
            MessageRole::System | MessageRole::Tool => None,
        })
        .collect()
}

/// Convert ModelRequest parts back to internal chat history format.
pub fn model_requests_to_chat_history(
    parts: Vec<ModelRequestPart>,
) -> Vec<(crate::session::MessageRole, String)> {
    use crate::session::MessageRole;

    parts
        .into_iter()
        .filter_map(|part| match part {
            ModelRequestPart::UserPrompt(user) => {
                let text = user.content.as_text().unwrap_or_default().to_string();
                if text.is_empty() {
                    None
                } else {
                    Some((MessageRole::User, text))
                }
            }
            ModelRequestPart::ModelResponse(response) => {
                // Extract text from response parts
                let text: String = response
                    .parts
                    .iter()
                    .filter_map(|p| {
                        if let serdes_ai_core::ModelResponsePart::Text(t) = p {
                            Some(t.content.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                if text.is_empty() {
                    None
                } else {
                    Some((MessageRole::Assistant, text))
                }
            }
            // Other parts don't map to simple chat history
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        // "Hello" = 5 chars -> 5/3.4 = 1.47 -> ceil = 2 tokens
        assert_eq!(estimate_tokens("Hello"), 2);

        // Empty string -> 0 tokens
        assert_eq!(estimate_tokens(""), 0);

        // Longer text
        let long_text = "This is a longer piece of text that should estimate to more tokens.";
        let expected = (long_text.len() as f64 / 3.4).ceil() as usize;
        assert_eq!(estimate_tokens(long_text), expected);
    }

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

        let parts = vec![
            ModelRequestPart::UserPrompt(serdes_ai_core::UserPromptPart::new(
                UserContent::text("Hello"),
            )),
        ];

        let estimate = create_context_estimate(
            "You are a helpful assistant.",
            &[],
            &parts,
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
            estimate.system_prompt_tokens
                + estimate.tool_definitions_tokens
                + estimate.messages_tokens
        );
    }

    #[test]
    fn test_context_estimate_helpers() {
        let estimate = ContextEstimate {
            system_prompt_tokens: 100,
            tool_definitions_tokens: 50,
            messages_tokens: 350,
            total_tokens: 500,
            context_window: 1000,
        };

        assert_eq!(estimate.available_tokens(), 500);
        assert_eq!(estimate.usage_percent(), 50.0);
        assert!(estimate.needs_compression(40)); // 40% of 1000 = 400, 500 > 400
        assert!(!estimate.needs_compression(60)); // 60% of 1000 = 600, 500 < 600
    }

    #[test]
    fn test_truncate_parts_by_tokens() {
        // Create parts with varying content to test truncation
        let long_text = "This is a fairly long message that will use up quite a few tokens. ".repeat(20);

        let parts = vec![
            ModelRequestPart::UserPrompt(serdes_ai_core::UserPromptPart::new(
                UserContent::text("System prompt - this should always be preserved"),
            )),
            ModelRequestPart::ModelResponse(Box::new(serdes_ai_core::ModelResponse {
                parts: vec![serdes_ai_core::ModelResponsePart::Text(
                    serdes_ai_core::TextPart::new(long_text.clone()),
                )],
                ..Default::default()
            })),
            ModelRequestPart::UserPrompt(serdes_ai_core::UserPromptPart::new(
                UserContent::text(long_text.clone()),
            )),
            ModelRequestPart::ModelResponse(Box::new(serdes_ai_core::ModelResponse {
                parts: vec![serdes_ai_core::ModelResponsePart::Text(
                    serdes_ai_core::TextPart::new(long_text.clone()),
                )],
                ..Default::default()
            })),
            ModelRequestPart::UserPrompt(serdes_ai_core::UserPromptPart::new(
                UserContent::text("Third user message - relatively short"),
            )),
            ModelRequestPart::ModelResponse(Box::new(serdes_ai_core::ModelResponse {
                parts: vec![serdes_ai_core::ModelResponsePart::Text(
                    serdes_ai_core::TextPart::new("Third response - most recent, also short".to_string()),
                )],
                ..Default::default()
            })),
        ];

        // With a small protected_tokens limit, only recent short parts should be kept
        let result = truncate_parts_by_tokens(parts.clone(), 1, 200);

        // Should keep first part (system prompt) + some recent parts
        assert!(!result.is_empty());
        assert!(
            result.len() < parts.len(),
            "Expected truncation but got {} of {} parts",
            result.len(),
            parts.len()
        );

        // With large protected_tokens, all should be kept
        let result_all = truncate_parts_by_tokens(parts.clone(), 1, 100_000);
        assert_eq!(result_all.len(), parts.len());
    }

    #[test]
    fn test_truncate_preserves_first() {
        let parts = vec![
            ModelRequestPart::UserPrompt(serdes_ai_core::UserPromptPart::new(
                UserContent::text("First message"),
            )),
            ModelRequestPart::ModelResponse(Box::new(serdes_ai_core::ModelResponse {
                parts: vec![serdes_ai_core::ModelResponsePart::Text(
                    serdes_ai_core::TextPart::new("Second message".to_string()),
                )],
                ..Default::default()
            })),
            ModelRequestPart::UserPrompt(serdes_ai_core::UserPromptPart::new(
                UserContent::text("Third message"),
            )),
        ];

        // Even with 0 protected tokens, first part should be preserved
        let result = truncate_parts_by_tokens(parts, 1, 0);
        assert_eq!(result.len(), 1);
    }
}
