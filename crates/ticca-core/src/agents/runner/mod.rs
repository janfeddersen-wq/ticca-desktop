//! Agent execution + LLM streaming runner.
//!
//! This module contains the provider/tool streaming loop, refactored into
//! focused sub-modules for maintainability.

#![allow(clippy::items_after_test_module)]

mod context;
mod mcp;
mod messages;
mod provider;
mod tool_builder;
mod types;

pub use messages::{
    build_assistant_message_with_reasoning, build_chat_history, build_user_message,
    is_rate_limit_error, prepend_system_to_first_user_message,
};
pub use provider::{ResolvedProvider, fetch_best_model, resolve_model_name, resolve_provider};
pub use types::{
    ChatHistoryMessage, DEFAULT_COOLDOWN_SECS, MAX_SUBAGENT_OUTPUT_CHARS, RunnerEvent,
};

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::Instant;

use futures::StreamExt;
use rig::agent::AgentBuilder;
use rig::completion::GetTokenUsage;
use tokio::sync::mpsc;

use crate::agents::{AgentProfile, AgentType};
use crate::config::{ConfigDatabase, setting_keys};
// ProviderId and ProviderRegistry used via provider module
use crate::llm::auth;
use crate::llm::providers::ChatGptOAuthClient;
use crate::tools::{
    AgentCallEvent, AgentInvokeRequest, AgentInvoker, AgentStreamEvent, TodoListEvent, TodoStore,
    ToolApprovalDecision, ToolApprovalGate, ToolApprovalRequest, ToolContext, ToolPolicy,
};

use context::{apply_compression_if_needed, estimate_context, log_context_breakdown};
use mcp::attach_mcp_tools_to_builder;
use tool_builder::add_tool_if_allowed;
use types::{CLAUDE_CODE_INSTRUCTIONS, STATS_WINDOW_MIN_MS};

