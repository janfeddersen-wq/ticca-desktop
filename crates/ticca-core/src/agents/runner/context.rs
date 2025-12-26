//! Context estimation and compression logic.

use crate::compression::{compress_messages, create_context_estimate, needs_compression};
use crate::config::{CompressionSettings, ConfigDatabase};
use super::types::RunnerEvent;
use tokio::sync::mpsc;

/// Context estimation result with all relevant metrics.
pub struct ContextEstimation {
    pub system_prompt_tokens: usize,
    pub tool_definitions_tokens: usize,
    pub messages_tokens: usize,
    pub total_tokens: usize,
    pub context_window: u64,
    pub usage_percent: u32,
    pub threshold_tokens: u64,
    pub needs_compression: bool,
}

/// Estimate context usage and check if compression is needed.
pub fn estimate_context(
    system_prompt: &str,
    messages: &[rig::message::Message],
    model_name: &str,
) -> ContextEstimation {
    // Estimate tool definitions tokens from specs
    let estimated_tool_tokens = crate::tools::spec::estimate_all_tools_tokens();

    // Create comprehensive context estimate
    let context_estimate = create_context_estimate(
        system_prompt,
        &[],
        messages,
        model_name,
        None,
    );

    // Load compression settings
    let compression_settings = ConfigDatabase::open()
        .ok()
        .map(|db| CompressionSettings::load_from(&db))
        .unwrap_or_default();

    let total_with_tools = context_estimate.total_tokens + estimated_tool_tokens;
    let threshold_tokens = context_estimate.threshold_tokens(compression_settings.threshold_percent);
    let compression_needed = needs_compression(&context_estimate, &compression_settings);

    ContextEstimation {
        system_prompt_tokens: context_estimate.system_prompt_tokens,
        tool_definitions_tokens: estimated_tool_tokens,
        messages_tokens: context_estimate.messages_tokens,
        total_tokens: total_with_tools,
        context_window: context_estimate.context_window,
        usage_percent: ((total_with_tools as f64 / context_estimate.context_window as f64) * 100.0) as u32,
        threshold_tokens,
        needs_compression: compression_needed,
    }
}

/// Apply compression to messages if needed and emit events.
pub fn apply_compression_if_needed(
    messages: Vec<rig::message::Message>,
    estimation: &ContextEstimation,
    event_tx: &mpsc::UnboundedSender<RunnerEvent>,
) -> Vec<rig::message::Message> {
    if !estimation.needs_compression {
        return messages;
    }

    let compression_settings = ConfigDatabase::open()
        .ok()
        .map(|db| CompressionSettings::load_from(&db))
        .unwrap_or_default();

    let original_count = messages.len();
    let original_tokens = estimation.messages_tokens;

    let compressed = compress_messages(
        messages,
        &compression_settings,
        estimation.threshold_tokens as usize,
    );

    let compressed_count = compressed.len();
    let compressed_tokens = rig::compression::estimate_messages_tokens(&compressed);

    tracing::info!(
        "Context compressed: {} -> {} messages, {} -> {} tokens (strategy: {:?})",
        original_count,
        compressed_count,
        original_tokens,
        compressed_tokens,
        compression_settings.strategy
    );

    let _ = event_tx.send(RunnerEvent::ContextCompressed {
        original_messages: original_count,
        compressed_messages: compressed_count,
        original_tokens,
        compressed_tokens,
        strategy: compression_settings.strategy.as_str().to_string(),
    });

    compressed
}

/// Log detailed context breakdown for debugging.
pub fn log_context_breakdown(
    system_prompt: &str,
    messages: &[rig::message::Message],
    estimation: &ContextEstimation,
) {
    use rig::compression::estimate_message_tokens;

    let mut total_reasoning_tokens = 0usize;
    let mut total_reasoning_chars = 0usize;
    let mut total_text_tokens = 0usize;
    let mut total_text_chars = 0usize;
    let mut messages_with_reasoning = 0usize;

    for (i, msg) in messages.iter().enumerate() {
        let _msg_tokens = estimate_message_tokens(msg);
        if let rig::message::Message::Assistant { content, .. } = msg {
            for c in content.iter() {
                if let rig::message::AssistantContent::Reasoning(r) = c {
                    let chars: usize = r.reasoning.iter().map(|s| s.len()).sum();
                    let reasoning_tokens = (chars as f32 / 3.4).ceil() as usize;
                    total_reasoning_tokens += reasoning_tokens;
                    total_reasoning_chars += chars;
                    messages_with_reasoning += 1;
                    tracing::debug!(
                        "Message {} has {} reasoning chars ({} tokens)",
                        i,
                        chars,
                        reasoning_tokens
                    );
                }
                if let rig::message::AssistantContent::Text(t) = c {
                    total_text_chars += t.text.len();
                    total_text_tokens += (t.text.len() as f32 / 3.4).ceil() as usize;
                }
            }
        }
        if let rig::message::Message::User { content } = msg {
            for c in content.iter() {
                if let rig::message::UserContent::Text(t) = c {
                    total_text_chars += t.text.len();
                    total_text_tokens += (t.text.len() as f32 / 3.4).ceil() as usize;
                }
            }
        }
    }

    tracing::info!(
        "Context estimate: system_prompt={} chars, {} messages ({} with reasoning)",
        system_prompt.len(),
        messages.len(),
        messages_with_reasoning
    );
    tracing::info!(
        "Content breakdown: text={} chars ({} tokens), reasoning={} chars ({} tokens)",
        total_text_chars,
        total_text_tokens,
        total_reasoning_chars,
        total_reasoning_tokens
    );
    tracing::info!(
        "Estimated totals: system={}, tools={} ({} tool specs), messages={}, grand_total={}",
        estimation.system_prompt_tokens,
        estimation.tool_definitions_tokens,
        crate::tools::spec::all_specs().len(),
        estimation.messages_tokens,
        estimation.total_tokens
    );
}
