//! Typed settings accessors

use crate::config::{ConfigRepo, defaults, setting_keys};

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
    /// Simple truncation - removes oldest messages first.
    Truncation,
    /// Sliding window - preserves first N and last M messages.
    #[default]
    SlidingWindow,
    /// LLM summarization - generates a continuity briefing from removed messages.
    Summarizing,
}

impl CompressionStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            CompressionStrategy::Truncation => "truncation",
            CompressionStrategy::SlidingWindow => "sliding_window",
            CompressionStrategy::Summarizing => "summarizing",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "truncation" => CompressionStrategy::Truncation,
            "summarizing" => CompressionStrategy::Summarizing,
            _ => CompressionStrategy::SlidingWindow,
        }
    }
}

impl std::fmt::Display for CompressionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompressionStrategy::Truncation => write!(f, "Truncation"),
            CompressionStrategy::SlidingWindow => write!(f, "Sliding Window"),
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
    /// Number of recent messages to always keep.
    pub preserve_recent: u32,
}

impl Default for CompressionSettings {
    fn default() -> Self {
        Self {
            enabled: defaults::COMPRESSION_ENABLED,
            threshold_percent: defaults::COMPRESSION_THRESHOLD_PERCENT,
            strategy: CompressionStrategy::SlidingWindow,
            summarizer_model: None,
            preserve_first: defaults::COMPRESSION_PRESERVE_FIRST,
            preserve_recent: defaults::COMPRESSION_PRESERVE_RECENT,
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
        let preserve_recent = get_u32(repo, setting_keys::COMPRESSION_PRESERVE_RECENT)
            .unwrap_or(defaults::COMPRESSION_PRESERVE_RECENT);

        Self {
            enabled,
            threshold_percent,
            strategy,
            summarizer_model,
            preserve_first,
            preserve_recent,
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
    pub expert_mode_enabled: bool,
    pub account_rotation_policy: AccountRotationPolicy,
    pub external_tools_prompt_dismissed: bool,
    pub update_check_skip_remaining: u32,
    pub update_check_dismissed_version: Option<String>,
    pub compression: CompressionSettings,
}

impl TypedSettings {
    pub fn load(repo: &impl ConfigRepo) -> Self {
        let theme =
            get_string(repo, setting_keys::THEME).unwrap_or_else(|| defaults::THEME.to_string());
        let default_model = get_string(repo, setting_keys::DEFAULT_MODEL).filter(|s| !s.is_empty());
        let max_tool_rounds =
            get_u32(repo, setting_keys::MAX_TOOL_ROUNDS).unwrap_or(defaults::MAX_TOOL_ROUNDS);
        let yolo_mode_enabled =
            get_bool(repo, setting_keys::YOLO_MODE).unwrap_or(defaults::YOLO_MODE);
        let expert_mode_enabled =
            get_bool(repo, setting_keys::EXPERT_MODE).unwrap_or(defaults::EXPERT_MODE);
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
            expert_mode_enabled,
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
