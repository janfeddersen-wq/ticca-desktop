//! Bridge types for Rust-Python communication
//!
//! These types define the data structures exchanged between Rust and Python,
//! using serde for serialization. The Python side should use Pydantic models
//! with matching field names and types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request from Rust to Python agent
///
/// Contains all information needed to execute an agent request,
/// including conversation context and model configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    /// Unique session identifier
    pub session_id: String,
    /// Unique conversation identifier within the session
    pub conversation_id: String,
    /// The user's message
    pub message: String,
    /// Name of the agent to invoke
    pub agent_name: String,
    /// Optional model override (e.g., "gpt-4", "claude-3-opus")
    pub model: Option<String>,
    /// Temperature for generation (0.0 - 2.0)
    pub temperature: Option<f32>,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// List of enabled tool names
    pub tools_enabled: Vec<String>,
    /// Additional context data
    pub context: HashMap<String, serde_json::Value>,
}

impl AgentRequest {
    /// Create a new agent request with minimal required fields
    pub fn new(
        session_id: impl Into<String>,
        conversation_id: impl Into<String>,
        message: impl Into<String>,
        agent_name: impl Into<String>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            conversation_id: conversation_id.into(),
            message: message.into(),
            agent_name: agent_name.into(),
            model: None,
            temperature: None,
            max_tokens: None,
            tools_enabled: Vec::new(),
            context: HashMap::new(),
        }
    }

    /// Set the model to use
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the temperature
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }

    /// Enable specific tools
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tools_enabled = tools;
        self
    }

    /// Add context data
    pub fn with_context(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.context.insert(key.into(), value);
        self
    }
}

/// Response from Python agent to Rust
///
/// Represents a complete agent response including any tool calls
/// and token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    /// Unique message identifier
    pub message_id: String,
    /// Response content
    pub content: String,
    /// Role of the responder (usually "assistant")
    pub role: String,
    /// Any tool calls made by the agent
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Token usage statistics
    pub usage: TokenUsage,
    /// Reason the generation finished
    pub finish_reason: FinishReason,
}

impl AgentResponse {
    /// Create a simple text response
    pub fn text(message_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            message_id: message_id.into(),
            content: content.into(),
            role: "assistant".to_string(),
            tool_calls: None,
            usage: TokenUsage::default(),
            finish_reason: FinishReason::Stop,
        }
    }

    /// Create an error response
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message_id: uuid::Uuid::new_v4().to_string(),
            content: String::new(),
            role: "assistant".to_string(),
            tool_calls: None,
            usage: TokenUsage::default(),
            finish_reason: FinishReason::Error(message.into()),
        }
    }
}

/// A tool call made by the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this tool call
    pub id: String,
    /// Name of the tool being called
    pub name: String,
    /// Arguments passed to the tool (JSON)
    pub arguments: serde_json::Value,
}

impl ToolCall {
    /// Create a new tool call
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: serde_json::Value,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments,
        }
    }
}

/// Token usage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Tokens used in the prompt
    pub prompt_tokens: u32,
    /// Tokens generated in completion
    pub completion_tokens: u32,
    /// Total tokens used
    pub total_tokens: u32,
}

impl TokenUsage {
    /// Create new token usage stats
    pub fn new(prompt: u32, completion: u32) -> Self {
        Self {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
        }
    }
}

/// Reason why generation finished
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "value")]
pub enum FinishReason {
    /// Normal completion (stop sequence hit)
    #[default]
    Stop,
    /// Agent wants to use a tool
    ToolUse,
    /// Hit the maximum token limit
    MaxTokens,
    /// An error occurred
    Error(String),
}

/// Streaming chunk from Python to Rust
///
/// Represents incremental updates during agent execution,
/// allowing for real-time UI updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamChunk {
    /// Text content delta
    TextDelta {
        content: String,
    },
    /// Thinking/reasoning delta (for models that expose this)
    ThinkingDelta {
        content: String,
    },
    /// A tool call is starting
    ToolStart {
        tool_call: ToolCall,
    },
    /// Result from a tool call
    ToolResult {
        tool_call_id: String,
        result: String,
    },
    /// Agent state transition
    StateChange {
        from: AgentState,
        to: AgentState,
    },
    /// Token usage update
    Usage {
        usage: TokenUsage,
    },
    /// Stream completed successfully
    Done {
        response: AgentResponse,
    },
    /// An error occurred
    Error {
        message: String,
    },
}

