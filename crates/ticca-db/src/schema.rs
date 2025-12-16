//! Database schema types
//!
//! Defines the core database entities that map to SQLite tables.
//! All IDs are TEXT (UUID strings) as per requirements.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A conversation in the database.
///
/// Represents a chat session with an AI agent, containing multiple messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    /// Unique identifier (TEXT UUID)
    pub id: String,
    /// Title of the conversation (can be user-defined or auto-generated)
    pub title: String,
    /// When the conversation was created
    pub created_at: DateTime<Utc>,
    /// When the conversation was last updated
    pub updated_at: DateTime<Utc>,
}

impl Conversation {
    /// Create a new Conversation instance.
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            created_at,
            updated_at,
        }
    }

    /// Create a new Conversation with auto-generated timestamps.
    pub fn create(id: impl Into<String>, title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            title: title.into(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// A message in the database.
///
/// Represents a single message within a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbMessage {
    /// Unique identifier (TEXT UUID)
    pub id: String,
    /// ID of the conversation this message belongs to
    pub conversation_id: String,
    /// Role of the message sender: "system", "user", "assistant", or "tool"
    pub role: String,
    /// The message content
    pub content: String,
    /// JSON array of tool calls if role is 'assistant' (optional)
    pub tool_calls: Option<String>,
    /// If role is 'tool', the ID of the tool call this responds to (optional)
    pub tool_call_id: Option<String>,
    /// When the message was created
    pub timestamp: DateTime<Utc>,
    /// Token count for this message (optional)
    pub token_count: Option<i32>,
}

impl DbMessage {
    /// Create a new DbMessage instance with all fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        conversation_id: impl Into<String>,
        role: impl Into<String>,
        content: impl Into<String>,
        tool_calls: Option<String>,
        tool_call_id: Option<String>,
        timestamp: DateTime<Utc>,
        token_count: Option<i32>,
    ) -> Self {
        Self {
            id: id.into(),
            conversation_id: conversation_id.into(),
            role: role.into(),
            content: content.into(),
            tool_calls,
            tool_call_id,
            timestamp,
            token_count,
        }
    }

    /// Create a simple message without tool-related fields.
    pub fn simple(
        id: impl Into<String>,
        conversation_id: impl Into<String>,
        role: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            conversation_id: conversation_id.into(),
            role: role.into(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            timestamp: Utc::now(),
            token_count: None,
        }
    }
}

/// Valid message roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl MessageRole {
    /// Convert role to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
        }
    }

    /// Parse a role from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "system" => Some(Self::System),
            "user" => Some(Self::User),
            "assistant" => Some(Self::Assistant),
            "tool" => Some(Self::Tool),
            _ => None,
        }
    }
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A key-value setting in the database.
///
/// Used for storing persistent configuration and state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    /// The setting key (unique identifier)
    pub key: String,
    /// The setting value (stored as text)
    pub value: String,
    /// When the setting was last updated
    pub updated_at: DateTime<Utc>,
}

impl Setting {
    /// Create a new Setting instance.
    pub fn new(
        key: impl Into<String>,
        value: impl Into<String>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            updated_at,
        }
    }

    /// Create a new Setting with current timestamp.
    pub fn create(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            updated_at: Utc::now(),
        }
    }
}

/// An agent session in the database.
///
/// Used for persisting agent sub-session state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbSession {
    /// Unique identifier (TEXT)
    pub id: String,
    /// Name of the agent this session belongs to
    pub agent_name: String,
    /// Initial prompt that started the session (optional)
    pub initial_prompt: Option<String>,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was last updated
    pub last_updated: DateTime<Utc>,
    /// Number of messages in this session
    pub message_count: i32,
    /// JSON metadata for additional data (optional)
    pub metadata: Option<String>,
}

impl DbSession {
    /// Create a new DbSession instance.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        agent_name: impl Into<String>,
        initial_prompt: Option<String>,
        created_at: DateTime<Utc>,
        last_updated: DateTime<Utc>,
        message_count: i32,
        metadata: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            agent_name: agent_name.into(),
            initial_prompt,
            created_at,
            last_updated,
            message_count,
            metadata,
        }
    }

    /// Create a new session with defaults.
    pub fn create(
        id: impl Into<String>,
        agent_name: impl Into<String>,
        initial_prompt: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            agent_name: agent_name.into(),
            initial_prompt,
            created_at: now,
            last_updated: now,
            message_count: 0,
            metadata: None,
        }
    }
}

/// Convenience methods for parsing setting values
impl Setting {
    /// Parse the value as a boolean.
    pub fn as_bool(&self) -> bool {
        matches!(
            self.value.to_lowercase().as_str(),
            "true" | "1" | "yes" | "on"
        )
    }

    /// Parse the value as an integer.
    pub fn as_i64(&self) -> Option<i64> {
        self.value.parse().ok()
    }

    /// Parse the value as a float.
    pub fn as_f64(&self) -> Option<f64> {
        self.value.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_role_as_str() {
        assert_eq!(MessageRole::System.as_str(), "system");
        assert_eq!(MessageRole::User.as_str(), "user");
        assert_eq!(MessageRole::Assistant.as_str(), "assistant");
        assert_eq!(MessageRole::Tool.as_str(), "tool");
    }

    #[test]
    fn test_message_role_parse() {
        assert_eq!(MessageRole::parse("system"), Some(MessageRole::System));
        assert_eq!(MessageRole::parse("USER"), Some(MessageRole::User));
        assert_eq!(MessageRole::parse("Assistant"), Some(MessageRole::Assistant));
        assert_eq!(MessageRole::parse("invalid"), None);
    }

    #[test]
    fn test_message_role_display() {
        assert_eq!(MessageRole::System.to_string(), "system");
        assert_eq!(MessageRole::User.to_string(), "user");
    }

    #[test]
    fn test_setting_as_bool() {
        assert!(Setting::create("test", "true").as_bool());
        assert!(Setting::create("test", "TRUE").as_bool());
        assert!(Setting::create("test", "1").as_bool());
        assert!(Setting::create("test", "yes").as_bool());
        assert!(Setting::create("test", "on").as_bool());

        assert!(!Setting::create("test", "false").as_bool());
        assert!(!Setting::create("test", "0").as_bool());
        assert!(!Setting::create("test", "no").as_bool());
    }

    #[test]
    fn test_setting_as_i64() {
        assert_eq!(Setting::create("test", "42").as_i64(), Some(42));
        assert_eq!(Setting::create("test", "-10").as_i64(), Some(-10));
        assert_eq!(Setting::create("test", "invalid").as_i64(), None);
    }

    #[test]
    fn test_setting_as_f64() {
        assert_eq!(Setting::create("test", "3.14").as_f64(), Some(3.14));
        assert_eq!(Setting::create("test", "invalid").as_f64(), None);
    }

    #[test]
    fn test_conversation_create() {
        let conv = Conversation::create("test-id", "Test Title");
        assert_eq!(conv.id, "test-id");
        assert_eq!(conv.title, "Test Title");
        // created_at and updated_at should be very close
        assert!(conv.updated_at >= conv.created_at);
    }

    #[test]
    fn test_db_session_create() {
        let session = DbSession::create("sess-1", "test-agent", Some("Hello".to_string()));
        assert_eq!(session.id, "sess-1");
        assert_eq!(session.agent_name, "test-agent");
        assert_eq!(session.initial_prompt, Some("Hello".to_string()));
        assert_eq!(session.message_count, 0);
    }
}
