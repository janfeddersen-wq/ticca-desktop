//! Iced Application state and main loop

use iced::widget::{button, column, container, row, text};
use iced::{Color, Element, Length, Subscription, Task, Theme, time};

use std::path::PathBuf;
use std::time::{Duration, Instant};

mod effects;
mod features;

use ticca_core::external_tools::ExternalToolId;
use ticca_core::llm::auth;

use crate::app_config::load_config;
use crate::messages::{Message, settings};
use crate::theme::AppTheme;

/// Toast notification state
#[derive(Debug, Clone)]
pub struct Toast {
    /// The message to display
    pub message: String,
    /// When the toast was created (for auto-dismiss)
    pub created_at: Instant,
}

impl Toast {
    /// Duration before toast auto-dismisses
    pub const DURATION: Duration = Duration::from_secs(4);

    /// Create a new toast notification
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            created_at: Instant::now(),
        }
    }

    /// Check if the toast has expired
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= Self::DURATION
    }
}

/// State for the external tools installation prompt
#[derive(Debug, Clone)]
pub struct ExternalToolsPromptState {
    /// Tools that are missing and can be installed
    pub missing_tools: Vec<ExternalToolId>,
    /// Whether we're currently installing
    pub is_installing: bool,
    /// Current tool being installed (for progress display)
    pub current_tool: Option<ExternalToolId>,
    /// Install progress percentage
    pub progress: u8,
}

/// State for the update available notification
#[derive(Debug, Clone)]
pub struct UpdateAvailableState {
    /// Current app version
    pub current_version: String,
    /// Latest available version
    pub latest_version: String,
    /// URL to the release page
    pub release_url: String,
}

/// Main application state
pub struct TiccaApp {
    current_view: View,
    theme: AppTheme,
    expert_mode_enabled: bool,
    chat: features::chat::ChatState,
    settings: features::settings::SettingsState,
    toast: Option<Toast>,
    external_tools_prompt: Option<ExternalToolsPromptState>,
    external_tools_prompt_dismissed: bool,
    update_available: Option<UpdateAvailableState>,
}

/// Views in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Chat,
    Settings,
}

impl TiccaApp {
    /// Create a new application instance
    pub fn new() -> (Self, Task<Message>) {
        // Initialize data directories, skills, and UV binary
        Self::initialize_runtime();

        // Load configuration
        let config = load_config();

        // Load working directory from config or use current directory
        let working_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let provider_auth_status = features::settings::check_provider_auth_status();

        let app = Self {
            current_view: View::Chat,
            theme: config.theme,
            expert_mode_enabled: config.expert_mode_enabled,
            chat: features::chat::ChatState::new(&config, working_directory),
            settings: features::settings::SettingsState::new(provider_auth_status),
            toast: None,
            external_tools_prompt: None,
            external_tools_prompt_dismissed: config.external_tools_prompt_dismissed,
            update_available: None,
        };

        // Automatically fetch models on startup if we have credentials
        let mut startup_tasks = vec![];
        if auth::has_any_valid_account() {
            startup_tasks.push(Task::done(Message::Settings(settings::Msg::RefreshModels)));
        }

        // Check for missing external tools on startup (if not dismissed)
        if !app.external_tools_prompt_dismissed {
            startup_tasks.push(Task::perform(
                async { features::settings::check_missing_external_tools().await },
                |missing| Message::Settings(settings::Msg::ShowExternalToolsPrompt(missing)),
            ));
        }

        // Check for updates (with skip counter logic)
        if config.update_check_skip_remaining > 0 {
            // Decrement the skip counter
            let new_count = config.update_check_skip_remaining - 1;
            let _ = ticca_core::config::ConfigService::set_setting(
                ticca_core::config::setting_keys::UPDATE_CHECK_SKIP_REMAINING,
                &new_count.to_string(),
            );
            tracing::debug!("Update check skipped, {} startups remaining", new_count);
        } else {
            // Check for updates
            let dismissed_version = config.update_check_dismissed_version.clone();
            startup_tasks.push(Task::perform(
                async move { ticca_core::check_for_update(dismissed_version.as_deref()).await },
                |result| {
                    Message::CheckForUpdateResult(result.map(|r| {
                        crate::messages::UpdateAvailableInfo {
                            current_version: ticca_core::CURRENT_VERSION.to_string(),
                            latest_version: r.version,
                            release_url: r.html_url,
                        }
                    }))
                },
            ));
        }

        let startup_task = Task::batch(startup_tasks);

        (app, startup_task)
    }

    /// Get the window title
    pub fn title(&self) -> String {
        format!("Ticca Desktop - {}", self.chat.current_agent.display_name())
    }

