//! Simple CLI mode for Ticca
//!
//! A readline-style interface that prints directly to the terminal.
//! No alternate screen, native text selection works, normal scrollback.

mod app;

use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::execute;
use crossterm::terminal;
use futures::StreamExt;
use tokio::sync::mpsc;

use streamdown_parser::Parser as MdParser;
use streamdown_render::Renderer as MdRenderer;

use ticca_core::agents::{
    AgentProfile, AgentType, ChatHistoryMessage, RunnerEvent, run_agent_stream,
};
use ticca_core::config::{ConfigDatabase, setting_keys};
use ticca_core::session::MessageRole;
use ticca_core::tools::{SystemExecStore, ToolApprovalDecision};

/// Configuration for the CLI
#[derive(Default)]
pub struct TuiConfig {
    pub initial_prompt: Option<String>,
    pub working_directory: Option<PathBuf>,
    pub model_override: Option<String>,
    pub yolo_override: bool,
}

/// Simple CLI state
struct CliState {
    working_directory: PathBuf,
    current_agent: AgentType,
    yolo_mode: bool,
    default_model: Option<String>,
    chat_history: Vec<ChatHistoryMessage>,
    terminal_width: usize,
}

impl CliState {
    fn new(
        working_directory: Option<PathBuf>,
        model_override: Option<String>,
        yolo_override: bool,
    ) -> Self {
        let wd = working_directory
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let width = terminal::size().map(|(w, _)| w as usize).unwrap_or(80);

        // Load settings
        let (yolo_from_settings, default_model) = load_settings();

        Self {
            working_directory: wd,
            current_agent: AgentType::Coding,
            yolo_mode: yolo_override || yolo_from_settings,
            default_model: model_override.or(default_model),
            chat_history: Vec::new(),
            terminal_width: width,
        }
    }
}

fn load_settings() -> (bool, Option<String>) {
    let db = match ConfigDatabase::open() {
        Ok(db) => db,
        Err(_) => return (false, None),
    };

    let yolo = db
        .get_setting(setting_keys::YOLO_MODE)
        .ok()
        .flatten()
        .map(|s| s.value == "true")
        .unwrap_or(false);

    let model = db
        .get_setting(setting_keys::DEFAULT_MODEL)
        .ok()
        .flatten()
        .map(|s| s.value)
        .filter(|s| !s.trim().is_empty());

    (yolo, model)
}

/// Run the simple CLI
pub fn run(config: TuiConfig) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    let mut state = CliState::new(
        config.working_directory,
        config.model_override,
        config.yolo_override,
    );

    // Print welcome banner
    print_banner(&state);

    // Handle initial prompt if provided
    if let Some(prompt) = config.initial_prompt {
        rt.block_on(handle_prompt(&mut state, &prompt))?;
    }

    // Main REPL loop
    loop {
        // Print prompt
        print_prompt(&state);

        // Read input
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim();

        // Handle special commands
        if input.is_empty() {
            continue;
        }

        if input == "/quit" || input == "/exit" || input == "/q" {
            println!();
            print_colored("👋 Goodbye!\n", Color::Cyan);
            break;
        }

        if input == "/help" || input == "/?" {
            print_help();
            continue;
        }

        if input == "/clear" {
            state.chat_history.clear();
            print_colored("🗑️  Chat history cleared.\n", Color::Yellow);
            continue;
        }

        if let Some(agent_name) = input.strip_prefix("/agent ") {
            let agent_name = agent_name.trim();
            match agent_name.to_lowercase().as_str() {
                "coding" | "code" => {
                    state.current_agent = AgentType::Coding;
                    print_colored("🔧 Switched to Coding agent.\n", Color::Green);
                }
                "planning" | "plan" => {
                    state.current_agent = AgentType::Planning;
                    print_colored("📋 Switched to Planning agent.\n", Color::Green);
                }
                _ => {
                    print_colored(
                        &format!("❌ Unknown agent: {}. Use 'coding' or 'planning'.\n", agent_name),
                        Color::Red,
                    );
                }
            }
            continue;
        }

        if input == "/yolo" {
            state.yolo_mode = !state.yolo_mode;
            if state.yolo_mode {
                print_colored("⚡ YOLO mode enabled - tools will auto-approve.\n", Color::Yellow);
            } else {
                print_colored("🛡️  YOLO mode disabled - tools will require approval.\n", Color::Green);
            }
            continue;
        }

        if input == "/test" {
            // Test streamdown rendering
            println!();
            print_colored("Testing streamdown markdown rendering:\n\n", Color::Cyan);
            let test_md = r#"# Heading 1

This is **bold** and *italic* text.

## Heading 2

- List item 1
- List item 2
- List item 3

```rust
fn main() {
    println!("Hello, world!");
}
```

> This is a blockquote

| Column A | Column B |
|----------|----------|
| Value 1  | Value 2  |
"#;
            render_markdown(test_md, state.terminal_width)?;
            println!();
            continue;
        }

        // Handle as a message to the LLM
        rt.block_on(handle_prompt(&mut state, input))?;
    }

    Ok(())
}

