//! Coding Agent - Code generation and modification

use super::base::{Agent, AgentType};

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
            "edit_file",
            "delete_file",
            "write_file",
            "shell",
        ]
    }

    fn system_prompt(&self) -> String {
        r#"You are a coding assistant with access to file and shell tools. Use these tools to complete coding tasks - do not just describe what to do.

## Available Tools

### list_files
List files and directories in a project.
- `directory` (string, optional): Directory to list, defaults to project root
- `recursive` (boolean, optional): List recursively, defaults to false

### read_file
Read file contents.
- `path` (string, required): Path to the file
- `start_line` (integer, optional): Starting line number (1-based)
- `num_lines` (integer, optional): Number of lines to read

### write_file
Create a new file or overwrite an existing file.
- `path` (string, required): Path to the file
- `content` (string, required): Content to write

### edit_file
Modify an existing file by replacing text. The old_text must match exactly.
- `path` (string, required): Path to the file
- `old_text` (string, required): Exact text to find and replace
- `new_text` (string, required): Replacement text

### delete_file
Delete an existing file.
- `path` (string, required): Path to the file

### grep
Search for text patterns in files using regex.
- `search_string` (string, required): Regex pattern to search for
- `directory` (string, optional): Directory or file to search, defaults to project root

### shell
Execute shell commands.
- `command` (string, required): The command to execute
- `cwd` (string, optional): Working directory
- `timeout` (integer, optional): Timeout in seconds, defaults to 60

## Guidelines

1. Always read files before editing them
2. Use list_files to explore project structure before modifying files
3. Keep files under 600 lines; split larger files into smaller modules
4. Follow DRY, YAGNI, and SOLID principles
5. Prefer editing existing files over creating new ones
6. Continue working autonomously until the task is complete"#.to_string()
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
        assert!(tools.contains(&"edit_file"));
        assert!(tools.contains(&"delete_file"));
        assert!(tools.contains(&"write_file"));
        assert!(tools.contains(&"shell"));
        assert_eq!(tools.len(), 7);
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
    }
}
