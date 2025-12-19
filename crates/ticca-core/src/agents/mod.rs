//! Agent definitions
//!
//! This module contains the agent definitions for Ticca Desktop:
//! - **Planning Agent**: Strategic task breakdown and roadmap creation
//! - **Coding Agent**: Code generation, modification, and execution

pub mod base;
pub mod coding;
pub mod planning;
pub mod profile;

pub use base::{Agent, AgentConfig, AgentType, get_agent, get_all_agents};
pub use coding::CodingAgent;
pub use planning::PlanningAgent;
pub use profile::AgentProfile;
