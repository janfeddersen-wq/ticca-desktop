//! GPUI-compatible LLM streaming adapter
//!
//! This module handles streaming LLM responses in a way that works with GPUI's
//! async model. For now, we use a simple synchronous approach that works within
//! the GPUI render cycle.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Handle for controlling a streaming operation
#[derive(Clone)]
pub struct StreamHandle {
    /// Flag to signal cancellation
    cancelled: Arc<AtomicBool>,
}

impl StreamHandle {
    /// Create a new stream handle
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Cancel the streaming operation
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Default for StreamHandle {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a demo response based on the prompt
pub fn generate_demo_response(prompt: &str) -> String {
    let prompt_lower = prompt.to_lowercase();

    if prompt_lower.contains("rust") || prompt_lower.contains("code") {
        r#"# Rust Code Example 🦀

Here's a simple Rust example:

```rust
use std::collections::HashMap;

fn main() {
    let mut scores = HashMap::new();
    
    scores.insert("Blue", 10);
    scores.insert("Yellow", 50);
    
    for (team, score) in &scores {
        println!("{}: {}", team, score);
    }
}
```

## Key Points

- **HashMaps** store key-value pairs
- Use `insert()` to add entries
- Iterate with `for` loops

Want me to explain any part in more detail?"#
            .to_string()
    } else if prompt_lower.contains("help") {
        r#"# How Can I Help? 🤝

I'm your **AI coding assistant**. I can help you with:

## Code Tasks
- ✍️ Writing new code
- 🔍 Reviewing existing code
- 🐛 Debugging issues
- 📝 Documentation

## Languages I Know
- Rust 🦀
- Python 🐍
- TypeScript/JavaScript
- Go, C++, and more!

## How to Use Me

1. **Ask a question** - I'll do my best to answer
2. **Share code** - I can review and improve it
3. **Describe a problem** - I'll help solve it

What would you like to work on?"#
            .to_string()
    } else if prompt_lower.contains("hello") || prompt_lower.contains("hi") {
        r#"# Hello! 👋

Great to meet you! I'm **Ticca**, your AI coding assistant.

I'm running on the new **GPUI** interface - much faster and smoother!

How can I help you today? Feel free to:
- Ask coding questions
- Share code for review
- Request help with a project

```
🐶 Woof! Ready to code!
```"#
            .to_string()
    } else {
        format!(
            r#"# Understanding Your Request

You asked: *"{}"*

Let me think about this...

## My Analysis

I'm processing your request. Here's what I can do:

1. **Analyze** the problem
2. **Research** best practices
3. **Generate** a solution
4. **Explain** the approach

## Next Steps

Could you provide more details about:
- What you're trying to accomplish?
- Any specific requirements?
- Error messages (if debugging)?

I'm here to help! 🚀"#,
            prompt
        )
    }
}
