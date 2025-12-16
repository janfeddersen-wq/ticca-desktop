//! Application state and root view.
//!
//! This module contains the main application state (`TiccaApp`) and
//! the root view that orchestrates all UI components.

use std::sync::Arc;

use chrono::Utc;
use gpui::{div, prelude::*, Context, Render, Styled, Window};
use tokio::sync::mpsc;
use tracing::{debug, info};

use ticca_bridge::SharedAgentController;
use ticca_config::Settings;
use ticca_db::Database;

use crate::components::{
    chat::{render_chat_view, ChatMessage, ChatView},
    input::{render_input_bar, InputBar},
    sidebar::{render_sidebar, ConversationPreview, Sidebar},
    toolbar::{render_toolbar, ConnectionStatus, Toolbar},
    MarkdownRenderer,
};
use crate::theme::Theme;

/// Application configuration.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Application settings
    pub settings: Settings,
    /// Database path
    pub db_path: Option<String>,
    /// Theme name
    pub theme_name: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            db_path: None,
            theme_name: "dark".to_string(),
        }
    }
}

/// Message types for async communication with the UI.
#[derive(Debug, Clone)]
pub enum AppMessage {
    /// Stream chunk received
    StreamChunk(String),
    /// Stream completed
    StreamDone,
    /// Error occurred
    Error(String),
    /// Conversation loaded
    ConversationLoaded(Vec<ChatMessage>),
    /// Conversations list updated
    ConversationsUpdated(Vec<ConversationPreview>),
}

/// Current session information.
#[derive(Debug, Clone)]
pub struct CurrentSession {
    /// Session ID
    pub id: String,
    /// Conversation ID
    pub conversation_id: String,
    /// Agent name
    pub agent_name: String,
    /// Model name
    pub model_name: Option<String>,
}

/// Main application state.
///
/// Holds all shared state and coordinates between components.
/// All async operations are offloaded to background tokio tasks
/// to maintain 120FPS UI performance.
pub struct TiccaApp {
    /// Database connection (Arc for thread-safe sharing)
    db: Option<Arc<Database>>,
    /// Application configuration
    config: Arc<AppConfig>,
    /// Agent controller for executing AI requests
    agent_controller: Option<SharedAgentController>,
    /// Current active session
    current_session: Option<CurrentSession>,
    /// Tokio runtime handle for async operations
    runtime: Option<tokio::runtime::Handle>,
    /// Message channel for async updates
    message_rx: Option<mpsc::UnboundedReceiver<AppMessage>>,
    /// Message sender (cloneable for background tasks)
    message_tx: Option<mpsc::UnboundedSender<AppMessage>>,
    /// Current theme
    theme: Theme,
    /// Markdown renderer
    renderer: MarkdownRenderer,
}