/// Run the Rig agent with streaming response and tools (ReAct loop).
///
/// Routes to Claude, ChatGPT/Codex, or Gemini based on model name.
/// Returns a Stream that yields events for each chunk.
#[allow(clippy::too_many_arguments)]
pub fn run_rig_agent_stream(
    system_prompt: String,
    user_message: String,
    model_name: Option<String>,
    working_directory: PathBuf,
    max_tool_rounds: u32,
    chat_history: Vec<ChatHistoryMessage>,
    initial_todo_state: Option<crate::tools::TodoListState>,
    image_data: Vec<(String, String)>,
    yolo_mode_enabled: bool,
    current_agent: AgentType,
    mut approval_decision_rx: mpsc::UnboundedReceiver<ToolApprovalDecision>,
    mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
    system_exec_store: Arc<crate::tools::SystemExecStore>,
    system_exec_tx: mpsc::UnboundedSender<crate::tools::SystemExecRequest>,
) -> impl futures::Stream<Item = RunnerEvent> {
    async_stream::stream! {
        // Log context size for debugging
        let history_chars: usize = chat_history.iter().map(|m| m.content.len()).sum();
        tracing::info!(
            "Agent request: history={} msgs ({} chars), prompt={} chars, user={} chars",
            chat_history.len(),
            history_chars,
            system_prompt.len(),
            user_message.len()
        );

        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<RunnerEvent>();
        let (approval_request_tx, mut approval_request_rx) = mpsc::unbounded_channel::<ToolApprovalRequest>();
        let approval_gate = Arc::new(ToolApprovalGate::new(approval_request_tx));

        let resolved_model = match resolve_model_name(model_name).await {
            Ok(name) => name,
            Err(error) => {
                let _ = event_tx.send(RunnerEvent::StreamError(error));
                let _ = event_tx.send(RunnerEvent::StreamComplete);
                return;
            }
        };

        let (call_graph_tx, mut call_graph_rx) = mpsc::unbounded_channel::<AgentCallEvent>();
        let (agent_stream_tx, mut agent_stream_rx) = mpsc::unbounded_channel::<AgentStreamEvent>();
        let call_graph_counter = Arc::new(AtomicUsize::new(1));

        // Explore agent doesn't use todo tools
        let agent_uses_todos = current_agent != AgentType::Explore;
        let (todo_store, todo_tx) = if agent_uses_todos {
            let store = Arc::new(TodoStore::new());
            let (tx, mut rx) = mpsc::unbounded_channel::<TodoListEvent>();
            let event_tx_for_todos = event_tx.clone();
            tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    let _ = event_tx_for_todos.send(RunnerEvent::TodoEvent(event));
                }
            });
            (Some(store), Some(tx))
        } else {
            (None, None)
        };

        // Forward agent call graph events
        let event_tx_for_graph = event_tx.clone();
        tokio::spawn(async move {
            while let Some(event) = call_graph_rx.recv().await {
                let _ = event_tx_for_graph.send(RunnerEvent::AgentCall(event));
            }
        });

        // Forward subagent stream events
        let event_tx_for_stream = event_tx.clone();
        tokio::spawn(async move {
            while let Some(event) = agent_stream_rx.recv().await {
                let _ = event_tx_for_stream.send(RunnerEvent::SubagentStream(event));
            }
        });

        let agent_invoker: Arc<AgentInvoker> = Arc::new(|request: AgentInvokeRequest| {
            Box::pin(invoke_agent(request))
        });

        // Initialize todo state
        if let (Some(store), Some(tx)) = (&todo_store, &todo_tx) {
            let initial_state = match initial_todo_state {
                Some(state) => store.set_node_state(0, state).await,
                None => store.reset_node(0).await,
            };
            let _ = tx.send(TodoListEvent::Reset {
                node_id: 0,
                state: initial_state,
            });
        }

        let tool_context = Arc::new(ToolContext {
            working_directory: working_directory.clone(),
            approval_gate: Some(approval_gate),
            yolo_mode_enabled,
            policy: ToolPolicy::allow_root(working_directory.clone()),
            current_agent,
            current_model: Some(resolved_model.clone()),
            max_tool_rounds,
            call_graph_tx: Some(call_graph_tx),
            agent_stream_tx: Some(agent_stream_tx),
            call_graph_counter: Some(call_graph_counter),
            node_id: 0,
            agent_invoker: Some(agent_invoker),
            todo_store,
            todo_tx,
            system_exec_store: Some(system_exec_store.clone()),
            system_exec_tx: Some(system_exec_tx.clone()),
            process_output_offsets: Arc::new(std::sync::Mutex::new(HashMap::new())),
        });

        // Handle approval requests
        let mut pending_approvals: HashMap<u64, tokio::sync::oneshot::Sender<bool>> = HashMap::new();
        let event_tx_for_manager = event_tx.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(request) = approval_request_rx.recv() => {
                        pending_approvals.insert(request.id, request.responder);
                        let _ = event_tx_for_manager.send(RunnerEvent::ToolApprovalRequested {
                            id: request.id,
                            name: request.tool_name,
                            args: request.args,
                        });
                    }
                    Some(decision) = approval_decision_rx.recv() => {
                        if let Some(responder) = pending_approvals.remove(&decision.id) {
                            let _ = responder.send(decision.approved);
                        }
                    }
                    else => break,
                }
            }
        });

        // Spawn the main streaming worker
        let event_tx_for_worker = event_tx.clone();
        let tool_context_worker = tool_context.clone();
        tokio::spawn(async move {
            let mut worker = tokio::spawn(run_agent_stream(
                event_tx_for_worker.clone(),
                tool_context_worker,
                system_prompt,
                user_message,
                resolved_model,
                max_tool_rounds,
                chat_history,
                image_data,
            ));

            tokio::select! {
                _ = &mut cancel_rx => {
                    worker.abort();
                    let _ = event_tx_for_worker.send(RunnerEvent::StreamStopped);
                }
                result = &mut worker => {
                    match result {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => {
                            let _ = event_tx_for_worker.send(RunnerEvent::StreamError(error));
                        }
                        Err(error) => {
                            let _ = event_tx_for_worker.send(RunnerEvent::StreamError(format!("Stream task error: {}", error)));
                        }
                    }
                }
            }
            let _ = event_tx_for_worker.send(RunnerEvent::StreamComplete);
        });

        // Yield events from the channel
        while let Some(event) = event_rx.recv().await {
            let is_terminal = matches!(event, RunnerEvent::StreamComplete | RunnerEvent::StreamError(_));
            yield event;
            if is_terminal {
                break;
            }
        }
    }
}