fn print_banner(state: &CliState) {
    println!();
    print_colored("🐶 Ticca CLI", Color::Cyan);
    println!(" - AI-assisted coding");
    print_colored(
        &format!("📁 {}\n", state.working_directory.display()),
        Color::DarkGrey,
    );

    if let Some(model) = &state.default_model {
        print_colored(
            &format!("📦 Model: {}\n", shorten_model_name(model)),
            Color::DarkGrey,
        );
    }

    if state.yolo_mode {
        print_colored("⚡ YOLO mode enabled\n", Color::Yellow);
    }

    print_colored("Type /help for commands, /quit to exit.\n\n", Color::DarkGrey);
}

fn print_prompt(state: &CliState) {
    let agent_icon = match state.current_agent {
        AgentType::Coding => "🔧",
        AgentType::Planning => "📋",
        _ => "🤖",
    };

    let mut stdout = io::stdout();
    let _ = execute!(
        stdout,
        SetForegroundColor(Color::Cyan),
        Print(format!("{} ", agent_icon)),
        SetForegroundColor(Color::Green),
        Print("❯ "),
        ResetColor
    );
    let _ = stdout.flush();
}

fn print_help() {
    println!();
    print_colored("Commands:\n", Color::Cyan);
    println!("  /help, /?        Show this help");
    println!("  /quit, /exit, /q Exit the CLI");
    println!("  /clear           Clear chat history");
    println!("  /agent <name>    Switch agent (coding, planning)");
    println!("  /yolo            Toggle YOLO mode (auto-approve tools)");
    println!("  /test            Test markdown rendering");
    println!();
    print_colored("Tips:\n", Color::Cyan);
    println!("  • Just type your message and press Enter");
    println!("  • Use Ctrl+C to cancel a streaming response");
    println!("  • Text selection and copy works normally");
    println!();
}

fn print_colored(text: &str, color: Color) {
    let mut stdout = io::stdout();
    let _ = execute!(stdout, SetForegroundColor(color), Print(text), ResetColor);
}

fn shorten_model_name(name: &str) -> String {
    if let Some(pos) = name.rfind("-20") {
        if name.len() > pos + 3
            && name[pos + 1..]
                .chars()
                .all(|c| c.is_ascii_digit() || c == '-')
        {
            return name[..pos].to_string();
        }
    }
    if name.len() > 30 {
        format!("{}...", &name[..27])
    } else {
        name.to_string()
    }
}

