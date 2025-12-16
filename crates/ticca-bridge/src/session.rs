//! Python bindings for session types
//!
//! This module provides Python-accessible session and message types.
//! These are standalone definitions to avoid circular dependencies with ticca-core.

use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use crate::agent::AgentId;

/// Unique identifier for a session
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new random session ID
    pub fn generate() -> Self {
        use std::time::UNIX_EPOCH;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        Self(format!("session-{timestamp:x}"))
    }
}

/// Role of a message sender
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: SystemTime,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            timestamp: SystemTime::now(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            timestamp: SystemTime::now(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            timestamp: SystemTime::now(),
        }
    }
}

/// A conversation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub agent_id: AgentId,
    pub title: Option<String>,
    pub messages: Vec<Message>,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

impl Session {
    pub fn new(agent_id: AgentId) -> Self {
        let now = SystemTime::now();
        Self {
            id: SessionId::generate(),
            agent_id,
            title: None,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.updated_at = SystemTime::now();
    }
}

/// Python-exposed session
#[pyclass(name = "Session")]
pub struct PySession {
    inner: Session,
}

#[pymethods]
impl PySession {
    #[new]
    fn new(agent_id: String) -> Self {
        Self {
            inner: Session::new(AgentId::new(agent_id)),
        }
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.0.clone()
    }

    #[getter]
    fn agent_id(&self) -> String {
        self.inner.agent_id.0.clone()
    }

    #[getter]
    fn title(&self) -> Option<String> {
        self.inner.title.clone()
    }

    #[setter]
    fn set_title(&mut self, title: Option<String>) {
        self.inner.title = title;
    }

    #[getter]
    fn message_count(&self) -> usize {
        self.inner.messages.len()
    }

    /// Add a user message to the session
    fn add_user_message(&mut self, content: String) {
        self.inner.add_message(Message::user(content));
    }

    /// Add an assistant message to the session
    fn add_assistant_message(&mut self, content: String) {
        self.inner.add_message(Message::assistant(content));
    }

    /// Add a system message to the session
    fn add_system_message(&mut self, content: String) {
        self.inner.add_message(Message::system(content));
    }

    /// Get all messages as a list of dicts
    fn get_messages(&self) -> Vec<PyMessage> {
        self.inner
            .messages
            .iter()
            .map(|m| PyMessage {
                role: match m.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    MessageRole::System => "system".to_string(),
                },
                content: m.content.clone(),
            })
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "Session(id='{}', agent_id='{}', messages={})",
            self.inner.id.0,
            self.inner.agent_id.0,
            self.inner.messages.len()
        )
    }
}

/// A simplified message representation for Python
#[pyclass(name = "Message")]
#[derive(Clone)]
pub struct PyMessage {
    #[pyo3(get)]
    role: String,
    #[pyo3(get)]
    content: String,
}

#[pymethods]
impl PyMessage {
    fn __repr__(&self) -> String {
        format!("Message(role='{}', content='{}')", self.role, self.content)
    }
}
