#![allow(dead_code)]

//! TUI Event Handling System
//!
//! Manages terminal events (keyboard, mouse, resize) and async events
//! from the LLM streaming and background tasks.

use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use tokio::sync::mpsc;

use ticca_core::tools::AgentCallEvent;

/// Application events
#[derive(Debug, Clone)]
pub enum Event {
    // === Terminal Events ===
    /// Keyboard input
    Key(KeyEvent),
    /// Mouse input
    Mouse(MouseEvent),
    /// Terminal resize
    Resize(u16, u16),

    // === Timer Events ===
    /// Animation/refresh tick (100ms)
    Tick,

    // === LLM Streaming Events ===
    /// Text chunk from LLM
    StreamChunk(String),
    /// Stream completed successfully
    StreamComplete,
    /// Stream stopped by user
    StreamStopped,
    /// Stream error occurred
    StreamError(String),

    // === Tool Events ===
    /// Tool is being called
    ToolCall { name: String, args: String },
    /// Tool requires approval
    ToolApprovalRequested { id: u64, name: String, args: String },
    /// Reasoning from LLM
    Reasoning { text: String, signature: Option<String> },

    // === Agent Events ===
    /// Sub-agent invocation
    AgentCall(AgentCallEvent),
    /// Sub-agent stream event
    SubagentStream(ticca_core::tools::AgentStreamEvent),
    /// Todo list event
    TodoEvent(ticca_core::tools::TodoListEvent),

    // === Token Usage ===
    /// Token usage update from API
    Usage { input_tokens: u64, output_tokens: u64 },
    /// Context estimate before request
    ContextEstimate {
        system_prompt_tokens: usize,
        tool_definitions_tokens: usize,
        messages_tokens: usize,
        total_tokens: usize,
        context_window: u64,
        usage_percent: u32,
    },
    /// Context was compressed
    ContextCompressed {
        original_messages: usize,
        compressed_messages: usize,
        original_tokens: usize,
        compressed_tokens: usize,
        strategy: String,
    },

    // === Background Task Events ===
    /// Models loaded from API
    ModelsLoaded(Result<Vec<String>, String>),
    /// OAuth flow completed
    OAuthComplete { provider: String, result: Result<(), String> },
    /// External tool install progress
    ExternalToolProgress { tool_id: ticca_core::ExternalToolId, progress: u8 },
    /// External tool install complete
    ExternalToolComplete { tool_id: ticca_core::ExternalToolId, result: Result<(), String> },
}

/// Event handler that merges terminal events with async channel events
pub struct EventHandler {
    /// Sender for async events
    tx: mpsc::UnboundedSender<Event>,
    /// Receiver for all events
    rx: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    /// Create a new event handler
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self { tx, rx }
    }

    /// Get a sender for async events
    pub fn sender(&self) -> mpsc::UnboundedSender<Event> {
        self.tx.clone()
    }

    /// Poll for the next event
    ///
    /// This combines terminal events with async events from the channel.
    /// Uses a 50ms tick rate for responsive animations while being efficient.
    pub async fn next(&mut self) -> Option<Event> {
        tokio::select! {
            biased;

            // Check for channel events first (higher priority)
            event = self.rx.recv() => {
                event
            }

            // Poll terminal events with timeout
            _ = tokio::time::sleep(Duration::from_millis(50)) => {
                // Check for terminal event
                if event::poll(Duration::ZERO).unwrap_or(false) {
                    match event::read() {
                        Ok(CrosstermEvent::Key(key)) => Some(Event::Key(key)),
                        Ok(CrosstermEvent::Mouse(mouse)) => Some(Event::Mouse(mouse)),
                        Ok(CrosstermEvent::Resize(w, h)) => Some(Event::Resize(w, h)),
                        Ok(CrosstermEvent::Paste(_)) => None, // Ignore paste events
                        Ok(CrosstermEvent::FocusGained) => None,
                        Ok(CrosstermEvent::FocusLost) => None,
                        Err(_) => None,
                    }
                } else {
                    // No terminal event, emit tick for animations
                    Some(Event::Tick)
                }
            }
        }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Keyboard shortcut definitions
#[derive(Debug, Clone, Copy)]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    pub const fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    pub fn matches(&self, key: &KeyEvent) -> bool {
        key.code == self.code && key.modifiers.contains(self.modifiers)
    }
}

/// Global keybindings
pub mod keys {
    use super::*;

    // Global
    pub const QUIT: KeyBinding = KeyBinding::new(KeyCode::Char('q'), KeyModifiers::NONE);
    pub const QUIT_CTRL: KeyBinding = KeyBinding::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
    pub const HELP: KeyBinding = KeyBinding::new(KeyCode::Char('?'), KeyModifiers::NONE);
    pub const HELP_F1: KeyBinding = KeyBinding::new(KeyCode::F(1), KeyModifiers::NONE);
    pub const ESCAPE: KeyBinding = KeyBinding::new(KeyCode::Esc, KeyModifiers::NONE);
    pub const SETTINGS: KeyBinding = KeyBinding::new(KeyCode::Char(','), KeyModifiers::CONTROL);
    // Alternative settings keybindings (CTRL+, may not work in all terminals)
    pub const SETTINGS_F10: KeyBinding = KeyBinding::new(KeyCode::F(10), KeyModifiers::NONE);
    pub const SETTINGS_F9: KeyBinding = KeyBinding::new(KeyCode::F(9), KeyModifiers::NONE);

