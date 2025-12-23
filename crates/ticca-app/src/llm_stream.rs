//! LLM streaming adapter (UI <-> core runner).
//!
//! The streaming engine lives in `ticca-core`; this module converts between
//! app-specific types (`ChatMessage`, `Message`) and core runner events.

use crate::chat_message::ChatMessage;
use crate::messages::{Message, chat};

use ticca_core::agents::{AgentType, ChatHistoryMessage, RunnerEvent};
use ticca_core::tools::{TodoListState, ToolApprovalDecision};

use futures::StreamExt;
use std::path::PathBuf;
use tokio::sync::mpsc;

impl From<RunnerEvent> for Message {
    fn from(value: RunnerEvent) -> Self {
        match value {
            RunnerEvent::StreamChunk(chunk) => Message::Chat(chat::Msg::StreamChunk(chunk)),
            RunnerEvent::Reasoning(text) => Message::Chat(chat::Msg::Reasoning(text)),
            RunnerEvent::StreamStats {
                chars_in_window,
                window_ms,
            } => Message::Chat(chat::Msg::StreamStats {
                chars_in_window,
                window_ms,
            }),
            RunnerEvent::ToolCall { name, args } => {
                Message::Chat(chat::Msg::ToolCall { name, args })
            }
            RunnerEvent::AgentCall(event) => Message::Chat(chat::Msg::AgentCall(event)),
            RunnerEvent::SubagentStream(event) => Message::Chat(chat::Msg::SubagentStream(event)),
            RunnerEvent::TodoEvent(event) => Message::Chat(chat::Msg::TodoEvent(event)),
            RunnerEvent::ToolApprovalRequested { id, name, args } => {
                Message::Chat(chat::Msg::ToolApprovalRequested { id, name, args })
            }
            RunnerEvent::Usage {
                input_tokens,
                output_tokens,
            } => Message::Chat(chat::Msg::Usage {
                input_tokens,
                output_tokens,
            }),
            RunnerEvent::StreamComplete => Message::Chat(chat::Msg::StreamComplete),
            RunnerEvent::StreamStopped => Message::Chat(chat::Msg::StreamStopped),
            RunnerEvent::StreamError(error) => Message::Chat(chat::Msg::StreamError(error)),
        }
    }
}

/// Run the core Rig agent runner and emit UI messages.
#[allow(clippy::too_many_arguments)]
pub fn run_rig_agent_stream(
    system_prompt: String,
    user_message: String,
    model_name: Option<String>,
    working_directory: PathBuf,
    max_tool_rounds: u32,
    chat_history: Vec<ChatMessage>,
    initial_todo_state: Option<TodoListState>,
    image_data: Vec<(String, String)>,
    yolo_mode_enabled: bool,
    current_agent: AgentType,
    approval_decision_rx: mpsc::UnboundedReceiver<ToolApprovalDecision>,
    cancel_rx: tokio::sync::oneshot::Receiver<()>,
    system_exec_store: std::sync::Arc<ticca_core::tools::SystemExecStore>,
    system_exec_tx: mpsc::UnboundedSender<ticca_core::tools::SystemExecRequest>,
) -> impl futures::Stream<Item = Message> {
    let history: Vec<ChatHistoryMessage> = chat_history
        .into_iter()
        .map(|msg| ChatHistoryMessage {
            role: msg.role,
            content: msg.content,
        })
        .collect();

    ticca_core::agents::run_rig_agent_stream(
        system_prompt,
        user_message,
        model_name,
        working_directory,
        max_tool_rounds,
        history,
        initial_todo_state,
        image_data,
        yolo_mode_enabled,
        current_agent,
        approval_decision_rx,
        cancel_rx,
        system_exec_store,
        system_exec_tx,
    )
    .map(Message::from)
}
