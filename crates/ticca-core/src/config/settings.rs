//! Typed settings accessors

use crate::config::{ConfigRepo, defaults, setting_keys};

/// UI complexity mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UiMode {
    /// Simplified UI - hides advanced options and internal agents
    #[default]
    Easy,
    /// Shows advanced options (models, agents tabs) but not internal agents
    Expert,
    /// Full access - shows everything including internal/subagents
    Debug,
}

impl UiMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            UiMode::Easy => "easy",
            UiMode::Expert => "expert",
            UiMode::Debug => "debug",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "expert" => UiMode::Expert,
            "debug" => UiMode::Debug,
            _ => UiMode::Easy,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            UiMode::Easy => "Easy",
            UiMode::Expert => "Expert",
            UiMode::Debug => "Debug",
        }
    }

    /// Returns all available UI modes
    pub fn all() -> &'static [UiMode] {
        &[UiMode::Easy, UiMode::Expert, UiMode::Debug]
    }

    /// Whether this mode shows expert UI features (models/agents tabs)
    pub fn shows_expert_ui(&self) -> bool {
        matches!(self, UiMode::Expert | UiMode::Debug)
    }

    /// Whether this mode shows internal/debug agents
    pub fn shows_internal_agents(&self) -> bool {
        matches!(self, UiMode::Debug)
    }
}

impl std::fmt::Display for UiMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountRotationPolicy {
    PriorityThenLeastRecentlyUsed,
    PriorityOnly,
    LeastRecentlyUsed,
}

impl AccountRotationPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccountRotationPolicy::PriorityThenLeastRecentlyUsed => "priority_then_lru",
            AccountRotationPolicy::PriorityOnly => "priority_only",
            AccountRotationPolicy::LeastRecentlyUsed => "least_recently_used",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "priority_only" => AccountRotationPolicy::PriorityOnly,
            "least_recently_used" => AccountRotationPolicy::LeastRecentlyUsed,
            _ => AccountRotationPolicy::PriorityThenLeastRecentlyUsed,
        }
    }
}

/// Compression strategy for managing context window limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompressionStrategy {
    /// Token-based truncation - keeps system prompt and recent messages up to protected token limit.
    /// Similar to code_puppy's LIFO approach: always preserves first message (system prompt),
    /// then keeps as many recent messages as fit within the protected token budget.
    #[default]
    Truncation,
    /// LLM summarization - generates a continuity briefing from removed messages.
    /// Uses an LLM to create a semantic summary of the compressed context.
    Summarizing,
}

impl CompressionStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            CompressionStrategy::Truncation => "truncation",
            CompressionStrategy::Summarizing => "summarizing",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "summarizing" => CompressionStrategy::Summarizing,
            // Default to truncation for any other value (including legacy "sliding_window")
            _ => CompressionStrategy::Truncation,
        }
    }
}

impl std::fmt::Display for CompressionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompressionStrategy::Truncation => write!(f, "Truncation"),
            CompressionStrategy::Summarizing => write!(f, "Summarizing (LLM)"),
        }
    }
}

/// Compression settings for context window management.
#[derive(Debug, Clone)]
pub struct CompressionSettings {
    /// Whether compression is enabled.
    pub enabled: bool,
    /// Percentage of context window that triggers compression (0-100).
    pub threshold_percent: u32,
    /// Which compression strategy to use.
    pub strategy: CompressionStrategy,
    /// Model to use for summarization (None = use current model).
    pub summarizer_model: Option<String>,
    /// Number of initial messages to preserve (e.g., system prompt).
    pub preserve_first: u32,
    /// Number of tokens to protect for recent messages.
    /// Like code_puppy's protected_token_count - recent messages totaling up to this
    /// many tokens will be preserved during compression.
    pub protected_tokens: u32,
}

impl Default for CompressionSettings {
    fn default() -> Self {
        Self {
            enabled: defaults::COMPRESSION_ENABLED,
            threshold_percent: defaults::COMPRESSION_THRESHOLD_PERCENT,
            strategy: CompressionStrategy::Truncation,
            summarizer_model: None,
            preserve_first: defaults::COMPRESSION_PRESERVE_FIRST,
            protected_tokens: defaults::COMPRESSION_PROTECTED_TOKENS,
        }
    }
}

impl CompressionSettings {
    /// Load compression settings from the database.
    pub fn load() -> Self {
        use crate::config::ConfigDatabase;
        match ConfigDatabase::open() {
            Ok(db) => Self::load_from(&db),
            Err(_) => Self::default(),
        }
    }

