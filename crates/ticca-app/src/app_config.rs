//! Application configuration loading and storage

use std::collections::HashMap;

use ticca_core::agents::AgentType;
use ticca_core::config::ConfigService;

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
    let snapshot = match ConfigService::load_settings_snapshot() {
        Ok(snapshot) => snapshot,
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

    let theme = AppTheme::parse(&snapshot.settings.theme);
    let default_model = snapshot.settings.default_model;
    let max_tool_rounds = snapshot.settings.max_tool_rounds;
    let yolo_mode_enabled = snapshot.settings.yolo_mode_enabled;

    let mut agent_pinned_models = HashMap::new();
    for (agent_str, model) in snapshot.agent_pinned_models {
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
