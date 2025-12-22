use iced::Element;
use iced::widget::text_editor;

use std::collections::HashMap;

use crate::messages::SettingsTab;
use crate::messages::{Message, settings};
use crate::views::config;
use crate::views::config::{McpServerFormState, ProviderAuthStatus};
use ticca_core::AgentType;
use ticca_core::config::OAuthAccount;
use ticca_core::config::models::providers;
use ticca_core::config::{ConfigService, McpServer, setting_keys};
use ticca_core::external_tools::{ExternalToolId, ExternalToolManager, ToolStatus};
use ticca_core::llm::ProviderId;
use ticca_core::llm::auth;
use ticca_core::session::{Session, SessionService};

use super::super::{TiccaApp, Toast, View, effects::Effect};

pub(in crate::app) struct SettingsState {
    pub(in crate::app) settings_tab: SettingsTab,
    pub(in crate::app) provider_auth_status: ProviderAuthStatus,
    pub(in crate::app) accounts_claude: Vec<OAuthAccount>,
    pub(in crate::app) accounts_gemini: Vec<OAuthAccount>,
    pub(in crate::app) accounts_chatgpt: Vec<OAuthAccount>,
    pub(in crate::app) recent_sessions: Vec<Session>,
    pub(in crate::app) mcp_servers: Vec<McpServer>,
    pub(in crate::app) agent_mcp_server_ids: HashMap<AgentType, Vec<String>>,
    pub(in crate::app) mcp_form: McpServerFormState,
    pub(in crate::app) mcp_import_json: text_editor::Content,
    pub(in crate::app) external_tools: HashMap<ExternalToolId, settings::ToolStatusInfo>,
}

impl SettingsState {
    pub(in crate::app) fn new(provider_auth_status: ProviderAuthStatus) -> Self {
        Self {
            settings_tab: SettingsTab::Accounts,
            provider_auth_status,
            accounts_claude: Vec::new(),
            accounts_gemini: Vec::new(),
            accounts_chatgpt: Vec::new(),
            recent_sessions: Vec::new(),
            mcp_servers: Vec::new(),
            agent_mcp_server_ids: HashMap::new(),
            mcp_form: McpServerFormState::default(),
            mcp_import_json: text_editor::Content::with_text(""),
            external_tools: HashMap::new(),
        }
    }

    fn refresh_accounts(&mut self) {
        self.accounts_claude =
            ConfigService::list_oauth_accounts_pruned(providers::CLAUDE).unwrap_or_default();
        self.accounts_gemini =
            ConfigService::list_oauth_accounts_pruned(providers::GEMINI).unwrap_or_default();
        self.accounts_chatgpt =
            ConfigService::list_oauth_accounts_pruned(providers::CHATGPT).unwrap_or_default();
    }

    fn refresh_sessions(&mut self) {
        self.recent_sessions = SessionService::list_recent(10).unwrap_or_default();
    }

    fn refresh_mcp(&mut self) {
        self.mcp_servers = ConfigService::list_mcp_servers().unwrap_or_default();

        let mut map = HashMap::new();
        for agent in [AgentType::Planning, AgentType::Coding] {
            let ids = ConfigService::get_agent_mcp_server_ids(agent.as_str()).unwrap_or_default();
            map.insert(agent, ids);
        }
        self.agent_mcp_server_ids = map;
    }
}

pub(in crate::app) fn check_provider_auth_status() -> ProviderAuthStatus {
    ProviderAuthStatus {
        claude: auth::has_valid_account(ticca_core::config::models::providers::CLAUDE),
        gemini: auth::has_valid_account(ticca_core::config::models::providers::GEMINI),
        chatgpt: auth::has_valid_account(ticca_core::config::models::providers::CHATGPT),
    }
}

