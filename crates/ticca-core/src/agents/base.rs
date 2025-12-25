//! Base agent trait and common types

use crate::llm::ProviderId;
use crate::tools::ToolRegistry;
use material_icons::Icon;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

/// Agent types available in Ticca
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentType {
    Planning,
    Coding,
    Skills,
    Explore,
}

/// Tool usage policy type for agents
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolPolicyType {
    /// Read-only access (Planning, Explore)
    ReadOnly,
    /// Full access including write and execute (Coding, Skills)
    FullAccess,
}

/// Metadata for an agent type - contains all static information
#[derive(Debug, Clone)]
pub struct AgentMetadata {
    /// The agent type identifier
    pub agent_type: AgentType,
    /// String identifier (e.g., "coding", "planning")
    pub id: &'static str,
    /// Display name for UI (e.g., "Coding Agent")
    pub display_name: &'static str,
    /// Short label for tabs/buttons (e.g., "Coding")
    pub label: &'static str,
    /// Description of agent capabilities
    pub description: &'static str,
    /// Material icon for UI
    pub icon: Icon,
    /// RGB color for flow panel nodes (r, g, b in 0.0-1.0)
    pub color: (f32, f32, f32),
    /// Provider preference order for model selection
    pub provider_order: &'static [ProviderId],
    /// Tool usage policy type
    pub tool_policy: ToolPolicyType,
}

/// Static provider order constants
const PROVIDER_ORDER_CODING: &[ProviderId] = &[ProviderId::Claude, ProviderId::ChatGpt, ProviderId::Gemini];
const PROVIDER_ORDER_PLANNING: &[ProviderId] = &[ProviderId::Claude, ProviderId::Gemini, ProviderId::ChatGpt];

/// Static registry lookup table
static AGENT_REGISTRY: LazyLock<HashMap<AgentType, AgentMetadata>> = LazyLock::new(|| {
    let mut map = HashMap::new();

    map.insert(AgentType::Coding, AgentMetadata {
        agent_type: AgentType::Coding,
        id: "coding",
        display_name: "Coding Agent",
        label: "Coding",
        description: "Writes, modifies, and executes code to complete development tasks.",
        icon: Icon::Code,
        color: (0.18, 0.55, 0.90), // Blue
        provider_order: PROVIDER_ORDER_CODING,
        tool_policy: ToolPolicyType::FullAccess,
    });

    map.insert(AgentType::Planning, AgentMetadata {
        agent_type: AgentType::Planning,
        id: "planning",
        display_name: "Planning Agent",
        label: "Planning",
        description: "Breaks down complex tasks into actionable steps and creates execution roadmaps.",
        icon: Icon::Assignment,
        color: (0.24, 0.70, 0.42), // Green
        provider_order: PROVIDER_ORDER_PLANNING,
        tool_policy: ToolPolicyType::ReadOnly,
    });

    map.insert(AgentType::Skills, AgentMetadata {
        agent_type: AgentType::Skills,
        id: "skills",
        display_name: "Skills Agent",
        label: "Skills",
        description: "Executes Python-based skills for specialized tasks like document generation and web automation.",
        icon: Icon::Build,
        color: (0.75, 0.45, 0.85), // Purple
        provider_order: PROVIDER_ORDER_CODING,
        tool_policy: ToolPolicyType::FullAccess,
    });

    map.insert(AgentType::Explore, AgentMetadata {
        agent_type: AgentType::Explore,
        id: "explore",
        display_name: "Explore Agent",
        label: "Explore",
        description: "Fast, read-only codebase exploration specialist for finding files and searching code.",
        icon: Icon::FolderOpen,
        color: (0.20, 0.70, 0.70), // Cyan/teal
        provider_order: PROVIDER_ORDER_CODING,
        tool_policy: ToolPolicyType::ReadOnly,
    });

    map
});

/// Static list of all agent types (for iteration)
static ALL_AGENTS: LazyLock<Vec<AgentType>> = LazyLock::new(|| {
    vec![AgentType::Coding, AgentType::Planning, AgentType::Skills, AgentType::Explore]
});

/// Agent registry providing centralized access to agent metadata
pub struct AgentRegistry;

