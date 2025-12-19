//! LLM streaming and agent execution
//!
//! Handles communication with Claude, ChatGPT/Codex, and Gemini APIs via the Rig framework,
//! including streaming responses and tool execution.
//!
//! Uses custom OAuth providers from `ticca_core::llm::providers` that wrap upstream rig
//! with OAuth authentication support.

use crate::chat_message::ChatMessage;
use crate::messages::Message;

use ticca_core::config::ConfigDatabase;
use ticca_core::config::models::providers;
use ticca_core::llm;
use ticca_core::llm::providers::chatgpt::{is_gpt_model, ChatGptOAuthClient};
use ticca_core::llm::providers::gemini::is_gemini_model;
use ticca_core::llm::providers::GeminiCodeAssistRigClient;
use ticca_core::llm::ClaudeOAuthClient;
use ticca_core::session::MessageRole;
use ticca_core::tools::ToolContext;

use rig::agent::AgentBuilder;

use futures::StreamExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// Get the Claude OAuth token if available and valid
pub fn get_claude_auth_token() -> Option<String> {
    let db = ConfigDatabase::open().ok()?;
    let token = db.get_oauth_token(providers::CLAUDE).ok()??;

    if token.is_expired() {
        tracing::warn!("Claude OAuth token is expired");
        return None;
    }

    Some(token.access_token)
}

/// Get the Gemini OAuth token if available and valid
pub fn get_gemini_auth_token() -> Option<String> {
    let db = ConfigDatabase::open().ok()?;
    let token = db.get_oauth_token(providers::GEMINI).ok()??;

    if token.is_expired() {
        tracing::warn!("Gemini OAuth token is expired");
        return None;
    }

    Some(token.access_token)
}

/// Get the ChatGPT OAuth token if available and valid
pub fn get_chatgpt_auth_token() -> Option<String> {
    let db = ConfigDatabase::open().ok()?;
    let token = db.get_oauth_token(providers::CHATGPT).ok()??;

    if token.is_expired() {
        tracing::warn!("ChatGPT OAuth token is expired");
        return None;
    }

    Some(token.access_token)
}

/// Get the ChatGPT id_token for API calls (stored in extra_json)
pub fn get_chatgpt_id_token() -> Option<String> {
    let db = ConfigDatabase::open().ok()?;
    let token = db.get_oauth_token(providers::CHATGPT).ok()??;

    // Parse id_token from extra_json
    token.extra_json.as_ref().and_then(|json_str| {
        serde_json::from_str::<serde_json::Value>(json_str)
            .ok()
            .and_then(|v| v.get("id_token").and_then(|t| t.as_str()).map(|s| s.to_string()))
    })
}

