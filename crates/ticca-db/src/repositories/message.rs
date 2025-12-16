//! Message repository for database operations

use chrono::Utc;
use sqlx::Row;
use tracing::debug;

use crate::pool::DbPool;
use crate::repositories::conversation::parse_datetime;
use crate::schema::DbMessage;
use crate::{DbError, Result};

/// Repository for message database operations.
pub struct MessageRepository {
    pool: DbPool,
}

impl MessageRepository {
    /// Create a new message repository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new message in a conversation.
    ///
    /// Returns the created message.
    ///
    /// # Arguments
    /// * `id` - Unique message ID
    /// * `conversation_id` - ID of the conversation
    /// * `role` - Message role: "system", "user", "assistant", or "tool"
    /// * `content` - The message content
    /// * `tool_calls` - JSON array of tool calls (if role is 'assistant')
    /// * `tool_call_id` - ID of the tool call this responds to (if role is 'tool')
    /// * `token_count` - Optional token count
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        id: &str,
        conversation_id: &str,
        role: &str,
        content: &str,
        tool_calls: Option<&str>,
        tool_call_id: Option<&str>,
        token_count: Option<i32>,
    ) -> Result<DbMessage> {
        // Validate role
        if !is_valid_role(role) {
            return Err(DbError::Validation(format!(
                "Invalid message role: '{}'. Must be one of: system, user, assistant, tool",
                role
            )));
        }

        let now = Utc::now();
        let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();

        sqlx::query(
            r#"
            INSERT INTO messages (id, conversation_id, role, content, tool_calls, tool_call_id, timestamp, token_count)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(conversation_id)
        .bind(role)
        .bind(content)
        .bind(tool_calls)
        .bind(tool_call_id)
        .bind(&timestamp)
        .bind(token_count)
        .execute(self.pool.inner())
        .await?;

        debug!(
            "Created message {} in conversation {} with role '{}'",
            id, conversation_id, role
        );

        Ok(DbMessage::new(
            id,
            conversation_id,
            role,
            content,
            tool_calls.map(String::from),
            tool_call_id.map(String::from),
            now,
            token_count,
        ))
    }

    /// Create a simple message without tool-related fields.
    pub async fn create_simple(
        &self,
        id: &str,
        conversation_id: &str,
        role: &str,
        content: &str,
    ) -> Result<DbMessage> {
        self.create(id, conversation_id, role, content, None, None, None)
            .await
    }

    /// Get all messages for a conversation, ordered by timestamp.
    pub async fn get_by_conversation(&self, conversation_id: &str) -> Result<Vec<DbMessage>> {
        let rows = sqlx::query(
            r#"
            SELECT id, conversation_id, role, content, tool_calls, tool_call_id, timestamp, token_count
            FROM messages
            WHERE conversation_id = ?
            ORDER BY timestamp ASC
            "#,
        )
        .bind(conversation_id)
        .fetch_all(self.pool.inner())
        .await?;

        let mut messages = Vec::with_capacity(rows.len());
        for row in rows {
            messages.push(row_to_message(&row)?);
        }

        Ok(messages)
    }

    /// Delete all messages in a conversation.
    pub async fn delete_by_conversation(&self, conversation_id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM messages WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(self.pool.inner())
            .await?;

        debug!(
            "Deleted {} messages from conversation {}",
            result.rows_affected(),
            conversation_id
        );
        Ok(result.rows_affected())
    }

    /// Count messages in a conversation.
    pub async fn count_by_conversation(&self, conversation_id: &str) -> Result<i64> {
        let result: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM messages WHERE conversation_id = ?")
                .bind(conversation_id)
                .fetch_one(self.pool.inner())
                .await?;

        Ok(result.0)
    }

    /// Get the last N messages in a conversation.
    pub async fn get_last_n(
        &self,
        conversation_id: &str,
        limit: i64,
    ) -> Result<Vec<DbMessage>> {
        let rows = sqlx::query(
            r#"
            SELECT id, conversation_id, role, content, tool_calls, tool_call_id, timestamp, token_count
            FROM messages
            WHERE conversation_id = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(conversation_id)
        .bind(limit)
        .fetch_all(self.pool.inner())
        .await?;

        let mut messages = Vec::with_capacity(rows.len());
        for row in rows {
            messages.push(row_to_message(&row)?);
        }

        // Reverse to get chronological order
        messages.reverse();
        Ok(messages)
    }

    /// Get a single message by ID.
    pub async fn get_by_id(&self, id: &str) -> Result<Option<DbMessage>> {
        let result = sqlx::query(
            r#"
            SELECT id, conversation_id, role, content, tool_calls, tool_call_id, timestamp, token_count
            FROM messages WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool.inner())
        .await?;

        match result {
            Some(row) => Ok(Some(row_to_message(&row)?)),
            None => Ok(None),
        }
    }

    /// Delete a single message by ID.
    pub async fn delete(&self, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM messages WHERE id = ?")
            .bind(id)
            .execute(self.pool.inner())
            .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("Message {} not found", id)));
        }

        debug!("Deleted message {}", id);
        Ok(())
    }

    /// Update the token count for a message.
    pub async fn update_token_count(&self, id: &str, token_count: i32) -> Result<()> {
        let result = sqlx::query("UPDATE messages SET token_count = ? WHERE id = ?")
            .bind(token_count)
            .bind(id)
            .execute(self.pool.inner())
            .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("Message {} not found", id)));
        }

        debug!("Updated token count for message {} to {}", id, token_count);
        Ok(())
    }

    /// Get messages by role in a conversation.
    pub async fn get_by_role(
        &self,
        conversation_id: &str,
        role: &str,
    ) -> Result<Vec<DbMessage>> {
        let rows = sqlx::query(
            r#"
            SELECT id, conversation_id, role, content, tool_calls, tool_call_id, timestamp, token_count
            FROM messages
            WHERE conversation_id = ? AND role = ?
            ORDER BY timestamp ASC
            "#,
        )
        .bind(conversation_id)
        .bind(role)
        .fetch_all(self.pool.inner())
        .await?;

        let mut messages = Vec::with_capacity(rows.len());
        for row in rows {
            messages.push(row_to_message(&row)?);
        }

        Ok(messages)
    }

    /// Get tool response messages for a specific tool call ID.
    pub async fn get_tool_response(&self, tool_call_id: &str) -> Result<Option<DbMessage>> {
        let result = sqlx::query(
            r#"
            SELECT id, conversation_id, role, content, tool_calls, tool_call_id, timestamp, token_count
            FROM messages
            WHERE tool_call_id = ?
            "#,
        )
        .bind(tool_call_id)
        .fetch_optional(self.pool.inner())
        .await?;

        match result {
            Some(row) => Ok(Some(row_to_message(&row)?)),
            None => Ok(None),
        }
    }
}

/// Convert a database row to a DbMessage.
fn row_to_message(row: &sqlx::sqlite::SqliteRow) -> Result<DbMessage> {
    let id: String = row.get("id");
    let conversation_id: String = row.get("conversation_id");
    let role: String = row.get("role");
    let content: String = row.get("content");
    let tool_calls: Option<String> = row.get("tool_calls");
    let tool_call_id: Option<String> = row.get("tool_call_id");
    let timestamp_str: String = row.get("timestamp");
    let token_count: Option<i32> = row.get("token_count");

    let timestamp = parse_datetime(&timestamp_str)?;

    Ok(DbMessage::new(
        id,
        conversation_id,
        role,
        content,
        tool_calls,
        tool_call_id,
        timestamp,
        token_count,
    ))
}

/// Check if a role string is valid.
fn is_valid_role(role: &str) -> bool {
    matches!(role, "system" | "user" | "assistant" | "tool")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_role() {
        assert!(is_valid_role("system"));
        assert!(is_valid_role("user"));
        assert!(is_valid_role("assistant"));
        assert!(is_valid_role("tool"));
        assert!(!is_valid_role("invalid"));
        assert!(!is_valid_role("USER")); // Case-sensitive
    }
}
