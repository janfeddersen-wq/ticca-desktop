//! Bridge error types for Rust-Python interop
//!
//! This module defines all errors that can occur during bridge operations,
//! including Python runtime errors, serialization failures, and async issues.

use pyo3::PyErr;
use thiserror::Error;

/// Errors that can occur in the Rust-Python bridge
#[derive(Error, Debug)]
pub enum BridgeError {
    /// Python runtime initialization failed
    #[error("Python runtime error: {0}")]
    Runtime(String),

    /// Python exception occurred
    #[error("Python exception: {0}")]
    PythonException(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Type conversion error
    #[error("Type conversion error: {0}")]
    TypeConversion(String),

    /// Agent not found
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    /// Agent execution error
    #[error("Agent execution error: {0}")]
    AgentExecution(String),

    /// Streaming error
    #[error("Streaming error: {0}")]
    Streaming(String),

    /// Channel communication error
    #[error("Channel error: {0}")]
    Channel(String),

    /// Timeout waiting for response
    #[error("Timeout: {0}")]
    Timeout(String),

    /// The Python module was not loaded
    #[error("Module not loaded: {0}")]
    ModuleNotLoaded(String),

    /// GIL acquisition failed
    #[error("GIL acquisition failed: {0}")]
    GilAcquisition(String),

    /// Async runtime error
    #[error("Async runtime error: {0}")]
    AsyncRuntime(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<PyErr> for BridgeError {
    fn from(err: PyErr) -> Self {
        BridgeError::PythonException(err.to_string())
    }
}

impl From<serde_json::Error> for BridgeError {
    fn from(err: serde_json::Error) -> Self {
        BridgeError::Serialization(err.to_string())
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for BridgeError {
    fn from(err: tokio::sync::mpsc::error::SendError<T>) -> Self {
        BridgeError::Channel(format!("Send error: {err}"))
    }
}

impl From<tokio::task::JoinError> for BridgeError {
    fn from(err: tokio::task::JoinError) -> Self {
        BridgeError::AsyncRuntime(err.to_string())
    }
}

/// Result type alias for bridge operations
pub type BridgeResult<T> = Result<T, BridgeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = BridgeError::AgentNotFound("test-agent".to_string());
        assert_eq!(err.to_string(), "Agent not found: test-agent");
    }

    #[test]
    fn test_serialization_error_conversion() {
        let json_err = serde_json::from_str::<String>("invalid").unwrap_err();
        let bridge_err: BridgeError = json_err.into();
        assert!(matches!(bridge_err, BridgeError::Serialization(_)));
    }
}