async fn handle_prompt(state: &mut CliState, prompt: &str) -> Result<()> {
    // Add user message to history
    state.chat_history.push(ChatHistoryMessage {
        role: MessageRole::User,
        content: prompt.to_string(),
        reasoning: None,
        reasoning_signature: None,
    });

    // Print assistant header
    println!();
    print_colored("🤖 Assistant:\n", Color::Green);

    // Get system prompt
    let profile = AgentProfile::for_type(state.current_agent, 30);
    let system_prompt = profile.system_prompt.clone();

    // Create channels
    let (approval_tx, approval_rx) = mpsc::unbounded_channel::<ToolApprovalDecision>();
    let (_cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();
    let system_exec_store = Arc::new(SystemExecStore::new());
    let (system_exec_tx, _) = mpsc::unbounded_channel();

    // Start the stream
    let stream = run_agent_stream(
        system_prompt,
        prompt.to_string(),
        state.default_model.clone(),
        state.working_directory.clone(),
        30,
        state.chat_history.clone(),
        None,
        vec![],
        state.yolo_mode,
        state.current_agent,
        approval_rx,
        cancel_rx,
        system_exec_store,
        system_exec_tx,
    );

    tokio::pin!(stream);

    // Streaming state
    let mut response_buffer = String::new();
    let mut line_buffer = String::new();
    let mut md_parser = MdParser::new(); // ONE parser for entire response - tracks block state
    let mut render_buf: Vec<u8> = Vec::new(); // Reusable render buffer
    let width = state.terminal_width.saturating_sub(4);
    let mut stdout = io::stdout();

    while let Some(event) = stream.next().await {
        match event {
            RunnerEvent::StreamChunk(text) => {
                response_buffer.push_str(&text);
                line_buffer.push_str(&text);

                // Process complete lines only (avoids splitting words)
                while let Some(newline_pos) = line_buffer.find('\n') {
                    let line = line_buffer[..newline_pos].to_string();
                    line_buffer = line_buffer[newline_pos + 1..].to_string();

                    // Parse line with persistent parser (maintains block state)
                    // Then render each event
                    for md_event in md_parser.parse_line(&line) {
                        render_buf.clear();
                        {
                            let mut renderer = MdRenderer::new(&mut render_buf, width);
                            let _ = renderer.render_event(&md_event);
                        }
                        let rendered = String::from_utf8_lossy(&render_buf);
                        print!("{}", rendered);
                    }
                }

                stdout.flush()?;
            }
            RunnerEvent::ToolCall { name, args } => {
                print_colored(&format!("\n🔧 Running {}...\n", name), Color::Yellow);
                if args.len() < 100 {
                    print_colored(&format!("   {}\n", args), Color::DarkGrey);
                }
            }
            RunnerEvent::ToolApprovalRequested { id, name, args } => {
                println!();
                print_colored(&format!("⚠️  Tool approval required: {}\n", name), Color::Yellow);
                if args.len() < 200 {
                    print_colored(&format!("   Args: {}\n", args), Color::DarkGrey);
                } else {
                    print_colored(&format!("   Args: {}...\n", &args[..200]), Color::DarkGrey);
                }
                print_colored("   Approve? [y/N]: ", Color::Yellow);
                stdout.flush()?;

                let mut response = String::new();
                io::stdin().read_line(&mut response)?;
                let approved = response.trim().to_lowercase() == "y";

                let _ = approval_tx.send(ToolApprovalDecision { id, approved });

                if approved {
                    print_colored("   ✅ Approved\n", Color::Green);
                } else {
                    print_colored("   ❌ Rejected\n", Color::Red);
                }
            }
            RunnerEvent::StreamComplete => {
                // Render any remaining buffered text
                if !line_buffer.is_empty() {
                    for md_event in md_parser.parse_line(&line_buffer) {
                        render_buf.clear();
                        {
                            let mut renderer = MdRenderer::new(&mut render_buf, width);
                            let _ = renderer.render_event(&md_event);
                        }
                        let rendered = String::from_utf8_lossy(&render_buf);
                        print!("{}", rendered);
                    }
                }

                // Finalize parser to close any open blocks (code blocks, lists, etc.)
                for md_event in md_parser.finalize() {
                    render_buf.clear();
                    {
                        let mut renderer = MdRenderer::new(&mut render_buf, width);
                        let _ = renderer.render_event(&md_event);
                    }
                    let rendered = String::from_utf8_lossy(&render_buf);
                    print!("{}", rendered);
                }

                stdout.flush()?;
                break;
            }
            RunnerEvent::StreamStopped => {
                print_colored("\n\n⏹️  Stopped by user.\n", Color::Yellow);
                break;
            }
            RunnerEvent::StreamError(error) => {
                print_colored(&format!("\n❌ Error: {}\n", error), Color::Red);
                break;
            }
            RunnerEvent::ContextEstimate {
                usage_percent,
                total_tokens,
                context_window,
                ..
            } => {
                if usage_percent > 70 {
                    print_colored(
                        &format!(
                            "\n⚠️  Context usage: {}% ({}/{})\n",
                            usage_percent, total_tokens, context_window
                        ),
                        if usage_percent > 85 {
                            Color::Red
                        } else {
                            Color::Yellow
                        },
                    );
                }
            }
            _ => {}
        }
    }

    // Add assistant response to history
    if !response_buffer.is_empty() {
        state.chat_history.push(ChatHistoryMessage {
            role: MessageRole::Assistant,
            content: response_buffer,
            reasoning: None,
            reasoning_signature: None,
        });
    }

    println!("\n");
    Ok(())
}

/// Render markdown content using streamdown
fn render_markdown(content: &str, width: usize) -> Result<()> {
    let mut parser = MdParser::new();
    let mut output = Vec::new();

    {
        let mut renderer = MdRenderer::new(&mut output, width.saturating_sub(4));

        // Parse and render each line
        for line in content.lines() {
            for event in parser.parse_line(line) {
                let _ = renderer.render_event(&event);
            }
        }

        // Finalize to close any open blocks (code blocks, lists, etc.)
        for event in parser.finalize() {
            let _ = renderer.render_event(&event);
        }
    }

    // Print the rendered output
    let rendered = String::from_utf8_lossy(&output);
    print!("{}", rendered);
    io::stdout().flush()?;

    Ok(())
}

// Keep TuiApp export for backwards compat but it's not really used anymore
pub use app::TuiApp;