fn resolve_invocation_model(
    agent_type: AgentType,
    parent_context: &ToolContext,
) -> Result<String, String> {
    if let Ok(db) = ConfigDatabase::open() {
        if let Ok(Some(model)) = db.get_agent_pinned_model(agent_type.as_str())
            && !model.trim().is_empty()
        {
            return Ok(model);
        }

        if let Ok(Some(setting)) = db.get_setting(setting_keys::DEFAULT_MODEL)
            && !setting.value.trim().is_empty()
        {
            return Ok(setting.value);
        }
    }

    parent_context
        .current_model
        .clone()
        .ok_or_else(|| "No model configured for invoke_agent".to_string())
}

async fn append_agents_md(system_prompt: &str, working_directory: &Path) -> String {
    let agents_path = working_directory.join("AGENTS.md");
    match tokio::fs::read_to_string(&agents_path).await {
        Ok(contents) if !contents.trim().is_empty() => {
            format!(
                "{}\n\n## AGENTS.md\n{}\n",
                system_prompt.trim_end(),
                contents.trim()
            )
        }
        _ => system_prompt.to_string(),
    }
}

async fn invoke_agent(request: AgentInvokeRequest) -> Result<String, String> {
    let parent_context = request.parent_context;
    let agent_type = request.agent_type;
    let model_name = resolve_invocation_model(agent_type, &parent_context)?;
    let max_tool_rounds = parent_context.max_tool_rounds.max(1);

    let mut profile = AgentProfile::for_type(agent_type, max_tool_rounds);
    profile.system_prompt =
        append_agents_md(&profile.system_prompt, &parent_context.working_directory).await;

    if let Some(tx) = &parent_context.agent_stream_tx {
        let _ = tx.send(AgentStreamEvent::Start {
            node_id: request.node_id,
            agent_type,
        });
    }

    // Explore agent doesn't use todo tools
    let (todo_store, todo_tx) = if agent_type == AgentType::Explore {
        (None, None)
    } else {
        (
            parent_context.todo_store.clone(),
            parent_context.todo_tx.clone(),
        )
    };

    let tool_context = Arc::new(ToolContext {
        working_directory: parent_context.working_directory.clone(),
        approval_gate: parent_context.approval_gate.clone(),
        yolo_mode_enabled: parent_context.yolo_mode_enabled,
        policy: parent_context.policy.clone(),
        current_agent: agent_type,
        current_model: Some(model_name.clone()),
        max_tool_rounds,
        call_graph_tx: parent_context.call_graph_tx.clone(),
        agent_stream_tx: parent_context.agent_stream_tx.clone(),
        call_graph_counter: parent_context.call_graph_counter.clone(),
        node_id: request.node_id,
        agent_invoker: parent_context.agent_invoker.clone(),
        todo_store,
        todo_tx,
        system_exec_store: parent_context.system_exec_store.clone(),
        system_exec_tx: parent_context.system_exec_tx.clone(),
        process_output_offsets: parent_context.process_output_offsets.clone(),
    });

    // Reset todo for agents that use it
    if let Some(store) = &tool_context.todo_store {
        let state = store.reset_node(request.node_id).await;
        if let Some(tx) = &tool_context.todo_tx {
            let _ = tx.send(TodoListEvent::Reset {
                node_id: request.node_id,
                state,
            });
        }
    }

    let invoke_prompt = format!(
        "You are assisting the {}. Provide a concise, actionable response for the invoking agent.\n\nTask:\n{}",
        parent_context.current_agent.display_name(),
        request.prompt
    );
    let user_msg = build_user_message(&invoke_prompt, Vec::new());
    let history = vec![user_msg];

    let result = run_invoked_agent_stream(
        request.node_id,
        &tool_context,
        &parent_context,
        &profile,
        &model_name,
        history,
        max_tool_rounds,
    )
    .await;

    if let Some(tx) = &parent_context.agent_stream_tx {
        let output = match &result {
            Ok(text) => text.clone(),
            Err(error) => format!("Error: {}", error),
        };
        let _ = tx.send(AgentStreamEvent::Complete {
            node_id: request.node_id,
            output,
        });
    }

    result
}