impl AgentRegistry {
    /// Get all agent types
    pub fn all() -> &'static [AgentType] {
        ALL_AGENTS.as_slice()
    }

    /// Get metadata for a specific agent type
    pub fn get(agent_type: AgentType) -> &'static AgentMetadata {
        AGENT_REGISTRY
            .get(&agent_type)
            .expect("All AgentType variants must be registered")
    }

    /// Find agent by string ID
    pub fn find_by_id(id: &str) -> Option<&'static AgentMetadata> {
        let id_lower = id.to_lowercase();
        AGENT_REGISTRY.values().find(|m| m.id == id_lower)
    }

    /// Create an agent instance by type
    pub fn create(agent_type: AgentType) -> Box<dyn Agent> {
        match agent_type {
            AgentType::Planning => Box::new(super::planning::PlanningAgent),
            AgentType::Coding => Box::new(super::coding::CodingAgent),
            AgentType::Skills => {
                match super::skills::SkillsAgent::new() {
                    Ok(agent) => Box::new(agent),
                    Err(e) => {
                        tracing::warn!("Failed to initialize SkillsAgent: {}, using empty agent", e);
                        Box::new(super::skills::SkillsAgent::empty())
                    }
                }
            }
            AgentType::Explore => Box::new(super::explore::ExploreAgent),
        }
    }

    /// Get all agent metadata (for UI iteration)
    pub fn all_metadata() -> impl Iterator<Item = &'static AgentMetadata> {
        AGENT_REGISTRY.values()
    }
}

impl AgentType {
    /// Returns all available agent types
    pub fn all() -> &'static [AgentType] {
        AgentRegistry::all()
    }
}

impl AgentType {
    /// Get the string identifier for this agent type
    pub fn as_str(&self) -> &'static str {
        AgentRegistry::get(*self).id
    }

    /// Parse an agent type from a string identifier
    pub fn parse(s: &str) -> Option<Self> {
        AgentRegistry::find_by_id(s).map(|m| m.agent_type)
    }

    /// Get the display name for this agent type
    pub fn display_name(&self) -> &'static str {
        AgentRegistry::get(*self).display_name
    }

    /// Get the description for this agent type
    pub fn description(&self) -> &'static str {
        AgentRegistry::get(*self).description
    }

    /// Get the full metadata for this agent type
    pub fn metadata(&self) -> &'static AgentMetadata {
        AgentRegistry::get(*self)
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
///
/// This function delegates to `AgentRegistry::create()`.
pub fn get_agent(agent_type: AgentType) -> Box<dyn Agent> {
    AgentRegistry::create(agent_type)
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
        Box::new(super::explore::ExploreAgent),
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
        assert_eq!(agents.len(), 4); // Planning, Coding, Skills, Explore
    }

    #[test]
    fn test_agent_registry_all() {
        let all = AgentRegistry::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&AgentType::Coding));
        assert!(all.contains(&AgentType::Planning));
        assert!(all.contains(&AgentType::Skills));
        assert!(all.contains(&AgentType::Explore));
    }

    #[test]
    fn test_agent_registry_get() {
        let coding = AgentRegistry::get(AgentType::Coding);
        assert_eq!(coding.id, "coding");
        assert_eq!(coding.display_name, "Coding Agent");
        assert_eq!(coding.label, "Coding");
        assert_eq!(coding.tool_policy, ToolPolicyType::FullAccess);

        let explore = AgentRegistry::get(AgentType::Explore);
        assert_eq!(explore.id, "explore");
        assert_eq!(explore.tool_policy, ToolPolicyType::ReadOnly);
    }

    #[test]
    fn test_agent_registry_find_by_id() {
        assert!(AgentRegistry::find_by_id("coding").is_some());
        assert!(AgentRegistry::find_by_id("CODING").is_some()); // case insensitive
        assert!(AgentRegistry::find_by_id("unknown").is_none());
    }

    #[test]
    fn test_agent_registry_create() {
        let agent = AgentRegistry::create(AgentType::Coding);
        assert_eq!(agent.agent_type(), AgentType::Coding);
    }

    #[test]
    fn test_agent_metadata_method() {
        let metadata = AgentType::Coding.metadata();
        assert_eq!(metadata.agent_type, AgentType::Coding);
        assert_eq!(metadata.id, "coding");
    }
}
