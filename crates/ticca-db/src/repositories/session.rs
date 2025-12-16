//! Session repository for database operations
//!
//! Manages agent sub-sessions in the database.

use chrono::Utc;
use sqlx::Row;
use tracing::debug;

use crate::pool::DbPool;
use crate::repositories::conversation::parse_datetime;
use crate::schema::DbSession;
use crate::{DbError, Result};

/// Repository for session database operations.
pub struct SessionRepository {
    pool: DbPool,
}

impl SessionRepository {
    /// Create a new session repository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new session.
    ///
    /// Returns the created session.
    pub async fn create(
        &self,
        id: &str,
        agent_name: &str,
        initial_prompt: Option<&str>,
    ) -> Result<DbSession> {
        let now = Utc::now();
        let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();

        sqlx::query(
            r#"
            INSERT INTO sessions (id, agent_name, initial_prompt, created_at, last_updated, message_count, metadata)
            VALUES (?, ?, ?, ?, ?, 0, NULL)
            "#,
        )
        .bind(id)
        .bind(agent_name)
        .bind(initial_prompt)
        .bind(&timestamp)
        .bind(&timestamp)
        .execute(self.pool.inner())
        .await?;

        debug!("Created session {} for agent '{}'", id, agent_name);

        Ok(DbSession::new(
            id,
            agent_name,
            initial_prompt.map(String::from),
            now,
            now,
            0,
            None,
        ))
    }

    /// Get a session by ID.
    ///
    /// Returns `None` if the session doesn't exist.
    pub async fn get_by_id(&self, id: &str) -> Result<Option<DbSession>> {
        let result = sqlx::query(
            r#"
            SELECT id, agent_name, initial_prompt, created_at, last_updated, message_count, metadata
            FROM sessions WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool.inner())
        .await?;

        match result {
            Some(row) => Ok(Some(row_to_session(&row)?)),
            None => Ok(None),
        }
    }

    /// Get all sessions for an agent, ordered by last_updated (newest first).
    pub async fn get_by_agent(&self, agent_name: &str) -> Result<Vec<DbSession>> {
        let rows = sqlx::query(
            r#"
            SELECT id, agent_name, initial_prompt, created_at, last_updated, message_count, metadata
            FROM sessions
            WHERE agent_name = ?
            ORDER BY last_updated DESC
            "#,
        )
        .bind(agent_name)
        .fetch_all(self.pool.inner())
        .await?;

        let mut sessions = Vec::with_capacity(rows.len());
        for row in rows {
            sessions.push(row_to_session(&row)?);
        }

        Ok(sessions)
    }

    /// List all sessions, ordered by last_updated (newest first).
    pub async fn list_all(&self) -> Result<Vec<DbSession>> {
        let rows = sqlx::query(
            r#"
            SELECT id, agent_name, initial_prompt, created_at, last_updated, message_count, metadata
            FROM sessions
            ORDER BY last_updated DESC
            "#,
        )
        .fetch_all(self.pool.inner())
        .await?;

        let mut sessions = Vec::with_capacity(rows.len());
        for row in rows {
            sessions.push(row_to_session(&row)?);
        }

        Ok(sessions)
    }

