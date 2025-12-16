//! Application paths and directories
//!
//! Uses `~/.ticca_desktop/` as the base application directory.
//! Falls back to platform-specific directories if home directory
//! cannot be determined.

use directories::BaseDirs;
use std::path::PathBuf;
use tracing::debug;

use crate::{ConfigError, Result};

/// Application directory name in user's home.
const APP_DIR_NAME: &str = ".ticca_desktop";

/// Application paths manager
///
/// Base directory: `~/.ticca_desktop/`
///
/// Layout:
/// - `~/.ticca_desktop/` - config dir (contains config.toml)
/// - `~/.ticca_desktop/data/` - data dir (databases, etc.)
/// - `~/.ticca_desktop/cache/` - cache dir (temporary files)
/// - `~/.ticca_desktop/logs/` - log files
#[derive(Debug, Clone)]
pub struct AppPaths {
    /// Base application directory (~/.ticca_desktop/)
    pub base_dir: PathBuf,
    /// Configuration directory (same as base_dir)
    pub config_dir: PathBuf,
    /// Data directory (~/.ticca_desktop/data/)
    pub data_dir: PathBuf,
    /// Cache directory (~/.ticca_desktop/cache/)
    pub cache_dir: PathBuf,
    /// Log directory (~/.ticca_desktop/logs/)
    pub log_dir: PathBuf,
}

impl AppPaths {
    /// Create a new AppPaths instance using `~/.ticca_desktop/`
    ///
    /// Uses the `directories` crate to resolve the home directory
    /// in a platform-appropriate way.
    pub fn new() -> Result<Self> {
        let base_dirs = BaseDirs::new()
            .ok_or_else(|| ConfigError::Path("Failed to determine home directory".into()))?;

        let base_dir = base_dirs.home_dir().join(APP_DIR_NAME);

        let paths = Self {
            config_dir: base_dir.clone(),
            data_dir: base_dir.join("data"),
            cache_dir: base_dir.join("cache"),
            log_dir: base_dir.join("logs"),
            base_dir,
        };

        debug!("App paths initialized: {:?}", paths);
        Ok(paths)
    }

    /// Create a new AppPaths with a custom base directory.
    ///
    /// Useful for testing or portable installations.
    pub fn with_base_dir(base_dir: impl Into<PathBuf>) -> Self {
        let base_dir = base_dir.into();
        Self {
            config_dir: base_dir.clone(),
            data_dir: base_dir.join("data"),
            cache_dir: base_dir.join("cache"),
            log_dir: base_dir.join("logs"),
            base_dir,
        }
    }

    /// Create directories if they don't exist
    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.base_dir)?;
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.cache_dir)?;
        std::fs::create_dir_all(&self.log_dir)?;
        debug!("App directories created at: {:?}", self.base_dir);
        Ok(())
    }

    /// Get the path to the main configuration file
    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    /// Get the path to the models configuration file
    pub fn models_file(&self) -> PathBuf {
        self.config_dir.join("models.toml")
    }

    /// Get the path to the agents configuration file
    pub fn agents_file(&self) -> PathBuf {
        self.config_dir.join("agents.toml")
    }

    /// Get the path to the main database file
    pub fn database_file(&self) -> PathBuf {
        self.data_dir.join("ticca.db")
    }

    /// Get the path to the sessions directory
    pub fn sessions_dir(&self) -> PathBuf {
        self.data_dir.join("sessions")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_with_base_dir() {
        let paths = AppPaths::with_base_dir("/tmp/test_ticca");
        
        assert_eq!(paths.base_dir, Path::new("/tmp/test_ticca"));
        assert_eq!(paths.config_dir, Path::new("/tmp/test_ticca"));
        assert_eq!(paths.data_dir, Path::new("/tmp/test_ticca/data"));
        assert_eq!(paths.cache_dir, Path::new("/tmp/test_ticca/cache"));
        assert_eq!(paths.log_dir, Path::new("/tmp/test_ticca/logs"));
    }

    #[test]
    fn test_config_files() {
        let paths = AppPaths::with_base_dir("/home/user/.ticca_desktop");
        
        assert_eq!(paths.config_file(), Path::new("/home/user/.ticca_desktop/config.toml"));
        assert_eq!(paths.models_file(), Path::new("/home/user/.ticca_desktop/models.toml"));
        assert_eq!(paths.agents_file(), Path::new("/home/user/.ticca_desktop/agents.toml"));
        assert_eq!(paths.database_file(), Path::new("/home/user/.ticca_desktop/data/ticca.db"));
        assert_eq!(paths.sessions_dir(), Path::new("/home/user/.ticca_desktop/data/sessions"));
    }

    #[test]
    fn test_new_uses_home_directory() {
        // This test verifies the structure is correct
        // Note: We can't test the exact path since it depends on the user's home
        if let Ok(paths) = AppPaths::new() {
            // base_dir should end with .ticca_desktop
            assert!(paths.base_dir.ends_with(".ticca_desktop"));
            
            // data_dir should be under base_dir
            assert!(paths.data_dir.starts_with(&paths.base_dir));
            
            // cache_dir should be under base_dir
            assert!(paths.cache_dir.starts_with(&paths.base_dir));
            
            // log_dir should be under base_dir
            assert!(paths.log_dir.starts_with(&paths.base_dir));
        }
    }
}
