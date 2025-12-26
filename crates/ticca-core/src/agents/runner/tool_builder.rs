//! Tool builder for creating agents with appropriate tools.
//!
//! This module consolidates the repeated tool creation and builder setup
//! that was previously duplicated for each provider.

#![allow(dead_code)]

use std::sync::Arc;
use rig::agent::{AgentBuilder, AgentBuilderSimple};

use crate::agents::AgentProfile;
use crate::tools::ToolContext;
use super::mcp::attach_mcp_tools_to_builder;

/// Macro to conditionally add a tool to an AgentBuilder based on profile.tool_names.
macro_rules! add_tool_if_allowed {
    ($builder:expr, $profile:expr, $tool_name:literal, $tool:expr) => {
        if $profile.tool_names.contains(&$tool_name) {
            $builder = $builder.tool($tool);
        }
    };
}

/// Build an agent with tools based on the profile and context.
///
/// This function consolidates the repeated pattern of:
/// 1. Creating tools from context
/// 2. Adding tools conditionally based on profile
/// 3. Attaching MCP tools
pub async fn build_agent_with_tools<M>(
    model: M,
    preamble: &str,
    profile: &AgentProfile,
    tool_context: Arc<ToolContext>,
) -> rig::agent::Agent<M>
where
    M: rig::completion::CompletionModel + 'static,
{
    let builder = create_builder_with_tools(model, preamble, profile, tool_context.clone()).await;
    builder.temperature(0.7).max_tokens(8192).build()
}

/// Build an agent with tools and ChatGPT-specific parameters.
pub async fn build_chatgpt_agent_with_tools<M>(
    model: M,
    preamble: &str,
    profile: &AgentProfile,
    tool_context: Arc<ToolContext>,
    additional_params: serde_json::Value,
) -> rig::agent::Agent<M>
where
    M: rig::completion::CompletionModel + 'static,
{
    let builder = create_builder_with_tools(model, preamble, profile, tool_context.clone()).await;
    builder
        .temperature(0.7)
        .max_tokens(8192)
        .additional_params(additional_params)
        .build()
}

/// Create an AgentBuilder with all appropriate tools attached.
async fn create_builder_with_tools<M>(
    model: M,
    preamble: &str,
    profile: &AgentProfile,
    tool_context: Arc<ToolContext>,
) -> AgentBuilderSimple<M>
where
    M: rig::completion::CompletionModel + 'static,
{
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
    ) = crate::tools::create_tools(tool_context.clone());

    // All agents have list_files - use it as anchor to convert AgentBuilder -> AgentBuilderSimple
    let mut builder = AgentBuilder::new(model)
        .preamble(preamble)
        .tool(list_files);

    // Now conditionally add remaining tools (skip list_files since already added)
    add_tool_if_allowed!(builder, profile, "execute_shell", execute_shell);
    add_tool_if_allowed!(builder, profile, "list_processes", list_processes);
    add_tool_if_allowed!(builder, profile, "read_process_output", read_process_output);
    add_tool_if_allowed!(builder, profile, "kill_process", kill_process);
    add_tool_if_allowed!(builder, profile, "read_file", read_file);
    add_tool_if_allowed!(builder, profile, "edit_file", edit_file);
    add_tool_if_allowed!(builder, profile, "delete_file", delete_file);
    add_tool_if_allowed!(builder, profile, "grep", grep);
    add_tool_if_allowed!(builder, profile, "write_file", write_file);
    add_tool_if_allowed!(builder, profile, "list_agents", list_agents);
    add_tool_if_allowed!(builder, profile, "todo_read", todo_read);
    add_tool_if_allowed!(builder, profile, "todo_write", todo_write);
    add_tool_if_allowed!(builder, profile, "todo_list", todo_list);
    add_tool_if_allowed!(builder, profile, "share_reasoning", share_reasoning);
    add_tool_if_allowed!(builder, profile, "invoke_agent", invoke_agent_tool);

    // Attach MCP tools
    attach_mcp_tools_to_builder(builder, tool_context.current_agent).await
}

/// Re-export the macro for use in tests
pub(crate) use add_tool_if_allowed;
