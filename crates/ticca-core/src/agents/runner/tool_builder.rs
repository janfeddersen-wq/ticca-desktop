//! Tool builder for creating agents with appropriate tools.
//!
//! This module handles adding tools to serdesAI agents based on agent profiles.

#![allow(dead_code)]

use std::sync::Arc;

use serdes_ai_toolsets::DynamicToolset;

use crate::agents::AgentProfile;
use crate::tools::{TiccaDeps, ToolContext};

/// Macro to conditionally add a tool to a toolset based on profile.tool_names.
macro_rules! add_tool_if_allowed {
    ($toolset:expr, $profile:expr, $tool_name:literal, $tool:expr) => {
        if $profile.tool_names.contains(&$tool_name) {
            $toolset.add_tool($tool);
        }
    };
}

/// Build a toolset with all appropriate tools based on the profile.
/// 
/// Returns a DynamicToolset that can be attached to a serdesAI agent.
pub fn build_toolset_for_profile(
    profile: &AgentProfile,
    tool_context: Arc<ToolContext>,
) -> DynamicToolset<TiccaDeps> {
    let toolset = DynamicToolset::new();

    let (
        execute_shell,
        list_processes,
        read_process_output,
        kill_process,
        read_file,
        list_files,
        edit_file,
        delete_file,
        grep,
        write_file,
        list_agents,
        todo_read,
        todo_write,
        todo_list,
        share_reasoning,
        invoke_agent_tool,
    ) = crate::tools::create_tools(tool_context);

    // All agents have list_files
    toolset.add_tool(list_files);

    // Conditionally add remaining tools based on profile
    add_tool_if_allowed!(toolset, profile, "execute_shell", execute_shell);
    add_tool_if_allowed!(toolset, profile, "list_processes", list_processes);
    add_tool_if_allowed!(toolset, profile, "read_process_output", read_process_output);
    add_tool_if_allowed!(toolset, profile, "kill_process", kill_process);
    add_tool_if_allowed!(toolset, profile, "read_file", read_file);
    add_tool_if_allowed!(toolset, profile, "edit_file", edit_file);
    add_tool_if_allowed!(toolset, profile, "delete_file", delete_file);
    add_tool_if_allowed!(toolset, profile, "grep", grep);
    add_tool_if_allowed!(toolset, profile, "write_file", write_file);
    add_tool_if_allowed!(toolset, profile, "list_agents", list_agents);
    add_tool_if_allowed!(toolset, profile, "todo_read", todo_read);
    add_tool_if_allowed!(toolset, profile, "todo_write", todo_write);
    add_tool_if_allowed!(toolset, profile, "todo_list", todo_list);
    add_tool_if_allowed!(toolset, profile, "share_reasoning", share_reasoning);
    add_tool_if_allowed!(toolset, profile, "invoke_agent", invoke_agent_tool);

    toolset
}

/// Re-export the macro for use in tests
pub(crate) use add_tool_if_allowed;
