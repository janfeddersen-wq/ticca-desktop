//! Fast token estimation without external dependencies.
//!
//! Uses character-based heuristics optimized for code-heavy content.
//! The 3.4 chars/token ratio accounts for code's higher symbol density
//! compared to natural language prose (~4.0 chars/token).

use crate::completion::Message;
use crate::completion::message::{AssistantContent, UserContent, ToolResultContent};

/// Characters per token ratio, optimized for code-heavy content.
/// Natural language is typically ~4.0, code is ~3.0-3.5.
const CHARS_PER_TOKEN: f32 = 3.4;

/// Overhead tokens per message for role and formatting.
const MESSAGE_OVERHEAD: usize = 4;

/// Estimate token count for a text string.
#[inline]
pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    (text.len() as f32 / CHARS_PER_TOKEN).ceil() as usize
}

/// Estimate token count for a single message.
pub fn estimate_message_tokens(message: &Message) -> usize {
    let content_tokens: usize = match message {
        Message::User { content } => {
            content.iter().map(estimate_user_content_tokens).sum()
        }
        Message::Assistant { content, .. } => {
            content.iter().map(estimate_assistant_content_tokens).sum()
        }
    };
    content_tokens + MESSAGE_OVERHEAD
}

/// Estimate token count for a sequence of messages.
pub fn estimate_messages_tokens(messages: &[Message]) -> usize {
    messages.iter().map(estimate_message_tokens).sum()
}

fn estimate_user_content_tokens(content: &UserContent) -> usize {
    match content {
        UserContent::Text(t) => estimate_tokens(&t.text),
        UserContent::ToolResult(tr) => {
            // Tool result ID + content
            estimate_tokens(&tr.id)
                + tr.content
                    .iter()
                    .map(|c| match c {
                        ToolResultContent::Text(t) => estimate_tokens(&t.text),
                        ToolResultContent::Image(_) => 85, // Base64 images are ~85 tokens for the reference
                    })
                    .sum::<usize>()
        }
        UserContent::Image(_) => 85,  // Typical image token estimate
        UserContent::Audio(_) => 100, // Audio reference tokens
        UserContent::Video(_) => 100, // Video reference tokens
        UserContent::Document(d) => estimate_tokens(&d.data.to_string()),
    }
}

fn estimate_assistant_content_tokens(content: &AssistantContent) -> usize {
    match content {
        AssistantContent::Text(t) => estimate_tokens(&t.text),
        AssistantContent::ToolCall(tc) => {
            // Tool name + arguments (usually JSON)
            estimate_tokens(&tc.function.name)
                + estimate_tokens(&tc.function.arguments.to_string())
        }
        AssistantContent::Reasoning(r) => {
            r.reasoning.iter().map(|s| estimate_tokens(s)).sum()
        }
        AssistantContent::Image(_) => 85,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_tokens_short() {
        // "hello" = 5 chars / 3.4 = 1.47 -> ceil = 2
        assert_eq!(estimate_tokens("hello"), 2);
    }

    #[test]
    fn test_estimate_tokens_code() {
        // Typical code line
        let code = "fn main() { println!(\"Hello, world!\"); }";
        let tokens = estimate_tokens(code);
        // 42 chars / 3.4 = 12.35 -> ceil = 13
        assert_eq!(tokens, 13);
    }

    #[test]
    fn test_estimate_tokens_longer() {
        // 340 chars should be ~100 tokens
        let text = "a".repeat(340);
        assert_eq!(estimate_tokens(&text), 100);
    }
}