    /// Load compression settings from a config repo.
    pub fn load_from(repo: &impl ConfigRepo) -> Self {
        let enabled = get_bool(repo, setting_keys::COMPRESSION_ENABLED)
            .unwrap_or(defaults::COMPRESSION_ENABLED);
        let threshold_percent = get_u32(repo, setting_keys::COMPRESSION_THRESHOLD_PERCENT)
            .unwrap_or(defaults::COMPRESSION_THRESHOLD_PERCENT)
            .clamp(10, 100);
        let strategy = get_string(repo, setting_keys::COMPRESSION_STRATEGY)
            .map(|s| CompressionStrategy::parse(&s))
            .unwrap_or_else(|| CompressionStrategy::parse(defaults::COMPRESSION_STRATEGY));
        let summarizer_model =
            get_string(repo, setting_keys::COMPRESSION_SUMMARIZER_MODEL).filter(|s| !s.is_empty());
        let preserve_first = get_u32(repo, setting_keys::COMPRESSION_PRESERVE_FIRST)
            .unwrap_or(defaults::COMPRESSION_PRESERVE_FIRST);
        let protected_tokens = get_u32(repo, setting_keys::COMPRESSION_PROTECTED_TOKENS)
            .unwrap_or(defaults::COMPRESSION_PROTECTED_TOKENS);

        Self {
            enabled,
            threshold_percent,
            strategy,
            summarizer_model,
            preserve_first,
            protected_tokens,
        }
    }

    /// Calculate the token threshold for a given context window size.
    pub fn token_threshold(&self, context_window: u64) -> u64 {
        (context_window * self.threshold_percent as u64) / 100
    }
}

#[derive(Debug, Clone)]
pub struct TypedSettings {
    pub theme: String,
    pub default_model: Option<String>,
    pub max_tool_rounds: u32,
    pub yolo_mode_enabled: bool,
    pub ui_mode: UiMode,
    pub account_rotation_policy: AccountRotationPolicy,
    pub external_tools_prompt_dismissed: bool,
    pub update_check_skip_remaining: u32,
    pub update_check_dismissed_version: Option<String>,
    pub compression: CompressionSettings,
}

impl TypedSettings {
    #[allow(deprecated)]
    pub fn load(repo: &impl ConfigRepo) -> Self {
        let theme =
            get_string(repo, setting_keys::THEME).unwrap_or_else(|| defaults::THEME.to_string());
        let default_model = get_string(repo, setting_keys::DEFAULT_MODEL).filter(|s| !s.is_empty());
        let max_tool_rounds =
            get_u32(repo, setting_keys::MAX_TOOL_ROUNDS).unwrap_or(defaults::MAX_TOOL_ROUNDS);
        let yolo_mode_enabled =
            get_bool(repo, setting_keys::YOLO_MODE).unwrap_or(defaults::YOLO_MODE);

        // Load ui_mode with fallback to legacy expert_mode for migration
        let ui_mode = get_string(repo, setting_keys::UI_MODE)
            .map(|s| UiMode::parse(&s))
            .unwrap_or_else(|| {
                // Migrate from legacy expert_mode if present
                let legacy_expert =
                    get_bool(repo, setting_keys::EXPERT_MODE).unwrap_or(defaults::EXPERT_MODE);
                if legacy_expert {
                    UiMode::Expert
                } else {
                    UiMode::Easy
                }
            });

        let account_rotation_policy = get_string(repo, setting_keys::ACCOUNT_ROTATION_POLICY)
            .map(|value| AccountRotationPolicy::parse(&value))
            .unwrap_or(defaults::ACCOUNT_ROTATION_POLICY);
        let external_tools_prompt_dismissed =
            get_bool(repo, setting_keys::EXTERNAL_TOOLS_PROMPT_DISMISSED)
                .unwrap_or(defaults::EXTERNAL_TOOLS_PROMPT_DISMISSED);
        let update_check_skip_remaining = get_u32(repo, setting_keys::UPDATE_CHECK_SKIP_REMAINING)
            .unwrap_or(defaults::UPDATE_CHECK_SKIP_REMAINING);
        let update_check_dismissed_version =
            get_string(repo, setting_keys::UPDATE_CHECK_DISMISSED_VERSION);
        let compression = CompressionSettings::load_from(repo);

        Self {
            theme,
            default_model,
            max_tool_rounds,
            yolo_mode_enabled,
            ui_mode,
            account_rotation_policy,
            external_tools_prompt_dismissed,
            update_check_skip_remaining,
            update_check_dismissed_version,
            compression,
        }
    }
}

fn get_string(repo: &impl ConfigRepo, key: &str) -> Option<String> {
    repo.get_setting(key).ok().flatten().map(|s| s.value)
}

fn get_u32(repo: &impl ConfigRepo, key: &str) -> Option<u32> {
    get_string(repo, key).and_then(|value| value.parse::<u32>().ok())
}

fn get_bool(repo: &impl ConfigRepo, key: &str) -> Option<bool> {
    get_string(repo, key).map(|value| value == "true")
}
