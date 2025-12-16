//! Session management for conversations
//!
//! This module provides the Rust-side session manager that coordinates
//! with the Python session manager for conversation state tracking.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};

use ticca_bridge::AgentResponse;
use ticca_db::Database;

use crate::error::CoreError;

/// Information about an active session
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// Unique session identifier
    pub id: String,
    /// Associated conversation ID
    pub conversation_id: String,
    /// When the session was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the session was last accessed
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    /// Number of messages in this session
    pub message_count: u32,
    /// Total tokens used in this session
    pub total_tokens: u32,
    /// Whether the session is currently active (processing)
    pub is_active: bool,
}

impl SessionInfo {
    /// Create a new session
    pub fn new(conversation_id: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        let conv_id = conversation_id.into();
        Self {
            id: generate_session_id(&conv_id),
            conversation_id: conv_id,
            created_at: now,
            last_accessed: now,
            message_count: 0,
            total_tokens: 0,
            is_active: false,
        }
    }

    /// Update last accessed timestamp
    pub fn touch(&mut self) {
        self.last_accessed = chrono::Utc::now();
    }

    /// Add message and token counts
    pub fn add_message(&mut self, tokens: u32) {
        self.message_count += 1;
        self.total_tokens += tokens;
        self.touch();
    }

    /// Check if session is stale (not accessed recently)
    pub fn is_stale(&self, max_age: chrono::Duration) -> bool {
        chrono::Utc::now() - self.last_accessed > max_age
    }
}

/// Generate a unique session ID from conversation ID
fn generate_session_id(conversation_id: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("session-{}-{:x}", conversation_id, timestamp)
}

/// Manager for tracking conversation sessions
///
/// The `SessionManager` keeps track of active conversation sessions,
/// managing their lifecycle and coordinating with the database for
/// persistence.
///
/// # Thread Safety
///
/// `SessionManager` uses `RwLock` for interior mutability and is
/// safe to share across threads.
#[derive(Clone)]
pub struct SessionManager {
    /// Active sessions by conversation ID
    sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
    /// Database reference for persistence
    db: Arc<Database>,
    /// Maximum session age before considered stale
    max_session_age: chrono::Duration,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            db,
            max_session_age: chrono::Duration::hours(24),
        }
    }

    /// Create with custom max session age
    pub fn with_max_age(db: Arc<Database>, max_age: chrono::Duration) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            db,
            max_session_age: max_age,
        }
    }

    /// Get or create a session for a conversation
    #[instrument(skip(self))]
    pub async fn get_or_create_session(
        &self,
        conversation_id: &str,
    ) -> Result<SessionInfo, CoreError> {
        // First, try to get existing session (read lock)
        {
            let sessions = self.sessions.read().await;
            if let Some(session) = sessions.get(conversation_id) {
                debug!(session_id = %session.id, "Found existing session");
                return Ok(session.clone());
            }
        }

        // Need to create new session (write lock)
        let mut sessions = self.sessions.write().await;

        // Double-check after acquiring write lock
        if let Some(session) = sessions.get(conversation_id) {
            return Ok(session.clone());
        }

        // Create new session
        let session = SessionInfo::new(conversation_id);
        debug!(session_id = %session.id, "Created new session");

        // Store in database (agent name defaults to "default")
        if let Err(e) = self
            .db
            .sessions()
            .create(&session.id, "default", Some(conversation_id))
            .await
        {
            warn!(error = %e, "Failed to persist session to database");
            // Continue anyway - session will work in memory
        }

        sessions.insert(conversation_id.to_string(), session.clone());
        Ok(session)
    }

    /// Get a session by conversation ID
    pub async fn get_session(&self, conversation_id: &str) -> Option<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions.get(conversation_id).cloned()
    }

    /// Update session after a response
    #[instrument(skip(self, response))]
    pub async fn update_session(
        &self,
        conversation_id: &str,
        response: &AgentResponse,
    ) -> Result<(), CoreError> {
        let mut sessions = self.sessions.write().await;

        if let Some(session) = sessions.get_mut(conversation_id) {
            session.add_message(response.usage.total_tokens);
            session.is_active = false;
            debug!(
                session_id = %session.id,
                message_count = session.message_count,
                total_tokens = session.total_tokens,
                "Session updated"
            );
        }

        Ok(())
    }

    /// Mark a session as active (currently processing)
    pub async fn mark_active(&self, conversation_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(conversation_id) {
            session.is_active = true;
            session.touch();
        }
    }

    /// Mark a session as inactive
    pub async fn mark_inactive(&self, conversation_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(conversation_id) {
            session.is_active = false;
        }
    }

    /// Remove a session
    pub async fn remove_session(&self, conversation_id: &str) {
        let mut sessions = self.sessions.write().await;
        if sessions.remove(conversation_id).is_some() {
            debug!(conversation_id = %conversation_id, "Session removed");
        }
    }

    /// List all active sessions
    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions.values().cloned().collect()
    }

    /// Count active sessions
    pub async fn session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    /// Clean up stale sessions
    #[instrument(skip(self))]
    pub async fn cleanup_stale_sessions(&self) -> usize {
        let mut sessions = self.sessions.write().await;
        let before_count = sessions.len();

        sessions.retain(|_, session| {
            let is_stale = session.is_stale(self.max_session_age);
            if is_stale {
                debug!(session_id = %session.id, "Removing stale session");
            }
            !is_stale
        });

        let removed = before_count - sessions.len();
        if removed > 0 {
            info!(removed = removed, "Cleaned up stale sessions");
        }
        removed
    }

    /// Shutdown and cleanup
    pub async fn shutdown(&self) {
        let mut sessions = self.sessions.write().await;
        info!(count = sessions.len(), "Shutting down session manager");
        sessions.clear();
    }

    /// Get session statistics
    pub async fn stats(&self) -> SessionStats {
        let sessions = self.sessions.read().await;
        let total_sessions = sessions.len();
        let active_sessions = sessions.values().filter(|s| s.is_active).count();
        let total_messages: u32 = sessions.values().map(|s| s.message_count).sum();
        let total_tokens: u32 = sessions.values().map(|s| s.total_tokens).sum();

        SessionStats {
            total_sessions,
            active_sessions,
            total_messages,
            total_tokens,
        }
    }
}

