#![deny(clippy::unwrap_used)]

//! Ticca DB - SQLite persistence layer
//!
//! This crate provides database access for Ticca Desktop using SQLx
//! with SQLite as the backend. Features include:
//!
//! - Automatic migrations on startup
//! - Connection pooling with WAL mode
//! - Repository pattern for clean data access
//! - Full async support via Tokio

pub mod error;
pub mod migrations;
pub mod pool;
pub mod repositories;
pub mod schema;

pub use error::DbError;
pub use migrations::MigrationRunner;
pub use pool::DbPool;
pub use repositories::{
    ConversationRepository, MessageRepository, SessionRepository, SettingRepository,
};
pub use schema::{Conversation, DbMessage, DbSession, MessageRole, Setting};

use std::path::Path;
use tracing::info;

/// Result type alias using DbError
pub type Result<T> = std::result::Result<T, DbError>;

/// Main database interface for Ticca Desktop.
///
/// Provides access to all repositories and handles connection management.
/// Automatically runs migrations on connect.
///
/// # Example
/// ```ignore
/// use ticca_db::Database;
///
/// let db = Database::connect("/path/to/ticca.db").await?;
/// let conversations = db.conversations().list_all().await?;
/// ```
#[derive(Clone)]
pub struct Database {
    pool: DbPool,
}

impl Database {
    /// Connect to a database file and run migrations.
    ///
    /// Creates the database file if it doesn't exist.
    /// Automatically runs any pending migrations.
    pub async fn connect(db_path: impl AsRef<Path>) -> Result<Self> {
        let pool = DbPool::new(db_path).await?;

        // Run migrations
        let runner = MigrationRunner::new(pool.inner());
        runner.run().await?;

        info!("Database connected and migrations applied");
        Ok(Self { pool })
    }

    /// Create an in-memory database (useful for testing).
    ///
    /// The database is automatically initialized with all migrations.
    pub async fn in_memory() -> Result<Self> {
        let pool = DbPool::in_memory().await?;

        // Run migrations
        let runner = MigrationRunner::new(pool.inner());
        runner.run().await?;

        info!("In-memory database created with migrations applied");
        Ok(Self { pool })
    }

    /// Get a reference to the underlying connection pool.
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    /// Get the raw SQLx pool (for advanced use cases).
    pub fn inner(&self) -> &sqlx::SqlitePool {
        self.pool.inner()
    }

    /// Get the conversation repository.
    pub fn conversations(&self) -> ConversationRepository {
        ConversationRepository::new(self.pool.clone())
    }

    /// Get the message repository.
    pub fn messages(&self) -> MessageRepository {
        MessageRepository::new(self.pool.clone())
    }

    /// Get the settings repository.
    pub fn settings(&self) -> SettingRepository {
        SettingRepository::new(self.pool.clone())
    }

    /// Get the session repository.
    pub fn sessions(&self) -> SessionRepository {
        SessionRepository::new(self.pool.clone())
    }

    /// Check if migrations are needed.
    pub async fn needs_migration(&self) -> Result<bool> {
        let runner = MigrationRunner::new(self.pool.inner());
        runner.needs_migration().await
    }

    /// Get the current database version.
    pub async fn current_version(&self) -> Result<Option<i32>> {
        let runner = MigrationRunner::new(self.pool.inner());
        runner.current_version().await
    }

    /// Close the database connection pool.
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_database() {
        let db = Database::in_memory().await.expect("Failed to create in-memory database");
        
        // Should have run migrations
        let version = db.current_version().await.expect("Failed to get version");
        assert!(version.is_some(), "Should have applied at least one migration");
        assert_eq!(version, Some(1), "Should be at migration version 1");
        
        // Verify tables exist by doing a count on each
        let _: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM conversations")
            .fetch_one(db.inner()).await.expect("conversations table");
        let _: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM messages")
            .fetch_one(db.inner()).await.expect("messages table");
        let _: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM settings")
            .fetch_one(db.inner()).await.expect("settings table");
        let _: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sessions")
            .fetch_one(db.inner()).await.expect("sessions table");
    }

    #[tokio::test]
    async fn test_repositories_accessible() {
        let db = Database::in_memory().await.expect("Failed to create database");
        
        // Just verify we can access repositories without panic
        let _conversations = db.conversations();
        let _messages = db.messages();
        let _settings = db.settings();
        let _sessions = db.sessions();
    }

    #[tokio::test]
    async fn test_conversation_crud() {
        let db = Database::in_memory().await.expect("Failed to create database");
        let repo = db.conversations();

        // Create
        let conv = repo.create("conv-1", "Test Conversation").await.expect("create");
        assert_eq!(conv.id, "conv-1");
        assert_eq!(conv.title, "Test Conversation");

        // Read
        let fetched = repo.get_by_id("conv-1").await.expect("get").expect("exists");
        assert_eq!(fetched.title, "Test Conversation");

        // Update
        repo.update_title("conv-1", "Updated Title").await.expect("update");
        let updated = repo.get_by_id("conv-1").await.expect("get").expect("exists");
        assert_eq!(updated.title, "Updated Title");

        // List
        let all = repo.list_all().await.expect("list");
        assert_eq!(all.len(), 1);

        // Delete
        repo.delete("conv-1").await.expect("delete");
        assert!(repo.get_by_id("conv-1").await.expect("get").is_none());
    }

