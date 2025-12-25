//! Agent execution + LLM streaming runner.
//!
//! This module contains the provider/tool streaming loop previously implemented in the app
//! crate, but rewritten to emit core events so the UI can remain a thin adapter.

#![allow(clippy::items_after_test_module)]

use crate::agents::{AgentProfile, AgentType};
use crate::compression::compress_messages;
use crate::config::models::providers;
use crate::config::{CompressionSettings, ConfigDatabase, McpServer, McpTransport, setting_keys};
use crate::llm;
use crate::llm::auth::{self, AuthToken};
use crate::llm::providers::GeminiCodeAssistRigClient;
use crate::llm::providers::chatgpt::ChatGptOAuthClient;
use crate::llm::providers::OpenAICompatibleApiClient;
use crate::llm::{ClaudeOAuthClient, ProviderId, ProviderRegistry};
use crate::registry::RegistryService;
use crate::session::MessageRole;
use crate::tools::{
    AgentCallEvent, AgentInvokeRequest, AgentInvoker, AgentStreamEvent, TodoListEvent,
    TodoListState, TodoStore, ToolApprovalDecision, ToolApprovalGate, ToolApprovalRequest,
    ToolContext, ToolPolicy,
};

use futures::StreamExt;
use rig::agent::AgentBuilder;
use rig::completion::GetTokenUsage;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::time::Duration;

const DEFAULT_COOLDOWN_SECS: i64 = 60;
const CLAUDE_CODE_INSTRUCTIONS: &str = "You are Claude Code, Anthropic's official CLI for Claude.";
const TODO_GUARD_FIRST_PROMPT: &str = r#"You must not finish until your To Do list is confirmed complete.

Use the `todo_write` tool now with ONE of these options:

Option A - Mark all items complete:
```json
{ "items": [...], "mark_all_complete": true }
```

Option B - Explicit status for each item:
```json
{
  "items": [{ "text": "Item 1", "status": "completed" }, ...],
  "confirmed_complete": true
}
```

Option C - If your list is empty:
```json
{ "items": [], "confirmed_complete": true }
```

If there is remaining work, add/update items and continue working instead of finishing."#;

const TODO_GUARD_RETRY_PROMPT: &str = r#"Your To Do list is STILL not confirmed complete. This is your final attempt.

REQUIRED: Call `todo_write` with `mark_all_complete: true` to complete your session:
```json
{ "items": [...your items...], "mark_all_complete": true }
```

Or if empty: `{ "items": [], "confirmed_complete": true }`"#;
const TODO_GUARD_MAX_PASSES: usize = 4;
const MCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct ChatHistoryMessage {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone)]
pub enum RunnerEvent {
    StreamChunk(String),
    Reasoning(String),
    StreamStats {
        chars_in_window: usize,
        window_ms: u64,
    },
    ToolCall {
        name: String,
        args: String,
    },
    AgentCall(AgentCallEvent),
    SubagentStream(AgentStreamEvent),
    TodoEvent(TodoListEvent),
    ToolApprovalRequested {
        id: u64,
        name: String,
        args: String,
    },
    /// Token usage from the API response (input includes system prompt, tools, messages)
    Usage {
        input_tokens: u64,
        output_tokens: u64,
    },
    /// Pre-request context estimate (calculated before sending to LLM)
    /// Includes system prompt, tool definitions, and all messages
    ContextEstimate {
        /// Tokens used by the system prompt
        system_prompt_tokens: usize,
        /// Tokens used by tool definitions
        tool_definitions_tokens: usize,
        /// Tokens used by messages (user, assistant, tool calls, reasoning)
        messages_tokens: usize,
        /// Total estimated tokens
        total_tokens: usize,
        /// Model's context window size
        context_window: u64,
        /// Percentage of context used
        usage_percent: u32,
    },
    /// Context compression was applied
    ContextCompressed {
        /// Messages before compression
        original_messages: usize,
        /// Messages after compression
        compressed_messages: usize,
        /// Estimated tokens before compression
        original_tokens: usize,
        /// Estimated tokens after compression
        compressed_tokens: usize,
        /// Strategy used
        strategy: String,
    },
    /// Context usage warning (approaching limit)
    ContextUsageWarning {
        /// Current estimated tokens
        current_tokens: usize,
        /// Threshold tokens (when compression triggers)
        threshold_tokens: u64,
        /// Context window size
        context_window: u64,
        /// Percentage used
        usage_percent: u32,
    },
    StreamComplete,
    StreamStopped,
    StreamError(String),
}

/// Fetch the best available model from the Claude API.
pub async fn fetch_best_model(auth_token: &AuthToken) -> Result<String, String> {
    let client = llm::ClaudeClient::new(auth_token.access_token.to_string());
    let models = client
        .fetch_latest_models()
        .await
        .map_err(|e| format!("Failed to fetch models: {}", e))?;

    // Prefer sonnet, then opus, then haiku
    let preferred_order = ["sonnet", "opus", "haiku"];

    for family in preferred_order {
        if let Some(model) = models.iter().find(|m| m.contains(family)) {
            return Ok(model.clone());
        }
    }

    // If no match, return the first available model or error
    models
        .into_iter()
        .next()
        .ok_or_else(|| "No models available from Claude API".to_string())
}

async fn resolve_model_name(model_name: Option<String>) -> Result<String, String> {
    match model_name {
        Some(name) => {
            tracing::info!("Using configured model: {}", name);
            Ok(name)
        }
        None => {
            let claude_token = auth::select_token(providers::CLAUDE).ok_or_else(|| {
                "Claude authentication required to auto-select a model".to_string()
            })?;
            match fetch_best_model(&claude_token).await {
                Ok(name) => {
                    tracing::info!("Using auto-detected model: {}", name);
                    Ok(name)
                }
                Err(e) => Err(e),
            }
        }
    }
}

