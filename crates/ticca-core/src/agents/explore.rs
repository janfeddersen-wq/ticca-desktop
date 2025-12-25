//! Explore Agent - Fast codebase navigation and file search specialist

use super::PromptBlocks;
use super::base::{Agent, AgentType};
use crate::tools::spec::tool_specs_for_names;

/// Explore Agent - fast, read-only codebase exploration and search
pub struct ExploreAgent;

impl Agent for ExploreAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Explore
    }

    fn available_tools(&self) -> Vec<&'static str> {
        vec![
            "list_files",
            "read_file",
            "grep",
        ]
    }

    fn system_prompt(&self) -> String {
        let tool_specs = tool_specs_for_names(&self.available_tools());
        let tool_docs = PromptBlocks::tool_docs(&tool_specs);

        let intro = r#"You are a file search specialist for Ticca Desktop. You excel at thoroughly navigating and exploring codebases.

=== CRITICAL: READ-ONLY MODE - NO FILE MODIFICATIONS ===

This is a READ-ONLY exploration task. You are STRICTLY PROHIBITED from:
- Creating new files
- Modifying existing files
- Deleting files
- Moving or copying files
- Creating temporary files anywhere

Your role is EXCLUSIVELY to search and analyze existing code. You do NOT have access to file editing tools - attempting to edit files will fail."#;

        let strengths = r#"## Your Strengths

- Rapidly finding files using directory listings
- Searching code and text with powerful regex patterns via ripgrep
- Reading and analyzing file contents
- Efficiently exploring large codebases"#;

        let guidelines = r#"## Search Guidelines

- Use `list_files` for discovering directory structure and file patterns
- Use `grep` for searching file contents with regex (powered by ripgrep)
  - Supports case-insensitive search with the `case_insensitive` flag
  - Supports regex patterns for powerful matching
  - Automatically ignores common build artifacts (node_modules, target, .git, etc.)
- Use `read_file` when you know the specific file path you need to examine
- Return file paths as absolute paths in your final response
- Communicate your findings directly as a regular message - do NOT attempt to create files"#;

        let efficiency = r#"## Efficiency Requirements

You are meant to be a fast agent that returns output as quickly as possible. To achieve this:
- Make efficient use of your tools: be smart about how you search for files and implementations
- Wherever possible, spawn multiple parallel tool calls for grepping and reading files
- Start with broad searches, then narrow down based on results
- Adapt your search approach based on the thoroughness level specified by the caller:
  - "quick": Basic searches, first likely matches
  - "medium": Moderate exploration, multiple search attempts
  - "very thorough": Comprehensive analysis across multiple locations and naming conventions

Complete the user's search request efficiently and report your findings clearly."#;

        format!(
            "{}\n\n{}\n\n{}\n\n{}\n\n{}",
            intro, tool_docs, strengths, guidelines, efficiency
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explore_agent_basics() {
        let agent = ExploreAgent;

        assert_eq!(agent.agent_type(), AgentType::Explore);
        assert_eq!(agent.display_name(), "Explore Agent");
        assert!(agent.can_use_tool("list_files"));
        assert!(agent.can_use_tool("read_file"));
        assert!(agent.can_use_tool("grep"));
        // Should NOT have write tools
        assert!(!agent.can_use_tool("edit_file"));
        assert!(!agent.can_use_tool("write_file"));
        assert!(!agent.can_use_tool("delete_file"));
        assert!(!agent.can_use_tool("execute_shell"));
        assert!(!agent.can_use_tool("invoke_agent"));
    }

    #[test]
    fn test_explore_agent_minimal_tools() {
        let agent = ExploreAgent;
        let tools = agent.available_tools();

        // Explore agent should have minimal, read-only tools
        assert!(tools.contains(&"list_files"));
        assert!(tools.contains(&"read_file"));
        assert!(tools.contains(&"grep"));
        assert_eq!(tools.len(), 3);
    }

    #[test]
    fn test_explore_system_prompt() {
        let agent = ExploreAgent;
        let prompt = agent.system_prompt();

        assert!(prompt.contains("READ-ONLY"));
        assert!(prompt.contains("file search specialist"));
        assert!(prompt.contains("ripgrep"));
        assert!(prompt.contains("### list_files"));
        assert!(prompt.contains("### read_file"));
        assert!(prompt.contains("### grep"));
    }

    #[test]
    fn test_explore_is_more_limited_than_planning() {
        let explore = ExploreAgent;
        let planning = super::super::planning::PlanningAgent;

        assert!(explore.available_tools().len() < planning.available_tools().len());
        assert!(!explore.can_use_tool("invoke_agent"));
        assert!(planning.can_use_tool("invoke_agent"));
    }
}
