//! Context estimation logic.
//!
//! Note: Full compression support will be added when serdesAI compression is implemented.

use super::types::RunnerEvent;
use crate::config::{CompressionSettings, ConfigDatabase};
use serdes_ai_core::ModelRequest;
use tokio::sync::mpsc;

/// Estimate tokens from text (rough approximation: ~3.4 chars per token)
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() as f32 / 3.4).ceil() as usize
}

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

/// Get context window for a model.
fn context_window_for_model(model_name: &str) -> u64 {
    if model_name.contains("claude") {
        200_000 // Claude models have 200k context
    } else if model_name.contains("gpt-4") || model_name.contains("chatgpt") {
        128_000 // GPT-4 models have 128k context
    } else {
        100_000 // Default fallback
    }
}

/// Estimate context usage and check if compression is needed.
pub fn estimate_context(
    system_prompt: &str,
    messages: &[ModelRequest],
    model_name: &str,
) -> ContextEstimation {
    // Estimate tool definitions tokens from specs
    let estimated_tool_tokens = crate::tools::spec::estimate_all_tools_tokens();

    // Estimate system prompt tokens
    let system_prompt_tokens = estimate_tokens(system_prompt);

    // Estimate message tokens
    let messages_tokens: usize = messages
        .iter()
        .map(|req| {
            serde_json::to_string(req)
                .map(|s| estimate_tokens(&s))
                .unwrap_or(100) // Fallback estimate
        })
        .sum();

    let context_window = context_window_for_model(model_name);

    // Load compression settings
    let compression_settings = ConfigDatabase::open()
        .ok()
        .map(|db| CompressionSettings::load_from(&db))
        .unwrap_or_default();

    let total_tokens = system_prompt_tokens + estimated_tool_tokens + messages_tokens;
    let threshold_percent = compression_settings.threshold_percent as f64 / 100.0;
    let threshold_tokens = (context_window as f64 * threshold_percent) as u64;
    
    let compression_needed = compression_settings.enabled && total_tokens as u64 > threshold_tokens;

    ContextEstimation {
        system_prompt_tokens,
        tool_definitions_tokens: estimated_tool_tokens,
        messages_tokens,
        total_tokens,
        context_window,
        usage_percent: ((total_tokens as f64 / context_window as f64) * 100.0) as u32,
        threshold_tokens,
        needs_compression: compression_needed,
    }
}

/// Apply compression to messages if needed and emit events.
/// 
/// Note: Currently a placeholder - full compression will be implemented
/// when serdesAI compression utilities are available.
pub fn apply_compression_if_needed(
    messages: Vec<ModelRequest>,
    estimation: &ContextEstimation,
    event_tx: &mpsc::UnboundedSender<RunnerEvent>,
) -> Vec<ModelRequest> {
    if !estimation.needs_compression {
        return messages;
    }

    // For now, just truncate by removing oldest messages if over threshold
    let compression_settings = ConfigDatabase::open()
        .ok()
        .map(|db| CompressionSettings::load_from(&db))
        .unwrap_or_default();

    let original_count = messages.len();
    let original_tokens = estimation.messages_tokens;

    // Simple truncation: keep last N messages that fit
    let target_tokens = estimation.threshold_tokens as usize / 2; // Aim for 50% of threshold
    let mut compressed = Vec::new();
    let mut running_tokens = 0usize;

    // Keep messages from the end
    for msg in messages.into_iter().rev() {
        let msg_tokens = serde_json::to_string(&msg)
            .map(|s| estimate_tokens(&s))
            .unwrap_or(100);
        
        if running_tokens + msg_tokens > target_tokens && !compressed.is_empty() {
            break;
        }
        running_tokens += msg_tokens;
        compressed.push(msg);
    }
    
    compressed.reverse(); // Restore chronological order

    let compressed_count = compressed.len();
    let compressed_tokens = running_tokens;

    tracing::info!(
        "Context compressed: {} -> {} messages, {} -> {} tokens (strategy: {})",
        original_count,
        compressed_count,
        original_tokens,
        compressed_tokens,
        compression_settings.strategy.as_str()
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
    messages: &[ModelRequest],
    estimation: &ContextEstimation,
) {
    tracing::info!(
        "Context estimate: system_prompt={} chars, {} messages",
        system_prompt.len(),
        messages.len()
    );
    tracing::info!(
        "Estimated totals: system={}, tools={} ({} tool specs), messages={}, grand_total={}",
        estimation.system_prompt_tokens,
        estimation.tool_definitions_tokens,
        crate::tools::spec::all_specs().len(),
        estimation.messages_tokens,
        estimation.total_tokens
    );
    tracing::info!(
        "Context window: {}, usage: {}%, threshold: {}",
        estimation.context_window,
        estimation.usage_percent,
        estimation.threshold_tokens
    );
}