/// Build chat history from stored messages.
fn build_chat_history(chat_history: Vec<ChatHistoryMessage>) -> Vec<rig::message::Message> {
    use rig::message::Message as RigMessage;

    chat_history
        .into_iter()
        .filter_map(|msg| match msg.role {
            MessageRole::User => Some(RigMessage::user(&msg.content)),
            MessageRole::Assistant => Some(RigMessage::assistant(&msg.content)),
            _ => None,
        })
        .collect()
}

/// Build user message with optional images.
fn build_user_message(
    user_message: &str,
    image_data: Vec<(String, String)>,
) -> rig::message::Message {
    use rig::message::Message as RigMessage;
    use rig::message::{ImageMediaType, UserContent};
    use rig::one_or_many::OneOrMany;

    if image_data.is_empty() {
        RigMessage::user(user_message)
    } else {
        let mut content_parts: Vec<UserContent> = Vec::new();

        for (media_type, base64_data) in image_data {
            let media = match media_type.as_str() {
                "image/png" => ImageMediaType::PNG,
                "image/jpeg" => ImageMediaType::JPEG,
                "image/gif" => ImageMediaType::GIF,
                "image/webp" => ImageMediaType::WEBP,
                _ => ImageMediaType::PNG,
            };
            content_parts.push(UserContent::image_base64(base64_data, Some(media), None));
        }

        if !user_message.is_empty() {
            content_parts.push(UserContent::text(user_message));
        }

        match OneOrMany::many(content_parts) {
            Ok(content) => RigMessage::User { content },
            Err(_) => RigMessage::user(user_message),
        }
    }
}

fn is_rate_limit_error(message: &str) -> bool {
    let lowered = message.to_lowercase();
    lowered.contains("429")
        || lowered.contains("rate limit")
        || lowered.contains("too many requests")
        || lowered.contains("quota")
}

fn prepend_system_to_first_user_message(
    system_prompt: &str,
    history: &mut [rig::message::Message],
) {
    if system_prompt.is_empty() {
        return;
    }

    for msg in history.iter_mut() {
        if let rig::message::Message::User { content } = msg {
            let first_content = content.first_mut();
            if let rig::message::UserContent::Text(text_content) = first_content {
                text_content.text = format!("{}\n\n{}", system_prompt, text_content.text);
            }
            break;
        }
    }
}

fn list_active_mcp_servers(agent: AgentType) -> Vec<McpServer> {
    let Ok(db) = ConfigDatabase::open() else {
        return Vec::new();
    };

    let ids = db
        .get_agent_mcp_server_ids(agent.as_str())
        .unwrap_or_default();
    if ids.is_empty() {
        return Vec::new();
    }

    let servers = db.list_mcp_servers().unwrap_or_default();
    servers
        .into_iter()
        .filter(|s| s.is_enabled && ids.contains(&s.id))
        .collect()
}

fn rmcp_client_info() -> rmcp::model::ClientInfo {
    use rmcp::model::{ClientCapabilities, ClientInfo, Implementation};
    ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "ticca-desktop".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            ..Default::default()
        },
    }
}

async fn connect_mcp_server(
    server: &McpServer,
) -> anyhow::Result<(Vec<rmcp::model::Tool>, rmcp::service::ServerSink)> {
    use rmcp::ServiceExt;

    match server.transport {
        McpTransport::StreamableHttp => {
            let url = server
                .endpoint_url
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Missing endpoint_url for HTTP MCP server"))?;

            let transport = rmcp::transport::StreamableHttpClientTransport::from_uri(url);
            let client = rmcp_client_info().serve(transport).await?;

            let tools = client.list_tools(Default::default()).await?.tools;
            Ok((tools, client.peer().to_owned()))
        }
        McpTransport::Stdio => {
            let command = server
                .command
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Missing command for stdio MCP server"))?;

            let mut cmd = tokio::process::Command::new(command);
            cmd.args(&server.args);
            cmd.envs(server.env.clone());

            let transport = rmcp::transport::TokioChildProcess::new(cmd)
                .map_err(|e| anyhow::anyhow!("Failed to spawn MCP server '{}': {}", command, e))?;

            let client = rmcp_client_info().serve(transport).await?;
            let tools = client.list_tools(Default::default()).await?.tools;
            Ok((tools, client.peer().to_owned()))
        }
    }
}

