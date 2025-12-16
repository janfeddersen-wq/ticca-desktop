#![deny(clippy::unwrap_used)]

//! Ticca Config - Configuration management
//!
//! This crate handles application configuration including:
//! - Loading/saving config files (TOML format)
//! - Default paths and directories
//! - Environment-based API key loading
//! - Per-model configuration settings
//! - Agent-specific model pinning
//!
//! # Example
//!
//! ```no_run
//! use ticca_config::{Settings, AppPaths, ApiKeys};
//!
//! // Load settings from default location
//! let paths = AppPaths::new().expect("get app paths");
//! let settings = Settings::load(paths.config_file()).expect("load settings");
//!
//! // Load API keys from environment
//! let api_keys = ApiKeys::from_env();
//!
//! println!("Default agent: {}", settings.core.default_agent);
//! ```

pub mod api_keys;
pub mod boolean_parsing;
pub mod error;
pub mod loader;
pub mod model_pinning;
pub mod model_settings;
pub mod paths;
pub mod settings;

// Re-export primary types for convenience
pub use api_keys::{ApiKeys, Provider};
pub use boolean_parsing::parse_bool;
pub use error::ConfigError;
pub use model_pinning::{ModelPinRegistry, ModelResolver, ModelSource, PinnedModel};
pub use model_settings::{calculate_max_tokens, sanitize_model_name, ModelSettings, ModelSettingsRegistry};
pub use loader::{reload_settings, validate_config, AppConfig, ConfigLoader};
pub use paths::AppPaths;
pub use settings::{
    AdvancedSettings, CompactionStrategy, ContextSettings, CoreSettings, DisplaySettings,
    OpenAiSettings, ReasoningEffort, SafetySettings, Settings, UiSettings, Verbosity,
};

/// Result type alias using ConfigError
pub type Result<T> = std::result::Result<T, ConfigError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings_are_valid() {
        let settings = Settings::default();
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_settings_serialization_roundtrip() {
        let settings = Settings::default();
        let toml_str = toml::to_string_pretty(&settings).expect("serialize");
        let parsed: Settings = toml::from_str(&toml_str).expect("deserialize");
        assert!(parsed.validate().is_ok());
    }
}
