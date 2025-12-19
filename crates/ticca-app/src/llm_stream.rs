//! LLM streaming and agent execution
//!
//! Handles communication with Claude, ChatGPT/Codex, and Gemini APIs via the Rig framework,
//! including streaming responses and tool execution.
//!
//! Uses custom OAuth providers from `ticca_core::llm::providers` that wrap upstream rig
//! with OAuth authentication support.

use crate::chat_message::ChatMessage;
use crate::messages::Message;

use ticca_core::config::models::providers;
use ticca_core::llm;
use ticca_core::llm::auth::{self, AuthToken};
use ticca_core::llm::providers::chatgpt::ChatGptOAuthClient;
use ticca_core::llm::providers::GeminiCodeAssistRigClient;
use ticca_core::llm::ClaudeOAuthClient;
use ticca_core::llm::{ProviderId, ProviderRegistry};
use ticca_core::session::MessageRole;
use ticca_core::tools::{ToolApprovalDecision, ToolApprovalGate, ToolApprovalRequest, ToolContext, ToolPolicy};

use rig::agent::AgentBuilder;

use futures::StreamExt;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

const DEFAULT_COOLDOWN_SECS: i64 = 60;
const CLAUDE_CODE_INSTRUCTIONS: &str = "You are Claude Code, Anthropic's official CLI for Claude.";

/// Fetch the best available model from the Claude API
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

/// Build chat history from ChatMessage list
fn build_chat_history(chat_history: Vec<ChatMessage>) -> Vec<rig::message::Message> {
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

/// Build user message with optional images
fn build_user_message(user_message: &str, image_data: Vec<(String, String)>) -> rig::message::Message {
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

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;
    use rig::completion::{CompletionError, CompletionModel, CompletionRequest, CompletionResponse, Usage};
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
        ) -> impl std::future::Future<Output = Result<CompletionResponse<Self::Response>, CompletionError>>
        + Send {
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
        let (shell, read_file, list_files, edit_file, delete_file, grep, write_file) =
            ticca_core::tools::create_tools(tool_context);

        let history = build_chat_history(Vec::new());
        let user_msg = build_user_message("hello", Vec::new());
        let mut full_history = history;
        full_history.push(user_msg);

        let agent = AgentBuilder::new(MockModel)
            .preamble("test system")
            .tool(shell)
            .tool(read_file)
            .tool(list_files)
            .tool(edit_file)
            .tool(delete_file)
            .tool(grep)
            .tool(write_file)
            .temperature(0.1)
            .max_tokens(64)
            .build();

        use rig::agent::MultiTurnStreamItem;
        use rig::streaming::StreamingPrompt;
        use rig::streaming::StreamedAssistantContent;

        let mut stream = agent
            .stream_prompt("")
            .with_history(full_history)
            .multi_turn(1)
            .await;

        let mut collected = String::new();
        while let Some(chunk_result) = stream.next().await {
            if let Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::Text(text_chunk),
            )) = chunk_result
            {
                collected.push_str(&text_chunk.text);
            }
        }

        assert!(collected.contains("mock stream"));
    }
}

