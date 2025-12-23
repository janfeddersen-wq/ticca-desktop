# Real-Time Token Usage Display in Rig.rs

## Rig's streaming architecture: usage arrives only at stream end

Rig's streaming uses the `RawStreamingChoice<T>` enum with three variants. **Usage data appears exclusively in `FinalResponse(T)`** at stream completion:

```rust
pub enum RawStreamingChoice<T> {
    Message(String),                // Text chunk - NO usage
    ToolCall(RawStreamingToolCall), // Tool call - NO usage  
    FinalResponse(T),               // Final item - CONTAINS usage
}
```

The `Usage` struct and access trait:

```rust
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

pub trait GetTokenUsage {
    fn token_usage(&self) -> Option<Usage>;
}
```

**Key limitation**: Input token counts cannot be known until the API call completes. For real-time progress during streaming, you must use pre-flight estimation.

## Multi-turn agent loops require manual usage aggregation

Rig provides **no automatic usage aggregation** across tool-calling loops. Each API call returns usage for that single request only:

```rust
let mut total_usage = Usage::default();

while let Some(item) = stream.next().await {
    match item {
        Ok(RawStreamingChoice::Message(text)) => {
            print!("{}", text);
        }
        Ok(RawStreamingChoice::FinalResponse(response)) => {
            if let Some(usage) = response.token_usage() {
                total_usage.input_tokens += usage.input_tokens;
                total_usage.output_tokens += usage.output_tokens;
                total_usage.total_tokens += usage.total_tokens;
                
                update_context_display(total_usage, max_context);
            }
        }
        _ => {}
    }
}
```

## Fast token estimation using character ratio

A simple character-to-token ratio provides fast estimates without external dependencies. The common ratio is **~4 characters per token** for English text (use ~3 for code-heavy content to be conservative):

```rust
const CHARS_PER_TOKEN: usize = 4;
const MSG_OVERHEAD_TOKENS: usize = 4; // role, separators, framing

fn estimate_tokens(text: &str) -> usize {
    (text.len() + CHARS_PER_TOKEN - 1) / CHARS_PER_TOKEN
}

fn estimate_message_tokens(message: &Message) -> usize {
    let content_tokens = match message {
        Message::User { content } => {
            content.iter().map(|c| match c {
                UserContent::Text(t) => estimate_tokens(&t.text),
                UserContent::ToolResult(tr) => {
                    estimate_tokens(&tr.id) + 
                    tr.content.iter().map(|c| estimate_tokens(&c.to_string())).sum::<usize>()
                }
                _ => 50 // Conservative for media
            }).sum()
        }
        Message::Assistant { content, .. } => {
            content.iter().map(|c| match c {
                AssistantContent::Text(t) => estimate_tokens(&t.text),
                AssistantContent::ToolCall(tc) => {
                    estimate_tokens(&tc.function.name) +
                    estimate_tokens(&tc.function.arguments.to_string())
                }
                _ => 50
            }).sum()
        }
    };
    
    MSG_OVERHEAD_TOKENS + content_tokens
}
```

## Context utilization tracker

```rust
pub struct ContextTracker {
    model_context_window: usize,
    cumulative_input_tokens: u64,
    cumulative_output_tokens: u64,
}

impl ContextTracker {
    pub fn estimate_utilization(&self, preamble: &str, history: &[Message], prompt: &str) -> f64 {
        let total: usize = estimate_tokens(preamble)
            + history.iter().map(estimate_message_tokens).sum::<usize>()
            + estimate_tokens(prompt);
        total as f64 / self.model_context_window as f64
    }

    pub fn record_usage(&mut self, usage: &Usage) {
        self.cumulative_input_tokens += usage.input_tokens;
        self.cumulative_output_tokens += usage.output_tokens;
    }

    pub fn format_display(&self, current_usage: &Usage) -> String {
        let pct = (current_usage.input_tokens as f64 / self.model_context_window as f64) * 100.0;
        format!("Context: {:.1}% ({}/{}) | Session: {}in/{}out",
            pct, current_usage.input_tokens, self.model_context_window,
            self.cumulative_input_tokens, self.cumulative_output_tokens)
    }
}
```

## Summary

| When | What's available | Source |
|------|------------------|--------|
| Before API call | Estimated tokens | `chars / 4` |
| During streaming | Text chunks only | `RawStreamingChoice::Message` |
| At stream end | Actual usage | `RawStreamingChoice::FinalResponse` |

**Pattern**: Show estimate (chars/4) → stream text → update with real usage from `FinalResponse` → accumulate across turns manually.
