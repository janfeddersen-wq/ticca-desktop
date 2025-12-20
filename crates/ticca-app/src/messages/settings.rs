use ticca_core::agents::AgentType;

use crate::theme::AppTheme;

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
}

