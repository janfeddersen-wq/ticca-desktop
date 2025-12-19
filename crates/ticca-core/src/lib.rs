//! Ticca Core - Business logic and tools
//!
//! This crate contains:
//! - Agent definitions (Planning, Coding)
//! - Native Rust tools (file ops, grep, shell)
//! - Configuration database (SQLite)
//! - Session storage (SQLite)
//! - LLM integration (Claude, etc.)

pub mod agents;
pub mod tools;
pub mod config;
pub mod session;
pub mod llm;

// Re-export commonly used types
pub use agents::{Agent, AgentConfig, AgentType, CodingAgent, PlanningAgent, get_agent, get_all_agents};
pub use config::{ConfigDatabase, ModelConfig, OAuthToken, Setting};
pub use session::{SessionDatabase, Session, SessionMessage, MessageRole};
pub use tools::{ToolRegistry, ToolResult, ToolDefinition, create_default_registry};
pub use llm::{ClaudeClient, get_claude_client, has_claude_credentials};
