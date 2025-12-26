//! OAuth tokens and accounts CRUD operations.

use anyhow::Result;
use rusqlite::params;

use crate::config::models::{OAuthAccount, OAuthToken};

use super::ConfigDatabase;

impl ConfigDatabase {
    // OAuth tokens CRUD

    /// Get an OAuth token by provider.
    pub fn get_oauth_token(&self, provider: &str) -> Result<Option<OAuthToken>> {
        let mut stmt = self.conn.prepare(
            "SELECT provider, access_token, refresh_token, expires_at, token_type, scope, extra_json, updated_at
             FROM oauth_tokens WHERE provider = ?",
        )?;

        let result = stmt.query_row(params![provider], |row| {
            Ok(OAuthToken {
                provider: row.get(0)?,
                access_token: row.get(1)?,
                refresh_token: row.get(2)?,
                expires_at: row.get(3)?,
                token_type: row.get(4)?,
                scope: row.get(5)?,
                extra_json: row.get(6)?,
                updated_at: row.get(7)?,
            })
        });

        match result {
            Ok(token) => Ok(Some(token)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Upsert an OAuth token.
    pub fn upsert_oauth_token(&self, token: &OAuthToken) -> Result<()> {
        self.conn.execute(
            "INSERT INTO oauth_tokens (provider, access_token, refresh_token, expires_at, token_type, scope, extra_json, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))
             ON CONFLICT(provider) DO UPDATE SET
                access_token = excluded.access_token,
                refresh_token = excluded.refresh_token,
                expires_at = excluded.expires_at,
                token_type = excluded.token_type,
                scope = excluded.scope,
                extra_json = excluded.extra_json,
                updated_at = datetime('now')",
            params![
                token.provider,
                token.access_token,
                token.refresh_token,
                token.expires_at,
                token.token_type,
                token.scope,
                token.extra_json,
            ],
        )?;
        Ok(())
    }

    /// Delete an OAuth token by provider.
    pub fn delete_oauth_token(&self, provider: &str) -> Result<bool> {
        let changes = self.conn.execute(
            "DELETE FROM oauth_tokens WHERE provider = ?",
            params![provider],
        )?;
        Ok(changes > 0)
    }

    /// Get all OAuth tokens.
    pub fn get_all_oauth_tokens(&self) -> Result<Vec<OAuthToken>> {
        let mut stmt = self.conn.prepare(
            "SELECT provider, access_token, refresh_token, expires_at, token_type, scope, extra_json, updated_at
             FROM oauth_tokens ORDER BY provider",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(OAuthToken {
                provider: row.get(0)?,
                access_token: row.get(1)?,
                refresh_token: row.get(2)?,
                expires_at: row.get(3)?,
                token_type: row.get(4)?,
                scope: row.get(5)?,
                extra_json: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    // OAuth accounts CRUD (multi-account)

    /// Get an OAuth account by ID.
    pub fn get_oauth_account(&self, id: &str) -> Result<Option<OAuthAccount>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, provider, access_token, refresh_token, expires_at, token_type, scope, extra_json,
                    label, is_active, priority, cooldown_until, last_error, last_429_at, last_used_at, created_at, updated_at
             FROM oauth_accounts WHERE id = ?",
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok(OAuthAccount {
                id: row.get(0)?,
                provider: row.get(1)?,
                access_token: row.get(2)?,
                refresh_token: row.get(3)?,
                expires_at: row.get(4)?,
                token_type: row.get(5)?,
                scope: row.get(6)?,
                extra_json: row.get(7)?,
                label: row.get(8)?,
                is_active: row.get::<_, i64>(9)? != 0,
                priority: row.get(10)?,
                cooldown_until: row.get(11)?,
                last_error: row.get(12)?,
                last_429_at: row.get(13)?,
                last_used_at: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        });

        match result {
            Ok(account) => Ok(Some(account)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List OAuth accounts, optionally filtered by provider.
    pub fn list_oauth_accounts(&self, provider: Option<&str>) -> Result<Vec<OAuthAccount>> {
        fn map_account(row: &rusqlite::Row<'_>) -> rusqlite::Result<OAuthAccount> {
            Ok(OAuthAccount {
                id: row.get(0)?,
                provider: row.get(1)?,
                access_token: row.get(2)?,
                refresh_token: row.get(3)?,
                expires_at: row.get(4)?,
                token_type: row.get(5)?,
                scope: row.get(6)?,
                extra_json: row.get(7)?,
                label: row.get(8)?,
                is_active: row.get::<_, i64>(9)? != 0,
                priority: row.get(10)?,
                cooldown_until: row.get(11)?,
                last_error: row.get(12)?,
                last_429_at: row.get(13)?,
                last_used_at: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        }

        let mut stmt = if provider.is_some() {
            self.conn.prepare(
                "SELECT id, provider, access_token, refresh_token, expires_at, token_type, scope, extra_json,
                        label, is_active, priority, cooldown_until, last_error, last_429_at, last_used_at, created_at, updated_at
                 FROM oauth_accounts WHERE provider = ? ORDER BY priority DESC, updated_at DESC",
            )?
        } else {
            self.conn.prepare(
                "SELECT id, provider, access_token, refresh_token, expires_at, token_type, scope, extra_json,
                        label, is_active, priority, cooldown_until, last_error, last_429_at, last_used_at, created_at, updated_at
                 FROM oauth_accounts ORDER BY provider, priority DESC, updated_at DESC",
            )?
        };

        let rows = if let Some(p) = provider {
            stmt.query_map(params![p], map_account)?
        } else {
            stmt.query_map([], map_account)?
        };

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// List OAuth accounts, pruning expired accounts without refresh tokens.
    pub fn list_oauth_accounts_pruned(&self, provider: Option<&str>) -> Result<Vec<OAuthAccount>> {
        let accounts = self.list_oauth_accounts(provider)?;

        let mut kept = Vec::with_capacity(accounts.len());
        for account in accounts {
            if account.is_expired() && !account.has_refresh_token() {
                let _ = self.delete_oauth_account(&account.id);
                continue;
            }
            kept.push(account);
        }

        Ok(kept)
    }

    /// Upsert an OAuth account.
    pub fn upsert_oauth_account(&self, account: &OAuthAccount) -> Result<()> {
        self.conn.execute(
            "INSERT INTO oauth_accounts (
                id, provider, access_token, refresh_token, expires_at, token_type, scope, extra_json,
                label, is_active, priority, cooldown_until, last_error, last_429_at, last_used_at, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, COALESCE(?, datetime('now')), datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                provider = excluded.provider,
                access_token = excluded.access_token,
                refresh_token = excluded.refresh_token,
                expires_at = excluded.expires_at,
                token_type = excluded.token_type,
                scope = excluded.scope,
                extra_json = excluded.extra_json,
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
                account.access_token,
                account.refresh_token,
                account.expires_at,
                account.token_type,
                account.scope,
                account.extra_json,
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

    /// Delete an OAuth account by ID.
    pub fn delete_oauth_account(&self, id: &str) -> Result<bool> {
        let changes = self
            .conn
            .execute("DELETE FROM oauth_accounts WHERE id = ?", params![id])?;
        Ok(changes > 0)
    }

    /// Set the active status of an OAuth account.
    pub fn set_oauth_account_active(&self, id: &str, is_active: bool) -> Result<bool> {
        let changes = self.conn.execute(
            "UPDATE oauth_accounts SET is_active = ?, updated_at = datetime('now') WHERE id = ?",
            params![if is_active { 1 } else { 0 }, id],
        )?;
        Ok(changes > 0)
    }

    /// Set cooldown for an OAuth account.
    pub fn set_oauth_account_cooldown(
        &self,
        id: &str,
        cooldown_until: Option<String>,
        last_error: Option<String>,
        last_429_at: Option<String>,
    ) -> Result<bool> {
        let changes = self.conn.execute(
            "UPDATE oauth_accounts
             SET cooldown_until = ?, last_error = ?, last_429_at = ?, updated_at = datetime('now')
             WHERE id = ?",
            params![cooldown_until, last_error, last_429_at, id],
        )?;
        Ok(changes > 0)
    }

    /// Clear cooldown for an OAuth account.
    pub fn clear_oauth_account_cooldown(&self, id: &str) -> Result<bool> {
        let changes = self.conn.execute(
            "UPDATE oauth_accounts
             SET cooldown_until = NULL, last_error = NULL, last_429_at = NULL, updated_at = datetime('now')
             WHERE id = ?",
            params![id],
        )?;
        Ok(changes > 0)
    }

    /// Set the last used timestamp for an OAuth account.
    pub fn set_oauth_account_last_used(&self, id: &str, last_used_at: String) -> Result<bool> {
        let changes = self.conn.execute(
            "UPDATE oauth_accounts SET last_used_at = ?, updated_at = datetime('now') WHERE id = ?",
            params![last_used_at, id],
        )?;
        Ok(changes > 0)
    }

    /// Set the priority of an OAuth account.
    pub fn set_oauth_account_priority(&self, id: &str, priority: i64) -> Result<bool> {
        let changes = self.conn.execute(
            "UPDATE oauth_accounts SET priority = ?, updated_at = datetime('now') WHERE id = ?",
            params![priority, id],
        )?;
        Ok(changes > 0)
    }
}
