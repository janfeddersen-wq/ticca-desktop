//! Session data models

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A chat session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub agent_type: String, // "planning" or "coding"
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub total_tokens: i64,
    pub message_count: i64,
}

impl Session {
    pub fn new(name: impl Into<String>, agent_type: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            agent_type: agent_type.into(),
            created_at: None,
            updated_at: None,
            total_tokens: 0,
            message_count: 0,
        }
    }

    pub fn planning(name: impl Into<String>) -> Self {
        Self::new(name, "planning")
    }

    pub fn coding(name: impl Into<String>) -> Self {
        Self::new(name, "coding")
    }
}

/// Message role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
            MessageRole::Tool => "tool",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "system" => MessageRole::System,
            "tool" => MessageRole::Tool,
            _ => MessageRole::User,
        }
    }
}

/// A message within a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub id: Option<i64>,
    pub session_id: String,
    pub role: MessageRole,
    pub content: String,
    pub tool_calls_json: Option<String>, // For assistant tool calls
    pub tool_result_json: Option<String>, // For tool responses
    pub reasoning: Option<String>,        // Thinking/reasoning content
    pub reasoning_signature: Option<String>, // Signature for reasoning verification (Claude)
    pub tokens: i64,
    pub created_at: Option<String>,
}

impl SessionMessage {
    pub fn user(session_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: None,
            session_id: session_id.into(),
            role: MessageRole::User,
            content: content.into(),
            tool_calls_json: None,
            tool_result_json: None,
            reasoning: None,
            reasoning_signature: None,
            tokens: 0,
            created_at: None,
        }
    }

    pub fn assistant(session_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: None,
            session_id: session_id.into(),
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls_json: None,
            tool_result_json: None,
            reasoning: None,
            reasoning_signature: None,
            tokens: 0,
            created_at: None,
        }
    }

    pub fn system(session_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: None,
            session_id: session_id.into(),
            role: MessageRole::System,
            content: content.into(),
            tool_calls_json: None,
            tool_result_json: None,
            reasoning: None,
            reasoning_signature: None,
            tokens: 0,
            created_at: None,
        }
    }

    pub fn tool(session_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: None,
            session_id: session_id.into(),
            role: MessageRole::Tool,
            content: content.into(),
            tool_calls_json: None,
            tool_result_json: None,
            reasoning: None,
            reasoning_signature: None,
            tokens: 0,
            created_at: None,
        }
    }

    pub fn with_tokens(mut self, tokens: i64) -> Self {
        self.tokens = tokens;
        self
    }

    pub fn with_tool_calls(mut self, json: impl Into<String>) -> Self {
        self.tool_calls_json = Some(json.into());
        self
    }

    pub fn with_tool_result(mut self, json: impl Into<String>) -> Self {
        self.tool_result_json = Some(json.into());
        self
    }

    pub fn with_reasoning(
        mut self,
        reasoning: Option<String>,
        signature: Option<String>,
    ) -> Self {
        self.reasoning = reasoning;
        self.reasoning_signature = signature;
        self
    }
}

/// Agent types
pub mod agent_types {
    pub const PLANNING: &str = "planning";
    pub const CODING: &str = "coding";
}
