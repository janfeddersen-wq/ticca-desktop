//! Database connection pool management
//!
//! Provides SQLite connection pooling with WAL mode and proper configuration.

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;
use tracing::info;

use crate::{DbError, Result};

/// Database connection pool wrapper.
///
/// Manages SQLite connections with connection pooling and WAL mode enabled.
#[derive(Clone)]
pub struct DbPool {
    pool: SqlitePool,
}

impl DbPool {
    /// Create a new database pool from a file path.
    ///
    /// Creates the database file if it doesn't exist.
    /// Uses WAL mode for better concurrency.
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = db_path.as_ref();
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        info!("Connecting to database: {}", db_path.display());

        let options = SqliteConnectOptions::from_str(&db_url)
            .map_err(|e| DbError::Pool(e.to_string()))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .foreign_keys(true); // Enable foreign key constraints

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        Ok(Self { pool })
    }

    /// Create an in-memory database (useful for testing).
    ///
    /// Uses a shared cache so all connections see the same data.
    /// Note: In-memory databases are ephemeral and don't support WAL mode.
    pub async fn in_memory() -> Result<Self> {
        // Use a unique name for each pool to avoid test interference
        // The shared cache allows multiple connections to see the same data
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let uri = format!("file:memdb{}?mode=memory&cache=shared", id);

        let options = SqliteConnectOptions::from_str(&uri)
            .map_err(|e| DbError::Pool(e.to_string()))?
            .foreign_keys(true); // Enable foreign key constraints

        // In-memory databases must use a single connection to persist data
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;

        Ok(Self { pool })
    }

    /// Get a reference to the underlying SQLx pool.
    pub fn inner(&self) -> &SqlitePool {
        &self.pool
    }

    /// Close the database pool.
    ///
    /// Waits for all active connections to complete before closing.
    pub async fn close(&self) {
        self.pool.close().await;
        info!("Database pool closed");
    }

    /// Check if the pool is closed.
    pub fn is_closed(&self) -> bool {
        self.pool.is_closed()
    }
}