pub(in crate::app) fn update(app: &mut TiccaApp, message: settings::Msg) -> Vec<Effect> {
    let mut effects = Vec::new();

    match message {
        settings::Msg::OpenSettings => {
            app.current_view = View::Settings;
            app.settings.refresh_accounts();
            app.settings.refresh_sessions();
            app.settings.refresh_mcp();
        }
        settings::Msg::CloseSettings => {
            app.current_view = View::Chat;
        }
        settings::Msg::SwitchSettingsTab(tab) => {
            if !app.expert_mode_enabled && matches!(tab, SettingsTab::Models | SettingsTab::Agents)
            {
                app.settings.settings_tab = SettingsTab::Accounts;
                return effects;
            }
            app.settings.settings_tab = tab;
            if tab == SettingsTab::Accounts {
                app.settings.refresh_accounts();
            }
            if tab == SettingsTab::Sessions {
                app.settings.refresh_sessions();
            }
            if tab == SettingsTab::Agents || tab == SettingsTab::McpServers {
                app.settings.refresh_mcp();
            }
            if tab == SettingsTab::Tools {
                effects.push(Effect::RefreshExternalTools);
            }
        }
        settings::Msg::ThemeToggle => {
            app.theme = app.theme.next();
            let _ = ConfigService::set_setting(setting_keys::THEME, app.theme.as_str());
        }
        settings::Msg::SetTheme(theme) => {
            app.theme = theme;
            let _ = ConfigService::set_setting(setting_keys::THEME, app.theme.as_str());
        }
        settings::Msg::SetYoloMode(enabled) => {
            app.chat.yolo_mode_enabled = enabled;
            let value = if enabled { "true" } else { "false" };
            let _ = ConfigService::set_setting(setting_keys::YOLO_MODE, value);
        }
        settings::Msg::SetExpertMode(enabled) => {
            app.expert_mode_enabled = enabled;
            let value = if enabled { "true" } else { "false" };
            let _ = ConfigService::set_setting(setting_keys::EXPERT_MODE, value);

            if !enabled
                && matches!(
                    app.settings.settings_tab,
                    SettingsTab::Models | SettingsTab::Agents
                )
            {
                app.settings.settings_tab = SettingsTab::Accounts;
            }

            if !enabled {
                app.chat.todo_selected_node = 0;
            }

            if !enabled
                && matches!(
                    app.chat.sidebar_tab,
                    crate::messages::RightSidebarTab::AgentsFlow
                        | crate::messages::RightSidebarTab::SystemExecutions
                )
            {
                app.chat.sidebar_tab = crate::messages::RightSidebarTab::TodoList;
            }
        }
        settings::Msg::StartOAuth(provider) => {
            effects.push(Effect::StartOAuth(provider));
        }
        settings::Msg::OAuthComplete(provider, result) => match result {
            Ok(()) => {
                app.toast = None;
                app.settings.provider_auth_status = check_provider_auth_status();
                app.settings.refresh_accounts();
                effects.push(Effect::RefreshModelsForProvider(match provider {
                    crate::messages::OAuthProvider::Claude => ProviderId::Claude,
                    crate::messages::OAuthProvider::Gemini => ProviderId::Gemini,
                    crate::messages::OAuthProvider::ChatGpt => ProviderId::ChatGpt,
                }));
            }
            Err(e) => {
                app.toast = Some(Toast::new(e));
            }
        },
        settings::Msg::RemoveOAuthAccount(account_id) => {
            let _ = ConfigService::delete_oauth_account(&account_id);
            app.settings.provider_auth_status = check_provider_auth_status();
            app.settings.refresh_accounts();
        }
        settings::Msg::ToggleOAuthAccountActive {
            account_id,
            is_active,
        } => {
            let _ = ConfigService::set_oauth_account_active(&account_id, is_active);
            app.settings.provider_auth_status = check_provider_auth_status();
            app.settings.refresh_accounts();
        }
        settings::Msg::ResetOAuthCooldown(account_id) => {
            let _ = ConfigService::clear_oauth_account_cooldown(&account_id);
            app.settings.refresh_accounts();
        }
        settings::Msg::AdjustOAuthAccountPriority { account_id, delta } => {
            let _ = ConfigService::adjust_oauth_account_priority(&account_id, delta);
            app.settings.refresh_accounts();
        }
        settings::Msg::RefreshModels => {
            if app.chat.is_loading_models {
                return effects;
            }
            app.chat.is_loading_models = true;

            if !auth::has_any_valid_account() {
                app.chat.is_loading_models = false;
                return effects;
            }

            effects.push(Effect::RefreshModels);
        }
        settings::Msg::RefreshModelsForProvider(provider) => {
            if app.chat.is_loading_models {
                return effects;
            }
            app.chat.is_loading_models = true;

            let mapped = match provider {
                crate::messages::OAuthProvider::Claude => ProviderId::Claude,
                crate::messages::OAuthProvider::Gemini => ProviderId::Gemini,
                crate::messages::OAuthProvider::ChatGpt => ProviderId::ChatGpt,
            };
            effects.push(Effect::RefreshModelsForProvider(mapped));
        }
        settings::Msg::ModelsLoaded(result) => {
            app.chat.is_loading_models = false;
            match result {
                Ok(models) => {
                    tracing::info!("Loaded {} models: {:?}", models.len(), models);
                    app.chat.available_models = models;
                }
                Err(e) => {
                    tracing::error!("Failed to load models: {}", e);
                    app.toast = Some(Toast::new(format!("Failed to load models: {}", e)));
                }
            }
        }
        settings::Msg::SetDefaultModel(model_name) => {
            app.chat.default_model = Some(model_name.clone());
            let _ = ConfigService::set_setting(setting_keys::DEFAULT_MODEL, &model_name);
            tracing::info!("Set default model: {}", model_name);
        }
        settings::Msg::SetAgentModel(agent_type, model) => match &model {
            Some(model_name) => {
                app.chat
                    .agent_pinned_models
                    .insert(agent_type, model_name.clone());
                let _ =
                    ConfigService::set_agent_pinned_model(agent_type.as_str(), Some(model_name));
                tracing::info!("Pinned {} to model: {}", agent_type.as_str(), model_name);
            }
            None => {
                app.chat.agent_pinned_models.remove(&agent_type);
                let _ = ConfigService::set_agent_pinned_model(agent_type.as_str(), None);
                tracing::info!("Cleared pinned model for {}", agent_type.as_str());
            }
        },

        // MCP servers
        settings::Msg::RefreshMcp => {
            app.settings.refresh_mcp();
        }
        settings::Msg::McpFormNew => {
            app.settings.mcp_form = McpServerFormState::default();
        }
        settings::Msg::McpFormEdit(server_id) => {
            if let Some(server) = app.settings.mcp_servers.iter().find(|s| s.id == server_id) {
                app.settings.mcp_form.editing_id = Some(server.id.clone());
                app.settings.mcp_form.name = server.name.clone();
                app.settings.mcp_form.transport = server.transport;
                app.settings.mcp_form.command = server.command.clone().unwrap_or_default();
                app.settings.mcp_form.args_json =
                    serde_json::to_string(&server.args).unwrap_or_else(|_| "[]".to_string());
                app.settings.mcp_form.env_json =
                    serde_json::to_string(&server.env).unwrap_or_else(|_| "{}".to_string());
                app.settings.mcp_form.endpoint_url =
                    server.endpoint_url.clone().unwrap_or_default();
                app.settings.mcp_form.is_enabled = server.is_enabled;
            }
        }
        settings::Msg::McpFormCancel => {
            app.settings.mcp_form = McpServerFormState::default();
        }
        settings::Msg::McpFormNameChanged(value) => {
            app.settings.mcp_form.name = value;
        }
        settings::Msg::McpFormTransportChanged(value) => {
            app.settings.mcp_form.transport = value;
        }
        settings::Msg::McpFormCommandChanged(value) => {
            app.settings.mcp_form.command = value;
        }
        settings::Msg::McpFormArgsJsonChanged(value) => {
            app.settings.mcp_form.args_json = value;
        }
        settings::Msg::McpFormEnvJsonChanged(value) => {
            app.settings.mcp_form.env_json = value;
        }
        settings::Msg::McpFormEndpointUrlChanged(value) => {
            app.settings.mcp_form.endpoint_url = value;
        }
        settings::Msg::McpFormEnabledChanged(value) => {
            app.settings.mcp_form.is_enabled = value;
        }
        settings::Msg::McpFormSave => {
            let name = app.settings.mcp_form.name.trim().to_string();
            if name.is_empty() {
                app.toast = Some(Toast::new("MCP server name is required"));
                return effects;
            }

            let args_json = app.settings.mcp_form.args_json.trim();
            let args: Vec<String> = if args_json.is_empty() {
                Vec::new()
            } else {
                match serde_json::from_str(args_json) {
                    Ok(v) => v,
                    Err(e) => {
                        app.toast = Some(Toast::new(format!("Invalid args JSON: {}", e)));
                        return effects;
                    }
                }
            };

            let env_json = app.settings.mcp_form.env_json.trim();
            let env: std::collections::BTreeMap<String, String> = if env_json.is_empty() {
                std::collections::BTreeMap::new()
            } else {
                match serde_json::from_str(env_json) {
                    Ok(v) => v,
                    Err(e) => {
                        app.toast = Some(Toast::new(format!("Invalid env JSON: {}", e)));
                        return effects;
                    }
                }
            };

            let id = app
                .settings
                .mcp_form
                .editing_id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            let server = match app.settings.mcp_form.transport {
                ticca_core::config::McpTransport::Stdio => {
                    let command = app.settings.mcp_form.command.trim().to_string();
                    if command.is_empty() {
                        app.toast = Some(Toast::new("Command is required for stdio MCP servers"));
                        return effects;
                    }
                    McpServer::new(id, name)
                        .with_stdio_command(command)
                        .with_args(args)
                        .with_env(env)
                        .set_enabled(app.settings.mcp_form.is_enabled)
                }
                ticca_core::config::McpTransport::StreamableHttp => {
                    let url = app.settings.mcp_form.endpoint_url.trim().to_string();
                    if url.is_empty() {
                        app.toast = Some(Toast::new("Endpoint URL is required for HTTP MCP servers"));
                        return effects;
                    }
                    McpServer::new(id, name)
                        .with_streamable_http(url)
                        .with_args(args)
                        .with_env(env)
                        .set_enabled(app.settings.mcp_form.is_enabled)
                }
            };

            match ConfigService::upsert_mcp_server(&server) {
                Ok(()) => {
                    app.toast = None;
                    app.settings.refresh_mcp();
                    app.settings.mcp_form = McpServerFormState::default();
                }
                Err(e) => {
                    app.toast = Some(Toast::new(format!("Failed to save MCP server: {}", e)));
                }
            }
        }
        settings::Msg::McpDeleteServer(server_id) => {
            let _ = ConfigService::delete_mcp_server(&server_id);
            app.settings.refresh_mcp();
            if app.settings.mcp_form.editing_id.as_deref() == Some(server_id.as_str()) {
                app.settings.mcp_form = McpServerFormState::default();
            }
        }
        settings::Msg::McpSetServerEnabled { server_id, enabled } => {
            let _ = ConfigService::set_mcp_server_enabled(&server_id, enabled);
            app.settings.refresh_mcp();
        }
        settings::Msg::McpImportEditorAction(action) => {
            app.settings.mcp_import_json.perform(action);
        }
        settings::Msg::McpImportClear => {
            app.settings.mcp_import_json = text_editor::Content::with_text("");
            app.toast = None;
        }
        settings::Msg::McpImportApply => {
            let input = app.settings.mcp_import_json.text();
            if input.trim().is_empty() {
                app.toast = Some(Toast::new("Paste MCP JSON first"));
                return effects;
            }

            match ConfigService::import_mcp_servers_json(&input) {
                Ok(_) => {
                    app.toast = None;
                    app.settings.refresh_mcp();
                    app.settings.mcp_import_json = text_editor::Content::with_text("");
                }
                Err(error) => {
                    app.toast = Some(Toast::new(format!("MCP import failed: {}", error)));
                }
            }
        }

        // Agent MCP mapping
        settings::Msg::AgentMcpToggled {
            agent_type,
            server_id,
            enabled,
        } => {
            let ids = app
                .settings
                .agent_mcp_server_ids
                .entry(agent_type)
                .or_default();

            if enabled {
                if !ids.contains(&server_id) {
                    ids.push(server_id.clone());
                }
            } else {
                ids.retain(|id| id != &server_id);
            }

            let _ = ConfigService::set_agent_mcp_server_ids(agent_type.as_str(), ids);
        }

        // External Tools
        settings::Msg::RefreshExternalTools => {
            effects.push(Effect::RefreshExternalTools);
        }
        settings::Msg::ExternalToolsLoaded(statuses) => {
            for (tool_id, status) in statuses {
                app.settings.external_tools.insert(tool_id, status);
            }
        }
        settings::Msg::InstallExternalTool(tool_id) => {
            // Mark as installing
            if let Some(status) = app.settings.external_tools.get_mut(&tool_id) {
                status.is_installing = true;
                status.install_progress = 0;
            }
            effects.push(Effect::InstallExternalTool(tool_id));
        }
        settings::Msg::UninstallExternalTool(tool_id) => {
            effects.push(Effect::UninstallExternalTool(tool_id));
        }
        settings::Msg::ExternalToolInstallProgress(tool_id, progress) => {
            if let Some(status) = app.settings.external_tools.get_mut(&tool_id) {
                status.install_progress = progress;
            }
            // Also update prompt state if active - always update current_tool
            // so modal shows correct tool name as install moves to next tool
            if let Some(ref mut prompt) = app.external_tools_prompt {
                prompt.current_tool = Some(tool_id);
                prompt.progress = progress;
            }
        }
        settings::Msg::ExternalToolInstallComplete(tool_id, result) => {
            if let Some(status) = app.settings.external_tools.get_mut(&tool_id) {
                status.is_installing = false;
                match &result {
                    Ok(()) => {
                        status.is_installed = true;
                        let def = ticca_core::external_tools::get_tool_definition(tool_id);
                        status.version = Some(def.version.to_string());
                    }
                    Err(e) => {
                        app.toast = Some(Toast::new(format!("Failed to install {}: {}", tool_id, e)));
                    }
                }
            }
        }
        settings::Msg::ExternalToolUninstallComplete(tool_id, result) => {
            match result {
                Ok(()) => {
                    if let Some(status) = app.settings.external_tools.get_mut(&tool_id) {
                        status.is_installed = false;
                        status.version = None;
                    }
                }
                Err(e) => {
                    app.toast = Some(Toast::new(format!("Failed to uninstall {}: {}", tool_id, e)));
                }
            }
        }

        // External Tools Startup Prompt
        settings::Msg::ShowExternalToolsPrompt(missing_tools) => {
            if !missing_tools.is_empty() && !app.external_tools_prompt_dismissed {
                app.external_tools_prompt = Some(crate::app::ExternalToolsPromptState {
                    missing_tools,
                    is_installing: false,
                    current_tool: None,
                    progress: 0,
                });
            }
        }
        settings::Msg::DismissExternalToolsPrompt => {
            app.external_tools_prompt = None;
        }
        settings::Msg::DismissExternalToolsPromptPermanently => {
            app.external_tools_prompt = None;
            app.external_tools_prompt_dismissed = true;
            let _ = ConfigService::set_setting(
                setting_keys::EXTERNAL_TOOLS_PROMPT_DISMISSED,
                "true",
            );
        }
        settings::Msg::InstallAllMissingTools => {
            if let Some(ref mut prompt) = app.external_tools_prompt {
                let tools = prompt.missing_tools.clone();
                prompt.is_installing = true;
                if let Some(first) = tools.first() {
                    prompt.current_tool = Some(*first);
                }
                effects.push(Effect::InstallAllMissingTools(tools));
            }
        }
        settings::Msg::OpenUrl(url) => {
            effects.push(Effect::OpenUrl(url));
        }
    }

    effects
}

