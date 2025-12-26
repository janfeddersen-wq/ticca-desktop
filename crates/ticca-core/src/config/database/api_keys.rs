//! API key accounts CRUD operations.

use anyhow::Result;
use rusqlite::params;

use crate::config::models::ApiKeyAccount;

use super::ConfigDatabase;

impl ConfigDatabase {
    /// Get an API key account by ID.
    pub fn get_api_key_account(&self, id: &str) -> Result<Option<ApiKeyAccount>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, provider, api_key, label, is_active, priority, cooldown_until,
                    last_error, last_429_at, last_used_at, created_at, updated_at
             FROM api_key_accounts WHERE id = ?",
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok(ApiKeyAccount {
                id: row.get(0)?,
                provider: row.get(1)?,
                api_key: row.get(2)?,
                label: row.get(3)?,
                is_active: row.get::<_, i64>(4)? != 0,
                priority: row.get(5)?,
                cooldown_until: row.get(6)?,
                last_error: row.get(7)?,
                last_429_at: row.get(8)?,
                last_used_at: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        });

        match result {
            Ok(account) => Ok(Some(account)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List API key accounts, optionally filtered by provider.
    pub fn list_api_key_accounts(&self, provider: Option<&str>) -> Result<Vec<ApiKeyAccount>> {
        fn map_account(row: &rusqlite::Row<'_>) -> rusqlite::Result<ApiKeyAccount> {
            Ok(ApiKeyAccount {
                id: row.get(0)?,
                provider: row.get(1)?,
                api_key: row.get(2)?,
                label: row.get(3)?,
                is_active: row.get::<_, i64>(4)? != 0,
                priority: row.get(5)?,
                cooldown_until: row.get(6)?,
                last_error: row.get(7)?,
                last_429_at: row.get(8)?,
                last_used_at: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        }

        let mut stmt = if provider.is_some() {
            self.conn.prepare(
                "SELECT id, provider, api_key, label, is_active, priority, cooldown_until,
                        last_error, last_429_at, last_used_at, created_at, updated_at
                 FROM api_key_accounts WHERE LOWER(provider) = LOWER(?) ORDER BY priority DESC, updated_at DESC",
            )?
        } else {
            self.conn.prepare(
                "SELECT id, provider, api_key, label, is_active, priority, cooldown_until,
                        last_error, last_429_at, last_used_at, created_at, updated_at
                 FROM api_key_accounts ORDER BY provider, priority DESC, updated_at DESC",
            )?
        };

        let rows = if let Some(p) = provider {
            stmt.query_map(params![p], map_account)?
        } else {
            stmt.query_map([], map_account)?
        };

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Upsert an API key account.
    pub fn upsert_api_key_account(&self, account: &ApiKeyAccount) -> Result<()> {
        self.conn.execute(
            "INSERT INTO api_key_accounts (
                id, provider, api_key, label, is_active, priority, cooldown_until,
                last_error, last_429_at, last_used_at, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, COALESCE(?, datetime('now')), datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                provider = excluded.provider,
                api_key = excluded.api_key,
                label = excluded.label,
                is_active = excluded.is_active,
                priority = excluded.priority,
                cooldown_until = excluded.cooldown_until,
                last_error = excluded.last_error,
                last_429_at = excluded.last_429_at,
                last_used_at = excluded.last_used_at,
                updated_at = datetime('now')",
            params![
                account.id,
                account.provider,
                account.api_key,
                account.label,
                if account.is_active { 1 } else { 0 },
                account.priority,
                account.cooldown_until,
                account.last_error,
                account.last_429_at,
                account.last_used_at,
                account.created_at,
            ],
        )?;
        Ok(())
    }

    /// Delete an API key account by ID.
    pub fn delete_api_key_account(&self, id: &str) -> Result<bool> {
        let changes = self
            .conn
            .execute("DELETE FROM api_key_accounts WHERE id = ?", params![id])?;
        Ok(changes > 0)
    }

    /// Set the active status of an API key account.
    pub fn set_api_key_account_active(&self, id: &str, is_active: bool) -> Result<bool> {
        let changes = self.conn.execute(
            "UPDATE api_key_accounts SET is_active = ?, updated_at = datetime('now') WHERE id = ?",
            params![if is_active { 1 } else { 0 }, id],
        )?;
        Ok(changes > 0)
    }

    /// Set cooldown for an API key account.
    pub fn set_api_key_account_cooldown(
        &self,
        id: &str,
        cooldown_until: Option<String>,
        last_error: Option<String>,
        last_429_at: Option<String>,
    ) -> Result<bool> {
        let changes = self.conn.execute(
            "UPDATE api_key_accounts
             SET cooldown_until = ?, last_error = ?, last_429_at = ?, updated_at = datetime('now')
             WHERE id = ?",
            params![cooldown_until, last_error, last_429_at, id],
        )?;
        Ok(changes > 0)
    }

    /// Clear cooldown for an API key account.
    pub fn clear_api_key_account_cooldown(&self, id: &str) -> Result<bool> {
        let changes = self.conn.execute(
            "UPDATE api_key_accounts
             SET cooldown_until = NULL, last_error = NULL, last_429_at = NULL, updated_at = datetime('now')
             WHERE id = ?",
            params![id],
        )?;
        Ok(changes > 0)
    }

    /// Set the last used timestamp for an API key account.
    pub fn set_api_key_account_last_used(&self, id: &str, last_used_at: String) -> Result<bool> {
        let changes = self.conn.execute(
            "UPDATE api_key_accounts SET last_used_at = ?, updated_at = datetime('now') WHERE id = ?",
            params![last_used_at, id],
        )?;
        Ok(changes > 0)
    }

    /// Set the priority of an API key account.
    pub fn set_api_key_account_priority(&self, id: &str, priority: i64) -> Result<bool> {
        let changes = self.conn.execute(
            "UPDATE api_key_accounts SET priority = ?, updated_at = datetime('now') WHERE id = ?",
            params![priority, id],
        )?;
        Ok(changes > 0)
    }
}
