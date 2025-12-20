//! Application configuration loading and storage

use std::collections::HashMap;

use ticca_core::agents::AgentType;
use ticca_core::config::{ConfigDatabase, TypedSettings};

use crate::theme::AppTheme;

/// Application configuration loaded from database
pub struct AppConfig {
    pub theme: AppTheme,
    pub default_model: Option<String>,
    pub agent_pinned_models: HashMap<AgentType, String>,
    pub max_tool_rounds: u32,
    pub yolo_mode_enabled: bool,
}

/// Load configuration from database
pub fn load_config() -> AppConfig {
    let db = match ConfigDatabase::open() {
        Ok(db) => db,
        Err(_) => {
            return AppConfig {
                theme: AppTheme::Dark,
                default_model: None,
                agent_pinned_models: HashMap::new(),
                max_tool_rounds: ticca_core::config::defaults::MAX_TOOL_ROUNDS,
                yolo_mode_enabled: ticca_core::config::defaults::YOLO_MODE,
            };
        }
    };

    let settings = TypedSettings::load(&db);
    let theme = AppTheme::parse(&settings.theme);
    let default_model = settings.default_model;
    let max_tool_rounds = settings.max_tool_rounds;
    let yolo_mode_enabled = settings.yolo_mode_enabled;

    // Load agent pinned models
    let pinned_map = db.get_all_agent_pinned_models().ok().unwrap_or_default();

    let mut agent_pinned_models = HashMap::new();
    for (agent_str, model) in pinned_map {
        if let Some(agent_type) = AgentType::parse(&agent_str) {
            agent_pinned_models.insert(agent_type, model);
        }
    }

    AppConfig {
        theme,
        default_model,
        agent_pinned_models,
        max_tool_rounds,
        yolo_mode_enabled,
    }
}
