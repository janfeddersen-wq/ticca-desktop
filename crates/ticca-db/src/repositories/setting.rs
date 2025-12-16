//! Setting repository for key-value database operations

use std::collections::HashMap;

use chrono::Utc;
use sqlx::Row;
use tracing::debug;

use crate::pool::DbPool;
use crate::repositories::conversation::parse_datetime;
use crate::schema::Setting;
use crate::Result;

/// Check if a setting key contains sensitive information that should be redacted in logs.
///
/// # Arguments
/// * `key` - The setting key to check
///
/// # Returns
/// `true` if the key contains sensitive patterns like "key", "token", "secret", etc.
fn is_sensitive_key(key: &str) -> bool {
    let sensitive_patterns = ["key", "token", "secret", "password", "credential"];
    let key_lower = key.to_lowercase();
    sensitive_patterns.iter().any(|p| key_lower.contains(p))
}

/// Repository for setting (key-value) database operations.
pub struct SettingRepository {
    pool: DbPool,
}

impl SettingRepository {
    /// Create a new setting repository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get a setting value by key.
    ///
    /// Returns `None` if the key doesn't exist.
    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let result = sqlx::query("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(self.pool.inner())
            .await?;

        Ok(result.map(|row| row.get("value")))
    }

    /// Get a full setting (with updated_at) by key.
    ///
    /// Returns `None` if the key doesn't exist.
    pub async fn get_full(&self, key: &str) -> Result<Option<Setting>> {
        let result = sqlx::query("SELECT key, value, updated_at FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(self.pool.inner())
            .await?;

        match result {
            Some(row) => {
                let key: String = row.get("key");
                let value: String = row.get("value");
                let updated_at_str: String = row.get("updated_at");
                let updated_at = parse_datetime(&updated_at_str)?;

                Ok(Some(Setting::new(key, value, updated_at)))
            }
            None => Ok(None),
        }
    }

    /// Set a setting value.
    ///
    /// Creates the setting if it doesn't exist, or updates it if it does.
    /// Also updates the `updated_at` timestamp.
    pub async fn set(&self, key: &str, value: &str) -> Result<()> {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        sqlx::query(
            r#"
            INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
            "#,
        )
        .bind(key)
        .bind(value)
        .bind(&timestamp)
        .execute(self.pool.inner())
        .await?;

        if is_sensitive_key(key) {
            debug!("Set setting '{}' = '[REDACTED]'", key);
        } else {
            debug!("Set setting '{}' = '{}'", key, value);
        }
        Ok(())
    }

    /// Delete a setting by key.
    ///
    /// Does not return an error if the key doesn't exist.
    pub async fn delete(&self, key: &str) -> Result<()> {
        sqlx::query("DELETE FROM settings WHERE key = ?")
            .bind(key)
            .execute(self.pool.inner())
            .await?;

        debug!("Deleted setting '{}'", key);
        Ok(())
    }

    /// Get all settings as a HashMap.
    pub async fn get_all(&self) -> Result<HashMap<String, String>> {
        let rows = sqlx::query("SELECT key, value FROM settings")
            .fetch_all(self.pool.inner())
            .await?;

        let mut settings = HashMap::with_capacity(rows.len());
        for row in rows {
            let key: String = row.get("key");
            let value: String = row.get("value");
            settings.insert(key, value);
        }

        Ok(settings)
    }

    /// Get all settings as full Setting objects.
    pub async fn get_all_full(&self) -> Result<Vec<Setting>> {
        let rows = sqlx::query("SELECT key, value, updated_at FROM settings")
            .fetch_all(self.pool.inner())
            .await?;

        let mut settings = Vec::with_capacity(rows.len());
        for row in rows {
            let key: String = row.get("key");
            let value: String = row.get("value");
            let updated_at_str: String = row.get("updated_at");
            let updated_at = parse_datetime(&updated_at_str)?;

            settings.push(Setting::new(key, value, updated_at));
        }

        Ok(settings)
    }

    /// Check if a setting exists.
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM settings WHERE key = ?")
            .bind(key)
            .fetch_one(self.pool.inner())
            .await?;

        Ok(result.0 > 0)
    }

    /// Get a setting as a specific type, with a default value.
    ///
    /// Returns the default if the key doesn't exist or parsing fails.
    pub async fn get_or_default<T>(&self, key: &str, default: T) -> Result<T>
    where
        T: std::str::FromStr,
    {
        match self.get(key).await? {
            Some(value) => Ok(value.parse().unwrap_or(default)),
            None => Ok(default),
        }
    }

    /// Set multiple settings at once.
    pub async fn set_many(&self, settings: &[(impl AsRef<str>, impl AsRef<str>)]) -> Result<()> {
        for (key, value) in settings {
            self.set(key.as_ref(), value.as_ref()).await?;
        }
        Ok(())
    }

    /// Delete multiple settings by prefix.
    ///
    /// Deletes all settings where the key starts with the given prefix.
    pub async fn delete_by_prefix(&self, prefix: &str) -> Result<u64> {
        let pattern = format!("{}%", prefix);
        let result = sqlx::query("DELETE FROM settings WHERE key LIKE ?")
            .bind(&pattern)
            .execute(self.pool.inner())
            .await?;

        debug!(
            "Deleted {} settings with prefix '{}'",
            result.rows_affected(),
            prefix
        );
        Ok(result.rows_affected())
    }

    /// Get all settings with a given prefix.
    pub async fn get_by_prefix(&self, prefix: &str) -> Result<HashMap<String, String>> {
        let pattern = format!("{}%", prefix);
        let rows = sqlx::query("SELECT key, value FROM settings WHERE key LIKE ?")
            .bind(&pattern)
            .fetch_all(self.pool.inner())
            .await?;

        let mut settings = HashMap::with_capacity(rows.len());
        for row in rows {
            let key: String = row.get("key");
            let value: String = row.get("value");
            settings.insert(key, value);
        }

        Ok(settings)
    }

    /// Count total number of settings.
    pub async fn count(&self) -> Result<i64> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM settings")
            .fetch_one(self.pool.inner())
            .await?;

        Ok(result.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
