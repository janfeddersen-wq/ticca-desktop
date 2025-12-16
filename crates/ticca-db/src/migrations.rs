//! Database migration system
//!
//! Provides version-based migrations with tracking in a `_migrations` table.
//! Migrations are applied automatically on startup in order.

use sqlx::SqlitePool;
use tracing::{debug, info, warn};

use crate::{DbError, Result};

/// A database migration.
#[derive(Debug, Clone)]
pub struct Migration {
    /// Version number (used for ordering)
    pub version: i32,
    /// Short description of the migration
    pub name: &'static str,
    /// SQL to apply the migration
    pub sql: &'static str,
}

impl Migration {
    /// Create a new migration.
    pub const fn new(version: i32, name: &'static str, sql: &'static str) -> Self {
        Self { version, name, sql }
    }
}

/// All migrations to apply, in order.
/// Add new migrations at the end with incrementing version numbers.
pub static MIGRATIONS: &[Migration] = &[
    Migration::new(1, "initial_schema", include_str!("../migrations/001_initial.sql")),
];

/// Migration runner that tracks and applies migrations.
pub struct MigrationRunner<'a> {
    pool: &'a SqlitePool,
}

impl<'a> MigrationRunner<'a> {
    /// Create a new migration runner.
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Run all pending migrations.
    ///
    /// Creates the migrations tracking table if it doesn't exist,
    /// then applies any migrations that haven't been run yet.
    pub async fn run(&self) -> Result<()> {
        self.ensure_migrations_table().await?;

        let applied = self.get_applied_versions().await?;
        info!("Found {} already applied migrations", applied.len());

        let mut applied_count = 0;
        for migration in MIGRATIONS {
            if applied.contains(&migration.version) {
                debug!("Migration {} ({}) already applied, skipping", migration.version, migration.name);
                continue;
            }

            info!("Applying migration {} ({})", migration.version, migration.name);
            self.apply_migration(migration).await?;
            applied_count += 1;
        }

        if applied_count > 0 {
            info!("Applied {} new migrations", applied_count);
        } else {
            info!("Database is up to date");
        }

        Ok(())
    }

    /// Ensure the migrations tracking table exists.
    async fn ensure_migrations_table(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS _migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(self.pool)
        .await?;

        debug!("Migrations table ensured");
        Ok(())
    }

    /// Get list of already applied migration versions.
    async fn get_applied_versions(&self) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as("SELECT version FROM _migrations ORDER BY version")
            .fetch_all(self.pool)
            .await?;

        Ok(rows.into_iter().map(|(v,)| v).collect())
    }

    /// Apply a single migration.
    async fn apply_migration(&self, migration: &Migration) -> Result<()> {
        // Start a transaction for the migration
        let mut tx = self.pool.begin().await?;

        // Split the SQL into statements and execute each one
        // This handles multi-statement SQL files
        for statement in migration.sql.split(';') {
            // Remove comment lines (lines starting with --)
            let statement: String = statement
                .lines()
                .filter(|line| !line.trim().starts_with("--"))
                .collect::<Vec<_>>()
                .join("\n");
            let statement = statement.trim();

            if statement.is_empty() {
                continue;
            }

            debug!("Executing SQL: {}", &statement[..statement.len().min(100)]);

            if let Err(e) = sqlx::query(statement).execute(&mut *tx).await {
                warn!("Migration {} failed at statement: {}", migration.version, statement);
                return Err(DbError::Migration(format!(
                    "Migration {} ({}) failed: {}",
                    migration.version, migration.name, e
                )));
            }
        }

        // Record the migration as applied
        sqlx::query("INSERT INTO _migrations (version, name) VALUES (?, ?)")
            .bind(migration.version)
            .bind(migration.name)
            .execute(&mut *tx)
            .await?;

        // Commit the transaction
        tx.commit().await?;

        debug!("Migration {} ({}) applied successfully", migration.version, migration.name);
        Ok(())
    }

    /// Check if a specific migration version has been applied.
    pub async fn is_applied(&self, version: i32) -> Result<bool> {
        let result: Option<(i32,)> = sqlx::query_as(
            "SELECT version FROM _migrations WHERE version = ?"
        )
            .bind(version)
            .fetch_optional(self.pool)
            .await?;

        Ok(result.is_some())
    }

    /// Get the current database version (highest applied migration).
    pub async fn current_version(&self) -> Result<Option<i32>> {
        self.ensure_migrations_table().await?;

        let result: Option<(i32,)> = sqlx::query_as(
            "SELECT MAX(version) FROM _migrations"
        )
            .fetch_optional(self.pool)
            .await?;

        Ok(result.and_then(|(v,)| if v == 0 { None } else { Some(v) }))
    }

    /// Get the latest available migration version.
    pub fn latest_version() -> Option<i32> {
        MIGRATIONS.last().map(|m| m.version)
    }

    /// Check if the database needs migrations.
    pub async fn needs_migration(&self) -> Result<bool> {
        let current = self.current_version().await?;
        let latest = Self::latest_version();

        match (current, latest) {
            (None, Some(_)) => Ok(true),
            (Some(c), Some(l)) => Ok(c < l),
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations_are_ordered() {
        let mut prev_version = 0;
        for migration in MIGRATIONS {
            assert!(
                migration.version > prev_version,
                "Migration {} should have version > {}",
                migration.name,
                prev_version
            );
            prev_version = migration.version;
        }
    }

    #[test]
    fn test_migrations_have_content() {
        for migration in MIGRATIONS {
            assert!(!migration.name.is_empty(), "Migration {} has empty name", migration.version);
            assert!(!migration.sql.is_empty(), "Migration {} has empty SQL", migration.version);
        }
    }
}
