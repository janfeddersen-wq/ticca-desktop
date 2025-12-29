//! Message building utilities for serdesAI integration.

use super::types::ChatHistoryMessage;
use crate::session::MessageRole;
use base64::Engine;
use serdes_ai_core::{
    ModelRequest, ModelRequestPart, ModelResponse, ModelResponsePart,
    messages::{ImageContent, ImageMediaType, ThinkingPart, TextPart, UserContent, UserContentPart},
};

/// Build chat history from stored messages into serdesAI ModelRequest format.
///
/// For assistant messages with reasoning, creates multi-part responses
/// to support interleaved thinking (Claude) and thought blocks.
pub fn build_chat_history(chat_history: Vec<ChatHistoryMessage>) -> Vec<ModelRequest> {
    chat_history
        .into_iter()
        .filter_map(|msg| match msg.role {
            MessageRole::User => {
                let mut req = ModelRequest::new();
                req.add_user_prompt(UserContent::text(&msg.content));
                Some(req)
            }
            MessageRole::Assistant => {
                Some(build_assistant_request_with_reasoning(
                    &msg.content,
                    msg.reasoning.as_deref(),
                ))
            }
            _ => None,
        })
        .collect()
}

/// Build an assistant message request with optional reasoning content.
///
/// For interleaved thinking support, reasoning is included as a ThinkingPart
/// before the text content.
pub fn build_assistant_request_with_reasoning(
    text: &str,
    reasoning: Option<&str>,
) -> ModelRequest {
    let mut parts = Vec::new();

    // Add reasoning/thinking if present
    if let Some(reasoning_text) = reasoning {
        if !reasoning_text.is_empty() {
            parts.push(ModelResponsePart::Thinking(ThinkingPart::new(reasoning_text)));
        }
    }

    // Add text content if present
    if !text.is_empty() {
        parts.push(ModelResponsePart::Text(TextPart::new(text)));
    }

    // Wrap in a ModelResponse and then into a ModelRequest
    let response = ModelResponse::with_parts(parts);
    ModelRequest::with_parts(vec![ModelRequestPart::ModelResponse(Box::new(response))])
}

/// Parse media type string to ImageMediaType.
fn parse_image_media_type(media_type: &str) -> ImageMediaType {
    match media_type.to_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => ImageMediaType::Jpeg,
        "image/png" => ImageMediaType::Png,
        "image/gif" => ImageMediaType::Gif,
        "image/webp" => ImageMediaType::Webp,
        _ => ImageMediaType::Png, // Default to PNG
    }
}

/// Build user message with optional images.
pub fn build_user_message(
    user_message: &str,
    image_data: Vec<(String, String)>,
) -> ModelRequest {
    let mut req = ModelRequest::new();

    if image_data.is_empty() {
        req.add_user_prompt(UserContent::text(user_message));
    } else {
        // Build multi-part content with images
        let mut parts: Vec<UserContentPart> = Vec::new();

        for (media_type, base64_data) in image_data {
            // Decode base64 to binary
            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&base64_data) {
                let image_type = parse_image_media_type(&media_type);
                parts.push(UserContentPart::Image { 
                    image: ImageContent::binary(bytes, image_type) 
                });
            } else {
                tracing::warn!("Failed to decode base64 image data");
            }
        }

        if !user_message.is_empty() {
            parts.push(UserContentPart::text(user_message));
        }

        req.add_user_prompt(UserContent::parts(parts));
    }

    req
}

/// Extract the last user message text from history, returning the remaining history.
///
/// This is needed because some APIs require the actual user message to be
/// passed separately from the history.
pub fn extract_last_user_message(
    mut history: Vec<ModelRequest>,
) -> (Vec<ModelRequest>, String) {
    // Find the last request that contains a user prompt
    if let Some(pos) = history.iter().rposition(|req| {
        req.parts.iter().any(|p| matches!(p, ModelRequestPart::UserPrompt(_)))
    }) {
        let last_req = history.remove(pos);
        
        // Extract text from user prompts
        let text: String = last_req.parts
            .iter()
            .filter_map(|p| {
                if let ModelRequestPart::UserPrompt(user) = p {
                    user.content.as_text().map(String::from)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        
        (history, text)
    } else {
        (history, String::new())
    }
}

/// Prepend system prompt to the first user message.
/// Used for providers that don't support separate system prompts.
pub fn prepend_system_to_first_user_message(
    system_prompt: &str,
    history: &mut [ModelRequest],
) {
    if system_prompt.is_empty() {
        return;
    }

    for req in history.iter_mut() {
        for part in req.parts.iter_mut() {
            if let ModelRequestPart::UserPrompt(user) = part {
                if let Some(text) = user.content.as_text() {
                    user.content = UserContent::text(format!("{}\n\n{}", system_prompt, text));
                    return;
                }
            }
        }
    }
}

/// Check if an error message indicates a rate limit.
pub fn is_rate_limit_error(message: &str) -> bool {
    let lowered = message.to_lowercase();
    lowered.contains("429")
        || lowered.contains("rate limit")
        || lowered.contains("too many requests")
        || lowered.contains("quota")
}
