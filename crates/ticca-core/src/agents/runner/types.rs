//! Core types and constants for the agent runner.

use crate::session::MessageRole;
use crate::tools::{AgentCallEvent, AgentStreamEvent, TodoListEvent};
use tokio::time::Duration;

/// Cooldown duration in seconds after rate limit errors.
pub const DEFAULT_COOLDOWN_SECS: i64 = 60;

/// System prompt identifier for Claude.
pub const CLAUDE_CODE_INSTRUCTIONS: &str =
    "You are Claude Code, Anthropic's official CLI for Claude.";

/// Timeout for MCP server connections.
pub const MCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Maximum characters for subagent output to prevent context explosion.
/// ~30k tokens at 3.4 chars/token = ~100k chars
pub const MAX_SUBAGENT_OUTPUT_CHARS: usize = 100_000;

/// Minimum window in milliseconds for stream statistics.
pub const STATS_WINDOW_MIN_MS: u64 = 200;

/// A message in the chat history with optional reasoning.
#[derive(Debug, Clone)]
pub struct ChatHistoryMessage {
    pub role: MessageRole,
    pub content: String,
    /// Reasoning/thinking content for interleaved thinking support
    pub reasoning: Option<String>,
    /// Signature for reasoning content (required by Claude for verification)
    pub reasoning_signature: Option<String>,
}

/// Events emitted by the agent runner during streaming.
#[derive(Debug, Clone)]
pub enum RunnerEvent {
    /// Text chunk from the LLM response
    StreamChunk(String),
    /// Reasoning/thinking content with optional signature (for Claude verification)
    Reasoning {
        text: String,
        signature: Option<String>,
    },
    /// Stream statistics for TPS calculation
    StreamStats {
        chars_in_window: usize,
        window_ms: u64,
    },
    /// Tool call notification
    ToolCall { name: String, args: String },
    /// Agent-to-agent call
    AgentCall(AgentCallEvent),
    /// Subagent stream forwarding
    SubagentStream(AgentStreamEvent),
    /// Todo list event
    TodoEvent(TodoListEvent),
    /// Tool approval requested
    ToolApprovalRequested { id: u64, name: String, args: String },
    /// Token usage from the API response
    Usage {
        input_tokens: u64,
        output_tokens: u64,
    },
    /// Pre-request context estimate
    ContextEstimate {
        system_prompt_tokens: usize,
        tool_definitions_tokens: usize,
        messages_tokens: usize,
        total_tokens: usize,
        context_window: u64,
        usage_percent: u32,
    },
    /// Context compression was applied
    ContextCompressed {
        original_messages: usize,
        compressed_messages: usize,
        original_tokens: usize,
        compressed_tokens: usize,
        strategy: String,
    },
    /// Context usage warning (approaching limit)
    ContextUsageWarning {
        current_tokens: usize,
        threshold_tokens: u64,
        context_window: u64,
        usage_percent: u32,
    },
    /// Stream completed successfully
    StreamComplete,
    /// Stream was stopped by user
    StreamStopped,
    /// Stream encountered an error
    StreamError(String),
}
