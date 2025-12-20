//! Ticca Core - Business logic and tools
//!
//! This crate contains:
//! - Agent definitions (Planning, Coding)
//! - Native Rust tools (file ops, grep, shell)
//! - Configuration database (SQLite)
//! - Session storage (SQLite)
//! - LLM integration (Claude, etc.)

pub mod agents;
pub mod config;
pub mod llm;
pub mod session;
pub mod tools;

// Re-export commonly used types
pub use agents::{
    Agent, AgentConfig, AgentType, CodingAgent, PlanningAgent, get_agent, get_all_agents,
};
pub use config::{ConfigDatabase, McpServer, McpTransport, ModelConfig, OAuthToken, Setting};
pub use llm::{ClaudeClient, get_claude_client, has_claude_credentials};
pub use session::{MessageRole, Session, SessionDatabase, SessionMessage};
pub use tools::{ToolDefinition, ToolRegistry, ToolResult, create_default_registry};
