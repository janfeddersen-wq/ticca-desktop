//! LLM streaming and agent execution
//!
//! Handles communication with the Claude API via the Rig framework,
//! including streaming responses and tool execution.

use crate::app::ChatMessage;
use crate::messages::Message;

use ticca_core::config::ConfigDatabase;
use ticca_core::config::models::providers;
use ticca_core::llm;
use ticca_core::tools::ToolContext;
use ticca_core::session::MessageRole;

use rig::prelude::*;
use rig::providers::anthropic;

use std::path::PathBuf;
use std::sync::Arc;
use futures::StreamExt;

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

/// Fetch the best available model from the Claude API
pub async fn fetch_best_model(auth_token: &str) -> Result<String, String> {
    let client = llm::ClaudeClient::new(auth_token.to_string());
    let models = client.fetch_latest_models().await
        .map_err(|e| format!("Failed to fetch models: {}", e))?;

    // Prefer sonnet, then opus, then haiku
    let preferred_order = ["sonnet", "opus", "haiku"];

    for family in preferred_order {
        if let Some(model) = models.iter().find(|m| m.contains(family)) {
            return Ok(model.clone());
        }
    }

    // If no match, return the first available model or error
    models.into_iter().next()
        .ok_or_else(|| "No models available from Claude API".to_string())
}

/// Run the Rig agent with streaming response and tools (ReAct loop)
///
/// Returns a Stream that yields Message events for each chunk
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

        // Create OAuth client with the token
        let client: anthropic::OAuthClient = match anthropic::OAuthClient::builder()
            .api_key(auth_token)
            .build()
        {
            Ok(client) => client,
            Err(e) => {
                yield Message::StreamError(format!("Failed to create OAuth client: {}", e));
                return;
            }
        };

        // Create tool context with working directory
        let tool_context = Arc::new(ToolContext {
            working_directory: working_directory.clone(),
        });

        // Create tools with the context
        let (shell, read_file, list_files, edit_file, grep, write_file) =
            ticca_core::tools::create_tools(tool_context);

        // Create agent with system prompt, tools, and model
        let agent = client
            .agent(&model_name)
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

        // Use streaming prompt with multi-turn enabled for ReAct loop
        use rig::streaming::StreamingPrompt;
        use rig::agent::MultiTurnStreamItem;
        use rig::streaming::{StreamedAssistantContent, StreamedUserContent};
        use rig::message::Message as RigMessage;
        use rig::message::{UserContent, Image, ImageMediaType};

        // Convert chat history to rig messages
        let history: Vec<RigMessage> = chat_history
            .into_iter()
            .filter_map(|msg| {
                match msg.role {
                    MessageRole::User => Some(RigMessage::user(&msg.content)),
                    MessageRole::Assistant => Some(RigMessage::assistant(&msg.content)),
                    _ => None, // Skip system and tool messages
                }
            })
            .collect();

        // Build the user message with optional images
        let user_msg = if image_data.is_empty() {
            // Simple text-only message
            RigMessage::user(&user_message)
        } else {
            // Build message with images + text
            let mut content_parts: Vec<UserContent> = Vec::new();

            // Add images first
            for (media_type, base64_data) in image_data {
                let media = match media_type.as_str() {
                    "image/png" => ImageMediaType::PNG,
                    "image/jpeg" => ImageMediaType::JPEG,
                    "image/gif" => ImageMediaType::GIF,
                    "image/webp" => ImageMediaType::WEBP,
                    _ => ImageMediaType::PNG, // Default to PNG
                };
                content_parts.push(UserContent::image(Image::base64(base64_data, media)));
            }

            // Add text if present
            if !user_message.is_empty() {
                content_parts.push(UserContent::text(&user_message));
            }

            RigMessage::User { content: content_parts }
        };

        // Add the new user message to history
        let mut full_history = history;
        full_history.push(user_msg);

        // Enable multi-turn for ReAct loop (configurable, default 500)
        // Use empty prompt since we already have the user message in history
        let mut stream = agent.stream_prompt("")
            .with_history(full_history)
            .multi_turn(max_tool_rounds as usize)
            .await;

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(text_chunk))) => {
                    if !text_chunk.text.is_empty() {
                        yield Message::StreamChunk(text_chunk.text);
                    }
                }
                Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Reasoning(reasoning))) => {
                    // Stream reasoning/thinking content separately
                    let text = reasoning.reasoning.join("");
                    if !text.is_empty() {
                        yield Message::Reasoning(text);
                    }
                }
                Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::ToolCall(tool_call))) => {
                    // Tool call initiated - yield a message so UI can show it
                    let args_str = serde_json::to_string_pretty(&tool_call.function.arguments)
                        .unwrap_or_else(|_| format!("{:?}", tool_call.function.arguments));
                    yield Message::ToolCall {
                        name: tool_call.function.name.clone(),
                        args: args_str,
                    };
                }
                Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult(tool_result))) => {
                    // Tool result received - yield a message so UI can show it
                    let result_text = tool_result.content.iter()
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
                    // Stream complete, we'll yield StreamComplete at the end
                }
                Ok(_) => {
                    // Other stream items (deltas, etc)
                }
                Err(e) => {
                    yield Message::StreamError(format!("Stream error: {}", e));
                    return;
                }
            }
        }
        yield Message::StreamComplete;
    }
}
