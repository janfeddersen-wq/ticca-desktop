//! TUI application state (legacy, kept for backwards compatibility)
//!
//! This module is no longer actively used - the CLI mode uses a simpler
//! inline state management approach. Kept for API compatibility.

use std::collections::HashMap;
use std::path::PathBuf;

use ticca_core::agents::AgentType;

/// TUI application state (legacy)
#[allow(dead_code)]
pub struct TuiApp {
    pub input: String,
    pub working_directory: PathBuf,
    pub current_agent: AgentType,
    pub yolo_mode: bool,
    pub default_model: Option<String>,
    pub agent_pinned_models: HashMap<String, String>,
    pub current_model: Option<String>,
    pub show_help: bool,
}

impl TuiApp {
    #[allow(dead_code)]
    pub fn new(
        _working_directory: Option<PathBuf>,
        _model_override: Option<String>,
        _yolo_override: bool,
    ) -> Self {
        Self {
            input: String::new(),
            working_directory: PathBuf::from("."),
            current_agent: AgentType::Coding,
            yolo_mode: false,
            default_model: None,
            agent_pinned_models: HashMap::new(),
            current_model: None,
            show_help: false,
        }
    }

    #[allow(dead_code)]
    pub fn get_model_for_agent(&self) -> Option<String> {
        self.default_model.clone()
    }
}
