//! Application configuration loading and storage

use std::collections::HashMap;

use ticca_core::agents::AgentType;
use ticca_core::config::{ConfigService, UiMode};

use crate::theme::AppTheme;

/// Application configuration loaded from database
pub struct AppConfig {
    pub theme: AppTheme,
    pub default_model: Option<String>,
    pub agent_pinned_models: HashMap<AgentType, String>,
    pub max_tool_rounds: u32,
    pub yolo_mode_enabled: bool,
    pub ui_mode: UiMode,
    pub external_tools_prompt_dismissed: bool,
    pub update_check_skip_remaining: u32,
    pub update_check_dismissed_version: Option<String>,
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
                ui_mode: UiMode::default(),
                external_tools_prompt_dismissed:
                    ticca_core::config::defaults::EXTERNAL_TOOLS_PROMPT_DISMISSED,
                update_check_skip_remaining:
                    ticca_core::config::defaults::UPDATE_CHECK_SKIP_REMAINING,
                update_check_dismissed_version: None,
            };
        }
    };

    let theme = AppTheme::parse(&snapshot.settings.theme);
    let default_model = snapshot.settings.default_model;
    let max_tool_rounds = snapshot.settings.max_tool_rounds;
    let yolo_mode_enabled = snapshot.settings.yolo_mode_enabled;
    let ui_mode = snapshot.settings.ui_mode;
    let external_tools_prompt_dismissed = snapshot.settings.external_tools_prompt_dismissed;
    let update_check_skip_remaining = snapshot.settings.update_check_skip_remaining;
    let update_check_dismissed_version = snapshot.settings.update_check_dismissed_version;

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
        ui_mode,
        external_tools_prompt_dismissed,
        update_check_skip_remaining,
        update_check_dismissed_version,
    }
}