    // Navigation
    pub const TAB: KeyBinding = KeyBinding::new(KeyCode::Tab, KeyModifiers::NONE);
    pub const BACKTAB: KeyBinding = KeyBinding::new(KeyCode::BackTab, KeyModifiers::SHIFT);
    pub const UP: KeyBinding = KeyBinding::new(KeyCode::Up, KeyModifiers::NONE);
    pub const DOWN: KeyBinding = KeyBinding::new(KeyCode::Down, KeyModifiers::NONE);
    pub const LEFT: KeyBinding = KeyBinding::new(KeyCode::Left, KeyModifiers::NONE);
    pub const RIGHT: KeyBinding = KeyBinding::new(KeyCode::Right, KeyModifiers::NONE);
    pub const PAGE_UP: KeyBinding = KeyBinding::new(KeyCode::PageUp, KeyModifiers::NONE);
    pub const PAGE_DOWN: KeyBinding = KeyBinding::new(KeyCode::PageDown, KeyModifiers::NONE);
    pub const HOME: KeyBinding = KeyBinding::new(KeyCode::Home, KeyModifiers::NONE);
    pub const END: KeyBinding = KeyBinding::new(KeyCode::End, KeyModifiers::NONE);
    pub const ENTER: KeyBinding = KeyBinding::new(KeyCode::Enter, KeyModifiers::NONE);

    // Chat specific
    pub const SEND: KeyBinding = KeyBinding::new(KeyCode::Enter, KeyModifiers::CONTROL);
    pub const SEND_ALT: KeyBinding = KeyBinding::new(KeyCode::Char('s'), KeyModifiers::ALT);
    pub const NEW_SESSION: KeyBinding = KeyBinding::new(KeyCode::Char('n'), KeyModifiers::CONTROL);
    pub const STOP_STREAM: KeyBinding = KeyBinding::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    pub const TOGGLE_YOLO: KeyBinding = KeyBinding::new(KeyCode::F(3), KeyModifiers::NONE);
    pub const PICK_AGENT: KeyBinding = KeyBinding::new(KeyCode::F(1), KeyModifiers::NONE);
    pub const PICK_MODEL: KeyBinding = KeyBinding::new(KeyCode::F(2), KeyModifiers::NONE);
    pub const WORKING_DIR: KeyBinding = KeyBinding::new(KeyCode::Char('w'), KeyModifiers::CONTROL);
    pub const AGENT_PLANNING: KeyBinding = KeyBinding::new(KeyCode::Char('1'), KeyModifiers::CONTROL);
    pub const AGENT_CODING: KeyBinding = KeyBinding::new(KeyCode::Char('2'), KeyModifiers::CONTROL);
    pub const AGENT_SKILLS: KeyBinding = KeyBinding::new(KeyCode::Char('3'), KeyModifiers::CONTROL);

    // Tool approval
    pub const APPROVE: KeyBinding = KeyBinding::new(KeyCode::Char('y'), KeyModifiers::NONE);
    pub const DENY: KeyBinding = KeyBinding::new(KeyCode::Char('n'), KeyModifiers::NONE);
    pub const ALWAYS_APPROVE: KeyBinding = KeyBinding::new(KeyCode::Char('a'), KeyModifiers::NONE);

    // Settings specific
    pub const TAB_1: KeyBinding = KeyBinding::new(KeyCode::Char('1'), KeyModifiers::NONE);
    pub const TAB_2: KeyBinding = KeyBinding::new(KeyCode::Char('2'), KeyModifiers::NONE);
    pub const TAB_3: KeyBinding = KeyBinding::new(KeyCode::Char('3'), KeyModifiers::NONE);
    pub const TAB_4: KeyBinding = KeyBinding::new(KeyCode::Char('4'), KeyModifiers::NONE);
    pub const TAB_5: KeyBinding = KeyBinding::new(KeyCode::Char('5'), KeyModifiers::NONE);
    pub const TAB_6: KeyBinding = KeyBinding::new(KeyCode::Char('6'), KeyModifiers::NONE);
    pub const TAB_7: KeyBinding = KeyBinding::new(KeyCode::Char('7'), KeyModifiers::NONE);
    pub const NEW_ITEM: KeyBinding = KeyBinding::new(KeyCode::Char('n'), KeyModifiers::NONE);
    pub const EDIT_ITEM: KeyBinding = KeyBinding::new(KeyCode::Char('e'), KeyModifiers::NONE);
    pub const DELETE_ITEM: KeyBinding = KeyBinding::new(KeyCode::Char('d'), KeyModifiers::NONE);
    pub const TOGGLE_SPACE: KeyBinding = KeyBinding::new(KeyCode::Char(' '), KeyModifiers::NONE);
    pub const REFRESH: KeyBinding = KeyBinding::new(KeyCode::Char('r'), KeyModifiers::NONE);
}
