use ticca_core::agents::AgentType;
use ticca_core::config::McpTransport;

use crate::theme::AppTheme;
use iced::widget::text_editor;

use super::{OAuthProvider, SettingsTab};

#[derive(Debug, Clone)]
pub enum Msg {
    // Navigation
    OpenSettings,
    CloseSettings,
    SwitchSettingsTab(SettingsTab),

    // Appearance
    ThemeToggle,
    SetTheme(AppTheme),
    SetYoloMode(bool),
    SetExpertMode(bool),

    // OAuth accounts
    StartOAuth(OAuthProvider),
    OAuthComplete(OAuthProvider, Result<(), String>),
    RemoveOAuthAccount(String),
    ToggleOAuthAccountActive {
        account_id: String,
        is_active: bool,
    },
    ResetOAuthCooldown(String),
    AdjustOAuthAccountPriority {
        account_id: String,
        delta: i64,
    },

    // Model selection
    RefreshModels,
    RefreshModelsForProvider(OAuthProvider),
    ModelsLoaded(Result<Vec<String>, String>),
    SetDefaultModel(String),
    SetAgentModel(AgentType, Option<String>),

    // MCP servers
    RefreshMcp,
    McpFormNew,
    McpFormEdit(String),
    McpFormCancel,
    McpFormNameChanged(String),
    McpFormTransportChanged(McpTransport),
    McpFormCommandChanged(String),
    McpFormArgsJsonChanged(String),
    McpFormEnvJsonChanged(String),
    McpFormEndpointUrlChanged(String),
    McpFormEnabledChanged(bool),
    McpFormSave,
    McpDeleteServer(String),
    McpSetServerEnabled {
        server_id: String,
        enabled: bool,
    },
    McpImportEditorAction(text_editor::Action),
    McpImportApply,
    McpImportClear,

    // Agent MCP mapping
    AgentMcpToggled {
        agent_type: AgentType,
        server_id: String,
        enabled: bool,
    },
}
