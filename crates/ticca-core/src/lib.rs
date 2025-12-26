//! Ticca Core - Business logic and tools
//!
//! This crate contains:
//! - Agent definitions (Planning, Coding, Skills)
//! - Native Rust tools (file ops, grep, shell)
//! - Configuration database (SQLite)
//! - Session storage (SQLite)
//! - LLM integration (Claude, etc.)

pub mod agents;
pub mod compression;
pub mod config;
pub mod external_tools;
pub mod llm;
pub mod python;
pub mod registry;
pub mod session;
pub mod skills;
pub mod tools;
pub mod version_check;

// Re-export commonly used types
pub use agents::{
    Agent, AgentConfig, AgentType, CodingAgent, PlanningAgent, SkillsAgent, get_agent,
    get_all_agents,
};
pub use config::{
    ConfigDatabase, DiscoveredModel, McpServer, McpTransport, ModelConfig, OAuthToken, Setting,
};
pub use external_tools::{
    ExternalToolId, ExternalToolManager, Platform, ToolInfo, ToolStatus, get_all_tool_definitions,
    get_tool_definition,
};
pub use llm::{ClaudeClient, ModelId, ModelService, get_claude_client, has_claude_credentials};
pub use python::{create_venv, ensure_uv_available, pip_install, run_python_script};
pub use registry::{
    AuthType, DEFAULT_CONTEXT_WINDOW, ModelCapabilities, ModelDefinition, ModelSource,
    ProviderDefinition, RegistryService,
};
pub use session::{MessageRole, Session, SessionDatabase, SessionMessage};
pub use skills::{SkillMetadata, discover_skills, extract_skills_if_needed, get_skill_path};
pub use tools::{ToolDefinition, ToolRegistry, ToolResult, create_default_registry};
pub use version_check::{CURRENT_VERSION, LatestRelease, check_for_update, is_newer_version};