/// Run the Rig agent with streaming response and tools (ReAct loop)
///
/// Routes to Claude, ChatGPT/Codex, or Gemini based on model name.
/// Returns a Stream that yields Message events for each chunk.
///
/// `image_data` is a list of (media_type, base64_data) tuples for attached images
pub fn run_rig_agent_stream(
    system_prompt: String,
    user_message: String,
    model_name: Option<String>,
    working_directory: PathBuf,
    max_tool_rounds: u32,
    chat_history: Vec<ChatMessage>,
    image_data: Vec<(String, String)>,
    yolo_mode_enabled: bool,
    mut approval_decision_rx: mpsc::UnboundedReceiver<ToolApprovalDecision>,
    mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
) -> impl futures::Stream<Item = Message> {
    async_stream::stream! {
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<Message>();
        let (approval_request_tx, mut approval_request_rx) = mpsc::unbounded_channel::<ToolApprovalRequest>();
        let approval_gate = Arc::new(ToolApprovalGate::new(approval_request_tx));

        let tool_context = Arc::new(ToolContext {
            working_directory: working_directory.clone(),
            approval_gate: Some(approval_gate),
            yolo_mode_enabled,
            policy: ToolPolicy::allow_root(working_directory.clone()),
        });

        let mut pending_approvals: HashMap<u64, tokio::sync::oneshot::Sender<bool>> = HashMap::new();
        let event_tx_for_manager = event_tx.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(request) = approval_request_rx.recv() => {
                        pending_approvals.insert(request.id, request.responder);
                        let _ = event_tx_for_manager.send(Message::ToolApprovalRequested {
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
                model_name,
                max_tool_rounds,
                chat_history,
                image_data,
            ));

            tokio::select! {
                _ = &mut cancel_rx => {
                    worker.abort();
                    let _ = event_tx_for_worker.send(Message::StreamStopped);
                }
                result = &mut worker => {
                    match result {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => {
                            let _ = event_tx_for_worker.send(Message::StreamError(error));
                        }
                        Err(error) => {
                            let _ = event_tx_for_worker.send(Message::StreamError(format!("Stream task error: {}", error)));
                        }
                    }
                }
            }

            let _ = event_tx_for_worker.send(Message::StreamComplete);
        });

        while let Some(event) = event_rx.recv().await {
            let is_terminal = matches!(event, Message::StreamComplete | Message::StreamError(_));
            yield event;
            if is_terminal {
                break;
            }
        }
    }
}

