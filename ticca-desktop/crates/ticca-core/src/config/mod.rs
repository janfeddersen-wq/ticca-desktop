//! Configuration storage module

pub mod database;
pub mod migrations;
pub mod models;

pub use database::ConfigDatabase;
pub use models::{ModelConfig, OAuthToken, Setting};

/// Well-known setting keys
pub mod setting_keys {
    pub const THEME: &str = "theme";
    pub const DEFAULT_MODEL: &str = "default_model";
    pub const AUTO_SAVE_SESSIONS: &str = "auto_save_sessions";
}
