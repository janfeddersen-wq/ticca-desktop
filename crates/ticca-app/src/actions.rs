//! GPUI Actions for Ticca Desktop
//!
//! Actions are the GPUI equivalent of ICED messages. They define all the
//! user interactions and events that can occur in the application.

use gpui::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

// Note: We use String IDs instead of ticca_core types because Action requires Serialize/Deserialize/JsonSchema

// =============================================================================
// View Navigation
// =============================================================================

/// The main views in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Chat,
    Settings,
}

// =============================================================================
// Sidebar & Panel Types
// =============================================================================

/// Right sidebar tab selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub enum RightSidebarTab {
    #[default]
    AgentsFlow,
    TodoList,
    SystemExecutions,
}

impl std::fmt::Display for RightSidebarTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RightSidebarTab::AgentsFlow => write!(f, "Agents Flow"),
            RightSidebarTab::TodoList => write!(f, "To Do List"),
            RightSidebarTab::SystemExecutions => write!(f, "System Executions"),
        }
    }
}

/// Settings tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub enum SettingsTab {
    #[default]
    Accounts,
    Models,
    Agents,
    McpServers,
    Tools,
    Appearance,
    Sessions,
}

impl std::fmt::Display for SettingsTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingsTab::Accounts => write!(f, "Accounts"),
            SettingsTab::Models => write!(f, "Models"),
            SettingsTab::Agents => write!(f, "Agents"),
            SettingsTab::McpServers => write!(f, "MCP Servers"),
            SettingsTab::Tools => write!(f, "Tools"),
            SettingsTab::Appearance => write!(f, "Appearance"),
            SettingsTab::Sessions => write!(f, "Sessions"),
        }
    }
}

/// OAuth provider selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum OAuthProvider {
    Claude,
    Gemini,
    ChatGpt,
}

impl std::fmt::Display for OAuthProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuthProvider::Claude => write!(f, "Claude"),
            OAuthProvider::Gemini => write!(f, "Gemini"),
            OAuthProvider::ChatGpt => write!(f, "ChatGPT"),
        }
    }
}

// =============================================================================
// Image Attachment
// =============================================================================

/// Image attachment data for sending to the LLM
#[derive(Debug, Clone)]
pub struct ImageAttachment {
    /// Raw image bytes (PNG format)
    pub data: Arc<Vec<u8>>,
    /// Original width
    pub width: u32,
    /// Original height
    pub height: u32,
    /// Optional filename (for dropped files)
    pub filename: Option<String>,
}

// =============================================================================
// Chat Actions (Unit Actions)
// =============================================================================

actions!(
    chat,
    [
        SendMessage,
        StopStreaming,
        NewSession,
        ToggleFlowPanel,
        SelectWorkingDirectory,
        SelectImageFile,
        PasteImage,
        AttachImage,
        AnimationTick,
        FlowAnimationTick,
    ]
);

// =============================================================================
// Chat Actions (Data Actions)
// =============================================================================

/// Chat input text changed
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action)]
pub struct InputChanged(pub String);

impl std::fmt::Debug for InputChanged {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("InputChanged")
            .field(&format!("[{} chars]", self.0.len()))
            .finish()
    }
}

/// Switch to a different agent (by string ID: "coding", "planning", "skills", "explore")
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct SwitchAgent(pub String);

/// Select a sidebar tab
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct SelectSidebarTab(pub RightSidebarTab);

/// Load a session by ID
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct LoadSession(pub String);

/// Working directory changed
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct WorkingDirectoryChanged(pub PathBuf);

/// File was dropped on the window
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct FileDropped(pub PathBuf);

/// Remove an attachment by index
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct RemoveAttachment(pub usize);

// =============================================================================
// Streaming Actions
// =============================================================================

/// A chunk of streamed content arrived
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action)]
pub struct StreamChunk(pub String);

impl std::fmt::Debug for StreamChunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("StreamChunk")
            .field(&format!("[{} chars]", self.0.len()))
            .finish()
    }
}

/// Stream completed successfully
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct StreamComplete;

/// Stream was stopped by user
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct StreamStopped;

/// Stream encountered an error
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct StreamError(pub String);

/// Tool call event
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct ToolCall {
    pub name: String,
    pub args: String,
}

/// Tool approval requested
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct ToolApprovalRequested {
    pub id: u64,
    pub name: String,
    pub args: String,
}

/// Tool approval decision made
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct ToolApprovalDecision {
    pub id: u64,
    pub approved: bool,
}

/// Reasoning/thinking content from the model
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct Reasoning {
    pub text: String,
    pub signature: Option<String>,
}

/// Token usage from the API response
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

// =============================================================================
// Settings Actions (Unit Actions)
// =============================================================================

actions!(
    settings,
    [
        OpenSettings,
        CloseSettings,
        ThemeToggle,
        RefreshModels,
        RefreshMcp,
        RefreshExternalTools,
    ]
);

// =============================================================================
// Settings Actions (Data Actions)
// =============================================================================

/// Switch settings tab
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct SwitchSettingsTab(pub SettingsTab);

/// Set YOLO mode (auto-approve tools)
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct SetYoloMode(pub bool);

/// Set UI mode ("easy", "expert", "debug")
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct SetUiMode(pub String);

/// Start OAuth flow for a provider
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct StartOAuth(pub OAuthProvider);

/// Remove an OAuth account
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct RemoveOAuthAccount(pub String);

/// Toggle OAuth account active state
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct ToggleOAuthAccountActive {
    pub account_id: String,
    pub is_active: bool,
}

/// Start adding an API key for a provider
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct StartAddApiKey(pub String);

/// Cancel adding API key
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct CancelAddApiKey;

/// API key form field changed
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct ApiKeyFormChanged(pub String);

/// API key label form field changed
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct ApiKeyLabelFormChanged(pub String);

/// Save API key
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct SaveApiKey;

/// Remove an API key account
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct RemoveApiKeyAccount(pub String);

/// Set the default model
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct SetDefaultModel(pub String);

/// Set model for a specific agent (agent_id: "coding", "planning", etc.)
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct SetAgentModel {
    pub agent_id: String,
    pub model: Option<String>,
}

/// Install an external tool (by string ID)
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct InstallExternalTool(pub String);

/// Uninstall an external tool (by string ID)
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct UninstallExternalTool(pub String);

// =============================================================================
// MCP Server Actions
// =============================================================================

/// Create new MCP server form
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct McpFormNew;

/// Edit existing MCP server
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct McpFormEdit(pub String);

/// Cancel MCP form
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct McpFormCancel;

/// MCP form name changed
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct McpFormNameChanged(pub String);

/// MCP form command changed
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct McpFormCommandChanged(pub String);

/// Save MCP server
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct McpFormSave;

/// Delete MCP server
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct McpDeleteServer(pub String);

/// Toggle MCP server enabled
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct McpSetServerEnabled {
    pub server_id: String,
    pub enabled: bool,
}

// =============================================================================
// Toast & Notification Actions
// =============================================================================

actions!(app, [DismissToast, ToastTick, DismissUpdate,]);

/// Open a URL in the browser
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct OpenUrl(pub String);

/// Skip a specific version update
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct SkipThisVersion(pub String);

/// Switch to a specific theme by name
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action, Debug)]
pub struct SwitchTheme(pub String);