/// Statistics about sessions
#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    /// Total number of sessions
    pub total_sessions: usize,
    /// Number of currently active sessions
    pub active_sessions: usize,
    /// Total messages across all sessions
    pub total_messages: u32,
    /// Total tokens used across all sessions
    pub total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_mock_response() -> AgentResponse {
        AgentResponse {
            message_id: "msg-1".to_string(),
            content: "Test response".to_string(),
            role: "assistant".to_string(),
            tool_calls: None,
            usage: ticca_bridge::TokenUsage::new(10, 20),
            finish_reason: ticca_bridge::FinishReason::Stop,
        }
    }

    #[test]
    fn test_session_info_creation() {
        let session = SessionInfo::new("conv-123");
        assert!(session.id.contains("conv-123"));
        assert_eq!(session.conversation_id, "conv-123");
        assert_eq!(session.message_count, 0);
        assert!(!session.is_active);
    }

    #[test]
    fn test_session_info_touch() {
        let mut session = SessionInfo::new("conv-1");
        let original = session.last_accessed;
        std::thread::sleep(std::time::Duration::from_millis(10));
        session.touch();
        assert!(session.last_accessed > original);
    }

    #[test]
    fn test_session_info_add_message() {
        let mut session = SessionInfo::new("conv-1");
        session.add_message(100);
        assert_eq!(session.message_count, 1);
        assert_eq!(session.total_tokens, 100);

        session.add_message(50);
        assert_eq!(session.message_count, 2);
        assert_eq!(session.total_tokens, 150);
    }

    #[test]
    fn test_generate_session_id() {
        let id1 = generate_session_id("conv-1");
        let id2 = generate_session_id("conv-1");
        // IDs should be unique even for same conversation
        assert_ne!(id1, id2);
        // But should contain the conversation ID
        assert!(id1.contains("conv-1"));
    }
}