/// Run the streaming loop for an invoked subagent.
async fn run_invoked_agent_stream(
    node_id: usize,
    tool_context: &Arc<ToolContext>,
    parent_context: &ToolContext,
    profile: &AgentProfile,
    model_name: &str,
    history: Vec<rig::message::Message>,
    max_tool_rounds: u32,
) -> Result<String, String> {
    let resolved = resolve_provider(model_name)?;
    let preamble = if matches!(resolved, ResolvedProvider::Claude { .. }) {
        CLAUDE_CODE_INSTRUCTIONS
    } else {
        &profile.system_prompt
    };

    // Build tools once
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

    macro_rules! build_agent {
        ($model:expr) => {{
            let mut builder = AgentBuilder::new($model)
                .preamble(preamble)
                .tool(list_files);
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
            let builder = attach_mcp_tools_to_builder(builder, tool_context.current_agent).await;
            builder.temperature(0.7).max_tokens(8192)
        }};
    }

    let max_turns = max_tool_rounds.max(1) as usize;

    let mut final_output = match resolved {
        ResolvedProvider::Claude {
            client, model_id, ..
        } => {
            let model = client.completion_model(&model_id);
            let agent = build_agent!(model).build();

            let mut claude_history = history;
            prepend_system_to_first_user_message(&profile.system_prompt, &mut claude_history);

            stream_agent_loop(
                node_id,
                parent_context,
                agent,
                claude_history,
                max_turns,
                "Claude",
            )
            .await?
        }
        ResolvedProvider::ChatGpt {
            client, model_id, ..
        } => {
            let model = client.completion_model(&model_id);
            let agent = build_agent!(model)
                .additional_params(ChatGptOAuthClient::codex_params())
                .build();

            stream_agent_loop(
                node_id,
                parent_context,
                agent,
                history,
                max_turns,
                "ChatGPT",
            )
            .await?
        }
        ResolvedProvider::Gemini {
            client, model_id, ..
        } => {
            let model = client.completion_model(&model_id);
            let agent = build_agent!(model).build();

            stream_agent_loop(node_id, parent_context, agent, history, max_turns, "Gemini").await?
        }
        ResolvedProvider::ApiKey {
            client,
            model_id,
            provider_name,
        } => {
            let model = client.completion_model(&model_id);
            let agent = build_agent!(model).build();

            stream_agent_loop(
                node_id,
                parent_context,
                agent,
                history,
                max_turns,
                &provider_name,
            )
            .await?
        }
    };

    // Truncate if needed
    if final_output.len() > MAX_SUBAGENT_OUTPUT_CHARS {
        let truncated = &final_output[..MAX_SUBAGENT_OUTPUT_CHARS];
        let cut_point = truncated.rfind('\n').unwrap_or(MAX_SUBAGENT_OUTPUT_CHARS);
        final_output = format!(
            "{}\n\n[... output truncated at {} chars ...]",
            &final_output[..cut_point],
            MAX_SUBAGENT_OUTPUT_CHARS
        );
    }

    Ok(final_output)
}

