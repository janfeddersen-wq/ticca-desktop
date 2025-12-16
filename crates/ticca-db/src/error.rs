//! Database error types

use thiserror::Error;

/// Database error type for the ticca-db crate
#[derive(Error, Debug)]
pub enum DbError {
    /// SQLx database errors
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// Migration errors
    #[error("Migration error: {0}")]
    Migration(String),

    /// Connection pool errors
    #[error("Connection pool error: {0}")]
    Pool(String),

    /// Record not found
    #[error("Record not found: {0}")]
    NotFound(String),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(String),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
