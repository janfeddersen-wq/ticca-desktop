//! Session management for saving and loading chat sessions
//!
//! Handles persistence of chat sessions to the database.

use iced::widget::markdown;

use ticca_core::agents::AgentType;
use ticca_core::session::{Session, SessionMessageInput, SessionService};
use ticca_core::tools::TodoListState;

use crate::chat_message::ChatMessage;
use std::collections::HashMap;

/// Session manager data returned after loading a session
pub struct LoadedSession {
    pub messages: Vec<ChatMessage>,
    pub session: Session,
    pub agent_type: Option<AgentType>,
    pub todo_lists: HashMap<usize, TodoListState>,
}

/// Load a session from the database
pub fn load_session(session_id: &str) -> Option<LoadedSession> {
    let loaded = SessionService::load(session_id).ok()??;
    let session = loaded.session;
    let messages = loaded.messages;

    // Convert session messages to chat messages
    let chat_messages: Vec<ChatMessage> = messages
        .iter()
        .map(|m| {
            let parsed_items = markdown::parse(&m.content).collect();
            ChatMessage {
                role: m.role,
                content: m.content.clone(),
                is_streaming: false,
                author_label: None,
                reasoning: None,
                parsed_items,
                last_was_tool_call: false,
            }
        })
        .collect();

    // Parse agent type
    let agent_type = AgentType::parse(&session.agent_type);

    let mut todo_lists = SessionService::load_todo_lists(&session.id)
        .unwrap_or_else(|error| {
            tracing::warn!("Failed to load todo lists for session {}: {}", session.id, error);
            HashMap::new()
        });
    todo_lists.retain(|node_id, _| *node_id == 0);

    Some(LoadedSession {
        messages: chat_messages,
        session,
        agent_type,
        todo_lists,
    })
}

/// Save a session to the database
pub fn save_session(
    current_session: Option<&Session>,
    messages: &[ChatMessage],
    current_agent: AgentType,
    todo_lists: &HashMap<usize, TodoListState>,
) -> Option<Session> {
    if messages.is_empty() {
        return None;
    }

    let inputs: Vec<SessionMessageInput> = messages
        .iter()
        .filter(|m| !m.is_streaming)
        .map(|m| SessionMessageInput {
            role: m.role,
            content: m.content.clone(),
        })
        .collect();

    let session = SessionService::save(current_session, &inputs, current_agent.as_str())
        .ok()
        .flatten()?;

    let mut to_persist = HashMap::new();
    if let Some(state) = todo_lists.get(&0) {
        to_persist.insert(0, state.clone());
    }
    if let Err(error) = SessionService::save_todo_lists(&session.id, &to_persist) {
        tracing::warn!(
            "Failed to save todo lists for session {}: {}",
            session.id,
            error
        );
    }

    Some(session)
}
