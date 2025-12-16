//! Conversation repository for database operations

use chrono::{DateTime, Utc};
use sqlx::Row;
use tracing::debug;

use crate::pool::DbPool;
use crate::schema::Conversation;
use crate::{DbError, Result};

/// Repository for conversation database operations.
pub struct ConversationRepository {
    pool: DbPool,
}

impl ConversationRepository {
    /// Create a new conversation repository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new conversation with the given ID and title.
    ///
    /// Returns the created conversation.
    pub async fn create(&self, id: &str, title: &str) -> Result<Conversation> {
        let now = Utc::now();
        let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();

        sqlx::query(
            "INSERT INTO conversations (id, title, created_at, updated_at) VALUES (?, ?, ?, ?)",
        )
        .bind(id)
        .bind(title)
        .bind(&timestamp)
        .bind(&timestamp)
        .execute(self.pool.inner())
        .await?;

        debug!("Created conversation {} with title '{}'", id, title);
        Ok(Conversation::new(id, title, now, now))
    }

    /// Get a conversation by ID.
    ///
    /// Returns `None` if the conversation doesn't exist.
    pub async fn get_by_id(&self, id: &str) -> Result<Option<Conversation>> {
        let result = sqlx::query(
            "SELECT id, title, created_at, updated_at FROM conversations WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.pool.inner())
        .await?;

        match result {
            Some(row) => {
                let id: String = row.get("id");
                let title: String = row.get("title");
                let created_at_str: String = row.get("created_at");
                let updated_at_str: String = row.get("updated_at");
                let created_at = parse_datetime(&created_at_str)?;
                let updated_at = parse_datetime(&updated_at_str)?;

                Ok(Some(Conversation::new(id, title, created_at, updated_at)))
            }
            None => Ok(None),
        }
    }

    /// List all conversations, ordered by updated date (newest first).
    pub async fn list_all(&self) -> Result<Vec<Conversation>> {
        let rows = sqlx::query(
            "SELECT id, title, created_at, updated_at FROM conversations ORDER BY updated_at DESC",
        )
        .fetch_all(self.pool.inner())
        .await?;

        let mut conversations = Vec::with_capacity(rows.len());
        for row in rows {
            let id: String = row.get("id");
            let title: String = row.get("title");
            let created_at_str: String = row.get("created_at");
            let updated_at_str: String = row.get("updated_at");
            let created_at = parse_datetime(&created_at_str)?;
            let updated_at = parse_datetime(&updated_at_str)?;

            conversations.push(Conversation::new(id, title, created_at, updated_at));
        }

        Ok(conversations)
    }

    /// Delete a conversation by ID.
    ///
    /// Also deletes all associated messages due to ON DELETE CASCADE.
    pub async fn delete(&self, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM conversations WHERE id = ?")
            .bind(id)
            .execute(self.pool.inner())
            .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("Conversation {} not found", id)));
        }

        debug!("Deleted conversation {}", id);
        Ok(())
    }

    /// Update the title of a conversation.
    ///
    /// Also updates the `updated_at` timestamp.
    pub async fn update_title(&self, id: &str, title: &str) -> Result<()> {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let result = sqlx::query(
            "UPDATE conversations SET title = ?, updated_at = ? WHERE id = ?",
        )
        .bind(title)
        .bind(&timestamp)
        .bind(id)
        .execute(self.pool.inner())
        .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("Conversation {} not found", id)));
        }

        debug!("Updated conversation {} title to '{}'", id, title);
        Ok(())
    }

    /// Touch the updated_at timestamp for a conversation.
    ///
    /// Useful for marking a conversation as recently accessed.
    pub async fn touch(&self, id: &str) -> Result<()> {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let result = sqlx::query("UPDATE conversations SET updated_at = ? WHERE id = ?")
            .bind(&timestamp)
            .bind(id)
            .execute(self.pool.inner())
            .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("Conversation {} not found", id)));
        }

        debug!("Touched conversation {}", id);
        Ok(())
    }

    /// Get the most recent conversation.
    pub async fn get_most_recent(&self) -> Result<Option<Conversation>> {
        let result = sqlx::query(
            "SELECT id, title, created_at, updated_at FROM conversations ORDER BY updated_at DESC LIMIT 1",
        )
        .fetch_optional(self.pool.inner())
        .await?;

        match result {
            Some(row) => {
                let id: String = row.get("id");
                let title: String = row.get("title");
                let created_at_str: String = row.get("created_at");
                let updated_at_str: String = row.get("updated_at");
                let created_at = parse_datetime(&created_at_str)?;
                let updated_at = parse_datetime(&updated_at_str)?;

                Ok(Some(Conversation::new(id, title, created_at, updated_at)))
            }
            None => Ok(None),
        }
    }

    /// Count total number of conversations.
    pub async fn count(&self) -> Result<i64> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM conversations")
            .fetch_one(self.pool.inner())
            .await?;

        Ok(result.0)
    }

    /// Check if a conversation exists.
    pub async fn exists(&self, id: &str) -> Result<bool> {
        let result: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM conversations WHERE id = ?")
                .bind(id)
                .fetch_one(self.pool.inner())
                .await?;

        Ok(result.0 > 0)
    }
}

/// Parse a datetime string from SQLite format to DateTime<Utc>.
///
/// SQLite stores datetimes as TEXT in ISO 8601 format.
pub(crate) fn parse_datetime(s: &str) -> Result<DateTime<Utc>> {
    // SQLite datetime format: "YYYY-MM-DD HH:MM:SS"
    // We need to handle both with and without timezone
    let dt = if s.contains('+') || s.ends_with('Z') {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                DbError::Serialization(format!("Failed to parse datetime '{}': {}", s, e))
            })?
    } else {
        // SQLite's datetime() function produces "YYYY-MM-DD HH:MM:SS"
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
            .map(|ndt| ndt.and_utc())
            .map_err(|e| {
                DbError::Serialization(format!("Failed to parse datetime '{}': {}", s, e))
            })?
    };

    Ok(dt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_datetime_sqlite_format() {
        let dt = parse_datetime("2024-01-15 10:30:45").expect("parse");
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_datetime_rfc3339() {
        let dt = parse_datetime("2024-01-15T10:30:45Z").expect("parse");
        assert_eq!(dt.year(), 2024);
    }
}