impl TiccaApp {
    /// Create a new application instance.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            db: None,
            config: Arc::new(AppConfig::default()),
            agent_controller: None,
            current_session: None,
            runtime: None,
            message_rx: Some(rx),
            message_tx: Some(tx),
            theme: Theme::dark(),
            renderer: MarkdownRenderer::new(),
        }
    }

    /// Create with configuration.
    pub fn with_config(mut self, config: AppConfig) -> Self {
        self.theme = Theme::from_name(&config.theme_name);
        self.config = Arc::new(config);
        self
    }

    /// Set the tokio runtime handle.
    pub fn with_runtime(mut self, runtime: tokio::runtime::Handle) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Set the database.
    pub fn with_database(mut self, db: Database) -> Self {
        self.db = Some(Arc::new(db));
        self
    }

    /// Set the agent controller.
    pub fn with_agent_controller(mut self, controller: SharedAgentController) -> Self {
        self.agent_controller = Some(controller);
        self
    }

    /// Get the current theme.
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Set theme by name.
    pub fn set_theme(&mut self, name: &str) {
        self.theme = Theme::from_name(name);
    }

    /// Get message sender for background tasks.
    pub fn message_sender(&self) -> Option<mpsc::UnboundedSender<AppMessage>> {
        self.message_tx.clone()
    }

    /// Get the markdown renderer.
    pub fn renderer(&self) -> &MarkdownRenderer {
        &self.renderer
    }

    /// Check if database is connected.
    pub fn is_db_connected(&self) -> bool {
        self.db.is_some()
    }

    /// Check if agent controller is available.
    pub fn has_agent_controller(&self) -> bool {
        self.agent_controller.is_some()
    }

    /// Get current session info.
    pub fn current_session(&self) -> Option<&CurrentSession> {
        self.current_session.as_ref()
    }

    /// Start a new session.
    pub fn start_session(&mut self, agent_name: &str, model_name: Option<&str>) {
        let session_id = format!("session-{}", uuid::Uuid::new_v4());
        let conversation_id = format!("conv-{}", uuid::Uuid::new_v4());

        self.current_session = Some(CurrentSession {
            id: session_id,
            conversation_id,
            agent_name: agent_name.to_string(),
            model_name: model_name.map(|s| s.to_string()),
        });

        info!(agent = agent_name, "Started new session");
    }

    /// End current session.
    pub fn end_session(&mut self) {
        self.current_session = None;
        debug!("Session ended");
    }
}

impl Default for TiccaApp {
    fn default() -> Self {
        Self::new()
    }
}

/// Root view that composes all UI components.
///
/// This is the main view rendered by GPUI, containing:
/// - Sidebar (conversation list)
/// - Toolbar (navigation and status)
/// - Chat view (messages)
/// - Input bar (message composition)
pub struct RootView {
    /// Application state
    app: TiccaApp,
    /// Chat view state
    chat: ChatView,
    /// Input bar state
    input: InputBar,
    /// Sidebar state
    sidebar: Sidebar,
    /// Toolbar state
    toolbar: Toolbar,
}

impl RootView {
    /// Create a new root view.
    pub fn new(app: TiccaApp) -> Self {
        let theme = app.theme().clone();

        Self {
            chat: ChatView::new(theme.clone()),
            input: InputBar::new(),
            sidebar: Sidebar::new(),
            toolbar: Toolbar::new(),
            app,
        }
    }

    /// Get the current theme.
    pub fn theme(&self) -> &Theme {
        self.app.theme()
    }

    /// Handle send message action.
    pub fn send_message(&mut self) {
        if !self.input.can_send() {
            return;
        }

        let text = self.input.text().to_string();
        let message_id = format!("msg-{}", uuid::Uuid::new_v4());

        // Add user message to chat
        self.chat.add_user_message(&message_id, &text);

        // Clear input
        self.input.clear();
        self.input.start_sending();

        // Update sidebar preview
        if let Some(session) = self.app.current_session() {
            self.sidebar.update_conversation(
                &session.conversation_id,
                None,
                Some(&text),
                None,
            );
        }

        // Start streaming response
        let response_id = format!("msg-{}", uuid::Uuid::new_v4());
        self.chat.start_streaming(&response_id);

        // TODO: Actually send to agent controller in background task
        // For now, simulate a response
        info!("Message sent: {}", text);
    }

    /// Handle stream chunk received.
    pub fn on_stream_chunk(&mut self, content: &str) {
        self.chat.append_streaming_content(content);
    }

    /// Handle stream completion.
    pub fn on_stream_done(&mut self) {
        self.chat.finish_streaming();
        self.input.finish_sending();
    }

    /// Handle new conversation.
    pub fn new_conversation(&mut self) {
        self.chat.clear();
        self.app.start_session(
            &self.input.selected_agent,
            Some(&self.input.selected_model),
        );

        // Add to sidebar
        if let Some(session) = self.app.current_session() {
            let preview = ConversationPreview::new(
                &session.conversation_id,
                "New Conversation",
                "Start typing...",
                Utc::now(),
            )
            .with_agent(session.agent_name.clone());

            self.sidebar.add_conversation(preview);
            self.sidebar.select(&session.conversation_id);
        }

        self.toolbar.set_title("New Conversation");
        info!("New conversation started");
    }

