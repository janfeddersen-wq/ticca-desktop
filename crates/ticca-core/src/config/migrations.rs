//! Database schema migrations

use anyhow::Result;
use rusqlite::Connection;

/// Run all configuration database migrations
pub fn run_config_migrations(conn: &Connection) -> Result<()> {
    // Create settings table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT DEFAULT (datetime('now'))
        )",
        [],
    )?;

    // Create models table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS models (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            model_type TEXT NOT NULL,
            endpoint_url TEXT,
            context_length INTEGER DEFAULT 128000,
            is_default INTEGER DEFAULT 0,
            config_json TEXT,
            created_at TEXT DEFAULT (datetime('now'))
        )",
        [],
    )?;

    // Create oauth_tokens table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS oauth_tokens (
            provider TEXT PRIMARY KEY,
            access_token TEXT NOT NULL,
            refresh_token TEXT,
            expires_at TEXT,
            token_type TEXT,
            scope TEXT,
            extra_json TEXT,
            updated_at TEXT DEFAULT (datetime('now'))
        )",
        [],
    )?;

    // Create oauth_accounts table for multi-account support
    conn.execute(
        "CREATE TABLE IF NOT EXISTS oauth_accounts (
            id TEXT PRIMARY KEY,
            provider TEXT NOT NULL,
            access_token TEXT NOT NULL,
            refresh_token TEXT,
            expires_at TEXT,
            token_type TEXT,
            scope TEXT,
            extra_json TEXT,
            label TEXT,
            is_active INTEGER DEFAULT 1,
            priority INTEGER DEFAULT 0,
            cooldown_until TEXT,
            last_error TEXT,
            last_429_at TEXT,
            last_used_at TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_oauth_accounts_provider ON oauth_accounts(provider)",
        [],
    )?;

    // Create MCP server tables
    conn.execute(
        "CREATE TABLE IF NOT EXISTS mcp_servers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            transport TEXT NOT NULL DEFAULT 'stdio',
            command TEXT,
            args_json TEXT,
            env_json TEXT,
            endpoint_url TEXT,
            is_enabled INTEGER DEFAULT 1,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS agent_mcp_servers (
            agent_type TEXT NOT NULL,
            server_id TEXT NOT NULL,
            created_at TEXT DEFAULT (datetime('now')),
            PRIMARY KEY(agent_type, server_id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_agent_mcp_servers_agent_type ON agent_mcp_servers(agent_type)",
        [],
    )?;

    // Best-effort migrations for new columns
    let _ = conn.execute(
        "ALTER TABLE oauth_accounts ADD COLUMN priority INTEGER DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE oauth_accounts ADD COLUMN last_used_at TEXT",
        [],
    );

    // Clean up legacy migrated placeholder accounts
    conn.execute("DELETE FROM oauth_accounts WHERE label = 'Migrated'", [])?;

    // Insert default settings if they don't exist
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES (?, ?)",
        ["theme", "dark"],
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES (?, ?)",
        ["allow_recursion", "true"],
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES (?, ?)",
        ["yolo_mode", "true"],
    )?;

    Ok(())
}
