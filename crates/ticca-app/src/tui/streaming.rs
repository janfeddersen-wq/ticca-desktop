//! LLM Streaming Integration for TUI
//!
//! Handles spawning the agent runner and forwarding events to the TUI event system.

use std::path::PathBuf;
use std::sync::Arc;

use futures::StreamExt;
use tokio::sync::{mpsc, oneshot};

use crate::chat_message::ChatMessage;
use crate::llm_stream;
use crate::tui::event::Event;

use ticca_core::agents::AgentType;
use ticca_core::tools::{SystemExecStore, TodoListState, ToolApprovalDecision};

/// Start streaming a chat response
#[allow(clippy::too_many_arguments)]
pub fn start_streaming(
    event_tx: mpsc::UnboundedSender<Event>,
    system_prompt: String,
    user_message: String,
    model_name: Option<String>,
    working_directory: PathBuf,
    max_tool_rounds: u32,
    chat_history: Vec<ChatMessage>,
    initial_todo_state: Option<TodoListState>,
    image_data: Vec<(String, String)>,
    yolo_mode: bool,
    current_agent: AgentType,
    approval_rx: mpsc::UnboundedReceiver<ToolApprovalDecision>,
    cancel_rx: oneshot::Receiver<()>,
    system_exec_store: Arc<SystemExecStore>,
    system_exec_tx: mpsc::UnboundedSender<ticca_core::tools::SystemExecRequest>,
) {
    tokio::spawn(async move {
        let stream = llm_stream::run_rig_agent_stream(
            system_prompt,
            user_message,
            model_name,
            working_directory,
            max_tool_rounds,
            chat_history,
            initial_todo_state,
            image_data,
            yolo_mode,
            current_agent,
            approval_rx,
            cancel_rx,
            system_exec_store,
            system_exec_tx,
        );

        futures::pin_mut!(stream);

        while let Some(msg) = stream.next().await {
            let event = match msg {
                crate::messages::Message::Chat(chat_msg) => match chat_msg {
                    crate::messages::chat::Msg::StreamChunk(chunk) => Event::StreamChunk(chunk),
                    crate::messages::chat::Msg::StreamComplete => Event::StreamComplete,
                    crate::messages::chat::Msg::StreamStopped => Event::StreamStopped,
                    crate::messages::chat::Msg::StreamError(err) => Event::StreamError(err),
                    crate::messages::chat::Msg::ToolCall { name, args } => {
                        Event::ToolCall { name, args }
                    }
                    crate::messages::chat::Msg::ToolApprovalRequested { id, name, args } => {
                        Event::ToolApprovalRequested { id, name, args }
                    }
                    crate::messages::chat::Msg::Reasoning { text, signature } => {
                        Event::Reasoning { text, signature }
                    }
                    crate::messages::chat::Msg::Usage {
                        input_tokens,
                        output_tokens,
                    } => Event::Usage {
                        input_tokens,
                        output_tokens,
                    },
                    crate::messages::chat::Msg::AgentCall(agent_call) => Event::AgentCall(agent_call),
                    crate::messages::chat::Msg::SubagentStream(stream_event) => {
                        Event::SubagentStream(stream_event)
                    }
                    crate::messages::chat::Msg::TodoEvent(todo_event) => Event::TodoEvent(todo_event),
                    crate::messages::chat::Msg::ContextEstimate {
                        system_prompt_tokens,
                        tool_definitions_tokens,
                        messages_tokens,
                        total_tokens,
                        context_window,
                        usage_percent,
                    } => Event::ContextEstimate {
                        system_prompt_tokens,
                        tool_definitions_tokens,
                        messages_tokens,
                        total_tokens,
                        context_window,
                        usage_percent,
                    },
                    crate::messages::chat::Msg::ContextCompressed {
                        original_messages,
                        compressed_messages,
                        original_tokens,
                        compressed_tokens,
                        strategy,
                    } => Event::ContextCompressed {
                        original_messages,
                        compressed_messages,
                        original_tokens,
                        compressed_tokens,
                        strategy,
                    },
                    _ => continue,
                },
                _ => continue,
            };

            if event_tx.send(event).is_err() {
                break;
            }
        }
    });
}