/// Fetch the best available model from the Claude API
pub async fn fetch_best_model(auth_token: &str) -> Result<String, String> {
    let client = llm::ClaudeClient::new(auth_token.to_string());
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

/// Run the Rig agent with streaming response and tools (ReAct loop)
///
/// Routes to Claude, ChatGPT/Codex, or Gemini based on model name.
/// Returns a Stream that yields Message events for each chunk.
///
/// `image_data` is a list of (media_type, base64_data) tuples for attached images
pub fn run_rig_agent_stream(
    auth_token: String,
    system_prompt: String,
    user_message: String,
    model_name: Option<String>,
    working_directory: PathBuf,
    max_tool_rounds: u32,
    chat_history: Vec<ChatMessage>,
    image_data: Vec<(String, String)>,
) -> impl futures::Stream<Item = Message> {
    async_stream::stream! {
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

        // Use provided model or fetch from API
        let model_name = match model_name {
            Some(name) => {
                tracing::info!("Using configured model: {}", name);
                name
            }
            None => {
                match fetch_best_model(&auth_token).await {
                    Ok(name) => {
                        tracing::info!("Using auto-detected model: {}", name);
                        name
                    }
                    Err(e) => {
                        yield Message::StreamError(e);
                        return;
                    }
                }
            }
        };

        // Build history and user message
        let history = build_chat_history(chat_history);
        let user_msg = build_user_message(&user_message, image_data);
        let mut full_history = history;
        full_history.push(user_msg);

        // Create tool context with working directory
        let tool_context = Arc::new(ToolContext {
            working_directory: working_directory.clone(),
        });

        // Route to appropriate provider based on model name
        if is_gpt_model(&model_name) {
            // Use ChatGPT/Codex backend
            tracing::info!("Using ChatGPT/Codex backend for model: {}", model_name);

            // Get ChatGPT auth token and id_token
            let chatgpt_token = match get_chatgpt_auth_token() {
                Some(token) => token,
                None => {
                    yield Message::StreamError(
                        "ChatGPT authentication required. Please authenticate with ChatGPT in Settings.".to_string()
                    );
                    return;
                }
            };

            let id_token = match get_chatgpt_id_token() {
                Some(token) => token,
                None => {
                    yield Message::StreamError(
                        "ChatGPT id_token not found. Please re-authenticate with ChatGPT.".to_string()
                    );
                    return;
                }
            };

            // Create ChatGPT OAuth client
            let client = match ChatGptOAuthClient::from_tokens(&chatgpt_token, &id_token) {
                Ok(c) => c,
                Err(e) => {
                    yield Message::StreamError(format!("Failed to create ChatGPT client: {}", e));
                    return;
                }
            };

            // Get completion model
            let model = client.completion_model(&model_name);

            // Create tools
            let (shell, read_file, list_files, edit_file, grep, write_file) =
                ticca_core::tools::create_tools(tool_context);

            // Create agent with Codex-specific params using AgentBuilder
            let agent = AgentBuilder::new(model)
                .preamble(&system_prompt)
                .tool(shell)
                .tool(read_file)
                .tool(list_files)
                .tool(edit_file)
                .tool(grep)
                .tool(write_file)
                .temperature(0.7)
                .max_tokens(8192)
                .additional_params(ChatGptOAuthClient::codex_params())
                .build();

            // Stream the response
            use rig::agent::MultiTurnStreamItem;
            use rig::streaming::StreamingPrompt;
            use rig::streaming::{StreamedAssistantContent, StreamedUserContent};

            let mut stream = agent
                .stream_prompt("")
                .with_history(full_history.clone())
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
                            yield Message::StreamChunk(text_chunk.text);
                            stats_window_chars += chunk_len;
                            if let Some(stats) =
                                emit_stream_stats(&mut stats_window_chars, &mut stats_window_start)
                            {
                                yield stats;
                            }
                        }
                    }
                    Ok(MultiTurnStreamItem::StreamAssistantItem(
                        StreamedAssistantContent::Reasoning(reasoning),
                    )) => {
                        let text = reasoning.reasoning.join("");
                        if !text.is_empty() {
                            tracing::debug!("ChatGPT reasoning chunk #{}: {} chars", chunk_count, text.len());
                            yield Message::Reasoning(text);
                        }
                    }
                    Ok(MultiTurnStreamItem::StreamAssistantItem(
                        StreamedAssistantContent::ToolCall(tool_call),
                    )) => {
                        tracing::debug!("ChatGPT tool call #{}: {}", chunk_count, tool_call.function.name);
                        let args_str = serde_json::to_string_pretty(&tool_call.function.arguments)
                            .unwrap_or_else(|_| format!("{:?}", tool_call.function.arguments));
                        yield Message::ToolCall {
                            name: tool_call.function.name.clone(),
                            args: args_str,
                        };
                    }
                    Ok(MultiTurnStreamItem::StreamUserItem(
                        StreamedUserContent::ToolResult(tool_result),
                    )) => {
                        tracing::debug!("ChatGPT tool result #{}: {}", chunk_count, tool_result.id);
                        let result_text = tool_result
                            .content
                            .iter()
                            .map(|c| match c {
                                rig::message::ToolResultContent::Text(t) => t.text.clone(),
                                _ => "[non-text content]".to_string(),
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        yield Message::ToolResult {
                            name: tool_result.id.clone(),
                            result: result_text,
                        };
                    }
                    Ok(MultiTurnStreamItem::FinalResponse(_)) => {
                        tracing::debug!("ChatGPT received FinalResponse #{}", chunk_count);
                    }
                    Ok(other) => {
                        tracing::debug!("ChatGPT other stream item #{}: {:?}", chunk_count, std::any::type_name_of_val(&other));
                    }
                    Err(e) => {
                        tracing::error!("ChatGPT stream error #{}: {}", chunk_count, e);
                        yield Message::StreamError(format!("ChatGPT stream error: {}", e));
                        return;
                    }
                }
            }
            tracing::debug!("ChatGPT stream finished after {} chunks", chunk_count);
        } else if is_gemini_model(&model_name) {
            tracing::info!("Using Gemini backend for model: {}", model_name);

            let gemini_token = match get_gemini_auth_token() {
                Some(token) => token,
                None => {
                    yield Message::StreamError(
                        "Gemini authentication required. Please authenticate with Gemini in Settings.".to_string()
                    );
                    return;
                }
            };

            let client = GeminiCodeAssistRigClient::new(&gemini_token);
            let model = client.completion_model(&model_name);

            let (shell, read_file, list_files, edit_file, grep, write_file) =
                ticca_core::tools::create_tools(tool_context);

            let agent = AgentBuilder::new(model)
                .preamble(&system_prompt)
                .tool(shell)
                .tool(read_file)
                .tool(list_files)
                .tool(edit_file)
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
                .with_history(full_history.clone())
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
                            yield Message::StreamChunk(text_chunk.text);
                            stats_window_chars += chunk_len;
                            if let Some(stats) =
                                emit_stream_stats(&mut stats_window_chars, &mut stats_window_start)
                            {
                                yield stats;
                            }
                        }
                    }
                    Ok(MultiTurnStreamItem::StreamAssistantItem(
                        StreamedAssistantContent::Reasoning(reasoning),
                    )) => {
                        let text = reasoning.reasoning.join("");
                        if !text.is_empty() {
                            tracing::debug!("Gemini reasoning chunk #{}: {} chars", chunk_count, text.len());
                            yield Message::Reasoning(text);
                        }
                    }
                    Ok(MultiTurnStreamItem::StreamAssistantItem(
                        StreamedAssistantContent::ToolCall(tool_call),
                    )) => {
                        tracing::debug!("Gemini tool call #{}: {}", chunk_count, tool_call.function.name);
                        let args_str = serde_json::to_string_pretty(&tool_call.function.arguments)
                            .unwrap_or_else(|_| format!("{:?}", tool_call.function.arguments));
                        yield Message::ToolCall {
                            name: tool_call.function.name.clone(),
                            args: args_str,
                        };
                    }
                    Ok(MultiTurnStreamItem::StreamUserItem(
                        StreamedUserContent::ToolResult(tool_result),
                    )) => {
                        tracing::debug!("Gemini tool result #{}: {}", chunk_count, tool_result.id);
                        let result_text = tool_result
                            .content
                            .iter()
                            .map(|c| match c {
                                rig::message::ToolResultContent::Text(t) => t.text.clone(),
                                _ => "[non-text content]".to_string(),
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        yield Message::ToolResult {
                            name: tool_result.id.clone(),
                            result: result_text,
                        };
                    }
                    Ok(MultiTurnStreamItem::FinalResponse(_)) => {
                        tracing::debug!("Gemini received FinalResponse #{}", chunk_count);
                    }
                    Ok(other) => {
                        tracing::debug!("Gemini other stream item #{}: {:?}", chunk_count, std::any::type_name_of_val(&other));
                    }
                    Err(e) => {
                        tracing::error!("Gemini stream error #{}: {}", chunk_count, e);
                        yield Message::StreamError(format!("Gemini stream error: {}", e));
                        return;
                    }
                }
            }
            tracing::debug!("Gemini stream finished after {} chunks", chunk_count);
        } else {
            // Use Claude/Anthropic backend (default)
            tracing::info!("Using Claude backend for model: {}", model_name);
            tracing::debug!("Claude auth_token present: {}", !auth_token.is_empty());
            tracing::debug!("System prompt length: {} chars", system_prompt.len());
            tracing::debug!("Full history count: {} messages", full_history.len());

            // Claude Code OAuth requires special system prompt handling:
            // 1. Prepend the original system prompt to the first user message
            // 2. Use hardcoded Claude Code instruction as the actual system prompt
            const CLAUDE_CODE_INSTRUCTIONS: &str = "You are Claude Code, Anthropic's official CLI for Claude.";

            // Prepend system_prompt to the first user message in history
            let mut claude_history = full_history;
            if !system_prompt.is_empty() && !claude_history.is_empty() {
                tracing::debug!("Prepending system prompt to first user message");
                // Find the first user message and prepend the system prompt to it
                for msg in claude_history.iter_mut() {
                    if let rig::message::Message::User { content } = msg {
                        // Get mutable access to the first content item
                        let first_content = content.first_mut();
                        if let rig::message::UserContent::Text(text_content) = first_content {
                            let original_len = text_content.text.len();
                            text_content.text = format!("{}\n\n{}", system_prompt, text_content.text);
                            tracing::debug!(
                                "Modified first user message: {} -> {} chars",
                                original_len,
                                text_content.text.len()
                            );
                            break; // Only prepend to the first user message
                        }
                    }
                }
            } else {
                tracing::debug!("Skipping system prompt prepend: system_prompt.is_empty()={}, history.is_empty()={}",
                    system_prompt.is_empty(), claude_history.is_empty());
            }

            // Create Claude OAuth client
            tracing::debug!("Creating ClaudeOAuthClient...");
            let client = match ClaudeOAuthClient::new(&auth_token) {
                Ok(c) => {
                    tracing::debug!("ClaudeOAuthClient created successfully");
                    c
                }
                Err(e) => {
                    tracing::error!("Failed to create Claude client: {}", e);
                    yield Message::StreamError(format!("Failed to create Claude client: {}", e));
                    return;
                }
            };

            // Get completion model
            tracing::debug!("Getting completion model for: {}", model_name);
            let model = client.completion_model(&model_name);

            // Create tools
            let (shell, read_file, list_files, edit_file, grep, write_file) =
                ticca_core::tools::create_tools(tool_context);

            // Create agent using AgentBuilder with hardcoded Claude Code instruction
            let agent = AgentBuilder::new(model)
                .preamble(CLAUDE_CODE_INSTRUCTIONS)  // Hardcoded, not system_prompt
                .tool(shell)
                .tool(read_file)
                .tool(list_files)
                .tool(edit_file)
                .tool(grep)
                .tool(write_file)
                .temperature(0.7)
                .max_tokens(8192)
                .build();

            // Stream the response
            use rig::agent::MultiTurnStreamItem;
            use rig::streaming::StreamingPrompt;
            use rig::streaming::{StreamedAssistantContent, StreamedUserContent};

            tracing::debug!("Starting Claude stream with {} history messages, max_tool_rounds={}",
                claude_history.len(), max_tool_rounds);
            
            let mut stream = agent
                .stream_prompt("")
                .with_history(claude_history)  // Use modified history with prepended system prompt
                .multi_turn(max_tool_rounds as usize)
                .await;
            
            tracing::debug!("Claude stream created, starting iteration...");

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
                            yield Message::StreamChunk(text_chunk.text);
                            stats_window_chars += chunk_len;
                            if let Some(stats) =
                                emit_stream_stats(&mut stats_window_chars, &mut stats_window_start)
                            {
                                yield stats;
                            }
                        }
                    }
                    Ok(MultiTurnStreamItem::StreamAssistantItem(
                        StreamedAssistantContent::Reasoning(reasoning),
                    )) => {
                        let text = reasoning.reasoning.join("");
                        if !text.is_empty() {
                            tracing::debug!("Claude reasoning chunk #{}: {} chars", chunk_count, text.len());
                            yield Message::Reasoning(text);
                        }
                    }
                    Ok(MultiTurnStreamItem::StreamAssistantItem(
                        StreamedAssistantContent::ToolCall(tool_call),
                    )) => {
                        tracing::debug!("Claude tool call #{}: {}", chunk_count, tool_call.function.name);
                        let args_str = serde_json::to_string_pretty(&tool_call.function.arguments)
                            .unwrap_or_else(|_| format!("{:?}", tool_call.function.arguments));
                        yield Message::ToolCall {
                            name: tool_call.function.name.clone(),
                            args: args_str,
                        };
                    }
                    Ok(MultiTurnStreamItem::StreamUserItem(
                        StreamedUserContent::ToolResult(tool_result),
                    )) => {
                        tracing::debug!("Claude tool result #{}: {}", chunk_count, tool_result.id);
                        let result_text = tool_result
                            .content
                            .iter()
                            .map(|c| match c {
                                rig::message::ToolResultContent::Text(t) => t.text.clone(),
                                _ => "[non-text content]".to_string(),
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        yield Message::ToolResult {
                            name: tool_result.id.clone(),
                            result: result_text,
                        };
                    }
                    Ok(MultiTurnStreamItem::FinalResponse(_)) => {
                        tracing::debug!("Claude received FinalResponse #{}", chunk_count);
                    }
                    Ok(other) => {
                        tracing::debug!("Claude other stream item #{}: {:?}", chunk_count, std::any::type_name_of_val(&other));
                    }
                    Err(e) => {
                        tracing::error!("Claude stream error #{}: {}", chunk_count, e);
                        yield Message::StreamError(format!("Stream error: {}", e));
                        return;
                    }
                }
            }
            tracing::debug!("Claude stream finished after {} chunks", chunk_count);
        }

        if stats_window_chars > 0 {
            let elapsed = stats_window_start.elapsed();
            if elapsed.as_millis() > 0 {
                yield Message::StreamStats {
                    chars_in_window: stats_window_chars,
                    window_ms: elapsed.as_millis() as u64,
                };
            }
        }

        yield Message::StreamComplete;
    }
}