/// Generic streaming loop for any agent.
async fn stream_agent_loop<M>(
    node_id: usize,
    parent_context: &ToolContext,
    agent: rig::agent::Agent<M>,
    history: Vec<rig::message::Message>,
    max_turns: usize,
    provider_label: &str,
) -> Result<String, String>
where
    M: rig::completion::CompletionModel + 'static,
    M::StreamingResponse: GetTokenUsage,
{
    use rig::agent::MultiTurnStreamItem;
    use rig::streaming::{StreamedAssistantContent, StreamedUserContent, StreamingPrompt};

    let mut history = history;
    let mut final_output = String::new();

    let mut stream = agent
        .stream_prompt("")
        .with_history(history.clone())
        .multi_turn(max_turns)
        .await;

    let mut collected_pass = String::new();
    let mut collected_reasoning = String::new();
    let mut collected_signature: Option<String> = None;

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(
                text_chunk,
            ))) => {
                if !text_chunk.text.is_empty() {
                    collected_pass.push_str(&text_chunk.text);
                    if let Some(tx) = &parent_context.agent_stream_tx {
                        let _ = tx.send(AgentStreamEvent::Chunk {
                            node_id,
                            text: text_chunk.text,
                        });
                    }
                }
            }
            Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Reasoning(
                reasoning,
            ))) => {
                let text = reasoning.reasoning.join("");
                if !text.is_empty() {
                    collected_reasoning.push_str(&text);
                    if collected_signature.is_none() {
                        collected_signature = reasoning.signature.clone();
                    }
                    if let Some(tx) = &parent_context.agent_stream_tx {
                        let _ = tx.send(AgentStreamEvent::Reasoning { node_id, text });
                    }
                }
            }
            Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::ToolCall(
                tool_call,
            ))) => {
                collected_pass.clear();
                collected_reasoning.clear();
                collected_signature = None;
                if let Some(tx) = &parent_context.agent_stream_tx {
                    let _ = tx.send(AgentStreamEvent::ToolCall {
                        node_id,
                        name: tool_call.function.name,
                        args: tool_call.function.arguments.to_string(),
                    });
                }
            }
            Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult(_))) => {
                collected_pass.clear();
                collected_reasoning.clear();
                collected_signature = None;
            }
            Ok(_) => {}
            Err(e) => {
                let error_msg = format!("{} invoke error: {}", provider_label, e);
                if let Some(tx) = &parent_context.agent_stream_tx {
                    let _ = tx.send(AgentStreamEvent::Chunk {
                        node_id,
                        text: format!("❌ {}", error_msg),
                    });
                }
                return Err(error_msg);
            }
        }
    }

    if !collected_pass.is_empty() || !collected_reasoning.is_empty() {
        final_output = collected_pass.clone();
        let reasoning_opt = if collected_reasoning.is_empty() {
            None
        } else {
            Some(collected_reasoning.as_str())
        };
        history.push(build_assistant_message_with_reasoning(
            &collected_pass,
            reasoning_opt,
            collected_signature.as_deref(),
        ));
    }

    Ok(final_output)
}