    /// Handle conversation selection.
    pub fn select_conversation(&mut self, conversation_id: &str) {
        self.sidebar.select(conversation_id);
        // TODO: Load conversation messages from database
        info!(conversation_id, "Conversation selected");
    }

    /// Toggle sidebar collapsed state.
    pub fn toggle_sidebar(&mut self) {
        self.sidebar.toggle_collapsed();
    }

    /// Update theme.
    pub fn set_theme(&mut self, name: &str) {
        self.app.set_theme(name);
        self.chat.set_theme(self.app.theme().clone());
    }

    /// Get connection status.
    pub fn connection_status(&self) -> ConnectionStatus {
        if self.app.has_agent_controller() {
            ConnectionStatus::Connected
        } else {
            ConnectionStatus::Disconnected
        }
    }
}

impl Render for RootView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let theme = self.theme().clone();

        // Update toolbar status
        self.toolbar.set_status(self.connection_status());
        self.toolbar.set_agent(&self.input.selected_agent);
        self.toolbar.set_model(&self.input.selected_model);

        div()
            .flex()
            .flex_row()
            .size_full()
            .bg(theme.background)
            // Sidebar
            .child(render_sidebar(&self.sidebar, &theme))
            // Main content area
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .h_full()
                    // Toolbar
                    .child(render_toolbar(&self.toolbar, &theme))
                    // Chat view (takes remaining space)
                    .child(
                        div()
                            .flex_1()
                            .overflow_y_hidden()
                            .child(render_chat_view(&self.chat)),
                    )
                    // Input bar (sticky bottom)
                    .child(render_input_bar(&self.input, &theme)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let app = TiccaApp::new();
        assert!(app.db.is_none());
        assert!(app.current_session.is_none());
    }

    #[test]
    fn test_app_with_config() {
        let config = AppConfig {
            theme_name: "light".to_string(),
            ..Default::default()
        };
        let app = TiccaApp::new().with_config(config);
        // Light theme should have high background lightness
        assert!(app.theme().background.l > 0.9);
    }

    #[test]
    fn test_start_session() {
        let mut app = TiccaApp::new();
        assert!(app.current_session().is_none());

        app.start_session("code-puppy", Some("gpt-4"));
        assert!(app.current_session().is_some());

        let session = app.current_session().unwrap();
        assert_eq!(session.agent_name, "code-puppy");
        assert_eq!(session.model_name.as_deref(), Some("gpt-4"));
    }

    #[test]
    fn test_end_session() {
        let mut app = TiccaApp::new();
        app.start_session("test", None);
        assert!(app.current_session().is_some());

        app.end_session();
        assert!(app.current_session().is_none());
    }

    #[test]
    fn test_root_view_creation() {
        let app = TiccaApp::new();
        let view = RootView::new(app);
        assert!(view.chat.messages().is_empty());
        assert!(view.input.text().is_empty());
    }

    #[test]
    fn test_send_message() {
        let app = TiccaApp::new();
        let mut view = RootView::new(app);

        // Can't send empty
        view.send_message();
        assert!(view.chat.messages().is_empty());

        // Set text and send
        view.input.set_text("Hello!");
        view.send_message();

        // Should have user message and streaming assistant message
        assert_eq!(view.chat.messages().len(), 2);
        assert!(view.input.text().is_empty());
    }

    #[test]
    fn test_new_conversation() {
        let app = TiccaApp::new();
        let mut view = RootView::new(app);

        view.new_conversation();

        assert!(view.app.current_session().is_some());
        assert_eq!(view.sidebar.conversation_count(), 1);
    }

    #[test]
    fn test_theme_switching() {
        let app = TiccaApp::new();
        let mut view = RootView::new(app);

        // Default is dark
        assert!(view.theme().background.l < 0.2);

        view.set_theme("light");
        assert!(view.theme().background.l > 0.9);
    }
}
