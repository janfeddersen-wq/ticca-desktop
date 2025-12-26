//! Configuration database operations.
//!
//! This module is split into focused submodules by domain:
//! - `settings`: Key-value settings CRUD
//! - `models`: Model configuration CRUD
//! - `discovered`: Discovered models registry cache
//! - `oauth`: OAuth tokens and accounts
//! - `api_keys`: API key accounts
//! - `mcp`: MCP server configuration
//! - `agents`: Agent model pinning

mod agents;
mod api_keys;
mod discovered;
mod mcp;
mod models;
mod oauth;
mod settings;
#[cfg(test)]
mod tests;

use crate::config::migrations;
use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;

/// Configuration database manager.
pub struct ConfigDatabase {
    conn: Connection,
}

impl ConfigDatabase {
    /// Open or create the configuration database.
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

    /// Create a database with an existing connection (for testing).
    #[cfg(test)]
    pub(crate) fn new_with_connection(conn: Connection) -> Self {
        Self { conn }
    }

    /// Get the database file path.
    pub fn get_db_path() -> Result<PathBuf> {
        Ok(super::paths::get_data_dir()?.join("config.db"))
    }

    /// Initialize database schema.
    pub(crate) fn initialize(&self) -> Result<()> {
        migrations::run_config_migrations(&self.conn)
    }
}
