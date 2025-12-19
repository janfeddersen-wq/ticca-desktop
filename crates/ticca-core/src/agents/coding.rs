//! Coding Agent - Code generation and modification

use super::base::{Agent, AgentType};
use super::profile::ToolUsagePolicy;
use super::PromptBlocks;
use crate::tools::spec::tool_specs_for_names;

/// Coding Agent - code generation and modification
pub struct CodingAgent;

impl Agent for CodingAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Coding
    }
    
    fn available_tools(&self) -> Vec<&'static str> {
        vec![
            "list_files",
            "read_file",
            "grep",
            "list_agents",
            "invoke_agent",
            "edit_file",
            "delete_file",
            "write_file",
            "shell",
        ]
    }

    fn system_prompt(&self) -> String {
        let tool_specs = tool_specs_for_names(&self.available_tools());
        let tool_docs = PromptBlocks::tool_docs(&tool_specs);
        let policy = ToolUsagePolicy::coding();
        let guidelines = PromptBlocks::agent_guidelines(
            &policy,
            &[
                "Use list_files to explore project structure before modifying files",
                "Follow DRY, YAGNI, and SOLID principles",
                "Keep solutions simple and readable (KISS)",
                "Keep individual files under 600 lines; split modules when needed",
                "Continue working autonomously until the task is complete",
                "When invoking another agent, provide clear context, desired output, and constraints",
            ],
        );

        format!(
            "You are a coding assistant with access to file and shell tools. Use these tools to complete coding tasks - do not just describe what to do.\n\n{}\n{}",
            tool_docs, guidelines
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coding_agent_basics() {
        let agent = CodingAgent;

        assert_eq!(agent.agent_type(), AgentType::Coding);
        assert_eq!(agent.display_name(), "Coding Agent");
        assert!(agent.can_use_tool("list_files"));
        assert!(agent.can_use_tool("read_file"));
        assert!(agent.can_use_tool("edit_file"));
        assert!(agent.can_use_tool("delete_file"));
        assert!(agent.can_use_tool("write_file"));
        assert!(agent.can_use_tool("shell"));
        assert!(agent.can_use_tool("list_agents"));
        assert!(agent.can_use_tool("invoke_agent"));
    }

    #[test]
    fn test_coding_system_prompt() {
        let agent = CodingAgent;
        let prompt = agent.system_prompt();

        assert!(prompt.contains("edit_file"));
        assert!(prompt.contains("DRY"));
        assert!(prompt.contains("600 lines"));
    }

    #[test]
    fn test_coding_agent_has_more_tools_than_planning() {
        let coding = CodingAgent;
        let planning = super::super::planning::PlanningAgent;

        assert!(coding.available_tools().len() > planning.available_tools().len());
        assert!(coding.can_use_tool("edit_file"));
        assert!(!planning.can_use_tool("edit_file"));
    }

    #[test]
    fn test_coding_agent_full_tools() {
        let agent = CodingAgent;
        let tools = agent.available_tools();

        // Coding agent should have all tools
        assert!(tools.contains(&"list_files"));
        assert!(tools.contains(&"read_file"));
        assert!(tools.contains(&"grep"));
        assert!(tools.contains(&"list_agents"));
        assert!(tools.contains(&"invoke_agent"));
        assert!(tools.contains(&"edit_file"));
        assert!(tools.contains(&"delete_file"));
        assert!(tools.contains(&"write_file"));
        assert!(tools.contains(&"shell"));
        assert_eq!(tools.len(), 9);
    }

    #[test]
    fn test_prompt_describes_tools() {
        let agent = CodingAgent;
        let prompt = agent.system_prompt();

        // Prompt should describe all available tools
        assert!(prompt.contains("### list_files"));
        assert!(prompt.contains("### read_file"));
        assert!(prompt.contains("### write_file"));
        assert!(prompt.contains("### edit_file"));
        assert!(prompt.contains("### delete_file"));
        assert!(prompt.contains("### grep"));
        assert!(prompt.contains("### shell"));
        assert!(prompt.contains("### list_agents"));
        assert!(prompt.contains("### invoke_agent"));
    }
}