    /// Handle a message and return any resulting tasks
    pub fn update(&mut self, message: Message) -> Task<Message> {
        let effects = match message {
            Message::Noop => Vec::new(),
            Message::DismissToast => {
                self.toast = None;
                Vec::new()
            }
            Message::ToastTick => {
                // Auto-dismiss expired toasts
                if self.toast.as_ref().is_some_and(|t| t.is_expired()) {
                    self.toast = None;
                }
                Vec::new()
            }
            Message::CheckForUpdateResult(info) => {
                if let Some(info) = info {
                    self.update_available = Some(UpdateAvailableState {
                        current_version: info.current_version,
                        latest_version: info.latest_version,
                        release_url: info.release_url,
                    });
                }
                Vec::new()
            }
            Message::DismissUpdate => {
                // "Remind Me Later" - set skip counter to 10
                let _ = ticca_core::config::ConfigService::set_setting(
                    ticca_core::config::setting_keys::UPDATE_CHECK_SKIP_REMAINING,
                    "10",
                );
                self.update_available = None;
                Vec::new()
            }
            Message::SkipThisVersion(version) => {
                // "Skip This Version" - save the dismissed version
                let _ = ticca_core::config::ConfigService::set_setting(
                    ticca_core::config::setting_keys::UPDATE_CHECK_DISMISSED_VERSION,
                    &version,
                );
                self.update_available = None;
                Vec::new()
            }
            Message::OpenReleaseUrl(url) => {
                let _ = open::that(&url);
                Vec::new()
            }
            Message::Chat(msg) => features::chat::update(self, msg),
            Message::Settings(msg) => features::settings::update(self, msg),
        };

        self.execute_effects(effects)
    }

    fn execute_effects(&self, effects: Vec<effects::Effect>) -> Task<Message> {
        let tasks: Vec<Task<Message>> = effects.into_iter().map(effects::task).collect();

        Task::batch(tasks)
    }

    /// Render the application view
    pub fn view(&self) -> Element<'_, Message> {
        let content = match self.current_view {
            View::Chat => features::chat::view(self),
            View::Settings => features::settings::view(self),
        };

        // Main content container
        let main = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(0);

        // Wrap with toast overlay if present
        let base: Element<Message> = if let Some(ref toast) = self.toast {
            // Toast notification in bottom-right corner
            let toast_content = container(
                row![
                    text(&toast.message).size(14).color(Color::WHITE),
                    iced::widget::Space::new().width(10),
                    button(text("×").size(12).color(Color::WHITE))
                        .on_press(Message::DismissToast)
                        .padding(4)
                        .style(crate::theme::styles::icon_button)
                ]
                .align_y(iced::Alignment::Center),
            )
            .padding(12)
            .style(|theme: &iced::Theme| {
                let palette = theme.extended_palette();
                container::Style {
                    background: Some(palette.danger.base.color.into()),
                    text_color: Some(Color::WHITE),
                    border: iced::Border {
                        radius: 8.0.into(),
                        ..Default::default()
                    },
                    shadow: iced::Shadow {
                        color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
                        offset: iced::Vector::new(0.0, 2.0),
                        blur_radius: 8.0,
                    },
                    ..Default::default()
                }
            });

            // Position toast in bottom-right corner
            let toast_overlay = container(toast_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Bottom)
                .padding(20);

            iced::widget::stack![main, toast_overlay].into()
        } else {
            main.into()
        };

