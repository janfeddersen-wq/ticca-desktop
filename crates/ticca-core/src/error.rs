//! Core error types for Ticca
//!
//! This module provides unified error handling across all Ticca components,
//! with conversions from underlying crate errors.

use thiserror::Error;

/// Core error type for the ticca-core crate
#[derive(Error, Debug)]
pub enum CoreError {
    /// Agent-related errors
    #[error("Agent error: {0}")]
    Agent(String),

    /// Session-related errors
    #[error("Session error: {0}")]
    Session(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Database errors
    #[error("Database error: {0}")]
    Database(String),

    /// Bridge/Python runtime errors
    #[error("Bridge error: {0}")]
    Bridge(String),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Initialization failed
    #[error("Initialization error: {0}")]
    Init(String),

    /// Operation timeout
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Operation cancelled
    #[error("Operation cancelled")]
    Cancelled,

    /// Internal error (unexpected state)
    #[error("Internal error: {0}")]
    Internal(String),
}

impl CoreError {
    /// Create an agent error
    pub fn agent(msg: impl Into<String>) -> Self {
        Self::Agent(msg.into())
    }

    /// Create a session error
    pub fn session(msg: impl Into<String>) -> Self {
        Self::Session(msg.into())
    }

    /// Create a config error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a database error
    pub fn database(msg: impl Into<String>) -> Self {
        Self::Database(msg.into())
    }

    /// Create a bridge error
    pub fn bridge(msg: impl Into<String>) -> Self {
        Self::Bridge(msg.into())
    }

    /// Create an initialization error
    pub fn init(msg: impl Into<String>) -> Self {
        Self::Init(msg.into())
    }

    /// Create a not found error
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    /// Create an internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Check if this is a transient error that could be retried
    pub fn is_transient(&self) -> bool {
        matches!(self, Self::Timeout(_) | Self::Io(_))
    }

    /// Check if this is a fatal error requiring restart
    pub fn is_fatal(&self) -> bool {
        matches!(self, Self::Init(_) | Self::Internal(_))
    }
}

// Conversion from bridge errors
impl From<ticca_bridge::BridgeError> for CoreError {
    fn from(err: ticca_bridge::BridgeError) -> Self {
        Self::Bridge(err.to_string())
    }
}

// Conversion from database errors
impl From<ticca_db::DbError> for CoreError {
    fn from(err: ticca_db::DbError) -> Self {
        Self::Database(err.to_string())
    }
}

// Conversion from config errors
impl From<ticca_config::ConfigError> for CoreError {
    fn from(err: ticca_config::ConfigError) -> Self {
        Self::Config(err.to_string())
    }
}

// Conversion from serde_json errors
impl From<serde_json::Error> for CoreError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = CoreError::agent("test error");
        assert!(err.to_string().contains("test error"));

        let err = CoreError::not_found("resource");
        assert!(err.to_string().contains("resource"));
    }

    #[test]
    fn test_transient_check() {
        let timeout = CoreError::Timeout("test".to_string());
        assert!(timeout.is_transient());

        let agent = CoreError::Agent("test".to_string());
        assert!(!agent.is_transient());
    }

    #[test]
    fn test_fatal_check() {
        let init = CoreError::Init("test".to_string());
        assert!(init.is_fatal());

        let session = CoreError::Session("test".to_string());
        assert!(!session.is_fatal());
    }
}
