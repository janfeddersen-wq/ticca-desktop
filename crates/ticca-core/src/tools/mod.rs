//! Native Rust tools for AI agents
//!
//! This module provides the following tools:
//! - `list_files`: List files and directories with intelligent filtering
//! - `read_file`: Read file contents with optional line range
//! - `edit_file`: Edit files using content replacement, text replacements, or snippet deletion
//! - `delete_file`: Delete files with diff generation
//! - `grep`: Search for text patterns using ripgrep
//! - `execute_shell`: Run shell commands in a UI terminal
//! - `list_processes`: List active UI terminal processes
//! - `read_process_output`: Read output from a UI terminal process
//! - `kill_process`: Terminate a UI terminal process
//! - `list_agents`: List available agents
//! - `invoke_agent`: Invoke another agent with its own history
//! - `todo_list`: Track an agent-scoped to do list

pub mod approval;
pub mod file_mods;
pub mod file_ops;
pub mod grep;
pub mod policy;
pub mod registry;
pub mod rig_tools;
pub mod spec;
pub mod system_exec;
pub mod todo;

// Re-export registry types
pub use registry::{
    RegisteredTool, ToolDefinition, ToolExecutor, ToolParameterSchema, ToolRegistry, ToolResult,
};

// Re-export tool implementations
pub use file_mods::{delete_file_impl, edit_file_impl};
pub use file_ops::{list_files_impl, read_file_impl};
pub use grep::grep_impl;

// Re-export rig-compatible tools
pub use approval::{ToolApprovalDecision, ToolApprovalGate, ToolApprovalRequest};
pub use policy::ToolPolicy;
pub use rig_tools::{
    AgentCallEvent, AgentInvokeRequest, AgentInvoker, AgentStreamEvent, DeleteFileTool,
    EditFileTool, ExecuteShellTool, GrepTool, InvokeAgentTool, KillProcessTool, ListAgentsTool,
    ListFilesTool, ListProcessesTool, ReadFileTool, ReadProcessOutputTool, ToolContext,
    WriteFileTool, create_tools,
};
pub use system_exec::{
    ProcessKind, ProcessSnapshot, SystemExecRequest, SystemExecResponse, SystemExecStore,
};
pub use todo::{TodoItem, TodoListEvent, TodoListState, TodoStatus, TodoStore};

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
    registry.register(grep::grep_definition(), grep::grep_executor());

    // System executions (UI-backed)
    registry.register(
        system_exec::execute_shell_definition(),
        system_exec::execute_shell_executor(),
    );
    registry.register(
        system_exec::list_processes_definition(),
        system_exec::list_processes_executor(),
    );
    registry.register(
        system_exec::read_process_output_definition(),
        system_exec::read_process_output_executor(),
    );
    registry.register(
        system_exec::kill_process_definition(),
        system_exec::kill_process_executor(),
    );

    registry
}

/// Get definitions for a subset of tools by name
pub fn get_tool_definitions<'a>(
    registry: &'a ToolRegistry,
    tool_names: &[&str],
) -> Vec<&'a ToolDefinition> {
    registry
        .get_definitions()
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
        assert!(registry.has_tool("execute_shell"));
        assert!(registry.has_tool("list_processes"));
        assert!(registry.has_tool("read_process_output"));
        assert!(registry.has_tool("kill_process"));

        assert_eq!(registry.len(), 9);
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

        let result = registry
            .execute(
                "list_files",
                serde_json::json!({
                    "directory": ".",
                    "recursive": false
                }),
            )
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.content.contains("DIRECTORY LISTING"));
    }
}
