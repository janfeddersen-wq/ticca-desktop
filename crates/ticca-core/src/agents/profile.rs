//! Agent profile abstraction

use super::{get_agent, AgentType};

#[derive(Debug, Clone)]
pub struct AgentProfile {
    pub agent_type: AgentType,
    pub system_prompt: String,
    pub tool_names: Vec<&'static str>,
    pub max_tool_rounds: u32,
}

impl AgentProfile {
    pub fn for_type(agent_type: AgentType, max_tool_rounds: u32) -> Self {
        let agent = get_agent(agent_type);
        Self {
            agent_type,
            system_prompt: agent.system_prompt(),
            tool_names: agent.available_tools(),
            max_tool_rounds,
        }
    }

    pub fn resolve_model(&self, pinned: Option<String>, default: Option<String>) -> Option<String> {
        pinned.or(default)
    }
}
