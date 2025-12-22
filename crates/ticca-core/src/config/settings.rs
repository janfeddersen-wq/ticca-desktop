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

#[derive(Debug, Clone)]
pub struct TypedSettings {
    pub theme: String,
    pub default_model: Option<String>,
    pub max_tool_rounds: u32,
    pub yolo_mode_enabled: bool,
    pub expert_mode_enabled: bool,
    pub account_rotation_policy: AccountRotationPolicy,
    pub external_tools_prompt_dismissed: bool,
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

        Self {
            theme,
            default_model,
            max_tool_rounds,
            yolo_mode_enabled,
            expert_mode_enabled,
            account_rotation_policy,
            external_tools_prompt_dismissed,
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
