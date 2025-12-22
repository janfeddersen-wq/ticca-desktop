//! Agent definitions
//!
//! This module contains the agent definitions for Ticca Desktop:
//! - **Planning Agent**: Strategic task breakdown and roadmap creation
//! - **Coding Agent**: Code generation, modification, and execution
//! - **Skills Agent**: Python-based skill execution for specialized tasks

pub mod base;
pub mod coding;
pub mod planning;
pub mod profile;
pub mod prompt_blocks;
pub mod runner;
pub mod skills;

pub use base::{Agent, AgentConfig, AgentType, get_agent, get_all_agents};
pub use coding::CodingAgent;
pub use planning::PlanningAgent;
pub use skills::SkillsAgent;
pub use profile::{
    AccountSelectionPolicy, AgentProfile, ModelRule, ModelSelectionContext, ModelStrategy,
    ToolUsagePolicy,
};
pub use prompt_blocks::PromptBlocks;
pub use runner::{ChatHistoryMessage, RunnerEvent, run_rig_agent_stream};
