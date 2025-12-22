//! Configuration storage module

pub mod database;
mod mcp_import;
pub mod migrations;
pub mod models;
pub mod paths;
pub mod repo;
pub mod service;
pub mod settings;

pub use database::ConfigDatabase;
pub use models::{McpServer, McpTransport, ModelConfig, OAuthAccount, OAuthToken, Setting};
pub use paths::{
    ensure_dirs_exist, get_bin_dir, get_data_dir, get_skills_dir, get_tools_dir,
    get_uv_binary_path, get_venvs_dir,
};
pub use repo::ConfigRepo;
pub use service::{ConfigService, SettingsSnapshot};
pub use settings::{AccountRotationPolicy, TypedSettings};

/// Well-known setting keys
pub mod setting_keys {
    pub const THEME: &str = "theme";
    pub const DEFAULT_MODEL: &str = "default_model";
    pub const AUTO_SAVE_SESSIONS: &str = "auto_save_sessions";
    /// When enabled, show advanced UI sections (accounts list, models/agents tabs, etc).
    pub const EXPERT_MODE: &str = "expert_mode";
    /// Maximum number of tool call rounds in the ReAct loop (default: 500)
    pub const MAX_TOOL_ROUNDS: &str = "max_tool_rounds";
    /// When enabled, tools run without per-action approval prompts.
    pub const YOLO_MODE: &str = "yolo_mode";
    /// How multiple OAuth accounts are rotated for a provider.
    pub const ACCOUNT_ROTATION_POLICY: &str = "account_rotation_policy";
    /// Whether the external tools install prompt has been dismissed permanently.
    pub const EXTERNAL_TOOLS_PROMPT_DISMISSED: &str = "external_tools_prompt_dismissed";
}

/// Default values for settings
pub mod defaults {
    pub const THEME: &str = "dark";
    pub const MAX_TOOL_ROUNDS: u32 = 500;
    pub const YOLO_MODE: bool = true;
    pub const EXPERT_MODE: bool = true;
    pub const ACCOUNT_ROTATION_POLICY: super::AccountRotationPolicy =
        super::AccountRotationPolicy::PriorityThenLeastRecentlyUsed;
    pub const EXTERNAL_TOOLS_PROMPT_DISMISSED: bool = false;
}
