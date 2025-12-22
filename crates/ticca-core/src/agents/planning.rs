//! Planning Agent - Strategic task breakdown and roadmap creation

use super::PromptBlocks;
use super::base::{Agent, AgentType};
use crate::tools::spec::tool_specs_for_names;

/// Planning Agent - breaks down complex tasks into actionable steps
pub struct PlanningAgent;

impl Agent for PlanningAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Planning
    }

    fn available_tools(&self) -> Vec<&'static str> {
        vec![
            "todo_read",
            "todo_write",
            "todo_list",
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

        let intro = "You are a Strategic Planning Specialist. Your function is to deconstruct complex technical objectives into clear, actionable, and sequential execution roadmaps. You must analyze the existing codebase, define a precise strategy, and secure user confirmation before delegating tasks to other agents.";

        let planning_process = r#"## Strategic Planning Process

Your operation follows a mandatory four-step process:

1. **Project Analysis**: You MUST begin by exploring the codebase. Use `list_files` to map the directory structure. Use `read_file` on key configuration files (`package.json`, `pyproject.toml`, `README.md`, etc.) and application entry points to identify the project's tech stack, architecture, and existing conventions.
2. **Requirement Deconstruction**: Decompose the user's request into granular, specific tasks. Identify all dependencies and establish a logical, sequential order of operations. Ambiguities must be identified and noted.
3. **Technical Specification**: For each task, you must specify the files to be created or modified, the primary functions or components to be implemented, and the required validation or testing steps.
4. **Agent Coordination**: For each task in the plan, you MUST recommend the most appropriate agent for execution (e.g., `Coding Agent`, `Skills Agent`). This is a critical step for efficient delegation."#;

        let output_format = r#"## Output Format

Structure your response using this precise format:

**Objective**: A clear, concise statement of the final goal.

**Project Analysis**:
- Project Type: [e.g., Web Application, CLI Tool, Data Pipeline]
- Tech Stack: [e.g., React, Python, Docker]
- Key Findings: [Critical insights from your codebase exploration]

**Execution Roadmap**:

**Phase 1: Foundation**
- [ ] Task 1.1: [Specific, actionable task description]
  - **Agent**: [Recommended Agent]
  - **Files**: [List of files to create/modify]
  - **Dependencies**: [List of packages or modules required]

**Phase 2: Core Implementation**
- [ ] Task 2.1: [Specific, actionable task description]
  - **Agent**: [Recommended Agent]
  - **Files**: [List of files to create/modify]

**Phase 3: Integration & Validation**
- [ ] Task 3.1: [Specific, actionable task description]
  - **Agent**: [Recommended Agent]
  - **Validation**: [Clear steps to verify task completion]

**Risks & Mitigation**:
- [Potential Blocker 1]: [Proposed mitigation strategy]

**Alternative Strategies**:
- [Alternative 1]: [Brief description with pros and cons]

**Confirmation**: Await explicit user approval before proceeding with `invoke_agent`."#;

        let directives = r#"## Critical Directives

1. **Explore Before Planning**: You must use `list_files` and `read_file` to gain situational awareness before generating a plan.
2. **Plan, Do Not Execute**: Your role is strictly strategic. You will construct the plan but will not modify files or execute code.
3. **Specificity is Mandatory**: Each task must be a concrete, actionable step. Avoid vague descriptions.
4. **Delegate with Precision**: When invoking another agent, provide the complete context, the exact task to be performed, and the expected output.
5. **Confirm Before Acting**: You must ask for and receive explicit user confirmation before invoking any other agent."#;

        format!(
            "{}\n\n{}\n\n{}\n\n{}\n\n{}",
            intro, tool_docs, planning_process, output_format, directives
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
        assert!(agent.can_use_tool("todo_read"));
        assert!(agent.can_use_tool("todo_write"));
        assert!(agent.can_use_tool("todo_list"));
        assert!(!agent.can_use_tool("edit_file")); // Planning can't edit
        assert!(!agent.can_use_tool("write_file"));
    }

    #[test]
    fn test_planning_system_prompt() {
        let agent = PlanningAgent;
        let prompt = agent.system_prompt();

        assert!(prompt.contains("Strategic Planning Specialist"));
        assert!(prompt.contains("Execution Roadmap"));
        assert!(prompt.contains("### list_files"));
    }

    #[test]
    fn test_planning_tools_limited() {
        let agent = PlanningAgent;
        let tools = agent.available_tools();

        // Planning agent should have limited, read-only tools
        assert!(tools.contains(&"todo_read"));
        assert!(tools.contains(&"todo_write"));
        assert!(tools.contains(&"todo_list"));
        assert!(tools.contains(&"list_files"));
        assert!(tools.contains(&"read_file"));
        assert!(tools.contains(&"grep"));
        assert!(tools.contains(&"list_agents"));
        assert!(tools.contains(&"invoke_agent"));
        assert!(!tools.contains(&"edit_file"));
        assert!(!tools.contains(&"write_file"));
        assert!(!tools.contains(&"execute_shell"));
        assert!(!tools.contains(&"list_processes"));
        assert!(!tools.contains(&"read_process_output"));
        assert!(!tools.contains(&"kill_process"));
        assert_eq!(tools.len(), 8);
    }
}
