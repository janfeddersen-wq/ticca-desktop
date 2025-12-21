//! Iced Application state and main loop

use iced::widget::{button, column, container, row, text};
use iced::{Element, Length, Subscription, Task, Theme};

use std::path::PathBuf;

mod effects;
mod features;

use ticca_core::llm::auth;

use crate::app_config::load_config;
use crate::messages::{Message, settings};
use crate::theme::AppTheme;

/// Main application state
pub struct TiccaApp {
    current_view: View,
    theme: AppTheme,
    expert_mode_enabled: bool,
    chat: features::chat::ChatState,
    settings: features::settings::SettingsState,
    error_message: Option<String>,
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
            error_message: None,
        };

        // Automatically fetch models on startup if we have credentials
        let startup_task = if auth::has_any_valid_account() {
            Task::done(Message::Settings(settings::Msg::RefreshModels))
        } else {
            Task::none()
        };

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
            Message::DismissError => {
                self.error_message = None;
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

        // Wrap in container with error overlay if needed
        let main = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(0);

        let base: Element<Message> = if let Some(ref error) = self.error_message {
            // Show error toast at top
            let error_banner = container(
                row![
                    text(error).size(14),
                    button("×").on_press(Message::DismissError).padding(4)
                ]
                .spacing(10),
            )
            .padding(10)
            .style(container::rounded_box);

            column![error_banner, main,].into()
        } else {
            main.into()
        };

        features::chat::wrap_with_approval_modal(self, base)
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
        Subscription::batch(subs)
    }
}

impl Default for TiccaApp {
    fn default() -> Self {
        Self::new().0
    }
}
