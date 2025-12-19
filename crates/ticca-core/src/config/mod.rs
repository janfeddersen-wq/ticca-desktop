//! Configuration storage module

pub mod database;
pub mod migrations;
pub mod models;
pub mod repo;
pub mod settings;

pub use database::ConfigDatabase;
pub use models::{ModelConfig, OAuthToken, OAuthAccount, Setting};
pub use repo::ConfigRepo;
pub use settings::{AccountRotationPolicy, TypedSettings};

/// Well-known setting keys
pub mod setting_keys {
    pub const THEME: &str = "theme";
    pub const DEFAULT_MODEL: &str = "default_model";
    pub const AUTO_SAVE_SESSIONS: &str = "auto_save_sessions";
    /// Maximum number of tool call rounds in the ReAct loop (default: 500)
    pub const MAX_TOOL_ROUNDS: &str = "max_tool_rounds";
    /// When enabled, tools run without per-action approval prompts.
    pub const YOLO_MODE: &str = "yolo_mode";
    /// How multiple OAuth accounts are rotated for a provider.
    pub const ACCOUNT_ROTATION_POLICY: &str = "account_rotation_policy";
}

/// Default values for settings
pub mod defaults {
    pub const THEME: &str = "dark";
    pub const MAX_TOOL_ROUNDS: u32 = 500;
    pub const YOLO_MODE: bool = true;
    pub const ACCOUNT_ROTATION_POLICY: super::AccountRotationPolicy =
        super::AccountRotationPolicy::PriorityThenLeastRecentlyUsed;
}
