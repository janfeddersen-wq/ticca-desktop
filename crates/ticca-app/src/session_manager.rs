//! Session management for saving and loading chat sessions
//!
//! Handles persistence of chat sessions to the database.

use uuid::Uuid;
use chrono::{Local, Utc};
use iced::widget::markdown;

use ticca_core::agents::AgentType;
use ticca_core::session::{Session, SessionDatabase, SessionMessage, MessageRole};

use crate::chat_message::ChatMessage;

/// Session manager data returned after loading a session
pub struct LoadedSession {
    pub messages: Vec<ChatMessage>,
    pub session: Session,
    pub agent_type: Option<AgentType>,
}

/// Load a session from the database
pub fn load_session(session_id: &str) -> Option<LoadedSession> {
    let db = SessionDatabase::open().ok()?;
    let session = db.get_session(session_id).ok()??;
    let messages = db.get_messages(session_id).ok()?;

    // Convert session messages to chat messages
    let chat_messages: Vec<ChatMessage> = messages
        .iter()
        .map(|m| {
            let parsed_items = markdown::parse(&m.content).collect();
            ChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
                is_streaming: false,
                reasoning: None,
                parsed_items,
                last_was_tool_call: false,
            }
        })
        .collect();

    // Parse agent type
    let agent_type = AgentType::from_str(&session.agent_type);

    Some(LoadedSession {
        messages: chat_messages,
        session,
        agent_type,
    })
}

/// Save a session to the database
pub fn save_session(
    current_session: Option<&Session>,
    messages: &[ChatMessage],
    current_agent: AgentType,
) -> Option<Session> {
    if messages.is_empty() {
        return None;
    }

    let db = SessionDatabase::open().ok()?;

    // Create or get session ID
    let session_id = current_session
        .map(|s| s.id.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Create session name from first user message or current time
    let session_name = messages
        .iter()
        .find(|m| m.role == MessageRole::User)
        .map(|m| {
            let preview: String = m.content.chars().take(50).collect();
            if m.content.len() > 50 {
                format!("{}...", preview)
            } else {
                preview
            }
        })
        .unwrap_or_else(|| format!("Session {}", Local::now().format("%Y-%m-%d %H:%M")));

    let session = Session {
        id: session_id.clone(),
        name: session_name,
        agent_type: current_agent.as_str().to_string(),
        created_at: current_session.and_then(|s| s.created_at.clone()),
        updated_at: Some(Utc::now().to_rfc3339()),
        total_tokens: 0,
        message_count: messages.len() as i64,
    };

    // Try to create, or update if exists
    if current_session.is_none() {
        if let Err(e) = db.create_session(&session) {
            tracing::warn!("Failed to create session: {}", e);
        }
    }

    // Clear existing messages and re-add
    let _ = db.clear_session_messages(&session_id);

    // Save all messages
    for msg in messages {
        if msg.is_streaming {
            continue; // Skip streaming messages
        }
        let session_msg = SessionMessage {
            id: None,
            session_id: session_id.clone(),
            role: msg.role,
            content: msg.content.clone(),
            tool_calls_json: None,
            tool_result_json: None,
            tokens: 0,
            created_at: None,
        };
        if let Err(e) = db.add_message(&session_msg) {
            tracing::warn!("Failed to save message: {}", e);
        }
    }

    tracing::debug!(
        "Saved session {} with {} messages",
        session_id,
        messages.len()
    );

    Some(session)
}