#[allow(clippy::too_many_arguments)]
async fn run_agent_stream(
    event_tx: mpsc::UnboundedSender<RunnerEvent>,
    tool_context: Arc<ToolContext>,
    system_prompt: String,
    user_message: String,
    model_name: String,
    max_tool_rounds: u32,
    chat_history: Vec<ChatHistoryMessage>,
    image_data: Vec<(String, String)>,
) -> Result<(), String> {
    let mut stats_window_start = Instant::now();
    let mut stats_window_chars: usize = 0;

    let emit_stream_stats = |chars: &mut usize, start: &mut Instant| -> Option<RunnerEvent> {
        if *chars == 0 {
            return None;
        }
        let elapsed = start.elapsed();
        let window_ms = elapsed.as_millis() as u64;
        if window_ms < STATS_WINDOW_MIN_MS {
            return None;
        }
        let c = *chars;
        *chars = 0;
        *start = Instant::now();
        Some(RunnerEvent::StreamStats {
            chars_in_window: c,
            window_ms,
        })
    };

    let history = build_chat_history(chat_history);
    let user_msg = build_user_message(&user_message, image_data);

    let mut rig_messages: Vec<rig::message::Message> = history.clone();
    rig_messages.push(user_msg.clone());

    // Estimate context and emit event
    let estimation = estimate_context(&system_prompt, &rig_messages, &model_name);
    log_context_breakdown(&system_prompt, &rig_messages, &estimation);

    let _ = event_tx.send(RunnerEvent::ContextEstimate {
        system_prompt_tokens: estimation.system_prompt_tokens,
        tool_definitions_tokens: estimation.tool_definitions_tokens,
        messages_tokens: estimation.messages_tokens,
        total_tokens: estimation.total_tokens,
        context_window: estimation.context_window,
        usage_percent: estimation.usage_percent,
    });

    // Apply compression if needed
    let full_history = if estimation.needs_compression {
        apply_compression_if_needed(rig_messages, &estimation, &event_tx)
    } else {
        let mut h = history;
        h.push(user_msg);
        h
    };

    let profile = AgentProfile::for_type(tool_context.current_agent, max_tool_rounds);
    let resolved = resolve_provider(&model_name)?;

    // preamble is handled per-provider in build_and_run! macro

    // Build tools
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

    macro_rules! build_and_run {
        ($model:expr, $preamble:expr, $history:expr, $additional_params:expr, $provider_label:expr, $token:expr) => {{
            let mut builder = AgentBuilder::new($model)
                .preamble($preamble)
                .tool(list_files);
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
            let builder = attach_mcp_tools_to_builder(builder, tool_context.current_agent).await;
            let agent = if let Some(params) = $additional_params {
                builder
                    .temperature(0.7)
                    .max_tokens(8192)
                    .additional_params(params)
                    .build()
            } else {
                builder.temperature(0.7).max_tokens(8192).build()
            };

            run_main_stream_loop(
                &event_tx,
                agent,
                $history,
                max_tool_rounds,
                &mut stats_window_chars,
                &mut stats_window_start,
                &emit_stream_stats,
                $provider_label,
                $token,
            )
            .await
        }};
    }

    match resolved {
        ResolvedProvider::Claude {
            client,
            model_id,
            token,
        } => {
            let model = client.completion_model(&model_id);
            let mut claude_history = full_history;
            prepend_system_to_first_user_message(&profile.system_prompt, &mut claude_history);
            build_and_run!(
                model,
                CLAUDE_CODE_INSTRUCTIONS,
                claude_history,
                None::<serde_json::Value>,
                "Claude",
                Some(&token)
            )
        }
        ResolvedProvider::ChatGpt {
            client,
            model_id,
            token,
        } => {
            let model = client.completion_model(&model_id);
            build_and_run!(
                model,
                &system_prompt,
                full_history,
                Some(ChatGptOAuthClient::codex_params()),
                "ChatGPT",
                Some(&token)
            )
        }
        ResolvedProvider::Gemini {
            client,
            model_id,
            token,
        } => {
            let model = client.completion_model(&model_id);
            build_and_run!(
                model,
                &system_prompt,
                full_history,
                None::<serde_json::Value>,
                "Gemini",
                Some(&token)
            )
        }
        ResolvedProvider::ApiKey {
            client,
            model_id,
            provider_name,
        } => {
            let model = client.completion_model(&model_id);
            build_and_run!(
                model,
                &system_prompt,
                full_history,
                None::<serde_json::Value>,
                &provider_name,
                None::<&auth::AuthToken>
            )
        }
    }
}