impl StreamChunk {
    /// Create a text delta chunk
    pub fn text(content: impl Into<String>) -> Self {
        Self::TextDelta {
            content: content.into(),
        }
    }

    /// Create a thinking delta chunk
    pub fn thinking(content: impl Into<String>) -> Self {
        Self::ThinkingDelta {
            content: content.into(),
        }
    }

    /// Create a tool start chunk
    pub fn tool_start(tool_call: ToolCall) -> Self {
        Self::ToolStart { tool_call }
    }

    /// Create a tool result chunk
    pub fn tool_result(tool_call_id: impl Into<String>, result: impl Into<String>) -> Self {
        Self::ToolResult {
            tool_call_id: tool_call_id.into(),
            result: result.into(),
        }
    }

    /// Create a state change chunk
    pub fn state_change(from: AgentState, to: AgentState) -> Self {
        Self::StateChange { from, to }
    }

    /// Create a done chunk
    pub fn done(response: AgentResponse) -> Self {
        Self::Done { response }
    }

    /// Create an error chunk
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
        }
    }
}

/// Agent execution state
///
/// Represents the current phase of agent execution,
/// useful for UI feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AgentState {
    /// Agent is idle, not processing
    #[default]
    Idle,
    /// Agent is thinking/reasoning
    Thinking,
    /// Agent is executing a tool
    Acting,
    /// Agent is observing tool results
    Observing,
    /// Agent is generating final response
    Responding,
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentState::Idle => write!(f, "idle"),
            AgentState::Thinking => write!(f, "thinking"),
            AgentState::Acting => write!(f, "acting"),
            AgentState::Observing => write!(f, "observing"),
            AgentState::Responding => write!(f, "responding"),
        }
    }
}

/// Information about an available agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Unique agent identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Agent description
    pub description: Option<String>,
    /// Default model for this agent
    pub default_model: String,
    /// Available tools for this agent
    pub available_tools: Vec<String>,
    /// Agent capabilities/tags
    pub capabilities: Vec<String>,
}

impl AgentInfo {
    /// Create new agent info
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        default_model: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            default_model: default_model.into(),
            available_tools: Vec::new(),
            capabilities: Vec::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add available tools
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.available_tools = tools;
        self
    }

    /// Add capabilities
    pub fn with_capabilities(mut self, caps: Vec<String>) -> Self {
        self.capabilities = caps;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_request_builder() {
        let request = AgentRequest::new("session-1", "conv-1", "Hello!", "default")
            .with_model("gpt-4")
            .with_temperature(0.7)
            .with_max_tokens(1000)
            .with_tools(vec!["search".to_string()])
            .with_context("key", serde_json::json!("value"));

        assert_eq!(request.session_id, "session-1");
        assert_eq!(request.model, Some("gpt-4".to_string()));
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(1000));
        assert_eq!(request.tools_enabled, vec!["search"]);
        assert_eq!(request.context.get("key"), Some(&serde_json::json!("value")));
    }

    #[test]
    fn test_stream_chunk_serialization() {
        let chunk = StreamChunk::text("Hello");
        let json = serde_json::to_string(&chunk).expect("serialize");
        assert!(json.contains("TextDelta"));
        assert!(json.contains("Hello"));

        let state_chunk = StreamChunk::state_change(AgentState::Idle, AgentState::Thinking);
        let json = serde_json::to_string(&state_chunk).expect("serialize");
        assert!(json.contains("StateChange"));
    }

    #[test]
    fn test_finish_reason_serialization() {
        let stop = FinishReason::Stop;
        let json = serde_json::to_string(&stop).expect("serialize");
        assert!(json.contains("Stop"));

        let error = FinishReason::Error("something went wrong".to_string());
        let json = serde_json::to_string(&error).expect("serialize");
        assert!(json.contains("Error"));
        assert!(json.contains("something went wrong"));
    }

    #[test]
    fn test_agent_state_display() {
        assert_eq!(AgentState::Thinking.to_string(), "thinking");
        assert_eq!(AgentState::Acting.to_string(), "acting");
    }
}