        let with_approval = features::chat::wrap_with_approval_modal(self, base);
        let with_tools_prompt = self.wrap_with_external_tools_prompt(with_approval);
        self.wrap_with_update_modal(with_tools_prompt)
    }

    /// Wrap content with external tools prompt modal if active
    fn wrap_with_external_tools_prompt<'a>(
        &self,
        base: Element<'a, Message>,
    ) -> Element<'a, Message> {
        use crate::theme::styles;

        let Some(ref prompt_state) = self.external_tools_prompt else {
            return base;
        };

        // Build the modal content
        let title = if prompt_state.is_installing {
            text("Installing External Tools...").size(20)
        } else {
            text("Install External Tools?").size(20)
        };

        let description = if prompt_state.is_installing {
            let tool_name = prompt_state
                .current_tool
                .map(|t| t.as_str())
                .unwrap_or("tool");
            text(format!(
                "Installing {}... {}%",
                tool_name, prompt_state.progress
            ))
            .size(14)
        } else {
            let tool_names: Vec<&str> = prompt_state
                .missing_tools
                .iter()
                .map(|t| t.as_str())
                .collect();
            text(format!(
                "The following tools are not installed: {}\n\nThese tools enable advanced features like document conversion and JavaScript execution.",
                tool_names.join(", ")
            ))
            .size(14)
        };

        let buttons = if prompt_state.is_installing {
            row![text(format!("Progress: {}%", prompt_state.progress)).size(14),]
                .spacing(10)
                .align_y(iced::Alignment::Center)
        } else {
            row![
                button(text("Install All").size(14))
                    .on_press(Message::Settings(settings::Msg::InstallAllMissingTools))
                    .style(styles::primary_button)
                    .padding([8, 16]),
                button(text("Not Now").size(14))
                    .on_press(Message::Settings(settings::Msg::DismissExternalToolsPrompt))
                    .style(styles::secondary_button)
                    .padding([8, 16]),
                button(text("Don't Ask Again").size(14))
                    .on_press(Message::Settings(
                        settings::Msg::DismissExternalToolsPromptPermanently
                    ))
                    .style(styles::secondary_button)
                    .padding([8, 16]),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center)
        };

        let modal_content = container(
            column![
                title,
                iced::widget::Space::new().height(10),
                description,
                iced::widget::Space::new().height(20),
                buttons,
            ]
            .spacing(5)
            .align_x(iced::Alignment::Center),
        )
        .padding(30)
        .width(Length::Fixed(500.0))
        .style(styles::card_container);

        // Create overlay
        let overlay = container(modal_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_theme: &iced::Theme| container::Style {
                background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.5).into()),
                ..Default::default()
            });

        // Stack base and overlay
        iced::widget::stack![base, overlay].into()
    }

    /// Wrap content with update available modal if active
    fn wrap_with_update_modal<'a>(&'a self, base: Element<'a, Message>) -> Element<'a, Message> {
        use crate::theme::styles;

        let Some(ref update_state) = self.update_available else {
            return base;
        };

        // Build the modal content
        let title = text("Update Available").size(20);

        let description = column![
            text("A new version of Ticca is available!").size(14),
            iced::widget::Space::new().height(10),
            row![
                text("Current version: ").size(14),
                text(&update_state.current_version)
                    .size(14)
                    .style(|theme: &iced::Theme| {
                        let palette = theme.extended_palette();
                        text::Style {
                            color: Some(palette.secondary.strong.color),
                        }
                    }),
            ],
            row![
                text("Latest version: ").size(14),
                text(&update_state.latest_version)
                    .size(14)
                    .style(|theme: &iced::Theme| {
                        let palette = theme.extended_palette();
                        text::Style {
                            color: Some(palette.success.base.color),
                        }
                    }),
            ],
        ]
        .spacing(4);

        let release_url = update_state.release_url.clone();
        let latest_version = update_state.latest_version.clone();

        let buttons = row![
            button(text("View Release").size(14))
                .on_press(Message::OpenReleaseUrl(release_url))
                .style(styles::primary_button)
                .padding([8, 16]),
            button(text("Remind Me Later").size(14))
                .on_press(Message::DismissUpdate)
                .style(styles::secondary_button)
                .padding([8, 16]),
            button(text("Skip This Version").size(14))
                .on_press(Message::SkipThisVersion(latest_version))
                .style(styles::secondary_button)
                .padding([8, 16]),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);

        let modal_content = container(
            column![
                title,
                iced::widget::Space::new().height(10),
                description,
                iced::widget::Space::new().height(20),
                buttons,
            ]
            .spacing(5)
            .align_x(iced::Alignment::Center),
        )
        .padding(30)
        .width(Length::Fixed(450.0))
        .style(styles::card_container);

        // Create overlay
        let overlay = container(modal_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_theme: &iced::Theme| container::Style {
                background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.5).into()),
                ..Default::default()
            });

        // Stack base and overlay
        iced::widget::stack![base, overlay].into()
    }

    /// Get the current theme
    pub fn theme(&self) -> Theme {
        self.theme.to_iced_theme()
    }

    /// Get subscriptions (keyboard shortcuts, file drop events, and streaming stats timer)
    pub fn subscription(&self) -> Subscription<Message> {
        let keybindings = crate::keybindings::subscription();

        let mut subs: Vec<Subscription<Message>> = vec![keybindings];
        subs.extend(self.chat.subscriptions());

        // Toast auto-dismiss timer (only when a toast is active)
        if self.toast.is_some() {
            subs.push(time::every(Duration::from_millis(100)).map(|_| Message::ToastTick));
        }

        Subscription::batch(subs)
    }

    /// Initialize runtime directories and extract bundled assets.
    ///
    /// This is called once at startup to ensure:
    /// - All data directories exist (skills, bin, venvs)
    /// - Skills bundle is extracted (if version changed)
    /// - UV binary is available (extracted on first use)
    ///
    /// Errors are logged but don't crash the app - features will fail
    /// gracefully if initialization failed.
    fn initialize_runtime() {
        use tracing::{info, warn};

        // Ensure all data directories exist
        if let Err(e) = ticca_core::config::ensure_dirs_exist() {
            warn!("Failed to create data directories: {}", e);
        }

        // Extract skills bundle if needed
        match ticca_core::extract_skills_if_needed() {
            Ok(skills_dir) => {
                info!("Skills available at: {}", skills_dir.display());
            }
            Err(e) => {
                warn!("Failed to extract skills bundle: {}", e);
            }
        }

        // Pre-extract UV binary (optional - could also be lazy on first use)
        // This ensures UV is ready when skills need Python environments
        match ticca_core::ensure_uv_available() {
            Ok(uv_path) => {
                info!("UV binary available at: {}", uv_path.display());
            }
            Err(e) => {
                warn!("Failed to extract UV binary: {}", e);
            }
        }
    }
}

impl Default for TiccaApp {
    fn default() -> Self {
        Self::new().0
    }
}