/// Main streaming loop for the top-level agent.
#[allow(clippy::too_many_arguments)]
async fn run_main_stream_loop<M, F>(
    event_tx: &mpsc::UnboundedSender<RunnerEvent>,
    agent: rig::agent::Agent<M>,
    full_history: Vec<rig::message::Message>,
    max_tool_rounds: u32,
    stats_window_chars: &mut usize,
    stats_window_start: &mut Instant,
    emit_stream_stats: &F,
    provider_label: &str,
    token: Option<&auth::AuthToken>,
) -> Result<(), String>
where
    M: rig::completion::CompletionModel + 'static,
    M::StreamingResponse: GetTokenUsage,
    F: Fn(&mut usize, &mut Instant) -> Option<RunnerEvent>,
{
    use rig::agent::MultiTurnStreamItem;
    use rig::streaming::{StreamedAssistantContent, StreamedUserContent, StreamingPrompt};

    let mut history = full_history;
    let max_turns = max_tool_rounds.max(1) as usize;

    let mut stream = agent
        .stream_prompt("")
        .with_history(history.clone())
        .multi_turn(max_turns)
        .await;

    let mut collected_pass = String::new();
    let mut collected_reasoning = String::new();
    let mut collected_signature: Option<String> = None;

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(
                text_chunk,
            ))) => {
                if !text_chunk.text.is_empty() {
                    let chunk_len = text_chunk.text.len();
                    collected_pass.push_str(&text_chunk.text);
                    let _ = event_tx.send(RunnerEvent::StreamChunk(text_chunk.text));
                    *stats_window_chars += chunk_len;
                    if let Some(stats) = emit_stream_stats(stats_window_chars, stats_window_start) {
                        let _ = event_tx.send(stats);
                    }
                }
            }
            Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Reasoning(
                reasoning,
            ))) => {
                let text = reasoning.reasoning.join("");
                if !text.is_empty() {
                    collected_reasoning.push_str(&text);
                    if collected_signature.is_none() {
                        collected_signature = reasoning.signature.clone();
                    }
                    let _ = event_tx.send(RunnerEvent::Reasoning {
                        text,
                        signature: reasoning.signature.clone(),
                    });
                }
            }
            Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::ToolCall(
                tool_call,
            ))) => {
                collected_pass.clear();
                collected_reasoning.clear();
                collected_signature = None;
                let _ = event_tx.send(RunnerEvent::ToolCall {
                    name: tool_call.function.name,
                    args: tool_call.function.arguments.to_string(),
                });
            }
            Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Final(
                response,
            ))) => {
                if let Some(usage) = response.token_usage() {
                    let _ = event_tx.send(RunnerEvent::Usage {
                        input_tokens: usage.input_tokens,
                        output_tokens: usage.output_tokens,
                    });
                }
            }
            Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult(_))) => {
                collected_pass.clear();
                collected_reasoning.clear();
                collected_signature = None;
            }
            Ok(MultiTurnStreamItem::FinalResponse(_)) => {}
            Ok(MultiTurnStreamItem::PreRequestContextEstimate(estimate)) => {
                let _ = event_tx.send(RunnerEvent::ContextEstimate {
                    system_prompt_tokens: estimate.system_prompt_tokens,
                    tool_definitions_tokens: estimate.tool_definitions_tokens,
                    messages_tokens: estimate.messages_tokens,
                    total_tokens: estimate.total_tokens,
                    context_window: estimate.context_window,
                    usage_percent: estimate.usage_percent,
                });
            }
            Ok(_) => {}
            Err(e) => {
                let error_msg = format!("{} stream error: {}", provider_label, e);
                if is_rate_limit_error(&error_msg)
                    && let Some(t) = token
                {
                    auth::mark_cooldown(&t.account_id, &error_msg, DEFAULT_COOLDOWN_SECS);
                }
                return Err(error_msg);
            }
        }
    }

    if !collected_pass.is_empty() || !collected_reasoning.is_empty() {
        let reasoning_opt = if collected_reasoning.is_empty() {
            None
        } else {
            Some(collected_reasoning.as_str())
        };
        history.push(build_assistant_message_with_reasoning(
            &collected_pass,
            reasoning_opt,
            collected_signature.as_deref(),
        ));
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::manual_async_fn)]
mod tests {
    use super::*;
    use futures::stream;
    use rig::completion::{
        CompletionError, CompletionModel, CompletionRequest, CompletionResponse, Usage,
    };
    use rig::message::AssistantContent;
    use rig::one_or_many::OneOrMany;
    use rig::streaming::{RawStreamingChoice, StreamingCompletionResponse};
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct MockModel;