/// Check which external tools are missing (for startup prompt)
pub async fn check_missing_external_tools() -> Vec<ExternalToolId> {
    let manager = match ExternalToolManager::new() {
        Ok(m) => m,
        Err(_) => return Vec::new(),
    };

    let mut missing = Vec::new();

    // Only check pandoc and node for the startup prompt (libreoffice is optional/large)
    let tools_to_check = [ExternalToolId::Pandoc, ExternalToolId::Node];

    for tool_id in tools_to_check {
        let status = manager.status(tool_id).await;
        if matches!(status, ToolStatus::NotInstalled) {
            missing.push(tool_id);
        }
    }

    missing
}

/// Load external tool statuses for the settings UI
pub async fn load_external_tools_status() -> Vec<(ExternalToolId, settings::ToolStatusInfo)> {
    let manager = match ExternalToolManager::new() {
        Ok(m) => m,
        Err(_) => return Vec::new(),
    };

    let mut statuses = Vec::new();
    let tools = manager.list_tools().await;

    for tool in tools {
        let status_info = match &tool.status {
            ToolStatus::Installed { version } => settings::ToolStatusInfo {
                is_installed: true,
                version: Some(version.clone()),
                is_installing: false,
                install_progress: 0,
                is_supported: true,
            },
            ToolStatus::NotInstalled => settings::ToolStatusInfo {
                is_installed: false,
                version: None,
                is_installing: false,
                install_progress: 0,
                is_supported: true,
            },
            ToolStatus::UnsupportedPlatform => settings::ToolStatusInfo {
                is_installed: false,
                version: None,
                is_installing: false,
                install_progress: 0,
                is_supported: false,
            },
            ToolStatus::Installing { progress_percent } => settings::ToolStatusInfo {
                is_installed: false,
                version: None,
                is_installing: true,
                install_progress: *progress_percent,
                is_supported: true,
            },
            ToolStatus::Failed { .. } => settings::ToolStatusInfo {
                is_installed: false,
                version: None,
                is_installing: false,
                install_progress: 0,
                is_supported: true,
            },
        };
        statuses.push((tool.definition.id, status_info));
    }

    statuses
}