async fn run_agent_stream(
    event_tx: mpsc::UnboundedSender<Message>,
    tool_context: Arc<ToolContext>,
    system_prompt: String,
    user_message: String,
    model_name: Option<String>,
    max_tool_rounds: u32,
    chat_history: Vec<ChatMessage>,
    image_data: Vec<(String, String)>,
) -> Result<(), String> {
    let mut stats_window_start = Instant::now();
    let mut stats_window_chars: usize = 0;
    const STATS_WINDOW_MIN_MS: u64 = 200;

    let mut emit_stream_stats = |stats_window_chars: &mut usize,
                                 stats_window_start: &mut Instant|
     -> Option<Message> {
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
        Some(Message::StreamStats {
            chars_in_window: chars,
            window_ms,
        })
    };

    let model_name = match model_name {
        Some(name) => {
            tracing::info!("Using configured model: {}", name);
            name
        }
        None => {
            let claude_token = auth::select_token(providers::CLAUDE)
                .ok_or_else(|| "Claude authentication required to auto-select a model".to_string())?;
            match fetch_best_model(&claude_token).await {
                Ok(name) => {
                    tracing::info!("Using auto-detected model: {}", name);
                    name
                }
                Err(e) => return Err(e),
            }
        }
    };

    let history = build_chat_history(chat_history);
    let user_msg = build_user_message(&user_message, image_data);
    let mut full_history = history;
    full_history.push(user_msg);

    match ProviderRegistry::resolve_provider(&model_name) {
    ProviderId::ChatGpt => {
        tracing::info!("Using ChatGPT/Codex backend for model: {}", model_name);

        let token = auth::select_token(providers::CHATGPT)
            .ok_or_else(|| "ChatGPT authentication required. Please authenticate in Settings.".to_string())?;
        let id_token = token
            .id_token
            .ok_or_else(|| "ChatGPT id_token not found. Please re-authenticate in Settings.".to_string())?;

        let client = ChatGptOAuthClient::from_tokens(&token.access_token, &id_token)
            .map_err(|e| format!("Failed to create ChatGPT client: {}", e))?;

        let model = client.completion_model(&model_name);
        let (shell, read_file, list_files, edit_file, delete_file, grep, write_file) =
            ticca_core::tools::create_tools(tool_context);

        let agent = AgentBuilder::new(model)
            .preamble(&system_prompt)
            .tool(shell)
            .tool(read_file)
            .tool(list_files)
            .tool(edit_file)
            .tool(delete_file)
            .tool(grep)
            .tool(write_file)
            .temperature(0.7)
            .max_tokens(8192)
            .additional_params(ChatGptOAuthClient::codex_params())
            .build();

        use rig::agent::MultiTurnStreamItem;
        use rig::streaming::StreamingPrompt;
        use rig::streaming::{StreamedAssistantContent, StreamedUserContent};

        let mut stream = agent
            .stream_prompt("")
            .with_history(full_history)
            .multi_turn(max_tool_rounds as usize)
            .await;

        let mut chunk_count = 0u32;
        while let Some(chunk_result) = stream.next().await {
            chunk_count += 1;
            match chunk_result {
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::Text(text_chunk),
                )) => {
                    if !text_chunk.text.is_empty() {
                        let chunk_len = text_chunk.text.len();
                        tracing::trace!("ChatGPT text chunk #{}: {} chars", chunk_count, text_chunk.text.len());
                        let _ = event_tx.send(Message::StreamChunk(text_chunk.text));
                        stats_window_chars += chunk_len;
                        if let Some(stats) =
                            emit_stream_stats(&mut stats_window_chars, &mut stats_window_start)
                        {
                            let _ = event_tx.send(stats);
                        }
                    }
                }
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::Reasoning(reasoning),
                )) => {
                    let text = reasoning.reasoning.join("");
                    if !text.is_empty() {
                        tracing::debug!("ChatGPT reasoning chunk #{}: {} chars", chunk_count, text.len());
                        let _ = event_tx.send(Message::Reasoning(text));
                    }
                }
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::ToolCall(tool_call),
                )) => {
                    let name = tool_call.function.name;
                    let args = tool_call.function.arguments.to_string();
                    let _ = event_tx.send(Message::ToolCall { name, args });
                }
                Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult(_))) => {}
                Ok(_) => {}
                Err(e) => {
                    let error_msg = format!("ChatGPT stream error: {}", e);
                    if is_rate_limit_error(&error_msg) {
                        auth::mark_cooldown(&token.account_id, &error_msg, DEFAULT_COOLDOWN_SECS);
                    }
                    return Err(error_msg);
                }
            }
        }

        return Ok(());
    }
    ProviderId::Gemini => {
        tracing::info!("Using Gemini backend for model: {}", model_name);

        let token = auth::select_token(providers::GEMINI)
            .ok_or_else(|| "Gemini authentication required. Please authenticate in Settings.".to_string())?;

        let client = GeminiCodeAssistRigClient::new(token.access_token);

        let model = client.completion_model(&model_name);
        let (shell, read_file, list_files, edit_file, delete_file, grep, write_file) =
            ticca_core::tools::create_tools(tool_context);

        let agent = AgentBuilder::new(model)
            .preamble(&system_prompt)
            .tool(shell)
            .tool(read_file)
            .tool(list_files)
            .tool(edit_file)
            .tool(delete_file)
            .tool(grep)
            .tool(write_file)
            .temperature(0.7)
            .max_tokens(8192)
            .build();

        use rig::agent::MultiTurnStreamItem;
        use rig::streaming::StreamingPrompt;
        use rig::streaming::{StreamedAssistantContent, StreamedUserContent};

        let mut stream = agent
            .stream_prompt("")
            .with_history(full_history)
            .multi_turn(max_tool_rounds as usize)
            .await;

        let mut chunk_count = 0u32;
        while let Some(chunk_result) = stream.next().await {
            chunk_count += 1;
            match chunk_result {
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::Text(text_chunk),
                )) => {
                    if !text_chunk.text.is_empty() {
                        let chunk_len = text_chunk.text.len();
                        tracing::trace!("Gemini text chunk #{}: {} chars", chunk_count, text_chunk.text.len());
                        let _ = event_tx.send(Message::StreamChunk(text_chunk.text));
                        stats_window_chars += chunk_len;
                        if let Some(stats) =
                            emit_stream_stats(&mut stats_window_chars, &mut stats_window_start)
                        {
                            let _ = event_tx.send(stats);
                        }
                    }
                }
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::Reasoning(reasoning),
                )) => {
                    let text = reasoning.reasoning.join("");
                    if !text.is_empty() {
                        tracing::debug!("Gemini reasoning chunk #{}: {} chars", chunk_count, text.len());
                        let _ = event_tx.send(Message::Reasoning(text));
                    }
                }
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::ToolCall(tool_call),
                )) => {
                    let name = tool_call.function.name;
                    let args = tool_call.function.arguments.to_string();
                    let _ = event_tx.send(Message::ToolCall { name, args });
                }
                Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult(_))) => {}
                Ok(_) => {}
                Err(e) => {
                    let error_msg = format!("Gemini stream error: {}", e);
                    if is_rate_limit_error(&error_msg) {
                        auth::mark_cooldown(&token.account_id, &error_msg, DEFAULT_COOLDOWN_SECS);
                    }
                    return Err(error_msg);
                }
            }
        }

        return Ok(());
    }
    ProviderId::Claude => {
    tracing::info!("Using Claude backend for model: {}", model_name);

    let token = auth::select_token(providers::CLAUDE)
        .ok_or_else(|| "Claude authentication required. Please authenticate in Settings.".to_string())?;

    let client = ClaudeOAuthClient::new(token.access_token)
        .map_err(|e| format!("Failed to create Claude client: {}", e))?;

    let model = client.completion_model(&model_name);
    let (shell, read_file, list_files, edit_file, delete_file, grep, write_file) =
        ticca_core::tools::create_tools(tool_context);

    let mut claude_history = full_history;
    prepend_system_to_first_user_message(&system_prompt, &mut claude_history);

    let agent = AgentBuilder::new(model)
        .preamble(CLAUDE_CODE_INSTRUCTIONS)
        .tool(shell)
        .tool(read_file)
        .tool(list_files)
        .tool(edit_file)
        .tool(delete_file)
        .tool(grep)
        .tool(write_file)
        .temperature(0.7)
        .max_tokens(8192)
        .build();

    use rig::agent::MultiTurnStreamItem;
    use rig::streaming::StreamingPrompt;
    use rig::streaming::{StreamedAssistantContent, StreamedUserContent};

    let mut stream = agent
        .stream_prompt("")
        .with_history(claude_history)
        .multi_turn(max_tool_rounds as usize)
        .await;

    let mut chunk_count = 0u32;
    while let Some(chunk_result) = stream.next().await {
        chunk_count += 1;
        match chunk_result {
            Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::Text(text_chunk),
            )) => {
                if !text_chunk.text.is_empty() {
                    let chunk_len = text_chunk.text.len();
                    tracing::trace!("Claude text chunk #{}: {} chars", chunk_count, text_chunk.text.len());
                    let _ = event_tx.send(Message::StreamChunk(text_chunk.text));
                    stats_window_chars += chunk_len;
                    if let Some(stats) =
                        emit_stream_stats(&mut stats_window_chars, &mut stats_window_start)
                    {
                        let _ = event_tx.send(stats);
                    }
                }
            }
            Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::Reasoning(reasoning),
            )) => {
                let text = reasoning.reasoning.join("");
                if !text.is_empty() {
                    tracing::debug!("Claude reasoning chunk #{}: {} chars", chunk_count, text.len());
                    let _ = event_tx.send(Message::Reasoning(text));
                }
            }
            Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::ToolCall(tool_call),
            )) => {
                let name = tool_call.function.name;
                let args = tool_call.function.arguments.to_string();
                let _ = event_tx.send(Message::ToolCall { name, args });
            }
            Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult(_))) => {}
            Ok(_) => {}
            Err(e) => {
                let error_msg = format!("Claude stream error: {}", e);
                if is_rate_limit_error(&error_msg) {
                    auth::mark_cooldown(&token.account_id, &error_msg, DEFAULT_COOLDOWN_SECS);
                }
                return Err(error_msg);
            }
        }
    }

    Ok(())
    }
    }
}