    /// Delete a session by ID.
    pub async fn delete(&self, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id)
            .execute(self.pool.inner())
            .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("Session {} not found", id)));
        }

        debug!("Deleted session {}", id);
        Ok(())
    }

    /// Update the message count for a session.
    ///
    /// Also updates the `last_updated` timestamp.
    pub async fn update_message_count(&self, id: &str, message_count: i32) -> Result<()> {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let result = sqlx::query(
            "UPDATE sessions SET message_count = ?, last_updated = ? WHERE id = ?",
        )
        .bind(message_count)
        .bind(&timestamp)
        .bind(id)
        .execute(self.pool.inner())
        .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("Session {} not found", id)));
        }

        debug!(
            "Updated session {} message count to {}",
            id, message_count
        );
        Ok(())
    }

    /// Increment the message count for a session by 1.
    ///
    /// Also updates the `last_updated` timestamp.
    pub async fn increment_message_count(&self, id: &str) -> Result<i32> {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // Get current count, increment, and update
        let session = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("Session {} not found", id)))?;

        let new_count = session.message_count + 1;

        sqlx::query(
            "UPDATE sessions SET message_count = ?, last_updated = ? WHERE id = ?",
        )
        .bind(new_count)
        .bind(&timestamp)
        .bind(id)
        .execute(self.pool.inner())
        .await?;

        debug!("Incremented session {} message count to {}", id, new_count);
        Ok(new_count)
    }

    /// Update the metadata for a session.
    ///
    /// Also updates the `last_updated` timestamp.
    pub async fn update_metadata(&self, id: &str, metadata: Option<&str>) -> Result<()> {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let result = sqlx::query(
            "UPDATE sessions SET metadata = ?, last_updated = ? WHERE id = ?",
        )
        .bind(metadata)
        .bind(&timestamp)
        .bind(id)
        .execute(self.pool.inner())
        .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("Session {} not found", id)));
        }

        debug!("Updated session {} metadata", id);
        Ok(())
    }

    /// Touch the last_updated timestamp for a session.
    pub async fn touch(&self, id: &str) -> Result<()> {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let result = sqlx::query("UPDATE sessions SET last_updated = ? WHERE id = ?")
            .bind(&timestamp)
            .bind(id)
            .execute(self.pool.inner())
            .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("Session {} not found", id)));
        }

        debug!("Touched session {}", id);
        Ok(())
    }

    /// Count sessions for an agent.
    pub async fn count_by_agent(&self, agent_name: &str) -> Result<i64> {
        let result: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM sessions WHERE agent_name = ?")
                .bind(agent_name)
                .fetch_one(self.pool.inner())
                .await?;

        Ok(result.0)
    }

    /// Count total number of sessions.
    pub async fn count(&self) -> Result<i64> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sessions")
            .fetch_one(self.pool.inner())
            .await?;

        Ok(result.0)
    }

    /// Check if a session exists.
    pub async fn exists(&self, id: &str) -> Result<bool> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sessions WHERE id = ?")
            .bind(id)
            .fetch_one(self.pool.inner())
            .await?;

        Ok(result.0 > 0)
    }

    /// Delete all sessions for an agent.
    pub async fn delete_by_agent(&self, agent_name: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM sessions WHERE agent_name = ?")
            .bind(agent_name)
            .execute(self.pool.inner())
            .await?;

        debug!(
            "Deleted {} sessions for agent '{}'",
            result.rows_affected(),
            agent_name
        );
        Ok(result.rows_affected())
    }
}

/// Convert a database row to a DbSession.
fn row_to_session(row: &sqlx::sqlite::SqliteRow) -> Result<DbSession> {
    let id: String = row.get("id");
    let agent_name: String = row.get("agent_name");
    let initial_prompt: Option<String> = row.get("initial_prompt");
    let created_at_str: String = row.get("created_at");
    let last_updated_str: String = row.get("last_updated");
    let message_count: i32 = row.get("message_count");
    let metadata: Option<String> = row.get("metadata");

    let created_at = parse_datetime(&created_at_str)?;
    let last_updated = parse_datetime(&last_updated_str)?;

    Ok(DbSession::new(
        id,
        agent_name,
        initial_prompt,
        created_at,
        last_updated,
        message_count,
        metadata,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_session_create() {
        let session = DbSession::create("test-id", "test-agent", Some("Hello world".to_string()));
        assert_eq!(session.id, "test-id");
        assert_eq!(session.agent_name, "test-agent");
        assert_eq!(session.initial_prompt, Some("Hello world".to_string()));
        assert_eq!(session.message_count, 0);
        assert!(session.metadata.is_none());
    }
}
