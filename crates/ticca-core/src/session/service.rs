use anyhow::Result;
use chrono::{Local, Utc};
use uuid::Uuid;

use crate::session::database::SessionDatabase;
use crate::session::models::{MessageRole, Session, SessionMessage};

pub struct SessionMessageInput {
    pub role: MessageRole,
    pub content: String,
}

pub struct LoadedSessionData {
    pub session: Session,
    pub messages: Vec<SessionMessage>,
}

pub struct SessionService;

impl SessionService {
    pub fn load(session_id: &str) -> Result<Option<LoadedSessionData>> {
        let db = SessionDatabase::open()?;
        let Some(session) = db.get_session(session_id)? else {
            return Ok(None);
        };
        let messages = db.get_messages(session_id)?;
        Ok(Some(LoadedSessionData { session, messages }))
    }

    pub fn list_recent(limit: usize) -> Result<Vec<Session>> {
        let db = SessionDatabase::open()?;
        let mut sessions = db.list_sessions()?;
        sessions.truncate(limit);
        Ok(sessions)
    }

    pub fn save(
        current_session: Option<&Session>,
        messages: &[SessionMessageInput],
        agent_type: &str,
    ) -> Result<Option<Session>> {
        if messages.is_empty() {
            return Ok(None);
        }

        let db = SessionDatabase::open()?;

        let session_id = current_session
            .map(|s| s.id.clone())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

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
            agent_type: agent_type.to_string(),
            created_at: current_session.and_then(|s| s.created_at.clone()),
            updated_at: Some(Utc::now().to_rfc3339()),
            total_tokens: 0,
            message_count: messages.len() as i64,
        };

        if current_session.is_none() {
            let _ = db.create_session(&session);
        }

        let _ = db.clear_session_messages(&session_id);

        for msg in messages {
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

        Ok(Some(session))
    }
}