async fn attach_mcp_tools_to_builder<M>(
    mut builder: rig::agent::AgentBuilderSimple<M>,
    agent: AgentType,
) -> rig::agent::AgentBuilderSimple<M>
where
    M: rig::completion::CompletionModel + 'static,
{
    let servers = list_active_mcp_servers(agent);
    if servers.is_empty() {
        return builder;
    }

    for server in servers {
        let res = tokio::time::timeout(MCP_CONNECT_TIMEOUT, connect_mcp_server(&server)).await;
        match res {
            Ok(Ok((tools, sink))) => {
                if tools.is_empty() {
                    tracing::info!("MCP server '{}' has no tools", server.name);
                    continue;
                }
                tracing::info!("Loaded {} MCP tools from '{}'", tools.len(), server.name);
                builder = builder.rmcp_tools(tools, sink);
            }
            Ok(Err(e)) => {
                tracing::warn!("Failed to connect to MCP server '{}': {}", server.name, e);
            }
            Err(_) => {
                tracing::warn!(
                    "Timed out connecting to MCP server '{}' ({}s)",
                    server.name,
                    MCP_CONNECT_TIMEOUT.as_secs()
                );
            }
        }
    }

    builder
}

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
    initial_todo_state: Option<TodoListState>,
    image_data: Vec<(String, String)>,
    yolo_mode_enabled: bool,
    current_agent: AgentType,
    mut approval_decision_rx: mpsc::UnboundedReceiver<ToolApprovalDecision>,
    mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
    system_exec_store: std::sync::Arc<crate::tools::SystemExecStore>,
    system_exec_tx: mpsc::UnboundedSender<crate::tools::SystemExecRequest>,
) -> impl futures::Stream<Item = RunnerEvent> {
    async_stream::stream! {
        // Log context size for debugging token usage
        let history_chars: usize = chat_history.iter().map(|m| m.content.len()).sum();
        let prompt_chars = system_prompt.len();
        let user_chars = user_message.len();
        tracing::info!(
            "Agent request: history={} msgs ({} chars), prompt={} chars, user={} chars",
            chat_history.len(),
            history_chars,
            prompt_chars,
            user_chars
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
        let (todo_tx, mut todo_rx) = mpsc::unbounded_channel::<TodoListEvent>();
        let call_graph_counter = Arc::new(AtomicUsize::new(1));
        let todo_store = Arc::new(TodoStore::new());

        let event_tx_for_graph = event_tx.clone();
        tokio::spawn(async move {
            while let Some(event) = call_graph_rx.recv().await {
                let _ = event_tx_for_graph.send(RunnerEvent::AgentCall(event));
            }
        });

        let event_tx_for_stream = event_tx.clone();
        tokio::spawn(async move {
            while let Some(event) = agent_stream_rx.recv().await {
                let _ = event_tx_for_stream.send(RunnerEvent::SubagentStream(event));
            }
        });

        let event_tx_for_todos = event_tx.clone();
        tokio::spawn(async move {
            while let Some(event) = todo_rx.recv().await {
                let _ = event_tx_for_todos.send(RunnerEvent::TodoEvent(event));
            }
        });

        let agent_invoker: Arc<AgentInvoker> = Arc::new(|request: AgentInvokeRequest| {
            Box::pin(invoke_agent(request))
        });

        let initial_todo_state = match initial_todo_state {
            Some(state) => todo_store.set_node_state(0, state).await,
            None => todo_store.reset_node(0).await,
        };
        let _ = todo_tx.send(TodoListEvent::Reset {
            node_id: 0,
            state: initial_todo_state,
        });

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
            todo_store: Some(todo_store),
            todo_tx: Some(todo_tx),
            system_exec_store: Some(system_exec_store.clone()),
            system_exec_tx: Some(system_exec_tx.clone()),
            process_output_offsets: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        });

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

        let event_tx_for_worker = event_tx.clone();
        let tool_context_worker = tool_context.clone();

        tokio::spawn(async move {
            let mut worker = tokio::spawn(run_agent_stream(
                event_tx_for_worker.clone(),
                tool_context_worker,
                system_prompt,
                user_message,
                resolved_model.clone(),
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
        todo_store: parent_context.todo_store.clone(),
        todo_tx: parent_context.todo_tx.clone(),
        system_exec_store: parent_context.system_exec_store.clone(),
        system_exec_tx: parent_context.system_exec_tx.clone(),
        process_output_offsets: parent_context.process_output_offsets.clone(),
    });

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

    let result = match ProviderRegistry::resolve_provider(&model_name) {
        ProviderId::ChatGpt => {
            let token = auth::select_token(providers::CHATGPT).ok_or_else(|| {
                "ChatGPT authentication required. Please authenticate in Settings.".to_string()
            })?;
            let id_token = token.id_token.ok_or_else(|| {
                "ChatGPT id_token not found. Please re-authenticate in Settings.".to_string()
            })?;

            let client = ChatGptOAuthClient::from_tokens(&token.access_token, &id_token)
                .map_err(|e| format!("Failed to create ChatGPT client: {}", e))?;

            // Extract the actual model ID without the provider suffix
            let actual_model_id = ProviderRegistry::extract_model_id(&model_name);
            let model = client.completion_model(actual_model_id);
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
                invoke_agent_tool,
            ) = crate::tools::create_tools(tool_context);

            let builder = AgentBuilder::new(model)
                .preamble(&profile.system_prompt)
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
                .tool(invoke_agent_tool);

            let builder = attach_mcp_tools_to_builder(builder, request.agent_type).await;

            let agent = builder
                .temperature(0.7)
                .max_tokens(8192)
                .additional_params(ChatGptOAuthClient::codex_params())
                .build();

            stream_invoked_agent(
                request.node_id,
                &parent_context,
                agent,
                history,
                max_tool_rounds,
                "ChatGPT",
            )
            .await
        }
        ProviderId::Gemini => {
            let token = auth::select_token(providers::GEMINI).ok_or_else(|| {
                "Gemini authentication required. Please authenticate in Settings.".to_string()
            })?;

            let client = GeminiCodeAssistRigClient::new(token.access_token);

            // Extract the actual model ID without the provider suffix
            let actual_model_id = ProviderRegistry::extract_model_id(&model_name);
            let model = client.completion_model(actual_model_id);
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
                invoke_agent_tool,
            ) = crate::tools::create_tools(tool_context);

            let builder = AgentBuilder::new(model)
                .preamble(&profile.system_prompt)
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
                .tool(invoke_agent_tool);

            let builder = attach_mcp_tools_to_builder(builder, request.agent_type).await;

            let agent = builder.temperature(0.7).max_tokens(8192).build();

            stream_invoked_agent(
                request.node_id,
                &parent_context,
                agent,
                history,
                max_tool_rounds,
                "Gemini",
            )
            .await
        }
        ProviderId::Claude => {
            let token = auth::select_token(providers::CLAUDE).ok_or_else(|| {
                "Claude authentication required. Please authenticate in Settings.".to_string()
            })?;

            let client = ClaudeOAuthClient::new(token.access_token)
                .map_err(|e| format!("Failed to create Claude client: {}", e))?;

            // Extract the actual model ID without the provider suffix
            let actual_model_id = ProviderRegistry::extract_model_id(&model_name);
            let model = client.completion_model(actual_model_id);
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
                invoke_agent_tool,
            ) = crate::tools::create_tools(tool_context);

            let mut claude_history = history;
            prepend_system_to_first_user_message(&profile.system_prompt, &mut claude_history);

            let builder = AgentBuilder::new(model)
                .preamble(CLAUDE_CODE_INSTRUCTIONS)
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
                .tool(invoke_agent_tool);

            let builder = attach_mcp_tools_to_builder(builder, request.agent_type).await;

            let agent = builder.temperature(0.7).max_tokens(8192).build();

            stream_invoked_agent(
                request.node_id,
                &parent_context,
                agent,
                claude_history,
                max_tool_rounds,
                "Claude",
            )
            .await
        }
        ProviderId::ApiKey(provider_id) => {
            // Look up provider info from registry
            let provider_def = RegistryService::find_provider(&provider_id)
                .ok_or_else(|| format!("Unknown provider: {}", provider_id))?;

            let provider_name = &provider_def.name;
            tracing::info!("Using {} backend for model: {}", provider_name, model_name);

            if !provider_def.is_openai_compatible {
                return Err(format!(
                    "{} API key provider requires special handling not yet implemented",
                    provider_name
                ));
            }

            let api_key_token = auth::select_api_key(&provider_id).ok_or_else(|| {
                format!(
                    "{} API key required. Please add an API key in Settings.",
                    provider_name
                )
            })?;

            let client = OpenAICompatibleApiClient::new(&provider_id, &api_key_token.api_key)?;

            // Extract the actual model ID without the provider suffix
            let actual_model_id = ProviderRegistry::extract_model_id(&model_name);
            tracing::info!(
                "API key provider model: '{}' -> extracted: '{}'",
                model_name,
                actual_model_id
            );
            let model = client.completion_model(actual_model_id);
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
                invoke_agent_tool,
            ) = crate::tools::create_tools(tool_context);

            let builder = AgentBuilder::new(model)
                .preamble(&profile.system_prompt)
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
                .tool(invoke_agent_tool);

            let builder = attach_mcp_tools_to_builder(builder, request.agent_type).await;

            let agent = builder.temperature(0.7).max_tokens(8192).build();

            stream_invoked_agent(
                request.node_id,
                &parent_context,
                agent,
                history,
                max_tool_rounds,
                provider_name,
            )
            .await
        }
    };

    if let Some(tx) = &parent_context.agent_stream_tx {
        let _ = tx.send(AgentStreamEvent::Complete {
            node_id: request.node_id,
        });
    }

    result
}

