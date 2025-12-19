//! Planning Agent - Strategic task breakdown and roadmap creation

use super::base::{Agent, AgentType};
use super::profile::ToolUsagePolicy;
use super::PromptBlocks;
use crate::tools::spec::tool_specs_for_names;

/// Planning Agent - breaks down complex tasks into actionable steps
pub struct PlanningAgent;

impl Agent for PlanningAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Planning
    }
    
    fn available_tools(&self) -> Vec<&'static str> {
        vec![
            "list_files",
            "read_file",
            "grep",
            "list_agents",
            "invoke_agent",
        ]
    }

    fn system_prompt(&self) -> String {
        let tool_specs = tool_specs_for_names(&self.available_tools());
        let tool_docs = PromptBlocks::tool_docs(&tool_specs);
        let policy = ToolUsagePolicy::planning();
        let guidelines = PromptBlocks::agent_guidelines(
            &policy,
            &[
                "Always explore the codebase before planning",
                "Be specific - each task should be concrete and actionable",
                "Consider task dependencies and ordering",
                "Include testing and validation steps",
                "This is planning only - you cannot modify files",
            ],
        );

        let planning_process = [
            "## Planning Process",
            "",
            "1. **Analyze**: Understand the user's request and explore the codebase",
            "2. **Identify**: Determine files to create/modify and dependencies",
            "3. **Plan**: Break work into logical, sequential steps",
            "4. **Assess**: Note risks and alternative approaches",
        ]
        .join("\n");

        let output_format = [
            "## Output Format",
            "",
            "Structure your response as:",
            "",
            "**Objective**: Clear statement of what needs to be accomplished",
            "",
            "**Project Analysis**:",
            "- Project type, tech stack, current state",
            "- Key findings from exploration",
            "",
            "**Execution Plan**:",
            "",
            "Phase 1: Foundation",
            "- Task 1.1: Specific action",
            "  - Files: Files to create/modify",
            "  - Dependencies: Packages needed",
            "",
            "Phase 2: Implementation",
            "- Task 2.1: Specific action",
            "  - Files: Files to create/modify",
            "",
            "Phase 3: Testing",
            "- Task 3.1: Validation steps",
            "",
            "**Risks**: Potential blockers with mitigation strategies",
            "",
            "**Alternatives**: Other approaches with pros/cons",
        ]
        .join("\n");

        format!(
            "You are a planning assistant that breaks down complex coding tasks into actionable steps.\n\n{}\n{}\n\n{}\n{}",
            tool_docs, planning_process, output_format, guidelines
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planning_agent_basics() {
        let agent = PlanningAgent;

        assert_eq!(agent.agent_type(), AgentType::Planning);
        assert_eq!(agent.display_name(), "Planning Agent");
        assert!(agent.can_use_tool("list_files"));
        assert!(agent.can_use_tool("read_file"));
        assert!(agent.can_use_tool("grep"));
        assert!(agent.can_use_tool("list_agents"));
        assert!(agent.can_use_tool("invoke_agent"));
        assert!(!agent.can_use_tool("edit_file")); // Planning can't edit
        assert!(!agent.can_use_tool("write_file"));
    }

    #[test]
    fn test_planning_system_prompt() {
        let agent = PlanningAgent;
        let prompt = agent.system_prompt();

        assert!(prompt.contains("planning assistant"));
        assert!(prompt.contains("Execution Plan"));
        assert!(prompt.contains("### list_files"));
    }

    #[test]
    fn test_planning_tools_limited() {
        let agent = PlanningAgent;
        let tools = agent.available_tools();

        // Planning agent should have limited, read-only tools
        assert!(tools.contains(&"list_files"));
        assert!(tools.contains(&"read_file"));
        assert!(tools.contains(&"grep"));
        assert!(tools.contains(&"list_agents"));
        assert!(tools.contains(&"invoke_agent"));
        assert!(!tools.contains(&"edit_file"));
        assert!(!tools.contains(&"write_file"));
        assert!(!tools.contains(&"shell"));
        assert_eq!(tools.len(), 5);
    }
}
