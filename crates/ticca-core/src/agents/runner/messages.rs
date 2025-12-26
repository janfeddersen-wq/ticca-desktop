//! Message building utilities for rig integration.

use super::types::ChatHistoryMessage;
use crate::session::MessageRole;

/// Build chat history from stored messages.
///
/// For assistant messages with reasoning, creates multi-content messages
/// to support interleaved thinking (Claude) and thought blocks (Gemini).
pub fn build_chat_history(chat_history: Vec<ChatHistoryMessage>) -> Vec<rig::message::Message> {
    use rig::message::Message as RigMessage;

    chat_history
        .into_iter()
        .filter_map(|msg| match msg.role {
            MessageRole::User => Some(RigMessage::user(&msg.content)),
            MessageRole::Assistant => Some(build_assistant_message_with_reasoning(
                &msg.content,
                msg.reasoning.as_deref(),
                msg.reasoning_signature.as_deref(),
            )),
            _ => None,
        })
        .collect()
}

/// Build an assistant message with optional reasoning content.
///
/// For interleaved thinking support, reasoning is included as a separate
/// content block before the text content. The signature is preserved for
/// Claude's thinking verification.
pub fn build_assistant_message_with_reasoning(
    text: &str,
    reasoning: Option<&str>,
    signature: Option<&str>,
) -> rig::message::Message {
    use rig::message::{AssistantContent, Message as RigMessage, Reasoning};
    use rig::one_or_many::OneOrMany;

    match reasoning {
        Some(reasoning) if !reasoning.is_empty() => {
            let mut contents = Vec::new();
            let reasoning_content =
                Reasoning::new(reasoning).with_signature(signature.map(String::from));
            contents.push(AssistantContent::Reasoning(reasoning_content));
            if !text.is_empty() {
                contents.push(AssistantContent::text(text));
            }
            OneOrMany::many(contents)
                .map(|content| RigMessage::Assistant { id: None, content })
                .unwrap_or_else(|_| RigMessage::assistant(text))
        }
        _ => RigMessage::assistant(text),
    }
}

/// Build user message with optional images.
pub fn build_user_message(
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

/// Prepend system prompt to the first user message (for providers that don't support system prompts).
pub fn prepend_system_to_first_user_message(
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

/// Check if an error message indicates a rate limit.
pub fn is_rate_limit_error(message: &str) -> bool {
    let lowered = message.to_lowercase();
    lowered.contains("429")
        || lowered.contains("rate limit")
        || lowered.contains("too many requests")
        || lowered.contains("quota")
}
