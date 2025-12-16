//! Configuration error types

use thiserror::Error;

/// Configuration error type
#[derive(Error, Debug)]
pub enum ConfigError {
    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// TOML parsing errors
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    /// TOML serialization errors
    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    /// Path errors
    #[error("Path error: {0}")]
    Path(String),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(String),
}
