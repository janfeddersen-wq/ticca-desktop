//! Native Rust tools for AI agents
//!
//! This module provides the following tools:
//! - `list_files`: List files and directories with intelligent filtering
//! - `read_file`: Read file contents with optional line range
//! - `edit_file`: Edit files using content replacement, text replacements, or snippet deletion
//! - `delete_file`: Delete files with diff generation
//! - `grep`: Search for text patterns using ripgrep
//! - `shell`: Execute shell commands with timeout support
//! - `list_agents`: List available agents
//! - `invoke_agent`: Invoke another agent with its own history

pub mod registry;
pub mod file_ops;
pub mod file_mods;
pub mod grep;
pub mod shell;
pub mod rig_tools;
pub mod approval;
pub mod spec;
pub mod policy;

// Re-export registry types
pub use registry::{
    ToolDefinition,
    ToolExecutor,
    ToolParameterSchema,
    ToolRegistry,
    ToolResult,
    RegisteredTool,
};

// Re-export tool implementations
pub use file_ops::{list_files_impl, read_file_impl};
pub use file_mods::{edit_file_impl, delete_file_impl};
pub use grep::grep_impl;
pub use shell::shell_impl;

// Re-export rig-compatible tools
pub use rig_tools::{
    ToolContext,
    ShellTool, ReadFileTool, ListFilesTool, EditFileTool, DeleteFileTool, GrepTool, WriteFileTool,
    ListAgentsTool, InvokeAgentTool, AgentCallEvent, AgentStreamEvent, AgentInvokeRequest, AgentInvoker,
    create_tools,
};
pub use approval::{ToolApprovalGate, ToolApprovalRequest, ToolApprovalDecision};
pub use policy::ToolPolicy;

/// Create a tool registry with all available tools registered
pub fn create_default_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    
    // File operations
    registry.register(
        file_ops::list_files_definition(),
        file_ops::list_files_executor(),
    );
    registry.register(
        file_ops::read_file_definition(),
        file_ops::read_file_executor(),
    );
    
    // File modifications
    registry.register(
        file_mods::edit_file_definition(),
        file_mods::edit_file_executor(),
    );
    registry.register(
        file_mods::delete_file_definition(),
        file_mods::delete_file_executor(),
    );
    
    // Search
    registry.register(
        grep::grep_definition(),
        grep::grep_executor(),
    );
    
    // Shell
    registry.register(
        shell::shell_definition(),
        shell::shell_executor(),
    );
    
    registry
}

/// Get definitions for a subset of tools by name
pub fn get_tool_definitions<'a>(registry: &'a ToolRegistry, tool_names: &[&str]) -> Vec<&'a ToolDefinition> {
    registry.get_definitions()
        .into_iter()
        .filter(|def| tool_names.contains(&def.name.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_default_registry() {
        let registry = create_default_registry();
        
        assert!(registry.has_tool("list_files"));
        assert!(registry.has_tool("read_file"));
        assert!(registry.has_tool("edit_file"));
        assert!(registry.has_tool("delete_file"));
        assert!(registry.has_tool("grep"));
        assert!(registry.has_tool("shell"));
        
        assert_eq!(registry.len(), 6);
    }

    #[test]
    fn test_get_tool_definitions() {
        let registry = create_default_registry();
        
        let defs = get_tool_definitions(&registry, &["list_files", "read_file"]);
        assert_eq!(defs.len(), 2);
        
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"list_files"));
        assert!(names.contains(&"read_file"));
    }

    #[tokio::test]
    async fn test_execute_list_files() {
        let registry = create_default_registry();
        
        let result = registry.execute("list_files", serde_json::json!({
            "directory": ".",
            "recursive": false
        })).await.unwrap();
        
        assert!(result.success);
        assert!(result.content.contains("DIRECTORY LISTING"));
    }
}