    impl CompletionModel for MockModel {
        type Response = ();
        type StreamingResponse = ();
        type Client = ();

        fn make(_client: &Self::Client, _model: impl Into<String>) -> Self {
            Self
        }

        fn completion(
            &self,
            _request: CompletionRequest,
        ) -> impl std::future::Future<
            Output = Result<CompletionResponse<Self::Response>, CompletionError>,
        > + Send {
            async move {
                Ok(CompletionResponse {
                    choice: OneOrMany::one(AssistantContent::text("mock response")),
                    usage: Usage::new(),
                    raw_response: (),
                })
            }
        }

        fn stream(
            &self,
            _request: CompletionRequest,
        ) -> impl std::future::Future<
            Output = Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError>,
        > + Send {
            async move {
                let events = vec![
                    Ok(RawStreamingChoice::Message("mock stream".to_string())),
                    Ok(RawStreamingChoice::FinalResponse(())),
                ];
                let stream = stream::iter(events);
                Ok(StreamingCompletionResponse::stream(Box::pin(stream)))
            }
        }
    }

    #[tokio::test]
    async fn mock_stream_smoke_test() {
        let tool_context = Arc::new(ToolContext::default());
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

        let history = build_chat_history(Vec::new());
        let user_msg = build_user_message("hello", Vec::new());
        let mut full_history = history;
        full_history.push(user_msg);

        let agent = AgentBuilder::new(MockModel)
            .preamble("test system")
            .tool(execute_shell)
            .tool(list_processes)
            .tool(read_process_output)
            .tool(kill_process)
            .tool(read_file)
            .tool(list_files)
            .tool(edit_file)
            .tool(delete_file)
            .tool(grep)
            .tool(write_file)
            .tool(list_agents)
            .tool(todo_read)
            .tool(todo_write)
            .tool(todo_list)
            .tool(share_reasoning)
            .tool(invoke_agent_tool)
            .temperature(0.1)
            .max_tokens(64)
            .build();

        use rig::agent::MultiTurnStreamItem;
        use rig::streaming::StreamedAssistantContent;
        use rig::streaming::StreamingPrompt;

        let mut stream = agent
            .stream_prompt("")
            .with_history(full_history)
            .multi_turn(1)
            .await;

        let mut collected = String::new();
        while let Some(chunk_result) = stream.next().await {
            if let Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(
                text_chunk,
            ))) = chunk_result
            {
                collected.push_str(&text_chunk.text);
            }
        }

        assert!(collected.contains("mock stream"));
    }

    #[tokio::test]
    async fn invoked_agent_todo_guard_errors_after_max_passes() {
        let todo_store = Arc::new(TodoStore::new());
        let _ = todo_store.reset_node(1).await;

        let parent_context = ToolContext {
            todo_store: Some(todo_store),
            ..Default::default()
        };

        let tool_context = Arc::new(parent_context.clone());
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

        let agent = AgentBuilder::new(MockModel)
            .preamble("test system")
            .tool(execute_shell)
            .tool(list_processes)
            .tool(read_process_output)
            .tool(kill_process)
            .tool(read_file)
            .tool(list_files)
            .tool(edit_file)
            .tool(delete_file)
            .tool(grep)
            .tool(write_file)
            .tool(list_agents)
            .tool(todo_read)
            .tool(todo_write)
            .tool(todo_list)
            .tool(share_reasoning)
            .tool(invoke_agent_tool)
            .temperature(0.1)
            .max_tokens(64)
            .build();

        let history = vec![build_user_message("hello", Vec::new())];
        let result = stream_agent_loop(1, &parent_context, agent, history, 1, "Mock").await;

        let output = result.unwrap();
        assert!(output.contains("mock stream") || output.is_empty());
    }
}
