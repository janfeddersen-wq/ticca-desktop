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
        vec!["list_files", "read_file", "grep"]
    }

    fn system_prompt(&self) -> String {
        let tool_specs = tool_specs_for_names(&self.available_tools());
        let tool_docs = PromptBlocks::tool_docs(&tool_specs);

        let intro = r#"You are a fast codebase exploration agent. Your job is to quickly find and report relevant code locations.

=== READ-ONLY MODE ===
You only have read access. No file modifications possible."#;

        let guidelines = r#"## Tools
- `grep` - Search file contents with regex (ripgrep). Use this first for most searches.
- `list_files` - Discover directory structure. Use `recursive: true` for deep scans.
- `read_file` - Read specific files when you need more context.

## Search Strategy
1. Start with `grep` to find relevant code quickly
2. Use `list_files` to understand project structure if needed
3. Read key files to understand implementation details
4. Run multiple tool calls in parallel when possible"#;

        let output_format = r#"## Output Format

Your output should be CONCISE and STRUCTURED. No filler text. Use this format:

```
## [Topic/Question Summary]

**Key files:**
- `path/to/file.rs:LINE` - Brief description
- `path/to/other.rs:LINE` - Brief description

**[Section as needed]:**
- Bullet points with specifics
- Include line numbers: `file.rs:123`

**Summary:** One sentence if needed.
```

## Example Outputs

### Example 1: "Where is authentication handled?"
```
## Authentication

**Key files:**
- `src/auth/handler.rs:45` - Main auth logic, `authenticate()` function
- `src/middleware/auth.rs:12` - Token verification middleware
- `src/models/user.rs:78` - User model with password hashing

**Flow:** Request → middleware validates JWT → handler processes auth
```

### Example 2: "Find usages of ConfigService"
```
## ConfigService Usages

**Definition:** `src/services/config.rs:34`

**Usages (8 total):**
- `src/main.rs:23` - Service initialization
- `src/handlers/settings.rs:15,45,67` - Settings endpoints
- `src/lib.rs:12` - Re-export
- `tests/config_test.rs:8` - Test setup
```

### Example 3: "How does the build system work?"
```
## Build System

**Entry:** `build.rs` - Cargo build script

**Key components:**
- `build.rs:12` - Generates `registry.rs` from JSON
- `src/codegen/mod.rs:34` - Code generation utilities
- `Cargo.toml:45` - Build dependencies

**Process:** build.rs runs at compile time, reads `data/*.json`, generates Rust code.
```

## Rules
- Always include file paths with line numbers when referencing code
- Keep descriptions brief - one line per item
- Use markdown formatting for structure
- No introductory phrases like "I found..." or "Let me explain..."
- Jump straight to the findings"#;

        format!(
            "{}\n\n{}\n\n{}\n\n{}",
            intro, tool_docs, guidelines, output_format
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
        assert!(prompt.contains("codebase exploration"));
        assert!(prompt.contains("Output Format"));
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