/// Install a single external tool with progress streaming
pub fn install_external_tool_stream(
    tool_id: ExternalToolId,
) -> impl futures::Stream<Item = Message> {
    async_stream::stream! {
        let manager = match ExternalToolManager::new() {
            Ok(m) => m,
            Err(e) => {
                yield Message::Settings(settings::Msg::ExternalToolInstallComplete(
                    tool_id,
                    Err(e.to_string()),
                ));
                return;
            }
        };

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        // Spawn the install task
        let install_future = manager.install(tool_id, move |progress| {
            let percent = progress.percent.unwrap_or(0.0) as u8;
            let _ = tx.send(percent);
        });

        // Poll for progress updates while install is running
        tokio::pin!(install_future);

        loop {
            tokio::select! {
                result = &mut install_future => {
                    let msg = match result {
                        Ok(()) => settings::Msg::ExternalToolInstallComplete(tool_id, Ok(())),
                        Err(e) => settings::Msg::ExternalToolInstallComplete(tool_id, Err(e.to_string())),
                    };
                    yield Message::Settings(msg);
                    break;
                }
                Some(progress) = rx.recv() => {
                    yield Message::Settings(settings::Msg::ExternalToolInstallProgress(tool_id, progress));
                }
            }
        }
    }
}

/// Install all missing tools sequentially with progress
pub fn install_all_tools_stream(
    tools: Vec<ExternalToolId>,
) -> impl futures::Stream<Item = Message> {
    async_stream::stream! {
        let manager = match ExternalToolManager::new() {
            Ok(m) => m,
            Err(e) => {
                // Report failure for all tools
                for tool_id in &tools {
                    yield Message::Settings(settings::Msg::ExternalToolInstallComplete(
                        *tool_id,
                        Err(e.to_string()),
                    ));
                }
                yield Message::Settings(settings::Msg::DismissExternalToolsPrompt);
                return;
            }
        };

        for tool_id in tools {
            // Report current tool
            yield Message::Settings(settings::Msg::ExternalToolInstallProgress(tool_id, 0));

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

            let install_future = manager.install(tool_id, move |progress| {
                let percent = progress.percent.unwrap_or(0.0) as u8;
                let _ = tx.send(percent);
            });

            tokio::pin!(install_future);

            loop {
                tokio::select! {
                    result = &mut install_future => {
                        let msg = match result {
                            Ok(()) => settings::Msg::ExternalToolInstallComplete(tool_id, Ok(())),
                            Err(e) => settings::Msg::ExternalToolInstallComplete(tool_id, Err(e.to_string())),
                        };
                        yield Message::Settings(msg);
                        break;
                    }
                    Some(progress) = rx.recv() => {
                        yield Message::Settings(settings::Msg::ExternalToolInstallProgress(tool_id, progress));
                    }
                }
            }
        }

        // Dismiss prompt after all done
        yield Message::Settings(settings::Msg::DismissExternalToolsPrompt);
    }
}

/// Uninstall an external tool
pub async fn uninstall_external_tool(tool_id: ExternalToolId) -> Result<(), String> {
    let manager = ExternalToolManager::new().map_err(|e| e.to_string())?;
    manager.uninstall(tool_id).await.map_err(|e| e.to_string())
}

pub(in crate::app) fn view(app: &TiccaApp) -> Element<'_, Message> {
    config::view(
        app.theme,
        &app.chat.available_models,
        app.chat.default_model.as_deref(),
        &app.chat.agent_pinned_models,
        app.chat.is_loading_models,
        &app.settings.provider_auth_status,
        &app.settings.accounts_claude,
        &app.settings.accounts_gemini,
        &app.settings.accounts_chatgpt,
        app.chat.yolo_mode_enabled,
        app.expert_mode_enabled,
        &app.settings.recent_sessions,
        app.settings.settings_tab,
        &app.settings.mcp_servers,
        &app.settings.mcp_form,
        &app.settings.mcp_import_json,
        &app.settings.agent_mcp_server_ids,
        &app.settings.external_tools,
    )
}