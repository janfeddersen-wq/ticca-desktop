use ticca_core::agents::AgentType;
use ticca_core::config::{ApiKeyProvider, McpTransport};
use ticca_core::external_tools::ExternalToolId;

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

    // API key accounts
    StartAddApiKey(ApiKeyProvider),
    CancelAddApiKey,
    ApiKeyFormChanged(String),
    ApiKeyLabelFormChanged(String),
    SaveApiKey,
    RemoveApiKeyAccount(String),
    ToggleApiKeyAccountActive {
        account_id: String,
        is_active: bool,
    },
    ResetApiKeyCooldown(String),
    AdjustApiKeyAccountPriority {
        account_id: String,
        delta: i64,
    },

    // Model selection
    RefreshModels,
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

    // External Tools
    RefreshExternalTools,
    ExternalToolsLoaded(Vec<(ExternalToolId, ToolStatusInfo)>),
    InstallExternalTool(ExternalToolId),
    UninstallExternalTool(ExternalToolId),
    ExternalToolInstallProgress(ExternalToolId, u8),
    ExternalToolInstallComplete(ExternalToolId, Result<(), String>),
    ExternalToolUninstallComplete(ExternalToolId, Result<(), String>),

    // External Tools Startup Prompt
    ShowExternalToolsPrompt(Vec<ExternalToolId>),
    DismissExternalToolsPrompt,
    DismissExternalToolsPromptPermanently,
    InstallAllMissingTools,

    // Compression settings
    SetCompressionEnabled(bool),
    SetCompressionThreshold(u32),
    SetCompressionStrategy(ticca_core::config::CompressionStrategy),
    SetCompressionPreserveFirst(u32),
    SetCompressionProtectedTokens(u32),

    // Misc
    OpenUrl(String),
}

/// Simplified tool status for UI display
#[derive(Debug, Clone)]
pub struct ToolStatusInfo {
    pub is_installed: bool,
    pub version: Option<String>,
    pub is_installing: bool,
    pub install_progress: u8,
    pub is_supported: bool,
}
