//! Settings CRUD operations.

use anyhow::Result;
use rusqlite::params;

use crate::config::models::Setting;

use super::ConfigDatabase;

impl ConfigDatabase {
    /// Get a setting by key.
    pub fn get_setting(&self, key: &str) -> Result<Option<Setting>> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, value, updated_at FROM settings WHERE key = ?")?;

        let result = stmt.query_row(params![key], |row| {
            Ok(Setting {
                key: row.get(0)?,
                value: row.get(1)?,
                updated_at: row.get(2)?,
            })
        });

        match result {
            Ok(setting) => Ok(Some(setting)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Set a setting value (insert or update).
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
            params![key, value],
        )?;
        Ok(())
    }

    /// Get all settings.
    pub fn get_all_settings(&self) -> Result<Vec<Setting>> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, value, updated_at FROM settings ORDER BY key")?;

        let rows = stmt.query_map([], |row| {
            Ok(Setting {
                key: row.get(0)?,
                value: row.get(1)?,
                updated_at: row.get(2)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Delete a setting by key.
    pub fn delete_setting(&self, key: &str) -> Result<bool> {
        let changes = self
            .conn
            .execute("DELETE FROM settings WHERE key = ?", params![key])?;
        Ok(changes > 0)
    }
}
