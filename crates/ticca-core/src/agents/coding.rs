//! Coding Agent - Code generation and modification

use super::PromptBlocks;
use super::base::{Agent, AgentType};
use crate::tools::spec::tool_specs_for_names;

/// Coding Agent - code generation and modification
pub struct CodingAgent;

impl Agent for CodingAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Coding
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
            "edit_file",
            "delete_file",
            "write_file",
            "execute_shell",
            "list_processes",
            "read_process_output",
            "kill_process",
        ]
    }

    fn system_prompt(&self) -> String {
        let tool_specs = tool_specs_for_names(&self.available_tools());
        let tool_docs = PromptBlocks::tool_docs(&tool_specs);

        let intro = "You are an expert Coding Agent. Your purpose is to execute technical tasks by writing, modifying, and managing code. You MUST use the provided tools to achieve the objective. Do not describe the solution; implement it directly. Operate with precision and adhere to software engineering best practices.";

        let workflow = r#"## Core Workflow

You must follow this iterative, three-step cycle for every action:

1. **Reason**: Articulate your immediate goal and the tool you will use to achieve it.
2. **Execute**: Invoke a single tool to perform the planned action.
3. **Validate**: Analyze the tool's output to confirm success or failure, then report the result and determine the next step."#;

        let best_practices = r#"## Tool Usage Best Practices

- **`read_file`**: MANDATORY before any modification. You must read a file to understand its current state before using `edit_file`.
- **`edit_file`**: This is your primary tool for code modification.
    - Prefer small, targeted edits over replacing entire files.
    - For large refactors, apply multiple, sequential `edit_file` calls.
    - Ensure the snippet being replaced (`old_str`) is minimal and unique to avoid unintended changes.
- **`write_file`**: Use this tool for creating new files. To prevent data loss, `edit_file` is the required tool for modifying existing files.
- **`execute_shell`**: Use for running tests, installing dependencies, or executing build scripts. When running full test suites, suppress verbose output unless debugging a specific failure."#;

        let directives = r#"## Critical Directives

1. **Action is Mandatory**: You MUST use tools to accomplish tasks. Do not output code blocks or descriptive text as your final answer.
2. **Autonomy is Key**: Continue the Reason-Execute-Validate cycle autonomously until the task is complete or user input is explicitly required.
3. **Adhere to File Size Limits**: No file may exceed 600 lines. If a file approaches this limit, you MUST refactor it by splitting logic into smaller, modular files.
4. **Follow Engineering Principles**: Your solutions must adhere to DRY, YAGNI, and SOLID principles. Code must be clean, readable, and maintainable.
5. **Update To-Do List**: You must use `todo_write` or `todo_list` to mark tasks as complete upon finishing the implementation."#;

        format!(
            "{}\n\n{}\n\n{}\n\n{}\n\n{}",
            intro, tool_docs, workflow, best_practices, directives
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
        assert!(agent.can_use_tool("execute_shell"));
        assert!(agent.can_use_tool("list_processes"));
        assert!(agent.can_use_tool("read_process_output"));
        assert!(agent.can_use_tool("kill_process"));
        assert!(agent.can_use_tool("list_agents"));
        assert!(agent.can_use_tool("invoke_agent"));
        assert!(agent.can_use_tool("todo_read"));
        assert!(agent.can_use_tool("todo_write"));
        assert!(agent.can_use_tool("todo_list"));
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
        assert!(tools.contains(&"todo_read"));
        assert!(tools.contains(&"todo_write"));
        assert!(tools.contains(&"todo_list"));
        assert!(tools.contains(&"list_files"));
        assert!(tools.contains(&"read_file"));
        assert!(tools.contains(&"grep"));
        assert!(tools.contains(&"list_agents"));
        assert!(tools.contains(&"invoke_agent"));
        assert!(tools.contains(&"edit_file"));
        assert!(tools.contains(&"delete_file"));
        assert!(tools.contains(&"write_file"));
        assert!(tools.contains(&"execute_shell"));
        assert!(tools.contains(&"list_processes"));
        assert!(tools.contains(&"read_process_output"));
        assert!(tools.contains(&"kill_process"));
        assert_eq!(tools.len(), 15);
    }

    #[test]
    fn test_prompt_describes_tools() {
        let agent = CodingAgent;
        let prompt = agent.system_prompt();

        // Prompt should describe all available tools
        assert!(prompt.contains("### todo_read"));
        assert!(prompt.contains("### todo_write"));
        assert!(prompt.contains("### todo_list"));
        assert!(prompt.contains("### list_files"));
        assert!(prompt.contains("### read_file"));
        assert!(prompt.contains("### write_file"));
        assert!(prompt.contains("### edit_file"));
        assert!(prompt.contains("### delete_file"));
        assert!(prompt.contains("### grep"));
        assert!(prompt.contains("### execute_shell"));
        assert!(prompt.contains("### list_processes"));
        assert!(prompt.contains("### read_process_output"));
        assert!(prompt.contains("### kill_process"));
        assert!(prompt.contains("### list_agents"));
        assert!(prompt.contains("### invoke_agent"));
    }
}
