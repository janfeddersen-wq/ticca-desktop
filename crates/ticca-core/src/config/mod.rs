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
#[allow(deprecated)]
pub use models::ApiKeyProvider;
pub use models::{
    ApiKeyAccount, DiscoveredModel, McpServer, McpTransport, ModelConfig, OAuthAccount, OAuthToken,
    Setting,
};
pub use paths::{
    ensure_dirs_exist, get_bin_dir, get_data_dir, get_skills_dir, get_tools_dir,
    get_uv_binary_path, get_venvs_dir,
};
pub use repo::ConfigRepo;
pub use service::{ConfigService, SettingsSnapshot};
pub use settings::{AccountRotationPolicy, CompressionSettings, CompressionStrategy, TypedSettings};

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
    /// Number of startups remaining before checking for updates again.
    pub const UPDATE_CHECK_SKIP_REMAINING: &str = "update_check_skip_remaining";
    /// Version that was dismissed via "Skip This Version" button.
    pub const UPDATE_CHECK_DISMISSED_VERSION: &str = "update_check_dismissed_version";

    // Compression settings
    /// Whether context compression is enabled.
    pub const COMPRESSION_ENABLED: &str = "compression_enabled";
    /// Percentage of context window that triggers compression (0-100).
    pub const COMPRESSION_THRESHOLD_PERCENT: &str = "compression_threshold_percent";
    /// Compression strategy: "truncation" or "summarizing".
    pub const COMPRESSION_STRATEGY: &str = "compression_strategy";
    /// Model to use for summarization (if strategy is "summarizing"). Empty = use current model.
    pub const COMPRESSION_SUMMARIZER_MODEL: &str = "compression_summarizer_model";
    /// Number of initial messages to preserve (e.g., system prompt).
    pub const COMPRESSION_PRESERVE_FIRST: &str = "compression_preserve_first";
    /// Number of tokens to protect for recent messages (like code_puppy's protected_token_count).
    pub const COMPRESSION_PROTECTED_TOKENS: &str = "compression_protected_tokens";
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
    /// Default skip count: check on first run
    pub const UPDATE_CHECK_SKIP_REMAINING: u32 = 0;

    // Compression defaults
    /// Compression disabled by default until user enables it.
    pub const COMPRESSION_ENABLED: bool = false;
    /// Trigger compression at 80% of context window.
    pub const COMPRESSION_THRESHOLD_PERCENT: u32 = 80;
    /// Default to truncation (token-based, like code_puppy).
    pub const COMPRESSION_STRATEGY: &str = "truncation";
    /// Number of initial messages to preserve (system prompt).
    pub const COMPRESSION_PRESERVE_FIRST: u32 = 1;
    /// Number of tokens to protect for recent messages (default 50k like code_puppy).
    pub const COMPRESSION_PROTECTED_TOKENS: u32 = 50_000;
}
