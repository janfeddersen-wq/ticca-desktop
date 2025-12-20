//! Configuration database operations

use crate::config::migrations;
use crate::config::models::{
    McpServer, McpTransport, ModelConfig, OAuthAccount, OAuthToken, Setting,
};
use anyhow::Result;
use directories::ProjectDirs;
use rusqlite::{Connection, params};
use std::path::PathBuf;

/// Configuration database manager
pub struct ConfigDatabase {
    conn: Connection,
}

impl ConfigDatabase {
    /// Open or create the configuration database
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
        let proj_dirs = ProjectDirs::from("", "", "ticca-desktop")
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

        let data_dir = proj_dirs.data_dir();
        Ok(data_dir.join("config.db"))
    }

    /// Initialize database schema
    fn initialize(&self) -> Result<()> {
        migrations::run_config_migrations(&self.conn)
    }

    // Settings CRUD
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

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
            params![key, value],
        )?;
        Ok(())
    }

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

    pub fn delete_setting(&self, key: &str) -> Result<bool> {
        let changes = self
            .conn
            .execute("DELETE FROM settings WHERE key = ?", params![key])?;
        Ok(changes > 0)
    }

    // Model configurations CRUD
    pub fn get_model(&self, id: &str) -> Result<Option<ModelConfig>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, model_type, endpoint_url, context_length, is_default, config_json, created_at 
             FROM models WHERE id = ?"
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok(ModelConfig {
                id: row.get(0)?,
                name: row.get(1)?,
                model_type: row.get(2)?,
                endpoint_url: row.get(3)?,
                context_length: row.get(4)?,
                is_default: row.get(5)?,
                config_json: row.get(6)?,
                created_at: row.get(7)?,
            })
        });

        match result {
            Ok(model) => Ok(Some(model)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_all_models(&self) -> Result<Vec<ModelConfig>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, model_type, endpoint_url, context_length, is_default, config_json, created_at 
             FROM models ORDER BY name"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(ModelConfig {
                id: row.get(0)?,
                name: row.get(1)?,
                model_type: row.get(2)?,
                endpoint_url: row.get(3)?,
                context_length: row.get(4)?,
                is_default: row.get(5)?,
                config_json: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn get_default_model(&self) -> Result<Option<ModelConfig>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, model_type, endpoint_url, context_length, is_default, config_json, created_at 
             FROM models WHERE is_default = 1 LIMIT 1"
        )?;

        let result = stmt.query_row([], |row| {
            Ok(ModelConfig {
                id: row.get(0)?,
                name: row.get(1)?,
                model_type: row.get(2)?,
                endpoint_url: row.get(3)?,
                context_length: row.get(4)?,
                is_default: row.get(5)?,
                config_json: row.get(6)?,
                created_at: row.get(7)?,
            })
        });

        match result {
            Ok(model) => Ok(Some(model)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn upsert_model(&self, model: &ModelConfig) -> Result<()> {
        // If this model is default, clear other defaults first
        if model.is_default {
            self.conn.execute("UPDATE models SET is_default = 0", [])?;
        }

        self.conn.execute(
            "INSERT INTO models (id, name, model_type, endpoint_url, context_length, is_default, config_json, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, COALESCE(?, datetime('now')))
             ON CONFLICT(id) DO UPDATE SET 
                name = excluded.name,
                model_type = excluded.model_type,
                endpoint_url = excluded.endpoint_url,
                context_length = excluded.context_length,
                is_default = excluded.is_default,
                config_json = excluded.config_json",
            params![
                model.id,
                model.name,
                model.model_type,
                model.endpoint_url,
                model.context_length,
                model.is_default,
                model.config_json,
                model.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn delete_model(&self, id: &str) -> Result<bool> {
        let changes = self
            .conn
            .execute("DELETE FROM models WHERE id = ?", params![id])?;
        Ok(changes > 0)
    }

    // OAuth tokens CRUD
    pub fn get_oauth_token(&self, provider: &str) -> Result<Option<OAuthToken>> {
        let mut stmt = self.conn.prepare(
            "SELECT provider, access_token, refresh_token, expires_at, token_type, scope, extra_json, updated_at 
             FROM oauth_tokens WHERE provider = ?"
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

    // OAuth accounts CRUD (multi-account)
    pub fn get_oauth_account(&self, id: &str) -> Result<Option<OAuthAccount>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, provider, access_token, refresh_token, expires_at, token_type, scope, extra_json,
                    label, is_active, priority, cooldown_until, last_error, last_429_at, last_used_at, created_at, updated_at
             FROM oauth_accounts WHERE id = ?"
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
                 FROM oauth_accounts WHERE provider = ? ORDER BY priority DESC, updated_at DESC"
            )?
        } else {
            self.conn.prepare(
                "SELECT id, provider, access_token, refresh_token, expires_at, token_type, scope, extra_json,
                        label, is_active, priority, cooldown_until, last_error, last_429_at, last_used_at, created_at, updated_at
                 FROM oauth_accounts ORDER BY provider, priority DESC, updated_at DESC"
            )?
        };

        let rows = if let Some(p) = provider {
            stmt.query_map(params![p], map_account)?
        } else {
            stmt.query_map([], map_account)?
        };

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

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

    pub fn delete_oauth_account(&self, id: &str) -> Result<bool> {
        let changes = self
            .conn
            .execute("DELETE FROM oauth_accounts WHERE id = ?", params![id])?;
        Ok(changes > 0)
    }

    pub fn set_oauth_account_active(&self, id: &str, is_active: bool) -> Result<bool> {
        let changes = self.conn.execute(
            "UPDATE oauth_accounts SET is_active = ?, updated_at = datetime('now') WHERE id = ?",
            params![if is_active { 1 } else { 0 }, id],
        )?;
        Ok(changes > 0)
    }

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

    pub fn clear_oauth_account_cooldown(&self, id: &str) -> Result<bool> {
        let changes = self.conn.execute(
            "UPDATE oauth_accounts
             SET cooldown_until = NULL, last_error = NULL, last_429_at = NULL, updated_at = datetime('now')
             WHERE id = ?",
            params![id],
        )?;
        Ok(changes > 0)
    }

    pub fn set_oauth_account_last_used(&self, id: &str, last_used_at: String) -> Result<bool> {
        let changes = self.conn.execute(
            "UPDATE oauth_accounts SET last_used_at = ?, updated_at = datetime('now') WHERE id = ?",
            params![last_used_at, id],
        )?;
        Ok(changes > 0)
    }

    pub fn set_oauth_account_priority(&self, id: &str, priority: i64) -> Result<bool> {
        let changes = self.conn.execute(
            "UPDATE oauth_accounts SET priority = ?, updated_at = datetime('now') WHERE id = ?",
            params![priority, id],
        )?;
        Ok(changes > 0)
    }

    pub fn delete_oauth_token(&self, provider: &str) -> Result<bool> {
        let changes = self.conn.execute(
            "DELETE FROM oauth_tokens WHERE provider = ?",
            params![provider],
        )?;
        Ok(changes > 0)
    }

    pub fn get_all_oauth_tokens(&self) -> Result<Vec<OAuthToken>> {
        let mut stmt = self.conn.prepare(
            "SELECT provider, access_token, refresh_token, expires_at, token_type, scope, extra_json, updated_at 
             FROM oauth_tokens ORDER BY provider"
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

    // Agent model pinning

    /// Get the pinned model for a specific agent
    pub fn get_agent_pinned_model(&self, agent_type: &str) -> Result<Option<String>> {
        let key = format!("agent_pinned_model.{}", agent_type);
        match self.get_setting(&key)? {
            Some(setting) if !setting.value.is_empty() => Ok(Some(setting.value)),
            _ => Ok(None),
        }
    }

    /// Set the pinned model for a specific agent
    pub fn set_agent_pinned_model(&self, agent_type: &str, model_name: &str) -> Result<()> {
        let key = format!("agent_pinned_model.{}", agent_type);
        self.set_setting(&key, model_name)
    }

    /// Clear the pinned model for a specific agent
    pub fn clear_agent_pinned_model(&self, agent_type: &str) -> Result<()> {
        let key = format!("agent_pinned_model.{}", agent_type);
        self.delete_setting(&key)?;
        Ok(())
    }

    /// Get all agent-model pinnings
    pub fn get_all_agent_pinned_models(&self) -> Result<std::collections::HashMap<String, String>> {
        let settings = self.get_all_settings()?;
        let mut pinnings = std::collections::HashMap::new();

        for setting in settings {
            if let Some(agent_type) = setting.key.strip_prefix("agent_pinned_model.")
                && !setting.value.is_empty()
            {
                pinnings.insert(agent_type.to_string(), setting.value);
            }
        }

        Ok(pinnings)
    }

    // MCP servers

    pub fn list_mcp_servers(&self) -> Result<Vec<McpServer>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, transport, command, args_json, env_json, endpoint_url, is_enabled, created_at, updated_at
             FROM mcp_servers
             ORDER BY name",
        )?;

        let rows = stmt.query_map([], |row| {
            let transport_str: String = row.get(2)?;
            let transport = transport_str
                .parse::<McpTransport>()
                .unwrap_or(McpTransport::Stdio);

            let args_json: Option<String> = row.get(4)?;
            let args: Vec<String> = args_json
                .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
                .unwrap_or_default();

            let env_json: Option<String> = row.get(5)?;
            let env: std::collections::BTreeMap<String, String> = env_json
                .and_then(|s| {
                    serde_json::from_str::<std::collections::BTreeMap<String, String>>(&s).ok()
                })
                .unwrap_or_default();

            Ok(McpServer {
                id: row.get(0)?,
                name: row.get(1)?,
                transport,
                command: row.get(3)?,
                args,
                env,
                endpoint_url: row.get(6)?,
                is_enabled: {
                    let val: i64 = row.get(7)?;
                    val != 0
                },
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn upsert_mcp_server(&self, server: &McpServer) -> Result<()> {
        let args_json = if server.args.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&server.args)?)
        };
        let env_json = if server.env.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&server.env)?)
        };

        self.conn.execute(
            "INSERT INTO mcp_servers (id, name, transport, command, args_json, env_json, endpoint_url, is_enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, COALESCE(?, datetime('now')), datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                transport = excluded.transport,
                command = excluded.command,
                args_json = excluded.args_json,
                env_json = excluded.env_json,
                endpoint_url = excluded.endpoint_url,
                is_enabled = excluded.is_enabled,
                updated_at = datetime('now')",
            params![
                server.id,
                server.name,
                server.transport.as_str(),
                server.command,
                args_json,
                env_json,
                server.endpoint_url,
                if server.is_enabled { 1 } else { 0 },
                server.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn delete_mcp_server(&self, server_id: &str) -> Result<bool> {
        self.conn.execute(
            "DELETE FROM agent_mcp_servers WHERE server_id = ?",
            params![server_id],
        )?;
        let changes = self
            .conn
            .execute("DELETE FROM mcp_servers WHERE id = ?", params![server_id])?;
        Ok(changes > 0)
    }

    pub fn set_mcp_server_enabled(&self, server_id: &str, enabled: bool) -> Result<bool> {
        let changes = self.conn.execute(
            "UPDATE mcp_servers SET is_enabled = ?, updated_at = datetime('now') WHERE id = ?",
            params![if enabled { 1 } else { 0 }, server_id],
        )?;
        Ok(changes > 0)
    }

    pub fn get_agent_mcp_server_ids(&self, agent_type: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT server_id
             FROM agent_mcp_servers
             WHERE agent_type = ?
             ORDER BY server_id",
        )?;

        let rows = stmt.query_map(params![agent_type], |row| Ok(row.get(0)?))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn set_agent_mcp_server_ids(&self, agent_type: &str, server_ids: &[String]) -> Result<()> {
        self.conn.execute(
            "DELETE FROM agent_mcp_servers WHERE agent_type = ?",
            params![agent_type],
        )?;
        for server_id in server_ids {
            self.conn.execute(
                "INSERT OR IGNORE INTO agent_mcp_servers (agent_type, server_id) VALUES (?, ?)",
                params![agent_type, server_id],
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use tempfile::TempDir;
    use uuid::Uuid;

    struct TestDb {
        db: ConfigDatabase,
        _dir: TempDir,
    }

    fn test_db() -> TestDb {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test_config.db");
        let conn = Connection::open(&db_path).unwrap();
        let db = ConfigDatabase { conn };
        db.initialize().unwrap();
        TestDb { db, _dir: dir }
    }

    #[test]
    fn test_mcp_servers_crud_and_agent_mapping() {
        let test = test_db();
        let db = &test.db;

        let server = McpServer::new(Uuid::new_v4().to_string(), "filesystem")
            .with_stdio_command("mcp-filesystem")
            .with_args(vec!["--root".to_string(), "/tmp".to_string()]);

        db.upsert_mcp_server(&server).unwrap();

        let listed = db.list_mcp_servers().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "filesystem");
        assert_eq!(listed[0].transport, McpTransport::Stdio);
        assert_eq!(listed[0].command.as_deref(), Some("mcp-filesystem"));

        db.set_agent_mcp_server_ids("coding", &[server.id.clone()])
            .unwrap();
        let ids = db.get_agent_mcp_server_ids("coding").unwrap();
        assert_eq!(ids, vec![server.id.clone()]);

        db.set_mcp_server_enabled(&server.id, false).unwrap();
        let listed = db.list_mcp_servers().unwrap();
        assert!(!listed[0].is_enabled);

        db.delete_mcp_server(&server.id).unwrap();
        assert!(db.list_mcp_servers().unwrap().is_empty());
        assert!(db.get_agent_mcp_server_ids("coding").unwrap().is_empty());
    }

    #[test]
    fn test_settings_crud() {
        let test = test_db();
        let db = &test.db;

        // Get default settings (theme is set by migrations)
        let theme = db.get_setting("theme").unwrap();
        assert!(theme.is_some());
        assert_eq!(theme.unwrap().value, "dark");

        // Set custom setting
        db.set_setting("test_key", "test_value").unwrap();
        let setting = db.get_setting("test_key").unwrap().unwrap();
        assert_eq!(setting.value, "test_value");

        // Update setting
        db.set_setting("test_key", "updated_value").unwrap();
        let setting = db.get_setting("test_key").unwrap().unwrap();
        assert_eq!(setting.value, "updated_value");

        // Delete setting
        assert!(db.delete_setting("test_key").unwrap());
        assert!(db.get_setting("test_key").unwrap().is_none());
    }

    #[test]
    fn test_models_crud() {
        let test = test_db();
        let db = &test.db;

        // Create model
        let model = ModelConfig::new("claude-3", "Claude 3 Sonnet", "anthropic")
            .with_context_length(200000)
            .as_default();
        db.upsert_model(&model).unwrap();

        // Read model
        let retrieved = db.get_model("claude-3").unwrap().unwrap();
        assert_eq!(retrieved.name, "Claude 3 Sonnet");
        assert_eq!(retrieved.context_length, 200000);
        assert!(retrieved.is_default);

        // Get default model
        let default = db.get_default_model().unwrap().unwrap();
        assert_eq!(default.id, "claude-3");

        // Delete model
        assert!(db.delete_model("claude-3").unwrap());
        assert!(db.get_model("claude-3").unwrap().is_none());
    }

    #[test]
    fn test_oauth_tokens_crud() {
        let test = test_db();
        let db = &test.db;

        // Create token
        let token = OAuthToken::new("claude", "access_token_123").with_refresh_token("refresh_456");
        db.upsert_oauth_token(&token).unwrap();

        // Read token
        let retrieved = db.get_oauth_token("claude").unwrap().unwrap();
        assert_eq!(retrieved.access_token, "access_token_123");
        assert_eq!(retrieved.refresh_token, Some("refresh_456".to_string()));

        // Delete token
        assert!(db.delete_oauth_token("claude").unwrap());
        assert!(db.get_oauth_token("claude").unwrap().is_none());
    }

    #[test]
    fn test_oauth_accounts_prune_expired_without_refresh_token() {
        let test = test_db();
        let db = &test.db;

        let mut expired = OAuthAccount::new("expired", "claude", "access_token");
        expired.expires_at = Some((Utc::now() - Duration::seconds(60)).to_rfc3339());
        expired.refresh_token = None;
        db.upsert_oauth_account(&expired).unwrap();

        let mut expired_empty_refresh =
            OAuthAccount::new("expired_empty", "claude", "access_token");
        expired_empty_refresh.expires_at = Some((Utc::now() - Duration::seconds(60)).to_rfc3339());
        expired_empty_refresh.refresh_token = Some("".to_string());
        db.upsert_oauth_account(&expired_empty_refresh).unwrap();

        let mut expired_with_refresh =
            OAuthAccount::new("expired_with_refresh", "claude", "access_token");
        expired_with_refresh.expires_at = Some((Utc::now() - Duration::seconds(60)).to_rfc3339());
        expired_with_refresh.refresh_token = Some("refresh_token".to_string());
        db.upsert_oauth_account(&expired_with_refresh).unwrap();

        let listed = db.list_oauth_accounts_pruned(Some("claude")).unwrap();
        let ids: Vec<String> = listed.into_iter().map(|a| a.id).collect();
        assert_eq!(ids, vec!["expired_with_refresh".to_string()]);

        assert!(db.get_oauth_account("expired").unwrap().is_none());
        assert!(db.get_oauth_account("expired_empty").unwrap().is_none());
        assert!(
            db.get_oauth_account("expired_with_refresh")
                .unwrap()
                .is_some()
        );
    }
}