    #[tokio::test]
    async fn test_message_crud_with_tool_fields() {
        let db = Database::in_memory().await.expect("Failed to create database");
        
        // Create a conversation first
        db.conversations().create("conv-1", "Test").await.expect("create conv");
        
        let repo = db.messages();

        // Create simple message
        let msg1 = repo.create_simple("msg-1", "conv-1", "user", "Hello!").await.expect("create");
        assert_eq!(msg1.id, "msg-1");
        assert_eq!(msg1.role, "user");
        assert!(msg1.tool_calls.is_none());

        // Create assistant message with tool calls
        let tool_calls_json = r#"[{"id":"call-1","name":"search","args":{}}]"#;
        let msg2 = repo.create(
            "msg-2", "conv-1", "assistant", "Let me search...",
            Some(tool_calls_json), None, Some(50)
        ).await.expect("create with tools");
        assert_eq!(msg2.tool_calls.as_deref(), Some(tool_calls_json));
        assert_eq!(msg2.token_count, Some(50));

        // Create tool response
        let msg3 = repo.create(
            "msg-3", "conv-1", "tool", "{\"results\":[]}",
            None, Some("call-1"), None
        ).await.expect("create tool response");
        assert_eq!(msg3.tool_call_id.as_deref(), Some("call-1"));

        // Get by conversation
        let messages = repo.get_by_conversation("conv-1").await.expect("get");
        assert_eq!(messages.len(), 3);

        // Get tool response
        let tool_resp = repo.get_tool_response("call-1").await.expect("get tool").expect("exists");
        assert_eq!(tool_resp.id, "msg-3");
    }

    #[tokio::test]
    async fn test_settings_crud() {
        let db = Database::in_memory().await.expect("Failed to create database");
        let repo = db.settings();

        // Set
        repo.set("theme", "dark").await.expect("set");
        repo.set("api.key", "secret123").await.expect("set");

        // Get
        assert_eq!(repo.get("theme").await.expect("get"), Some("dark".to_string()));
        assert_eq!(repo.get("missing").await.expect("get"), None);

        // Get or default
        let val: i32 = repo.get_or_default("max_tokens", 1000).await.expect("get");
        assert_eq!(val, 1000);

        // Get by prefix
        repo.set("api.endpoint", "https://api.example.com").await.expect("set");
        let api_settings = repo.get_by_prefix("api.").await.expect("get");
        assert_eq!(api_settings.len(), 2);

        // Delete by prefix
        let deleted = repo.delete_by_prefix("api.").await.expect("delete");
        assert_eq!(deleted, 2);
    }

    #[tokio::test]
    async fn test_session_crud() {
        let db = Database::in_memory().await.expect("Failed to create database");
        let repo = db.sessions();

        // Create
        let session = repo.create("sess-1", "code-agent", Some("Write tests")).await.expect("create");
        assert_eq!(session.id, "sess-1");
        assert_eq!(session.agent_name, "code-agent");
        assert_eq!(session.message_count, 0);

        // Increment message count
        let new_count = repo.increment_message_count("sess-1").await.expect("increment");
        assert_eq!(new_count, 1);

        // Get by agent
        let sessions = repo.get_by_agent("code-agent").await.expect("get");
        assert_eq!(sessions.len(), 1);

        // Update metadata
        repo.update_metadata("sess-1", Some(r#"{"model":"gpt-4"}"#)).await.expect("update");
        let updated = repo.get_by_id("sess-1").await.expect("get").expect("exists");
        assert!(updated.metadata.is_some());

        // Delete
        repo.delete("sess-1").await.expect("delete");
        assert!(!repo.exists("sess-1").await.expect("exists"));
    }

    #[tokio::test]
    async fn test_cascade_delete() {
        let db = Database::in_memory().await.expect("Failed to create database");
        
        // Create conversation with messages
        db.conversations().create("conv-1", "Test").await.expect("create conv");
        db.messages().create_simple("msg-1", "conv-1", "user", "Hi").await.expect("create");
        db.messages().create_simple("msg-2", "conv-1", "assistant", "Hello!").await.expect("create");

        // Verify messages exist
        let count = db.messages().count_by_conversation("conv-1").await.expect("count");
        assert_eq!(count, 2);

        // Delete conversation - messages should cascade
        db.conversations().delete("conv-1").await.expect("delete");

        // Messages should be gone
        let count = db.messages().count_by_conversation("conv-1").await.expect("count");
        assert_eq!(count, 0);
    }
}
