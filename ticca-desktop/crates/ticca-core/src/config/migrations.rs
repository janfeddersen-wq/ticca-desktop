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
    
    // Insert default settings if they don't exist
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES (?, ?)",
        ["theme", "dark"],
    )?;
    
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES (?, ?)",
        ["allow_recursion", "true"],
    )?;
    
    Ok(())
}
