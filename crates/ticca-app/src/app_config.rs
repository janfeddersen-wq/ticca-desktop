//! Application configuration loading and storage

use std::collections::HashMap;

use ticca_core::agents::AgentType;
use ticca_core::config::{ConfigDatabase, setting_keys, defaults};

use crate::theme::AppTheme;

/// Application configuration loaded from database
pub struct AppConfig {
    pub theme: AppTheme,
    pub default_model: Option<String>,
    pub agent_pinned_models: HashMap<AgentType, String>,
    pub max_tool_rounds: u32,
}

/// Load configuration from database
pub fn load_config() -> AppConfig {
    let db = match ConfigDatabase::open() {
        Ok(db) => db,
        Err(_) => return AppConfig {
            theme: AppTheme::Dark,
            default_model: None,
            agent_pinned_models: HashMap::new(),
            max_tool_rounds: defaults::MAX_TOOL_ROUNDS,
        },
    };

    let theme = db.get_setting(setting_keys::THEME)
        .ok()
        .flatten()
        .map(|s| AppTheme::from_str(&s.value))
        .unwrap_or(AppTheme::Dark);

    let default_model = db.get_setting(setting_keys::DEFAULT_MODEL)
        .ok()
        .flatten()
        .map(|s| s.value)
        .filter(|s| !s.is_empty());

    let max_tool_rounds = db.get_setting(setting_keys::MAX_TOOL_ROUNDS)
        .ok()
        .flatten()
        .and_then(|s| s.value.parse().ok())
        .unwrap_or(defaults::MAX_TOOL_ROUNDS);

    // Load agent pinned models
    let pinned_map = db.get_all_agent_pinned_models()
        .ok()
        .unwrap_or_default();

    let mut agent_pinned_models = HashMap::new();
    for (agent_str, model) in pinned_map {
        if let Some(agent_type) = AgentType::from_str(&agent_str) {
            agent_pinned_models.insert(agent_type, model);
        }
    }

    AppConfig {
        theme,
        default_model,
        agent_pinned_models,
        max_tool_rounds,
    }
}
