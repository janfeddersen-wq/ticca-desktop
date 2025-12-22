//! Base agent trait and common types

use crate::tools::ToolRegistry;
use serde::{Deserialize, Serialize};

/// Agent types available in Ticca
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentType {
    Planning,
    Coding,
    Skills,
}

impl AgentType {
    /// Returns all available agent types
    pub fn all() -> &'static [AgentType] {
        &[AgentType::Coding, AgentType::Planning, AgentType::Skills]
    }
}

impl AgentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentType::Planning => "planning",
            AgentType::Coding => "coding",
            AgentType::Skills => "skills",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "planning" => Some(AgentType::Planning),
            "coding" => Some(AgentType::Coding),
            "skills" => Some(AgentType::Skills),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AgentType::Planning => "Planning Agent",
            AgentType::Coding => "Coding Agent",
            AgentType::Skills => "Skills Agent",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            AgentType::Planning => {
                "Breaks down complex tasks into actionable steps and creates execution roadmaps."
            }
            AgentType::Coding => {
                "Writes, modifies, and executes code to complete development tasks."
            }
            AgentType::Skills => {
                "Executes Python-based skills for specialized tasks like document generation and web automation."
            }
        }
    }
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for AgentType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or(())
    }
}

/// Agent definition trait
pub trait Agent: Send + Sync {
    /// Get the agent type
    fn agent_type(&self) -> AgentType;

    /// Get the agent's display name
    fn display_name(&self) -> &'static str {
        self.agent_type().display_name()
    }

    /// Get the agent's description.
    ///
    /// This returns a String to allow for dynamic descriptions (e.g., based on
    /// discovered skills). The default implementation uses the static description
    /// from the AgentType.
    fn description(&self) -> String {
        self.agent_type().description().to_string()
    }

    /// Get the system prompt for this agent
    fn system_prompt(&self) -> String;

    /// Get the list of tool names this agent can use
    fn available_tools(&self) -> Vec<&'static str>;

    /// Check if this agent can use a specific tool
    fn can_use_tool(&self, tool_name: &str) -> bool {
        self.available_tools().contains(&tool_name)
    }

    /// Filter a tool registry to only include tools this agent can use
    fn filter_tools(&self, registry: &ToolRegistry) -> Vec<String> {
        let available = self.available_tools();
        registry
            .tool_names()
            .into_iter()
            .filter(|name| available.contains(name))
            .map(|s| s.to_string())
            .collect()
    }
}

/// Configuration for an agent instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent_type: AgentType,
    pub model_name: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

impl AgentConfig {
    pub fn new(agent_type: AgentType) -> Self {
        Self {
            agent_type,
            model_name: None,
            temperature: None,
            max_tokens: None,
        }
    }

    pub fn planning() -> Self {
        Self::new(AgentType::Planning)
    }

    pub fn coding() -> Self {
        Self::new(AgentType::Coding)
    }

    pub fn with_model(mut self, model_name: impl Into<String>) -> Self {
        self.model_name = Some(model_name.into());
        self
    }

    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn with_max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self::coding()
    }
}

/// Get an agent by type
///
/// Note: For `AgentType::Skills`, this creates an empty SkillsAgent if
/// initialization fails. For full skill discovery, use `SkillsAgent::new()`
/// directly which returns a `Result`.
pub fn get_agent(agent_type: AgentType) -> Box<dyn Agent> {
    match agent_type {
        AgentType::Planning => Box::new(super::planning::PlanningAgent),
        AgentType::Coding => Box::new(super::coding::CodingAgent),
        AgentType::Skills => {
            // Try to create a proper SkillsAgent, fall back to empty on failure
            match super::skills::SkillsAgent::new() {
                Ok(agent) => Box::new(agent),
                Err(e) => {
                    tracing::warn!("Failed to initialize SkillsAgent: {}, using empty agent", e);
                    Box::new(super::skills::SkillsAgent::empty())
                }
            }
        }
    }
}

/// Get all available agents
///
/// Note: This includes SkillsAgent which requires runtime initialization.
/// If skill discovery fails, an empty SkillsAgent is included.
pub fn get_all_agents() -> Vec<Box<dyn Agent>> {
    vec![
        Box::new(super::planning::PlanningAgent),
        Box::new(super::coding::CodingAgent),
        get_agent(AgentType::Skills), // Use get_agent for proper error handling
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type_str_conversion() {
        assert_eq!(AgentType::Planning.as_str(), "planning");
        assert_eq!(AgentType::Coding.as_str(), "coding");

        assert_eq!(AgentType::parse("planning"), Some(AgentType::Planning));
        assert_eq!(AgentType::parse("CODING"), Some(AgentType::Coding));
        assert_eq!(AgentType::parse("unknown"), None);
    }

    #[test]
    fn test_agent_type_display() {
        assert!(AgentType::Planning.display_name().contains("Planning"));
        assert!(AgentType::Coding.display_name().contains("Coding"));
    }

    #[test]
    fn test_agent_config_builder() {
        let config = AgentConfig::planning()
            .with_model("gpt-4")
            .with_temperature(0.7);

        assert_eq!(config.agent_type, AgentType::Planning);
        assert_eq!(config.model_name, Some("gpt-4".to_string()));
        assert_eq!(config.temperature, Some(0.7));
    }

    #[test]
    fn test_get_agent() {
        let planning = get_agent(AgentType::Planning);
        assert_eq!(planning.agent_type(), AgentType::Planning);

        let coding = get_agent(AgentType::Coding);
        assert_eq!(coding.agent_type(), AgentType::Coding);
    }

    #[test]
    fn test_get_all_agents() {
        let agents = get_all_agents();
        assert_eq!(agents.len(), 3); // Planning, Coding, Skills
    }
}
