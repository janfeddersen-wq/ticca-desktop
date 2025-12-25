//! Session storage database operations

use crate::session::models::{MessageRole, Session, SessionMessage};
use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::PathBuf;

/// Session database manager
pub struct SessionDatabase {
    conn: Connection,
}

impl SessionDatabase {
    /// Open or create the session database
    pub fn open() -> Result<Self> {
        let db_path = Self::get_db_path()?;

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;
        let db = Self { conn };
        db.initialize()?;
        Ok(db)
    }

    /// Get the database file path
    pub fn get_db_path() -> Result<PathBuf> {
        Ok(crate::config::paths::get_data_dir()?.join("sessions.db"))
    }

    /// Initialize database schema
    fn initialize(&self) -> Result<()> {
        // Create sessions table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                agent_type TEXT NOT NULL,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now')),
                total_tokens INTEGER DEFAULT 0,
                message_count INTEGER DEFAULT 0
            )",
            [],
        )?;

        // Create messages table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                tool_calls_json TEXT,
                tool_result_json TEXT,
                reasoning TEXT,
                reasoning_signature TEXT,
                tokens INTEGER DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now'))
            )",
            [],
        )?;

        // Migration: add reasoning columns if they don't exist (for existing databases)
        let _ = self
            .conn
            .execute("ALTER TABLE messages ADD COLUMN reasoning TEXT", []);
        let _ = self.conn.execute(
            "ALTER TABLE messages ADD COLUMN reasoning_signature TEXT",
            [],
        );

        // Create index for efficient message retrieval
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_messages_session 
             ON messages(session_id, created_at)",
            [],
        )?;

        // Create todo state table (agent-scoped per session/node)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS todo_states (
                session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                node_id INTEGER NOT NULL,
                state_json TEXT NOT NULL,
                updated_at TEXT DEFAULT (datetime('now')),
                PRIMARY KEY (session_id, node_id)
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_todo_states_session
             ON todo_states(session_id, updated_at)",
            [],
        )?;

        // Enable foreign key support
        self.conn.execute("PRAGMA foreign_keys = ON", [])?;

        Ok(())
    }

    // Session CRUD

    /// Create a new session
    pub fn create_session(&self, session: &Session) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sessions (id, name, agent_type, created_at, updated_at, total_tokens, message_count)
             VALUES (?, ?, ?, COALESCE(?, datetime('now')), COALESCE(?, datetime('now')), ?, ?)",
            params![
                session.id,
                session.name,
                session.agent_type,
                session.created_at,
                session.updated_at,
                session.total_tokens,
                session.message_count,
            ],
        )?;
        Ok(())
    }

    /// Get a session by ID
    pub fn get_session(&self, id: &str) -> Result<Option<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, agent_type, created_at, updated_at, total_tokens, message_count 
             FROM sessions WHERE id = ?",
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok(Session {
                id: row.get(0)?,
                name: row.get(1)?,
                agent_type: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                total_tokens: row.get(5)?,
                message_count: row.get(6)?,
            })
        });

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all sessions, most recent first
    pub fn list_sessions(&self) -> Result<Vec<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, agent_type, created_at, updated_at, total_tokens, message_count 
             FROM sessions ORDER BY updated_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Session {
                id: row.get(0)?,
                name: row.get(1)?,
                agent_type: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                total_tokens: row.get(5)?,
                message_count: row.get(6)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Update session metadata (call after adding messages)
    pub fn update_session_stats(&self, session_id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET 
                updated_at = datetime('now'),
                message_count = (SELECT COUNT(*) FROM messages WHERE session_id = ?),
                total_tokens = (SELECT COALESCE(SUM(tokens), 0) FROM messages WHERE session_id = ?)
             WHERE id = ?",
            params![session_id, session_id, session_id],
        )?;
        Ok(())
    }

    /// Rename a session
    pub fn rename_session(&self, id: &str, new_name: &str) -> Result<bool> {
        let changes = self.conn.execute(
            "UPDATE sessions SET name = ?, updated_at = datetime('now') WHERE id = ?",
            params![new_name, id],
        )?;
        Ok(changes > 0)
    }

    /// Delete a session and all its messages
    pub fn delete_session(&self, id: &str) -> Result<bool> {
        // Messages are deleted via CASCADE
        let changes = self
            .conn
            .execute("DELETE FROM sessions WHERE id = ?", params![id])?;
        Ok(changes > 0)
    }

    // Message CRUD

    /// Add a message to a session
    pub fn add_message(&self, message: &SessionMessage) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO messages (session_id, role, content, tool_calls_json, tool_result_json, reasoning, reasoning_signature, tokens, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, COALESCE(?, datetime('now')))",
            params![
                message.session_id,
                message.role.as_str(),
                message.content,
                message.tool_calls_json,
                message.tool_result_json,
                message.reasoning,
                message.reasoning_signature,
                message.tokens,
                message.created_at,
            ],
        )?;

        // Update session stats
        self.update_session_stats(&message.session_id)?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get all messages for a session
    pub fn get_messages(&self, session_id: &str) -> Result<Vec<SessionMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, role, content, tool_calls_json, tool_result_json, reasoning, reasoning_signature, tokens, created_at
             FROM messages WHERE session_id = ? ORDER BY created_at ASC"
        )?;

        let rows = stmt.query_map(params![session_id], |row| {
            let role_str: String = row.get(2)?;
            Ok(SessionMessage {
                id: Some(row.get(0)?),
                session_id: row.get(1)?,
                role: MessageRole::parse(&role_str),
                content: row.get(3)?,
                tool_calls_json: row.get(4)?,
                tool_result_json: row.get(5)?,
                reasoning: row.get(6)?,
                reasoning_signature: row.get(7)?,
                tokens: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Get the last N messages for a session
    pub fn get_recent_messages(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<SessionMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, role, content, tool_calls_json, tool_result_json, reasoning, reasoning_signature, tokens, created_at
             FROM messages WHERE session_id = ? ORDER BY created_at DESC LIMIT ?"
        )?;

        let rows = stmt.query_map(params![session_id, limit as i64], |row| {
            let role_str: String = row.get(2)?;
            Ok(SessionMessage {
                id: Some(row.get(0)?),
                session_id: row.get(1)?,
                role: MessageRole::parse(&role_str),
                content: row.get(3)?,
                tool_calls_json: row.get(4)?,
                tool_result_json: row.get(5)?,
                reasoning: row.get(6)?,
                reasoning_signature: row.get(7)?,
                tokens: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;

        // Reverse to get chronological order
        let mut messages: Vec<_> = rows.collect::<Result<Vec<_>, _>>()?;
        messages.reverse();
        Ok(messages)
    }

    /// Delete all messages in a session (but keep the session)
    pub fn clear_session_messages(&self, session_id: &str) -> Result<usize> {
        let changes = self.conn.execute(
            "DELETE FROM messages WHERE session_id = ?",
            params![session_id],
        )?;

        self.update_session_stats(session_id)?;
        Ok(changes)
    }

    // Todo state

    pub fn upsert_todo_state(
        &self,
        session_id: &str,
        node_id: usize,
        state_json: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO todo_states (session_id, node_id, state_json, updated_at)
             VALUES (?, ?, ?, datetime('now'))
             ON CONFLICT(session_id, node_id)
             DO UPDATE SET state_json = excluded.state_json, updated_at = datetime('now')",
            params![session_id, node_id as i64, state_json],
        )?;
        Ok(())
    }

    pub fn get_todo_states(&self, session_id: &str) -> Result<Vec<(usize, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT node_id, state_json
             FROM todo_states
             WHERE session_id = ?
             ORDER BY updated_at ASC",
        )?;

        let rows = stmt.query_map(params![session_id], |row| {
            let node_id: i64 = row.get(0)?;
            let json: String = row.get(1)?;
            Ok((node_id as usize, json))
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn clear_todo_states(&self, session_id: &str) -> Result<usize> {
        let changes = self.conn.execute(
            "DELETE FROM todo_states WHERE session_id = ?",
            params![session_id],
        )?;
        Ok(changes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    struct TestDb {
        db: SessionDatabase,
        _dir: TempDir, // Keep the tempdir alive
    }

    fn test_db() -> TestDb {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test_sessions.db");
        let conn = Connection::open(&db_path).unwrap();
        let db = SessionDatabase { conn };
        db.initialize().unwrap();
        TestDb { db, _dir: dir }
    }

    #[test]
    fn test_session_crud() {
        let test = test_db();
        let db = &test.db;

        // Create session
        let session = Session::coding("Test Session");
        let session_id = session.id.clone();
        db.create_session(&session).unwrap();

        // Read session
        let retrieved = db.get_session(&session_id).unwrap().unwrap();
        assert_eq!(retrieved.name, "Test Session");
        assert_eq!(retrieved.agent_type, "coding");

        // List sessions
        let sessions = db.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);

        // Rename session
        db.rename_session(&session_id, "Renamed Session").unwrap();
        let renamed = db.get_session(&session_id).unwrap().unwrap();
        assert_eq!(renamed.name, "Renamed Session");

        // Delete session
        assert!(db.delete_session(&session_id).unwrap());
        assert!(db.get_session(&session_id).unwrap().is_none());
    }

    #[test]
    fn test_messages_crud() {
        let test = test_db();
        let db = &test.db;

        // Create session first
        let session = Session::coding("Message Test");
        let session_id = session.id.clone();
        db.create_session(&session).unwrap();

        // Add messages
        let user_msg = SessionMessage::user(&session_id, "Hello!").with_tokens(10);
        db.add_message(&user_msg).unwrap();

        let assistant_msg = SessionMessage::assistant(&session_id, "Hi there!").with_tokens(15);
        db.add_message(&assistant_msg).unwrap();

        // Get messages
        let messages = db.get_messages(&session_id).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, MessageRole::User);
        assert_eq!(messages[1].role, MessageRole::Assistant);

        // Check session stats updated
        let updated_session = db.get_session(&session_id).unwrap().unwrap();
        assert_eq!(updated_session.message_count, 2);
        assert_eq!(updated_session.total_tokens, 25);

        // Get recent messages
        let recent = db.get_recent_messages(&session_id, 1).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].content, "Hi there!");

        // Clear messages
        let cleared = db.clear_session_messages(&session_id).unwrap();
        assert_eq!(cleared, 2);

        let empty = db.get_messages(&session_id).unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn test_cascade_delete() {
        let test = test_db();
        let db = &test.db;

        // Create session with messages
        let session = Session::planning("Cascade Test");
        let session_id = session.id.clone();
        db.create_session(&session).unwrap();

        db.add_message(&SessionMessage::user(&session_id, "Message 1"))
            .unwrap();
        db.add_message(&SessionMessage::assistant(&session_id, "Message 2"))
            .unwrap();

        // Delete session should cascade to messages
        db.delete_session(&session_id).unwrap();

        // Messages should be gone (verify by trying to get them)
        let messages = db.get_messages(&session_id).unwrap();
        assert!(messages.is_empty());
    }

    #[test]
    fn test_todo_states_crud() {
        let test = test_db();
        let db = &test.db;

        // Create session first
        let session = Session::coding("Todo Test");
        let session_id = session.id.clone();
        db.create_session(&session).unwrap();

        // Upsert todo states
        db.upsert_todo_state(
            &session_id,
            0,
            "{\"items\":[],\"confirmed_complete\":false}",
        )
        .unwrap();
        db.upsert_todo_state(&session_id, 1, "{\"items\":[],\"confirmed_complete\":true}")
            .unwrap();

        // Read todo states
        let states = db.get_todo_states(&session_id).unwrap();
        assert_eq!(states.len(), 2);
        assert_eq!(states[0].0, 0);
        assert_eq!(states[1].0, 1);

        // Clear states
        let cleared = db.clear_todo_states(&session_id).unwrap();
        assert_eq!(cleared, 2);
        let empty = db.get_todo_states(&session_id).unwrap();
        assert!(empty.is_empty());
    }
}
