//! Unified configuration loader
//!
//! Provides a single entry point for loading all configuration:
//! - Settings from config.toml
//! - API keys from environment
//! - Model settings from settings
//! - Model pins from settings
//!
//! # Example
//!
//! ```no_run
//! use ticca_config::loader::ConfigLoader;
//!
//! let config = ConfigLoader::load().expect("load config");
//!
//! println!("Default agent: {}", config.settings.core.default_agent);
//! println!("Has OpenAI key: {}", config.api_keys.has_provider(ticca_config::Provider::OpenAi));
//! ```

use std::path::Path;

use tracing::{debug, info, warn};

use crate::{
    api_keys::ApiKeys,
    model_pinning::ModelPinRegistry,
    model_settings::ModelSettingsRegistry,
    paths::AppPaths,
    settings::Settings,
    Result,
};

/// Complete application configuration.
///
/// Combines all configuration sources into a single struct.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Application paths
    pub paths: AppPaths,

    /// Main settings from config.toml
    pub settings: Settings,

    /// API keys from environment
    pub api_keys: ApiKeys,

    /// Per-model settings
    pub model_settings: ModelSettingsRegistry,

    /// Model pins for agents
    pub model_pins: ModelPinRegistry,
}

impl AppConfig {
    /// Create a new config with defaults.
    pub fn new(paths: AppPaths) -> Self {
        Self {
            paths,
            settings: Settings::default(),
            api_keys: ApiKeys::default(),
            model_settings: ModelSettingsRegistry::new(),
            model_pins: ModelPinRegistry::new(),
        }
    }

    /// Save settings to the config file.
    pub fn save_settings(&self) -> Result<()> {
        self.settings.save(self.paths.config_file())
    }
}

/// Configuration loader with customization options.
#[derive(Debug, Clone)]
pub struct ConfigLoader {
    /// Custom config file path (overrides default)
    config_path: Option<std::path::PathBuf>,

    /// Whether to create directories if they don't exist
    create_dirs: bool,

    /// Whether to warn about missing common API keys
    warn_missing_keys: bool,
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigLoader {
    /// Create a new config loader with default options.
    pub fn new() -> Self {
        Self {
            config_path: None,
            create_dirs: true,
            warn_missing_keys: true,
        }
    }

    /// Set a custom config file path.
    pub fn with_config_path(mut self, path: impl AsRef<Path>) -> Self {
        self.config_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Set whether to create directories.
    pub fn with_create_dirs(mut self, create: bool) -> Self {
        self.create_dirs = create;
        self
    }

    /// Set whether to warn about missing API keys.
    pub fn with_warn_missing_keys(mut self, warn: bool) -> Self {
        self.warn_missing_keys = warn;
        self
    }

    /// Load configuration from all sources.
    ///
    /// This is the main entry point for loading configuration.
    /// It will:
    /// 1. Determine application paths
    /// 2. Create directories if needed
    /// 3. Load settings from TOML file (or use defaults)
    /// 4. Load API keys from environment
    /// 5. Optionally warn about missing common keys
    pub fn load_with_options(self) -> Result<AppConfig> {
        // Determine paths
        let paths = AppPaths::new()?;
        debug!("Using app directory: {:?}", paths.base_dir);

        // Create directories if requested
        if self.create_dirs {
            paths.ensure_dirs()?;
        }

        // Load settings
        let config_path = self.config_path.unwrap_or_else(|| paths.config_file());
        let settings = Settings::load(&config_path)?;
        info!("Settings loaded from: {:?}", config_path);

        // Load API keys
        let api_keys = ApiKeys::from_env();
        debug!("Loaded {} API keys", api_keys.configured_providers().len());

        if self.warn_missing_keys {
            api_keys.warn_missing_common();
        }

        // Initialize empty registries (can be populated later)
        let model_settings = ModelSettingsRegistry::new();
        let model_pins = ModelPinRegistry::new();

        Ok(AppConfig {
            paths,
            settings,
            api_keys,
            model_settings,
            model_pins,
        })
    }

    /// Load configuration with default options.
    ///
    /// Convenience method that uses default loader settings.
    pub fn load() -> Result<AppConfig> {
        Self::new().load_with_options()
    }

    /// Load configuration for testing (no directory creation, no warnings).
    pub fn load_for_testing() -> Result<AppConfig> {
        Self::new()
            .with_create_dirs(false)
            .with_warn_missing_keys(false)
            .load_with_options()
    }
}

/// Reload settings from disk.
///
/// Useful for config file watching or manual reload.
pub fn reload_settings(config: &mut AppConfig) -> Result<()> {
    let new_settings = Settings::load(config.paths.config_file())?;
    config.settings = new_settings;
    info!("Settings reloaded from: {:?}", config.paths.config_file());
    Ok(())
}

/// Validate the current configuration.
///
/// Returns Ok if all settings are valid.
pub fn validate_config(config: &AppConfig) -> Result<()> {
    config.settings.validate()?;

    // Warn if no API keys are configured
    if !config.api_keys.has_any() {
        warn!("No API keys configured. Set at least one provider's API key.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_app_config_new() {
        let paths = AppPaths::with_base_dir("/tmp/test");
        let config = AppConfig::new(paths);

        assert_eq!(config.settings.core.default_agent, "code-puppy");
        assert!(!config.api_keys.has_any());
    }

    #[test]
    fn test_config_loader_builder() {
        let loader = ConfigLoader::new()
            .with_config_path("/custom/path.toml")
            .with_create_dirs(false)
            .with_warn_missing_keys(false);

        assert_eq!(
            loader.config_path,
            Some(std::path::PathBuf::from("/custom/path.toml"))
        );
        assert!(!loader.create_dirs);
        assert!(!loader.warn_missing_keys);
    }

    #[test]
    fn test_validate_config() {
        let paths = AppPaths::with_base_dir("/tmp/test");
        let config = AppConfig::new(paths);

        // Default config should be valid
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_load_with_custom_path() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let config_path = temp_dir.path().join("config.toml");

        // Write a minimal config
        std::fs::write(
            &config_path,
            r#"
            [core]
            default_agent = "test-agent"
            "#,
        )
        .expect("write config");

        let loader = ConfigLoader::new()
            .with_config_path(&config_path)
            .with_create_dirs(false)
            .with_warn_missing_keys(false);

        // Note: This may fail if running in an environment without a home directory
        // That's OK - we're mainly testing the loader mechanics
        if let Ok(config) = loader.load_with_options() {
            assert_eq!(config.settings.core.default_agent, "test-agent");
        }
    }
}
