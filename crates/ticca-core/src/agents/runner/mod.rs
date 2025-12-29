//! Agent execution + LLM streaming runner.
//!
//! This module contains the provider/tool streaming loop using serdesAI models.
//!
//! Note: This is a simplified implementation that uses models directly.
//! Full agent abstraction with tools will be added in a future iteration.

#![allow(clippy::items_after_test_module)]

mod context;
mod messages;
mod provider;
mod tool_builder;
mod types;

pub use messages::{
    build_chat_history, build_user_message,
    extract_last_user_message, is_rate_limit_error, prepend_system_to_first_user_message,
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
use tokio::sync::mpsc;

use serdes_ai_core::{ModelRequest, ModelResponse, ModelSettings};
use serdes_ai_core::messages::{ModelRequestPart, ModelResponsePart, ThinkingPart, ToolCallPart, ToolReturnPart, UserContent};
use serdes_ai_models::{Model, ModelRequestParameters};
use serdes_ai_tools::{RunContext, ToolDefinition};
use serdes_ai_toolsets::AbstractToolset;

use crate::agents::{AgentProfile, AgentType};
use crate::config::{ConfigDatabase, setting_keys};
use crate::llm::auth;
use crate::tools::{
    AgentCallEvent, AgentInvokeRequest, AgentInvoker, AgentStreamEvent, TiccaDeps, TodoListEvent, TodoStore,
    ToolApprovalDecision, ToolApprovalGate, ToolApprovalRequest, ToolContext, ToolPolicy,
};

use context::{apply_compression_if_needed, estimate_context, log_context_breakdown};
use tool_builder::build_toolset_for_profile;
use types::{CLAUDE_CODE_INSTRUCTIONS, STATS_WINDOW_MIN_MS};

/// Run the agent with streaming response.
///
/// Routes to Claude or ChatGPT based on model name.
/// Returns a Stream that yields events for each chunk.
#[allow(clippy::too_many_arguments)]
pub fn run_agent_stream(
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
                while let Some(event) = event_rx.recv().await {
                    yield event;
                }
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
            let mut worker = tokio::spawn(run_streaming_agent(
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

    let mut profile = AgentProfile::for_type(agent_type, parent_context.max_tool_rounds);
    profile.system_prompt =
        append_agents_md(&profile.system_prompt, &parent_context.working_directory).await;

    if let Some(tx) = &parent_context.agent_stream_tx {
        let _ = tx.send(AgentStreamEvent::Start {
            node_id: request.node_id,
            agent_type,
        });
    }

    // Build a simple request and run it
    let resolved = resolve_provider(&model_name)?;

    let mut final_output = String::new();

    // For subagents, use a simple non-streaming request
    match resolved {
        ResolvedProvider::Claude { model, .. } => {
            let mut req = ModelRequest::new();
            req.add_system_prompt(&profile.system_prompt);
            req.add_user_prompt(UserContent::text(&request.prompt));

            let settings = ModelSettings::new().temperature(0.7).max_tokens(8192);
            let params = ModelRequestParameters::default();

            match model.request(&[req], &settings, &params).await {
                Ok(response) => {
                    final_output = response.text_content();
                }
                Err(e) => {
                    return Err(format!("Claude invoke error: {}", e));
                }
            }
        }
        ResolvedProvider::ChatGpt { model, .. } => {
            let mut req = ModelRequest::new();
            req.add_system_prompt(&profile.system_prompt);
            req.add_user_prompt(UserContent::text(&request.prompt));

            let settings = ModelSettings::new().temperature(0.7).max_tokens(16384);
            let params = ModelRequestParameters::default();

            match model.request(&[req], &settings, &params).await {
                Ok(response) => {
                    final_output = response.text_content();
                }
                Err(e) => {
                    return Err(format!("ChatGPT invoke error: {}", e));
                }
            }
        }
        ResolvedProvider::OpenAICompatible { model, provider_id, .. } => {
            let mut req = ModelRequest::new();
            req.add_system_prompt(&profile.system_prompt);
            req.add_user_prompt(UserContent::text(&request.prompt));

            let settings = ModelSettings::new().temperature(0.7).max_tokens(8192);
            let params = ModelRequestParameters::default();

            match model.request(&[req], &settings, &params).await {
                Ok(response) => {
                    final_output = response.text_content();
                }
                Err(e) => {
                    return Err(format!("{} invoke error: {}", provider_id, e));
                }
            }
        }
    }

    if let Some(tx) = &parent_context.agent_stream_tx {
        let _ = tx.send(AgentStreamEvent::Complete {
            node_id: request.node_id,
            output: final_output.clone(),
        });
    }

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

/// Main streaming agent runner with tool execution loop.
#[allow(clippy::too_many_arguments)]
async fn run_streaming_agent(
    event_tx: mpsc::UnboundedSender<RunnerEvent>,
    tool_context: Arc<ToolContext>,
    system_prompt: String,
    user_message: String,
    model_name: String,
    max_tool_rounds: u32,
    chat_history: Vec<ChatHistoryMessage>,
    image_data: Vec<(String, String)>,
) -> Result<(), String> {
    // Build the toolset for this agent
    let agent_type = tool_context.current_agent;
    let profile = AgentProfile::for_type(agent_type, max_tool_rounds);
    let toolset = build_toolset_for_profile(&profile, tool_context.clone());
    
    // Get tool definitions
    let deps = TiccaDeps::new(tool_context.clone());
    let run_context = RunContext::<TiccaDeps>::new(deps, &model_name);
    let tools_map = toolset.get_tools(&run_context).await
        .map_err(|e| format!("Failed to get tools: {}", e))?;
    
    let tool_defs: Vec<ToolDefinition> = tools_map.values()
        .map(|t| t.tool_def.clone())
        .collect();
    
    tracing::info!("Agent {} has {} tools available", agent_type.as_str(), tool_defs.len());
    
    let tool_defs_arc = Arc::new(tool_defs);

    // Build initial messages
    let history = build_chat_history(chat_history);
    let user_msg = build_user_message(&user_message, image_data);

    let mut messages: Vec<ModelRequest> = history;
    messages.push(user_msg);

    // Estimate context and emit event
    let estimation = estimate_context(&system_prompt, &messages, &model_name);
    log_context_breakdown(&system_prompt, &messages, &estimation);

    let _ = event_tx.send(RunnerEvent::ContextEstimate {
        system_prompt_tokens: estimation.system_prompt_tokens,
        tool_definitions_tokens: estimation.tool_definitions_tokens,
        messages_tokens: estimation.messages_tokens,
        total_tokens: estimation.total_tokens,
        context_window: estimation.context_window,
        usage_percent: estimation.usage_percent,
    });

    // Apply compression if needed
    let mut messages = if estimation.needs_compression {
        apply_compression_if_needed(messages, &estimation, &event_tx)
    } else {
        messages
    };

    let resolved = resolve_provider(&model_name)?;

    // Determine system prompt based on provider
    let effective_system_prompt = match &resolved {
        ResolvedProvider::Claude { .. } => CLAUDE_CODE_INSTRUCTIONS.to_string(),
        ResolvedProvider::ChatGpt { .. } => system_prompt.clone(),
        ResolvedProvider::OpenAICompatible { .. } => system_prompt.clone(),
    };

    // For Claude, prepend the full system prompt to the first user message
    messages = match &resolved {
        ResolvedProvider::Claude { .. } => {
            let mut msgs = messages;
            prepend_system_to_first_user_message(&system_prompt, &mut msgs);
            msgs
        }
        ResolvedProvider::ChatGpt { .. } => messages,
        ResolvedProvider::OpenAICompatible { .. } => messages,
    };

    // Add system prompt as first message
    let mut sys_req = ModelRequest::new();
    sys_req.add_system_prompt(&effective_system_prompt);
    messages.insert(0, sys_req);

    let settings = ModelSettings::new().temperature(0.7).max_tokens(8192);
    
    // Create params with tools
    let params = ModelRequestParameters::new()
        .with_tools_arc(tool_defs_arc.clone())
        .with_allow_text(true);

    // Tool execution loop
    let mut tool_round = 0;
    loop {
        tool_round += 1;
        if tool_round > max_tool_rounds as usize {
            tracing::warn!("Max tool rounds ({}) reached, stopping", max_tool_rounds);
            break;
        }
        
        tracing::debug!("Tool round {} starting", tool_round);
        
        // Run streaming and collect response
        let response = run_streaming_with_response(
            &resolved,
            &messages,
            &settings,
            &params,
            &event_tx,
        ).await?;
        
        // Extract tool calls from response
        let tool_calls: Vec<ToolCallPart> = response.parts.iter()
            .filter_map(|part| {
                if let ModelResponsePart::ToolCall(tc) = part {
                    Some(tc.clone())
                } else {
                    None
                }
            })
            .collect();
        
        if tool_calls.is_empty() {
            // No tool calls - we're done
            tracing::debug!("No tool calls in response, finishing");
            break;
        }
        
        tracing::info!("Executing {} tool calls", tool_calls.len());
        
        // Add model response to messages
        let mut response_req = ModelRequest::new();
        response_req.parts.push(ModelRequestPart::ModelResponse(Box::new(response.clone())));
        messages.push(response_req);
        
        // Execute tools and collect results
        let mut tool_returns: Vec<ModelRequestPart> = Vec::new();
        
        for tc in tool_calls {
            let tool_name = &tc.tool_name;
            // Ensure we always have a tool_call_id (required by OpenAI-compatible APIs)
            let tool_call_id = tc.tool_call_id.clone()
                .unwrap_or_else(|| format!("call_{}", tool_name));
            
            // Emit tool execution event
            let _ = event_tx.send(RunnerEvent::ToolExecution {
                name: tool_name.clone(),
                args: tc.args.to_json_string().unwrap_or_default(),
            });
            
            // Get the tool from the map
            let tool = match tools_map.get(tool_name) {
                Some(t) => t,
                None => {
                    let error_msg = format!("Error: Unknown tool '{}'", tool_name);
                    let _ = event_tx.send(RunnerEvent::ToolResult {
                        name: tool_name.clone(),
                        success: false,
                        result: error_msg.clone(),
                    });
                    // Must use ToolReturnPart with tool_call_id for OpenAI compatibility
                    let tool_return = ToolReturnPart::error(tool_name, error_msg)
                        .with_tool_call_id(tool_call_id);
                    tool_returns.push(ModelRequestPart::ToolReturn(tool_return));
                    continue;
                }
            };
            
            // Execute the tool
            let args_json = tc.args.to_json();
            match toolset.call_tool(tool_name, args_json, &run_context, tool).await {
                Ok(result) => {
                    let result_str = result.content.to_string_content();
                    let _ = event_tx.send(RunnerEvent::ToolResult {
                        name: tool_name.clone(),
                        success: true,
                        result: if result_str.len() > 500 { 
                            format!("{}...", &result_str[..500]) 
                        } else { 
                            result_str.clone() 
                        },
                    });
                    
                    let tool_return = ToolReturnPart::new(tool_name, result.content)
                        .with_tool_call_id(tool_call_id);
                    tool_returns.push(ModelRequestPart::ToolReturn(tool_return));
                }
                Err(e) => {
                    let error_msg = format!("Error: {}", e);
                    let _ = event_tx.send(RunnerEvent::ToolResult {
                        name: tool_name.clone(),
                        success: false,
                        result: error_msg.clone(),
                    });
                    // Must use ToolReturnPart with tool_call_id for OpenAI compatibility
                    let tool_return = ToolReturnPart::error(tool_name, error_msg)
                        .with_tool_call_id(tool_call_id);
                    tool_returns.push(ModelRequestPart::ToolReturn(tool_return));
                }
            }
        }
        
        // Add tool returns to messages
        let mut returns_req = ModelRequest::new();
        returns_req.parts = tool_returns;
        messages.push(returns_req);
        
        // Continue the loop for another round
    }
    
    Ok(())
}

/// Run streaming and return the full response (for tool loop).
async fn run_streaming_with_response(
    resolved: &ResolvedProvider,
    messages: &[ModelRequest],
    settings: &ModelSettings,
    params: &ModelRequestParameters,
    event_tx: &mpsc::UnboundedSender<RunnerEvent>,
) -> Result<ModelResponse, String> {
    use serdes_ai_core::messages::{ModelResponseStreamEvent, ModelResponsePartDelta, TextPart};
    
    let mut accumulated_text = String::new();
    let mut accumulated_tool_calls: Vec<ToolCallPart> = Vec::new();
    let mut accumulated_thinking: Vec<ThinkingPart> = Vec::new();
    // Track in-progress parts by index
    let mut tool_call_builders: HashMap<usize, ToolCallPart> = HashMap::new();
    let mut thinking_builders: HashMap<usize, ThinkingPart> = HashMap::new();
    let mut stats_window_start = Instant::now();
    let mut stats_window_chars: usize = 0;
    
    // Helper to emit stats
    let emit_stats = |chars: &mut usize, start: &mut Instant| -> Option<RunnerEvent> {
        if *chars == 0 { return None; }
        let elapsed = start.elapsed();
        let window_ms = elapsed.as_millis() as u64;
        if window_ms < STATS_WINDOW_MIN_MS { return None; }
        let c = *chars;
        *chars = 0;
        *start = Instant::now();
        Some(RunnerEvent::StreamStats { chars_in_window: c, window_ms })
    };

    match resolved {
        ResolvedProvider::Claude { model, token, .. } => {
            let mut stream = model.request_stream(messages, settings, params).await
                .map_err(|e| {
                    let msg = format!("Claude stream error: {}", e);
                    if is_rate_limit_error(&msg) {
                        auth::mark_cooldown(&token.account_id, &msg, DEFAULT_COOLDOWN_SECS);
                    }
                    msg
                })?;
            
            while let Some(event_result) = stream.next().await {
                match event_result {
                    Ok(event) => {
                        match event {
                            ModelResponseStreamEvent::PartStart(start_event) => {
                                // Handle part starts
                                match start_event.part {
                                    ModelResponsePart::ToolCall(tc) => {
                                        tracing::info!("🔧 Tool call starting: {} (index: {})", tc.tool_name, start_event.index);
                                        tool_call_builders.insert(start_event.index, tc);
                                    }
                                    ModelResponsePart::Thinking(tp) => {
                                        thinking_builders.insert(start_event.index, tp);
                                    }
                                    _ => {}
                                }
                            }
                            ModelResponseStreamEvent::PartDelta(delta_event) => {
                                match &delta_event.delta {
                                    ModelResponsePartDelta::Text(text_delta) => {
                                        let text = &text_delta.content_delta;
                                        if !text.is_empty() {
                                            accumulated_text.push_str(text);
                                            let _ = event_tx.send(RunnerEvent::StreamChunk(text.clone()));
                                            stats_window_chars += text.len();
                                            if let Some(stats) = emit_stats(&mut stats_window_chars, &mut stats_window_start) {
                                                let _ = event_tx.send(stats);
                                            }
                                        }
                                    }
                                    ModelResponsePartDelta::Thinking(thinking_delta) => {
                                        // Apply delta to accumulate both content and signature
                                        if let Some(tp) = thinking_builders.get_mut(&delta_event.index) {
                                            thinking_delta.apply(tp);
                                        }
                                        // Emit event for UI
                                        if !thinking_delta.content_delta.is_empty() {
                                            let _ = event_tx.send(RunnerEvent::Reasoning {
                                                text: thinking_delta.content_delta.clone(),
                                                signature: thinking_delta.signature_delta.clone(),
                                            });
                                        }
                                    }
                                    ModelResponsePartDelta::ToolCall(tc_delta) => {
                                        // Apply delta to the tool call at this index
                                        if let Some(tc) = tool_call_builders.get_mut(&delta_event.index) {
                                            tc_delta.apply(tc);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            ModelResponseStreamEvent::PartEnd(end_event) => {
                                tracing::debug!("PartEnd received for index: {}", end_event.index);
                                // Finalize parts
                                if let Some(tc) = tool_call_builders.remove(&end_event.index) {
                                    tracing::info!("🔧 Tool call detected: {} with args: {}", tc.tool_name, tc.args.to_json_string().unwrap_or_default());
                                    // Emit tool call event for UI display
                                    let _ = event_tx.send(RunnerEvent::ToolCall {
                                        name: tc.tool_name.clone(),
                                        args: tc.args.to_json_string().unwrap_or_default(),
                                    });
                                    accumulated_tool_calls.push(tc);
                                }
                                if let Some(tp) = thinking_builders.remove(&end_event.index) {
                                    accumulated_thinking.push(tp);
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        let msg = format!("Claude stream error: {}", e);
                        if is_rate_limit_error(&msg) {
                            auth::mark_cooldown(&token.account_id, &msg, DEFAULT_COOLDOWN_SECS);
                        }
                        return Err(msg);
                    }
                }
            }
        }
        ResolvedProvider::ChatGpt { model, token, .. } => {
            let mut stream = model.request_stream(messages, settings, params).await
                .map_err(|e| {
                    let msg = format!("ChatGPT stream error: {}", e);
                    if is_rate_limit_error(&msg) {
                        auth::mark_cooldown(&token.account_id, &msg, DEFAULT_COOLDOWN_SECS);
                    }
                    msg
                })?;
            
            while let Some(event_result) = stream.next().await {
                match event_result {
                    Ok(event) => {
                        match event {
                            ModelResponseStreamEvent::PartStart(start_event) => {
                                match start_event.part {
                                    ModelResponsePart::ToolCall(tc) => {
                                        tracing::info!("🔧 Tool call starting: {} (index: {})", tc.tool_name, start_event.index);
                                        tool_call_builders.insert(start_event.index, tc);
                                    }
                                    ModelResponsePart::Thinking(tp) => {
                                        thinking_builders.insert(start_event.index, tp);
                                    }
                                    _ => {}
                                }
                            }
                            ModelResponseStreamEvent::PartDelta(delta_event) => {
                                match &delta_event.delta {
                                    ModelResponsePartDelta::Text(text_delta) => {
                                        let text = &text_delta.content_delta;
                                        if !text.is_empty() {
                                            accumulated_text.push_str(text);
                                            let _ = event_tx.send(RunnerEvent::StreamChunk(text.clone()));
                                            stats_window_chars += text.len();
                                            if let Some(stats) = emit_stats(&mut stats_window_chars, &mut stats_window_start) {
                                                let _ = event_tx.send(stats);
                                            }
                                        }
                                    }
                                    ModelResponsePartDelta::Thinking(thinking_delta) => {
                                        if let Some(tp) = thinking_builders.get_mut(&delta_event.index) {
                                            thinking_delta.apply(tp);
                                        }
                                        if !thinking_delta.content_delta.is_empty() {
                                            let _ = event_tx.send(RunnerEvent::Reasoning {
                                                text: thinking_delta.content_delta.clone(),
                                                signature: thinking_delta.signature_delta.clone(),
                                            });
                                        }
                                    }
                                    ModelResponsePartDelta::ToolCall(tc_delta) => {
                                        if let Some(tc) = tool_call_builders.get_mut(&delta_event.index) {
                                            tc_delta.apply(tc);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            ModelResponseStreamEvent::PartEnd(end_event) => {
                                tracing::debug!("PartEnd received for index: {}", end_event.index);
                                if let Some(tc) = tool_call_builders.remove(&end_event.index) {
                                    tracing::info!("🔧 Tool call detected: {} with args: {}", tc.tool_name, tc.args.to_json_string().unwrap_or_default());
                                    // Emit tool call event for UI display
                                    let _ = event_tx.send(RunnerEvent::ToolCall {
                                        name: tc.tool_name.clone(),
                                        args: tc.args.to_json_string().unwrap_or_default(),
                                    });
                                    accumulated_tool_calls.push(tc);
                                }
                                if let Some(tp) = thinking_builders.remove(&end_event.index) {
                                    accumulated_thinking.push(tp);
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        let msg = format!("ChatGPT stream error: {}", e);
                        if is_rate_limit_error(&msg) {
                            auth::mark_cooldown(&token.account_id, &msg, DEFAULT_COOLDOWN_SECS);
                        }
                        return Err(msg);
                    }
                }
            }
        }
        ResolvedProvider::OpenAICompatible { model, api_key_token, provider_id, .. } => {
            let mut stream = model.request_stream(messages, settings, params).await
                .map_err(|e| {
                    let msg = format!("{} stream error: {}", provider_id, e);
                    if is_rate_limit_error(&msg) {
                        auth::mark_api_key_cooldown(&api_key_token.account_id, &msg, DEFAULT_COOLDOWN_SECS);
                    }
                    msg
                })?;
            
            while let Some(event_result) = stream.next().await {
                match event_result {
                    Ok(event) => {
                        match event {
                            ModelResponseStreamEvent::PartStart(start_event) => {
                                match start_event.part {
                                    ModelResponsePart::ToolCall(tc) => {
                                        tracing::info!("🔧 Tool call starting: {} (index: {})", tc.tool_name, start_event.index);
                                        tool_call_builders.insert(start_event.index, tc);
                                    }
                                    ModelResponsePart::Thinking(tp) => {
                                        thinking_builders.insert(start_event.index, tp);
                                    }
                                    _ => {}
                                }
                            }
                            ModelResponseStreamEvent::PartDelta(delta_event) => {
                                match &delta_event.delta {
                                    ModelResponsePartDelta::Text(text_delta) => {
                                        let text = &text_delta.content_delta;
                                        if !text.is_empty() {
                                            accumulated_text.push_str(text);
                                            let _ = event_tx.send(RunnerEvent::StreamChunk(text.clone()));
                                            stats_window_chars += text.len();
                                            if let Some(stats) = emit_stats(&mut stats_window_chars, &mut stats_window_start) {
                                                let _ = event_tx.send(stats);
                                            }
                                        }
                                    }
                                    ModelResponsePartDelta::Thinking(thinking_delta) => {
                                        if let Some(tp) = thinking_builders.get_mut(&delta_event.index) {
                                            thinking_delta.apply(tp);
                                        }
                                        if !thinking_delta.content_delta.is_empty() {
                                            let _ = event_tx.send(RunnerEvent::Reasoning {
                                                text: thinking_delta.content_delta.clone(),
                                                signature: thinking_delta.signature_delta.clone(),
                                            });
                                        }
                                    }
                                    ModelResponsePartDelta::ToolCall(tc_delta) => {
                                        if let Some(tc) = tool_call_builders.get_mut(&delta_event.index) {
                                            tc_delta.apply(tc);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            ModelResponseStreamEvent::PartEnd(end_event) => {
                                tracing::debug!("PartEnd received for index: {}", end_event.index);
                                if let Some(tc) = tool_call_builders.remove(&end_event.index) {
                                    tracing::info!("🔧 Tool call detected: {} with args: {}", tc.tool_name, tc.args.to_json_string().unwrap_or_default());
                                    // Emit tool call event for UI display
                                    let _ = event_tx.send(RunnerEvent::ToolCall {
                                        name: tc.tool_name.clone(),
                                        args: tc.args.to_json_string().unwrap_or_default(),
                                    });
                                    accumulated_tool_calls.push(tc);
                                }
                                if let Some(tp) = thinking_builders.remove(&end_event.index) {
                                    accumulated_thinking.push(tp);
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        let msg = format!("{} stream error: {}", provider_id, e);
                        if is_rate_limit_error(&msg) {
                            auth::mark_api_key_cooldown(&api_key_token.account_id, &msg, DEFAULT_COOLDOWN_SECS);
                        }
                        return Err(msg);
                    }
                }
            }
        }
    }
    
    // Finalize any remaining parts (in case PartEnd wasn't received)
    for (_, tc) in tool_call_builders {
        accumulated_tool_calls.push(tc);
    }
    for (_, tp) in thinking_builders {
        accumulated_thinking.push(tp);
    }
    
    // Build the final response
    let mut parts: Vec<ModelResponsePart> = Vec::new();
    
    // Add thinking parts FIRST (important for message ordering per Claude docs)
    for tp in accumulated_thinking {
        parts.push(ModelResponsePart::Thinking(tp));
    }
    
    if !accumulated_text.is_empty() {
        parts.push(ModelResponsePart::Text(TextPart::new(accumulated_text)));
    }
    
    for tc in accumulated_tool_calls {
        parts.push(ModelResponsePart::ToolCall(tc));
    }
    
    Ok(ModelResponse {
        parts,
        model_name: Some(resolved.model_id().to_string()),
        timestamp: chrono::Utc::now(),
        finish_reason: None,
        usage: None,
        vendor_id: None,
        vendor_details: None,
        kind: "response".to_string(),
    })
}

/// Streaming loop for Claude model.
#[allow(clippy::too_many_arguments)]
async fn run_claude_stream<F>(
    model: serdes_ai_models::claude_code_oauth::ClaudeCodeOAuthModel,
    messages: &[ModelRequest],
    settings: &ModelSettings,
    params: &ModelRequestParameters,
    event_tx: &mpsc::UnboundedSender<RunnerEvent>,
    stats_window_chars: &mut usize,
    stats_window_start: &mut Instant,
    emit_stream_stats: &F,
    token: Option<&auth::AuthToken>,
) -> Result<(), String>
where
    F: Fn(&mut usize, &mut Instant) -> Option<RunnerEvent>,
{
    use serdes_ai_core::messages::{ModelResponseStreamEvent, ModelResponsePartDelta};

    let mut stream = match model.request_stream(messages, settings, params).await {
        Ok(s) => s,
        Err(e) => {
            let error_msg = format!("Claude stream error: {}", e);
            if is_rate_limit_error(&error_msg) {
                if let Some(t) = token {
                    auth::mark_cooldown(&t.account_id, &error_msg, DEFAULT_COOLDOWN_SECS);
                }
            }
            return Err(error_msg);
        }
    };

    while let Some(event_result) = stream.next().await {
        match event_result {
            Ok(event) => {
                // Handle delta events which contain the streaming content
                if let ModelResponseStreamEvent::PartDelta(delta_event) = event {
                    match &delta_event.delta {
                        ModelResponsePartDelta::Text(text_delta) => {
                            let text = &text_delta.content_delta;
                            if !text.is_empty() {
                                let chunk_len = text.len();
                                let _ = event_tx.send(RunnerEvent::StreamChunk(text.clone()));
                                *stats_window_chars += chunk_len;
                                if let Some(stats) = emit_stream_stats(stats_window_chars, stats_window_start) {
                                    let _ = event_tx.send(stats);
                                }
                            }
                        }
                        ModelResponsePartDelta::Thinking(thinking_delta) => {
                            let thinking = &thinking_delta.content_delta;
                            if !thinking.is_empty() {
                                let _ = event_tx.send(RunnerEvent::Reasoning {
                                    text: thinking.clone(),
                                    signature: None,
                                });
                            }
                        }
                        _ => {} // Ignore tool call deltas for now
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Claude stream error: {}", e);
                if is_rate_limit_error(&error_msg) {
                    if let Some(t) = token {
                        auth::mark_cooldown(&t.account_id, &error_msg, DEFAULT_COOLDOWN_SECS);
                    }
                }
                return Err(error_msg);
            }
        }
    }

    Ok(())
}

/// Streaming loop for ChatGPT model.
#[allow(clippy::too_many_arguments)]
async fn run_chatgpt_stream<F>(
    model: serdes_ai_models::chatgpt_oauth::ChatGptOAuthModel,
    messages: &[ModelRequest],
    settings: &ModelSettings,
    params: &ModelRequestParameters,
    event_tx: &mpsc::UnboundedSender<RunnerEvent>,
    stats_window_chars: &mut usize,
    stats_window_start: &mut Instant,
    emit_stream_stats: &F,
    token: Option<&auth::AuthToken>,
) -> Result<(), String>
where
    F: Fn(&mut usize, &mut Instant) -> Option<RunnerEvent>,
{
    use serdes_ai_core::messages::{ModelResponseStreamEvent, ModelResponsePartDelta};

    let mut stream = match model.request_stream(messages, settings, params).await {
        Ok(s) => s,
        Err(e) => {
            let error_msg = format!("ChatGPT stream error: {}", e);
            if is_rate_limit_error(&error_msg) {
                if let Some(t) = token {
                    auth::mark_cooldown(&t.account_id, &error_msg, DEFAULT_COOLDOWN_SECS);
                }
            }
            return Err(error_msg);
        }
    };

    while let Some(event_result) = stream.next().await {
        match event_result {
            Ok(event) => {
                if let ModelResponseStreamEvent::PartDelta(delta_event) = event {
                    match &delta_event.delta {
                        ModelResponsePartDelta::Text(text_delta) => {
                            let text = &text_delta.content_delta;
                            if !text.is_empty() {
                                let chunk_len = text.len();
                                let _ = event_tx.send(RunnerEvent::StreamChunk(text.clone()));
                                *stats_window_chars += chunk_len;
                                if let Some(stats) = emit_stream_stats(stats_window_chars, stats_window_start) {
                                    let _ = event_tx.send(stats);
                                }
                            }
                        }
                        ModelResponsePartDelta::Thinking(thinking_delta) => {
                            let thinking = &thinking_delta.content_delta;
                            if !thinking.is_empty() {
                                let _ = event_tx.send(RunnerEvent::Reasoning {
                                    text: thinking.clone(),
                                    signature: None,
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("ChatGPT stream error: {}", e);
                if is_rate_limit_error(&error_msg) {
                    if let Some(t) = token {
                        auth::mark_cooldown(&t.account_id, &error_msg, DEFAULT_COOLDOWN_SECS);
                    }
                }
                return Err(error_msg);
            }
        }
    }

    Ok(())
}

/// Streaming loop for OpenAI-compatible API key providers.
#[allow(clippy::too_many_arguments)]
async fn run_openai_compatible_stream<F>(
    model: serdes_ai_models::openai::OpenAIChatModel,
    messages: &[ModelRequest],
    settings: &ModelSettings,
    params: &ModelRequestParameters,
    event_tx: &mpsc::UnboundedSender<RunnerEvent>,
    stats_window_chars: &mut usize,
    stats_window_start: &mut Instant,
    emit_stream_stats: &F,
    api_key_token: &auth::ApiKeyToken,
    provider_id: &str,
) -> Result<(), String>
where
    F: Fn(&mut usize, &mut Instant) -> Option<RunnerEvent>,
{
    use serdes_ai_core::messages::{ModelResponseStreamEvent, ModelResponsePartDelta};

    let mut stream = match model.request_stream(messages, settings, params).await {
        Ok(s) => s,
        Err(e) => {
            let error_msg = format!("{} stream error: {}", provider_id, e);
            if is_rate_limit_error(&error_msg) {
                auth::mark_api_key_cooldown(&api_key_token.account_id, &error_msg, DEFAULT_COOLDOWN_SECS);
            }
            return Err(error_msg);
        }
    };

    while let Some(event_result) = stream.next().await {
        match event_result {
            Ok(event) => {
                if let ModelResponseStreamEvent::PartDelta(delta_event) = event {
                    match &delta_event.delta {
                        ModelResponsePartDelta::Text(text_delta) => {
                            let text = &text_delta.content_delta;
                            if !text.is_empty() {
                                let chunk_len = text.len();
                                let _ = event_tx.send(RunnerEvent::StreamChunk(text.clone()));
                                *stats_window_chars += chunk_len;
                                if let Some(stats) = emit_stream_stats(stats_window_chars, stats_window_start) {
                                    let _ = event_tx.send(stats);
                                }
                            }
                        }
                        ModelResponsePartDelta::Thinking(thinking_delta) => {
                            let thinking = &thinking_delta.content_delta;
                            if !thinking.is_empty() {
                                let _ = event_tx.send(RunnerEvent::Reasoning {
                                    text: thinking.clone(),
                                    signature: None,
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("{} stream error: {}", provider_id, e);
                if is_rate_limit_error(&error_msg) {
                    auth::mark_api_key_cooldown(&api_key_token.account_id, &error_msg, DEFAULT_COOLDOWN_SECS);
                }
                return Err(error_msg);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_detection() {
        assert!(is_rate_limit_error("Error 429: Too many requests"));
        assert!(is_rate_limit_error("Rate limit exceeded"));
        assert!(is_rate_limit_error("quota exceeded"));
        assert!(!is_rate_limit_error("Internal server error"));
    }
}