async fn stream_invoked_agent<M>(
    node_id: usize,
    parent_context: &ToolContext,
    agent: rig::agent::Agent<M>,
    history: Vec<rig::message::Message>,
    max_tool_rounds: u32,
    provider_label: &str,
) -> Result<String, String>
where
    M: rig::completion::CompletionModel + 'static,
    M::StreamingResponse: rig::completion::GetTokenUsage,
{
    use rig::agent::MultiTurnStreamItem;
    use rig::message::Message as RigMessage;
    use rig::streaming::StreamingPrompt;
    use rig::streaming::{StreamedAssistantContent, StreamedUserContent};

    let mut passes = 0usize;
    let mut history = history;
    let mut collected_all = String::new();
    let max_turns = max_tool_rounds.max(1) as usize;

    loop {
        passes += 1;

        let mut stream = agent
            .stream_prompt("")
            .with_history(history.clone())
            .multi_turn(max_turns)
            .await;

        let mut collected_pass = String::new();

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
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::Reasoning(reasoning),
                )) => {
                    let text = reasoning.reasoning.join("");
                    if !text.is_empty()
                        && let Some(tx) = &parent_context.agent_stream_tx
                    {
                        let _ = tx.send(AgentStreamEvent::Reasoning { node_id, text });
                    }
                }
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::ToolCall(tool_call),
                )) => {
                    if let Some(tx) = &parent_context.agent_stream_tx {
                        let name = tool_call.function.name;
                        let args = tool_call.function.arguments.to_string();
                        let _ = tx.send(AgentStreamEvent::ToolCall {
                            node_id,
                            name,
                            args,
                        });
                    }
                }
                Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult(_))) => {}
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

        if !collected_pass.is_empty() {
            collected_all.push_str(&collected_pass);
            history.push(RigMessage::assistant(&collected_pass));
        }

        let todo_ok = match &parent_context.todo_store {
            Some(store) => store.snapshot(node_id).await.is_completed_and_confirmed(),
            None => true,
        };

        if todo_ok {
            break;
        }

        if passes >= TODO_GUARD_MAX_PASSES {
            // Return collected text with error note appended rather than discarding all output
            let error_note =
                "\n\n---\n⚠️ Note: To Do list was not confirmed complete after multiple attempts.";
            return Ok(format!("{}{}", collected_all, error_note));
        }

        // Use first prompt on pass 1, retry prompt on subsequent passes
        let guard_prompt = if passes == 1 {
            TODO_GUARD_FIRST_PROMPT
        } else {
            TODO_GUARD_RETRY_PROMPT
        };
        history.push(RigMessage::user(guard_prompt));
    }

    Ok(collected_all)
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
    const STATS_WINDOW_MIN_MS: u64 = 200;

    let emit_stream_stats =
        |stats_window_chars: &mut usize, stats_window_start: &mut Instant| -> Option<RunnerEvent> {
            if *stats_window_chars == 0 {
                return None;
            }
            let elapsed = stats_window_start.elapsed();
            let window_ms = elapsed.as_millis() as u64;
            if window_ms < STATS_WINDOW_MIN_MS {
                return None;
            }
            let chars = *stats_window_chars;
            *stats_window_chars = 0;
            *stats_window_start = Instant::now();
            Some(RunnerEvent::StreamStats {
                chars_in_window: chars,
                window_ms,
            })
        };

    let history = build_chat_history(chat_history);
    let user_msg = build_user_message(&user_message, image_data);

    // Load compression settings and apply if needed
    let compression_settings = ConfigDatabase::open()
        .ok()
        .map(|db| CompressionSettings::load_from(&db))
        .unwrap_or_default();

    // Convert to rig messages for compression check
    let mut rig_messages: Vec<rig::message::Message> = history.clone();
    rig_messages.push(user_msg.clone());

    // Create comprehensive context estimate (includes system prompt, tools, messages)
    // Note: We use empty tool definitions here since we don't have access to them yet
    // The full estimate with tools will be done per-provider below
    let context_estimate = crate::compression::create_context_estimate(
        &system_prompt,
        &[], // Tool definitions added per-provider
        &rig_messages,
        &model_name,
        None, // No API-provided context window yet
    );

    // Emit context estimate for UI display
    let _ = event_tx.send(RunnerEvent::ContextEstimate {
        system_prompt_tokens: context_estimate.system_prompt_tokens,
        tool_definitions_tokens: context_estimate.tool_definitions_tokens,
        messages_tokens: context_estimate.messages_tokens,
        total_tokens: context_estimate.total_tokens,
        context_window: context_estimate.context_window,
        usage_percent: context_estimate.usage_percent,
    });

    // Check if compression is needed
    let compression_needed = crate::compression::needs_compression(&context_estimate, &compression_settings);
    let threshold_tokens = context_estimate.threshold_tokens(compression_settings.threshold_percent);

    // Apply compression if needed
    let full_history = if compression_needed {
        let original_count = rig_messages.len();
        let original_tokens = context_estimate.total_tokens;

        // Apply compression
        let compressed = compress_messages(
            rig_messages,
            &compression_settings,
            threshold_tokens as usize,
        );

        let compressed_count = compressed.len();
        let compressed_tokens = rig::compression::estimate_messages_tokens(&compressed);

        tracing::info!(
            "Context compressed: {} -> {} messages, {} -> {} tokens (strategy: {:?})",
            original_count,
            compressed_count,
            original_tokens,
            compressed_tokens,
            compression_settings.strategy
        );

        // Emit compression event
        let _ = event_tx.send(RunnerEvent::ContextCompressed {
            original_messages: original_count,
            compressed_messages: compressed_count,
            original_tokens,
            compressed_tokens,
            strategy: compression_settings.strategy.as_str().to_string(),
        });

        compressed
    } else {
        let mut full_history = history;
        full_history.push(user_msg);
        full_history
    };

    let node_id = tool_context.node_id;

    match ProviderRegistry::resolve_provider(&model_name) {
        ProviderId::ChatGpt => {
            tracing::info!("Using ChatGPT/Codex backend for model: {}", model_name);

            let token = auth::select_token(providers::CHATGPT).ok_or_else(|| {
                "ChatGPT authentication required. Please authenticate in Settings.".to_string()
            })?;
            let id_token = token.id_token.ok_or_else(|| {
                "ChatGPT id_token not found. Please re-authenticate in Settings.".to_string()
            })?;

            let client = ChatGptOAuthClient::from_tokens(&token.access_token, &id_token)
                .map_err(|e| format!("Failed to create ChatGPT client: {}", e))?;

            // Extract the actual model ID without the provider suffix
            let actual_model_id = ProviderRegistry::extract_model_id(&model_name);
            let model = client.completion_model(actual_model_id);
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
                invoke_agent_tool,
            ) = crate::tools::create_tools(tool_context.clone());

            let builder = AgentBuilder::new(model)
                .preamble(&system_prompt)
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
                .tool(invoke_agent_tool);

            let builder = attach_mcp_tools_to_builder(builder, tool_context.current_agent).await;

            let agent = builder
                .temperature(0.7)
                .max_tokens(8192)
                .additional_params(ChatGptOAuthClient::codex_params())
                .build();

            use rig::agent::MultiTurnStreamItem;
            use rig::streaming::StreamingPrompt;
            use rig::streaming::{StreamedAssistantContent, StreamedUserContent};

            use rig::message::Message as RigMessage;

            let mut passes = 0usize;
            let mut history = full_history;
            let max_turns = max_tool_rounds.max(1) as usize;

            loop {
                passes += 1;

                let mut stream = agent
                    .stream_prompt("")
                    .with_history(history.clone())
                    .multi_turn(max_turns)
                    .await;

                let mut collected_pass = String::new();
                let mut chunk_count = 0u32;
                while let Some(chunk_result) = stream.next().await {
                    chunk_count += 1;
                    match chunk_result {
                        Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Text(text_chunk),
                        )) => {
                            if !text_chunk.text.is_empty() {
                                let chunk_len = text_chunk.text.len();
                                collected_pass.push_str(&text_chunk.text);
                                tracing::trace!(
                                    "ChatGPT text chunk #{}: {} chars",
                                    chunk_count,
                                    text_chunk.text.len()
                                );
                                let _ = event_tx.send(RunnerEvent::StreamChunk(text_chunk.text));
                                stats_window_chars += chunk_len;
                                if let Some(stats) = emit_stream_stats(
                                    &mut stats_window_chars,
                                    &mut stats_window_start,
                                ) {
                                    let _ = event_tx.send(stats);
                                }
                            }
                        }
                        Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Reasoning(reasoning),
                        )) => {
                            let text = reasoning.reasoning.join("");
                            if !text.is_empty() {
                                tracing::debug!(
                                    "ChatGPT reasoning chunk #{}: {} chars",
                                    chunk_count,
                                    text.len()
                                );
                                let _ = event_tx.send(RunnerEvent::Reasoning(text));
                            }
                        }
                        Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::ToolCall(tool_call),
                        )) => {
                            let name = tool_call.function.name;
                            let args = tool_call.function.arguments.to_string();
                            let _ = event_tx.send(RunnerEvent::ToolCall { name, args });
                        }
                        Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Final(response),
                        )) => {
                            // Emit usage after each LLM turn (before tool execution)
                            if let Some(usage) = response.token_usage() {
                                tracing::info!(
                                    "Usage update (provider=chatgpt): input={}, output={}",
                                    usage.input_tokens,
                                    usage.output_tokens
                                );
                                let _ = event_tx.send(RunnerEvent::Usage {
                                    input_tokens: usage.input_tokens,
                                    output_tokens: usage.output_tokens,
                                });
                            }
                        }
                        Ok(MultiTurnStreamItem::StreamUserItem(
                            StreamedUserContent::ToolResult(_),
                        )) => {}
                        // Skip FinalResponse - its aggregated_usage sums all turns which is wrong
                        // for input_tokens (each turn already includes full history)
                        Ok(MultiTurnStreamItem::FinalResponse(_)) => {}
                        // Pre-request context estimate from rig
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
                            let error_msg = format!("ChatGPT stream error: {}", e);
                            if is_rate_limit_error(&error_msg) {
                                auth::mark_cooldown(
                                    &token.account_id,
                                    &error_msg,
                                    DEFAULT_COOLDOWN_SECS,
                                );
                            }
                            return Err(error_msg);
                        }
                    }
                }

                if !collected_pass.is_empty() {
                    history.push(RigMessage::assistant(&collected_pass));
                }

                let todo_ok = match &tool_context.todo_store {
                    Some(store) => store.snapshot(node_id).await.is_completed_and_confirmed(),
                    None => true,
                };

                if todo_ok {
                    return Ok(());
                }

                if passes >= TODO_GUARD_MAX_PASSES {
                    // Append warning as chunk instead of replacing all streamed output with error
                    let _ = event_tx.send(RunnerEvent::StreamChunk(
                        "\n\n---\n⚠️ Note: To Do list was not confirmed complete after multiple attempts.".to_string()
                    ));
                    return Ok(());
                }

                // Use first prompt on pass 1, retry prompt on subsequent passes
                let guard_prompt = if passes == 1 {
                    TODO_GUARD_FIRST_PROMPT
                } else {
                    TODO_GUARD_RETRY_PROMPT
                };
                history.push(RigMessage::user(guard_prompt));
            }
        }
        ProviderId::Gemini => {
            tracing::info!("Using Gemini backend for model: {}", model_name);

            let token = auth::select_token(providers::GEMINI).ok_or_else(|| {
                "Gemini authentication required. Please authenticate in Settings.".to_string()
            })?;

            let client = GeminiCodeAssistRigClient::new(token.access_token);

            // Extract the actual model ID without the provider suffix
            let actual_model_id = ProviderRegistry::extract_model_id(&model_name);
            let model = client.completion_model(actual_model_id);
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
                invoke_agent_tool,
            ) = crate::tools::create_tools(tool_context.clone());

            let builder = AgentBuilder::new(model)
                .preamble(&system_prompt)
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
                .tool(invoke_agent_tool);

            let builder = attach_mcp_tools_to_builder(builder, tool_context.current_agent).await;

            let agent = builder.temperature(0.7).max_tokens(8192).build();

            use rig::agent::MultiTurnStreamItem;
            use rig::streaming::StreamingPrompt;
            use rig::streaming::{StreamedAssistantContent, StreamedUserContent};

            use rig::message::Message as RigMessage;

            let mut passes = 0usize;
            let mut history = full_history;
            let max_turns = max_tool_rounds.max(1) as usize;

            loop {
                passes += 1;

                let mut stream = agent
                    .stream_prompt("")
                    .with_history(history.clone())
                    .multi_turn(max_turns)
                    .await;

                let mut collected_pass = String::new();
                let mut chunk_count = 0u32;
                while let Some(chunk_result) = stream.next().await {
                    chunk_count += 1;
                    match chunk_result {
                        Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Text(text_chunk),
                        )) => {
                            if !text_chunk.text.is_empty() {
                                let chunk_len = text_chunk.text.len();
                                collected_pass.push_str(&text_chunk.text);
                                tracing::trace!(
                                    "Gemini text chunk #{}: {} chars",
                                    chunk_count,
                                    text_chunk.text.len()
                                );
                                let _ = event_tx.send(RunnerEvent::StreamChunk(text_chunk.text));
                                stats_window_chars += chunk_len;
                                if let Some(stats) = emit_stream_stats(
                                    &mut stats_window_chars,
                                    &mut stats_window_start,
                                ) {
                                    let _ = event_tx.send(stats);
                                }
                            }
                        }
                        Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Reasoning(reasoning),
                        )) => {
                            let text = reasoning.reasoning.join("");
                            if !text.is_empty() {
                                tracing::debug!(
                                    "Gemini reasoning chunk #{}: {} chars",
                                    chunk_count,
                                    text.len()
                                );
                                let _ = event_tx.send(RunnerEvent::Reasoning(text));
                            }
                        }
                        Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::ToolCall(tool_call),
                        )) => {
                            let name = tool_call.function.name;
                            let args = tool_call.function.arguments.to_string();
                            let _ = event_tx.send(RunnerEvent::ToolCall { name, args });
                        }
                        Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Final(response),
                        )) => {
                            // Emit usage after each LLM turn (before tool execution)
                            if let Some(usage) = response.token_usage() {
                                tracing::info!(
                                    "Usage update (provider=gemini): input={}, output={}",
                                    usage.input_tokens,
                                    usage.output_tokens
                                );
                                let _ = event_tx.send(RunnerEvent::Usage {
                                    input_tokens: usage.input_tokens,
                                    output_tokens: usage.output_tokens,
                                });
                            }
                        }
                        Ok(MultiTurnStreamItem::StreamUserItem(
                            StreamedUserContent::ToolResult(_),
                        )) => {}
                        // Skip FinalResponse - its aggregated_usage sums all turns which is wrong
                        // for input_tokens (each turn already includes full history)
                        Ok(MultiTurnStreamItem::FinalResponse(_)) => {}
                        // Pre-request context estimate from rig
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
                            let error_msg = format!("Gemini stream error: {}", e);
                            if is_rate_limit_error(&error_msg) {
                                auth::mark_cooldown(
                                    &token.account_id,
                                    &error_msg,
                                    DEFAULT_COOLDOWN_SECS,
                                );
                            }
                            return Err(error_msg);
                        }
                    }
                }

                if !collected_pass.is_empty() {
                    history.push(RigMessage::assistant(&collected_pass));
                }

                let todo_ok = match &tool_context.todo_store {
                    Some(store) => store.snapshot(node_id).await.is_completed_and_confirmed(),
                    None => true,
                };

                if todo_ok {
                    return Ok(());
                }

                if passes >= TODO_GUARD_MAX_PASSES {
                    // Append warning as chunk instead of replacing all streamed output with error
                    let _ = event_tx.send(RunnerEvent::StreamChunk(
                        "\n\n---\n⚠️ Note: To Do list was not confirmed complete after multiple attempts.".to_string()
                    ));
                    return Ok(());
                }

                // Use first prompt on pass 1, retry prompt on subsequent passes
                let guard_prompt = if passes == 1 {
                    TODO_GUARD_FIRST_PROMPT
                } else {
                    TODO_GUARD_RETRY_PROMPT
                };
                history.push(RigMessage::user(guard_prompt));
            }
        }
        ProviderId::Claude => {
            tracing::info!("Using Claude backend for model: {}", model_name);

            let token = auth::select_token(providers::CLAUDE).ok_or_else(|| {
                "Claude authentication required. Please authenticate in Settings.".to_string()
            })?;

            let client = ClaudeOAuthClient::new(token.access_token)
                .map_err(|e| format!("Failed to create Claude client: {}", e))?;

            // Extract the actual model ID without the provider suffix
            let actual_model_id = ProviderRegistry::extract_model_id(&model_name);
            let model = client.completion_model(actual_model_id);
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
                invoke_agent_tool,
            ) = crate::tools::create_tools(tool_context.clone());

            let mut history = full_history;
            prepend_system_to_first_user_message(&system_prompt, &mut history);

            let builder = AgentBuilder::new(model)
                .preamble(CLAUDE_CODE_INSTRUCTIONS)
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
                .tool(invoke_agent_tool);

            let builder = attach_mcp_tools_to_builder(builder, tool_context.current_agent).await;

            let agent = builder.temperature(0.7).max_tokens(8192).build();

            use rig::agent::MultiTurnStreamItem;
            use rig::message::Message as RigMessage;
            use rig::streaming::StreamingPrompt;
            use rig::streaming::{StreamedAssistantContent, StreamedUserContent};

            let mut passes = 0usize;
            let max_turns = max_tool_rounds.max(1) as usize;

            loop {
                passes += 1;

                let mut stream = agent
                    .stream_prompt("")
                    .with_history(history.clone())
                    .multi_turn(max_turns)
                    .await;

                let mut collected_pass = String::new();
                let mut chunk_count = 0u32;
                while let Some(chunk_result) = stream.next().await {
                    chunk_count += 1;
                    match chunk_result {
                        Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Text(text_chunk),
                        )) => {
                            if !text_chunk.text.is_empty() {
                                let chunk_len = text_chunk.text.len();
                                collected_pass.push_str(&text_chunk.text);
                                tracing::trace!(
                                    "Claude text chunk #{}: {} chars",
                                    chunk_count,
                                    text_chunk.text.len()
                                );
                                let _ = event_tx.send(RunnerEvent::StreamChunk(text_chunk.text));
                                stats_window_chars += chunk_len;
                                if let Some(stats) = emit_stream_stats(
                                    &mut stats_window_chars,
                                    &mut stats_window_start,
                                ) {
                                    let _ = event_tx.send(stats);
                                }
                            }
                        }
                        Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Reasoning(reasoning),
                        )) => {
                            let text = reasoning.reasoning.join("");
                            if !text.is_empty() {
                                tracing::debug!(
                                    "Claude reasoning chunk #{}: {} chars",
                                    chunk_count,
                                    text.len()
                                );
                                let _ = event_tx.send(RunnerEvent::Reasoning(text));
                            }
                        }
                        Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::ToolCall(tool_call),
                        )) => {
                            let name = tool_call.function.name;
                            let args = tool_call.function.arguments.to_string();
                            let _ = event_tx.send(RunnerEvent::ToolCall { name, args });
                        }
                        Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Final(response),
                        )) => {
                            // Emit usage after each LLM turn
                            match response.token_usage() {
                                Some(usage) => {
                                    tracing::info!(
                                        "Usage update (provider=claude): input={}, output={}",
                                        usage.input_tokens,
                                        usage.output_tokens
                                    );
                                    let _ = event_tx.send(RunnerEvent::Usage {
                                        input_tokens: usage.input_tokens,
                                        output_tokens: usage.output_tokens,
                                    });
                                }
                                None => {
                                    tracing::warn!("Claude Final: token_usage() returned None");
                                }
                            }
                        }
                        Ok(MultiTurnStreamItem::StreamUserItem(
                            StreamedUserContent::ToolResult(_),
                        )) => {
                            tracing::debug!("Claude ToolResult received");
                        }
                        Ok(MultiTurnStreamItem::FinalResponse(final_response)) => {
                            // Log what FinalResponse contains (but don't use it)
                            let usage = final_response.usage();
                            tracing::info!(
                                "Claude FinalResponse (aggregated, not used): input={}, output={}",
                                usage.input_tokens,
                                usage.output_tokens
                            );
                        }
                        // Pre-request context estimate from rig
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
                            let error_msg = format!("Claude stream error: {}", e);
                            if is_rate_limit_error(&error_msg) {
                                auth::mark_cooldown(
                                    &token.account_id,
                                    &error_msg,
                                    DEFAULT_COOLDOWN_SECS,
                                );
                            }
                            return Err(error_msg);
                        }
                    }
                }

                if !collected_pass.is_empty() {
                    history.push(RigMessage::assistant(&collected_pass));
                }

                let todo_ok = match &tool_context.todo_store {
                    Some(store) => store.snapshot(node_id).await.is_completed_and_confirmed(),
                    None => true,
                };

                if todo_ok {
                    return Ok(());
                }

                if passes >= TODO_GUARD_MAX_PASSES {
                    // Append warning as chunk instead of replacing all streamed output with error
                    let _ = event_tx.send(RunnerEvent::StreamChunk(
                        "\n\n---\n⚠️ Note: To Do list was not confirmed complete after multiple attempts.".to_string()
                    ));
                    return Ok(());
                }

                // Use first prompt on pass 1, retry prompt on subsequent passes
                let guard_prompt = if passes == 1 {
                    TODO_GUARD_FIRST_PROMPT
                } else {
                    TODO_GUARD_RETRY_PROMPT
                };
                history.push(RigMessage::user(guard_prompt));
            }
        }
        ProviderId::ApiKey(provider_id) => {
            // Look up provider info from registry
            let provider_def = RegistryService::find_provider(&provider_id)
                .ok_or_else(|| format!("Unknown provider: {}", provider_id))?;

            let provider_name = &provider_def.name;
            tracing::info!("Using {} backend for model: {}", provider_name, model_name);

            if !provider_def.is_openai_compatible {
                return Err(format!(
                    "{} API key provider requires special handling not yet implemented",
                    provider_name
                ));
            }

            let api_key_token = auth::select_api_key(&provider_id).ok_or_else(|| {
                format!(
                    "{} API key required. Please add an API key in Settings.",
                    provider_name
                )
            })?;

            let client = OpenAICompatibleApiClient::new(&provider_id, &api_key_token.api_key)
                .map_err(|e| format!("Failed to create {} client: {}", provider_name, e))?;

            // Extract the actual model ID without the provider suffix
            let actual_model_id = ProviderRegistry::extract_model_id(&model_name);
            tracing::info!(
                "API key provider model: '{}' -> extracted: '{}'",
                model_name,
                actual_model_id
            );
            let model = client.completion_model(actual_model_id);
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
                invoke_agent_tool,
            ) = crate::tools::create_tools(tool_context.clone());

            let builder = AgentBuilder::new(model)
                .preamble(&system_prompt)
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
                .tool(invoke_agent_tool);

            let builder = attach_mcp_tools_to_builder(builder, tool_context.current_agent).await;

            let agent = builder.temperature(0.7).max_tokens(8192).build();

            use rig::agent::MultiTurnStreamItem;
            use rig::message::Message as RigMessage;
            use rig::streaming::StreamingPrompt;
            use rig::streaming::{StreamedAssistantContent, StreamedUserContent};

            let mut passes = 0usize;
            let mut history = full_history;
            let max_turns = max_tool_rounds.max(1) as usize;

            loop {
                passes += 1;

                let mut stream = agent
                    .stream_prompt("")
                    .with_history(history.clone())
                    .multi_turn(max_turns)
                    .await;

                let mut collected_pass = String::new();
                let mut chunk_count = 0u32;
                while let Some(chunk_result) = stream.next().await {
                    chunk_count += 1;
                    match chunk_result {
                        Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Text(text_chunk),
                        )) => {
                            if !text_chunk.text.is_empty() {
                                let chunk_len = text_chunk.text.len();
                                collected_pass.push_str(&text_chunk.text);
                                tracing::trace!(
                                    "{} text chunk #{}: {} chars",
                                    provider_name,
                                    chunk_count,
                                    text_chunk.text.len()
                                );
                                let _ = event_tx.send(RunnerEvent::StreamChunk(text_chunk.text));
                                stats_window_chars += chunk_len;
                                if let Some(stats) = emit_stream_stats(
                                    &mut stats_window_chars,
                                    &mut stats_window_start,
                                ) {
                                    let _ = event_tx.send(stats);
                                }
                            }
                        }
                        Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Reasoning(reasoning),
                        )) => {
                            let text = reasoning.reasoning.join("");
                            if !text.is_empty() {
                                tracing::debug!(
                                    "{} reasoning chunk #{}: {} chars",
                                    provider_name,
                                    chunk_count,
                                    text.len()
                                );
                                let _ = event_tx.send(RunnerEvent::Reasoning(text));
                            }
                        }
                        Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::ToolCall(tool_call),
                        )) => {
                            let name = tool_call.function.name;
                            let args = tool_call.function.arguments.to_string();
                            let _ = event_tx.send(RunnerEvent::ToolCall { name, args });
                        }
                        Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Final(response),
                        )) => {
                            if let Some(usage) = response.token_usage() {
                                tracing::info!(
                                    "Usage update (provider={}): input={}, output={}",
                                    provider_name,
                                    usage.input_tokens,
                                    usage.output_tokens
                                );
                                let _ = event_tx.send(RunnerEvent::Usage {
                                    input_tokens: usage.input_tokens,
                                    output_tokens: usage.output_tokens,
                                });
                            }
                        }
                        Ok(MultiTurnStreamItem::StreamUserItem(
                            StreamedUserContent::ToolResult(_),
                        )) => {}
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
                            let error_msg = format!("{} stream error: {}", provider_name, e);
                            if is_rate_limit_error(&error_msg) {
                                auth::mark_api_key_cooldown(
                                    &api_key_token.account_id,
                                    &error_msg,
                                    DEFAULT_COOLDOWN_SECS,
                                );
                            }
                            return Err(error_msg);
                        }
                    }
                }

                if !collected_pass.is_empty() {
                    history.push(RigMessage::assistant(&collected_pass));
                }

                let todo_ok = match &tool_context.todo_store {
                    Some(store) => store.snapshot(node_id).await.is_completed_and_confirmed(),
                    None => true,
                };

                if todo_ok {
                    return Ok(());
                }

                if passes >= TODO_GUARD_MAX_PASSES {
                    let _ = event_tx.send(RunnerEvent::StreamChunk(
                        "\n\n---\n⚠️ Note: To Do list was not confirmed complete after multiple attempts.".to_string()
                    ));
                    return Ok(());
                }

                let guard_prompt = if passes == 1 {
                    TODO_GUARD_FIRST_PROMPT
                } else {
                    TODO_GUARD_RETRY_PROMPT
                };
                history.push(RigMessage::user(guard_prompt));
            }
        }
    }
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
            .tool(invoke_agent_tool)
            .temperature(0.1)
            .max_tokens(64)
            .build();

        let history = vec![build_user_message("hello", Vec::new())];
        let result = stream_invoked_agent(1, &parent_context, agent, history, 1, "Mock").await;

        // Now returns Ok with warning appended instead of Err (to preserve streamed content)
        let output = result.unwrap();
        assert!(output.contains("To Do list was not confirmed complete"));
    }
}