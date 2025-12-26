//! Config view types and helpers.

use std::collections::HashMap;

use iced::widget::Space;
use iced::Length;

use ticca_core::config::{ApiKeyAccount, McpTransport, OAuthAccount, UiMode};

/// Create horizontal space that fills available width (iced 0.14 helper)
pub(super) fn horizontal_space() -> Space {
    Space::new().width(Length::Fill)
}

/// Provider authentication status
#[derive(Debug, Clone, Default)]
pub struct ProviderAuthStatus {
    pub claude: bool,
    pub gemini: bool,
    pub chatgpt: bool,
}

/// MCP server form state for editing/creating servers
#[derive(Debug, Clone)]
pub struct McpServerFormState {
    pub editing_id: Option<String>,
    pub name: String,
    pub transport: McpTransport,
    pub command: String,
    pub args_json: String,
    pub env_json: String,
    pub endpoint_url: String,
    pub is_enabled: bool,
}

impl Default for McpServerFormState {
    fn default() -> Self {
        Self {
            editing_id: None,
            name: String::new(),
            transport: McpTransport::Stdio,
            command: String::new(),
            args_json: "[]".to_string(),
            env_json: "{}".to_string(),
            endpoint_url: String::new(),
            is_enabled: true,
        }
    }
}

/// Parameters for building the accounts section
pub struct AccountsSectionParams<'a> {
    pub auth_status: &'a ProviderAuthStatus,
    pub claude_accounts: &'a [OAuthAccount],
    pub gemini_accounts: &'a [OAuthAccount],
    pub chatgpt_accounts: &'a [OAuthAccount],
    pub api_key_accounts: &'a HashMap<String, Vec<ApiKeyAccount>>,
    pub api_key_form_provider: Option<&'a str>,
    pub api_key_form_value: &'a str,
    pub api_key_form_label: &'a str,
    pub add_provider_search: &'a str,
    pub add_provider_expanded: bool,
    pub ui_mode: UiMode,
}

/// Wrapper for model ID with nice display formatting
/// Stores the canonical ID internally but displays as "model_name (Provider)"
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DisplayModel(pub String);

impl DisplayModel {
    /// Get the canonical ID (for sending in messages)
    pub fn canonical_id(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DisplayModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Parse canonical format and display nicely
        if let Some(model_id) = ticca_core::llm::ModelId::parse(&self.0) {
            write!(f, "{}", model_id.display_name())
        } else {
            write!(f, "{}", self.0)
        }
    }
}

/// Option type for agent model picker
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ModelOption {
    UseDefault,
    Model(DisplayModel),
}

impl std::fmt::Display for ModelOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelOption::UseDefault => write!(f, "Use Default"),
            ModelOption::Model(m) => write!(f, "{}", m),
        }
    }
}
