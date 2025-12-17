//! Iced Application state and main loop

use iced::widget::{button, column, container, row, scrollable, text, text_input, Column};
use iced::{Element, Length, Subscription, Task, Theme};

use crate::icons::{self, icon};
use crate::messages::Message;
use crate::theme::{styles, AppTheme};

use ticca_core::agents::{AgentConfig as CoreAgentConfig, AgentType, get_agent};
use ticca_core::config::{ConfigDatabase, OAuthToken, setting_keys};
use ticca_core::config::models::providers;
use ticca_core::llm;
use ticca_core::session::{Session, SessionDatabase, SessionMessage, MessageRole};
use ticca_oauth::{ClaudeOAuth, ChatGptOAuth};

// Rig LLM Framework
use rig::prelude::*;
use rig::providers::anthropic;

use uuid::Uuid;
use chrono::{Local, Utc};
use std::collections::HashMap;
use futures::StreamExt;

/// Main application state
pub struct TiccaApp {
    // UI State
    current_view: View,
    theme: AppTheme,

    // Chat state
    input_value: String,
    messages: Vec<ChatMessage>,
    is_streaming: bool,

    // Agent state
    current_agent: AgentType,
    agent_config: CoreAgentConfig,

    // Model selection state
    available_models: Vec<String>,
    default_model: Option<String>,
    agent_pinned_models: HashMap<AgentType, String>,
    is_loading_models: bool,

    // Session state
    current_session: Option<Session>,

    // Error display
    error_message: Option<String>,
}

/// Views in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Chat,
    Settings,
}

/// A chat message for display
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub is_streaming: bool,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            is_streaming: false,
        }
    }
    
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            is_streaming: false,
        }
    }
    
    pub fn assistant_streaming() -> Self {
        Self {
            role: MessageRole::Assistant,
            content: String::new(),
            is_streaming: true,
        }
    }
}

impl TiccaApp {
    /// Create a new application instance
    pub fn new() -> (Self, Task<Message>) {
        // Load configuration
        let (theme, default_model, agent_pinned_models) = load_config();

        let app = Self {
            current_view: View::Chat,
            theme,
            input_value: String::new(),
            messages: vec![
                ChatMessage::assistant(
                    "Hey there! I'm Ticca, your coding companion. What can I help you build today?"
                ),
            ],
            is_streaming: false,
            current_agent: AgentType::Coding,
            agent_config: CoreAgentConfig::coding(),
            available_models: Vec::new(),
            default_model,
            agent_pinned_models,
            is_loading_models: false,
            current_session: None,
            error_message: None,
        };

        // Automatically fetch models on startup if we have credentials
        let startup_task = if llm::has_claude_credentials() {
            Task::done(Message::RefreshModels)
        } else {
            Task::none()
        };

        (app, startup_task)
    }

    /// Get the window title
    pub fn title(&self) -> String {
        format!("Ticca Desktop - {}", self.current_agent.display_name())
    }

    /// Handle a message and return any resulting tasks
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InputChanged(value) => {
                self.input_value = value;
                Task::none()
            }
            
            Message::SendMessage => {
                if self.input_value.trim().is_empty() || self.is_streaming {
                    return Task::none();
                }

                let user_message = self.input_value.clone();
                self.input_value.clear();

                // Add user message
                self.messages.push(ChatMessage::user(&user_message));

                // Check if we have credentials
                if !llm::has_claude_credentials() {
                    self.messages.push(ChatMessage::assistant(
                        "⚠️ No Claude credentials found. Please go to Settings and authenticate with Claude OAuth first."
                    ));
                    return Task::none();
                }

                // Determine which model to use: pinned > default > fetch from API
                let model_name = self.agent_pinned_models.get(&self.current_agent)
                    .cloned()
                    .or_else(|| self.default_model.clone());

                // Add streaming placeholder
                self.messages.push(ChatMessage::assistant_streaming());
                self.is_streaming = true;

                // Get the system prompt based on current agent
                let agent = get_agent(self.current_agent);
                let system_prompt = agent.system_prompt();

                // Get the auth token for Claude Code
                let auth_token = match get_claude_auth_token() {
                    Some(token) => token,
                    None => {
                        self.messages.push(ChatMessage::assistant(
                            "❌ Failed to get Claude OAuth token. Please re-authenticate in Settings."
                        ));
                        self.is_streaming = false;
                        self.messages.pop(); // Remove the streaming placeholder
                        return Task::none();
                    }
                };

                // Send to Claude via Rig OAuthClient with streaming
                Task::run(
                    run_rig_agent_stream(auth_token, system_prompt, user_message, model_name),
                    |event| event,
                )
            }
            
            Message::StreamChunk(chunk) => {
                if let Some(last) = self.messages.last_mut() {
                    if last.is_streaming {
                        last.content.push_str(&chunk);
                    }
                }
                Task::none()
            }

            Message::StreamComplete => {
                self.is_streaming = false;
                if let Some(last) = self.messages.last_mut() {
                    if last.is_streaming {
                        last.is_streaming = false;
                    }
                }
                // Auto-save session after streaming completes
                self.save_current_session();
                Task::none()
            }

            Message::StreamError(error) => {
                self.is_streaming = false;
                if let Some(last) = self.messages.last_mut() {
                    if last.is_streaming {
                        last.content = format!("❌ Error: {}", error);
                        last.is_streaming = false;
                    }
                }
                Task::none()
            }
            
            Message::OpenSettings => {
                self.current_view = View::Settings;
                Task::none()
            }
            
            Message::CloseSettings => {
                self.current_view = View::Chat;
                Task::none()
            }
            
            Message::ThemeToggle => {
                self.theme = self.theme.next();
                // Save to config
                if let Ok(db) = ConfigDatabase::open() {
                    let _ = db.set_setting(setting_keys::THEME, self.theme.as_str());
                }
                Task::none()
            }

            Message::SetTheme(theme) => {
                self.theme = theme;
                // Save to config
                if let Ok(db) = ConfigDatabase::open() {
                    let _ = db.set_setting(setting_keys::THEME, self.theme.as_str());
                }
                Task::none()
            }
            
            Message::SwitchAgent(agent_type) => {
                self.current_agent = agent_type;
                self.agent_config = CoreAgentConfig::new(agent_type);
                Task::none()
            }
            
            Message::StartOAuth(provider) => {
                // Start OAuth flow in background using spawn_blocking to avoid
                // runtime conflicts with reqwest::blocking inside async context
                Task::perform(
                    async move {
                        // Move the blocking OAuth flow to a dedicated blocking thread
                        tokio::task::spawn_blocking(move || {
                            match provider {
                                crate::messages::OAuthProvider::Claude => {
                                    let oauth = ClaudeOAuth::new();
                                    match oauth.authorize() {
                                        Ok(token_response) => {
                                            // Save token to database
                                            if let Ok(db) = ConfigDatabase::open() {
                                                let expires_at_str = token_response.expires_at()
                                                    .map(|t| t.to_rfc3339())
                                                    .unwrap_or_default();
                                                let refresh = token_response.refresh_token
                                                    .clone()
                                                    .unwrap_or_default();
                                                let token = OAuthToken::new("claude", &token_response.access_token)
                                                    .with_refresh_token(refresh)
                                                    .with_expires_at(expires_at_str);
                                                let _ = db.upsert_oauth_token(&token);
                                            }
                                            Ok(())
                                        }
                                        Err(e) => Err(e.to_string()),
                                    }
                                }
                                crate::messages::OAuthProvider::Gemini => {
                                    // Gemini requires user's own credentials
                                    Err("Gemini OAuth requires setting GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET environment variables. Please set these and restart.".to_string())
                                }
                                crate::messages::OAuthProvider::ChatGpt => {
                                    let oauth = ChatGptOAuth::new();
                                    match oauth.authorize() {
                                        Ok(token_response) => {
                                            if let Ok(db) = ConfigDatabase::open() {
                                                let expires_at_str = token_response.expires_at()
                                                    .map(|t| t.to_rfc3339())
                                                    .unwrap_or_default();
                                                let refresh = token_response.refresh_token
                                                    .clone()
                                                    .unwrap_or_default();
                                                let token = OAuthToken::new("chatgpt", &token_response.access_token)
                                                    .with_refresh_token(refresh)
                                                    .with_expires_at(expires_at_str);
                                                let _ = db.upsert_oauth_token(&token);
                                            }
                                            Ok(())
                                        }
                                        Err(e) => Err(e.to_string()),
                                    }
                                }
                            }
                        })
                        .await
                        .map_err(|e| format!("OAuth task panicked: {}", e))?
                    },
                    move |result| Message::OAuthComplete(provider, result)
                )
            }
            
            Message::OAuthComplete(_provider, result) => {
                match result {
                    Ok(()) => {
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.error_message = Some(e);
                    }
                }
                Task::none()
            }
            
            Message::NewSession => {
                self.messages.clear();
                self.messages.push(ChatMessage::assistant(
                    "Fresh start! What would you like to work on?"
                ));
                self.current_session = None;
                Task::none()
            }
            
            Message::LoadSession(session_id) => {
                tracing::info!("Loading session: {}", session_id);
                if let Ok(db) = SessionDatabase::open() {
                    if let Ok(Some(session)) = db.get_session(&session_id) {
                        if let Ok(messages) = db.get_messages(&session_id) {
                            // Convert session messages to chat messages
                            self.messages = messages.into_iter()
                                .map(|m| ChatMessage {
                                    role: m.role,
                                    content: m.content,
                                    is_streaming: false,
                                })
                                .collect();
                            
                            // Set current session
                            self.current_session = Some(session.clone());
                            
                            // Set agent type if it matches
                            if let Some(agent_type) = AgentType::from_str(&session.agent_type) {
                                self.current_agent = agent_type;
                                self.agent_config = CoreAgentConfig::new(agent_type);
                            }
                            
                            tracing::info!("Loaded session with {} messages", self.messages.len());
                        }
                    }
                }
                // Switch to chat view after loading
                self.current_view = View::Chat;
                Task::none()
            }
            
            Message::RefreshModels => {
                if self.is_loading_models {
                    return Task::none();
                }
                self.is_loading_models = true;

                // Get auth token
                let auth_token = match get_claude_auth_token() {
                    Some(token) => token,
                    None => {
                        self.is_loading_models = false;
                        return Task::none();
                    }
                };

                // Fetch models async
                Task::perform(
                    async move {
                        let client = llm::ClaudeClient::new(auth_token);
                        client.fetch_latest_models().await
                            .map_err(|e| e.to_string())
                    },
                    Message::ModelsLoaded
                )
            }

            Message::ModelsLoaded(result) => {
                self.is_loading_models = false;
                match result {
                    Ok(models) => {
                        tracing::info!("Loaded {} models: {:?}", models.len(), models);
                        self.available_models = models;
                    }
                    Err(e) => {
                        tracing::error!("Failed to load models: {}", e);
                        self.error_message = Some(format!("Failed to load models: {}", e));
                    }
                }
                Task::none()
            }

            Message::SetDefaultModel(model_name) => {
                self.default_model = Some(model_name.clone());
                // Persist to database
                if let Ok(db) = ConfigDatabase::open() {
                    let _ = db.set_setting(setting_keys::DEFAULT_MODEL, &model_name);
                }
                tracing::info!("Set default model: {}", model_name);
                Task::none()
            }

            Message::SetAgentModel(agent_type, model) => {
                match &model {
                    Some(model_name) => {
                        self.agent_pinned_models.insert(agent_type, model_name.clone());
                        // Persist to database
                        if let Ok(db) = ConfigDatabase::open() {
                            let _ = db.set_agent_pinned_model(agent_type.as_str(), model_name);
                        }
                        tracing::info!("Pinned {} to model: {}", agent_type.as_str(), model_name);
                    }
                    None => {
                        self.agent_pinned_models.remove(&agent_type);
                        // Clear from database
                        if let Ok(db) = ConfigDatabase::open() {
                            let _ = db.clear_agent_pinned_model(agent_type.as_str());
                        }
                        tracing::info!("Cleared pinned model for {}", agent_type.as_str());
                    }
                }
                Task::none()
            }

            Message::DismissError => {
                self.error_message = None;
                Task::none()
            }
        }
    }

    /// Render the application view
    pub fn view(&self) -> Element<'_, Message> {
        let content = match self.current_view {
            View::Chat => self.view_chat(),
            View::Settings => self.view_settings(),
        };
        
        // Wrap in container with error overlay if needed
        let main = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(0);
        
        if let Some(ref error) = self.error_message {
            // Show error toast at top
            let error_banner = container(
                row![
                    text(error).size(14),
                    button("×")
                        .on_press(Message::DismissError)
                        .padding(4)
                ]
                .spacing(10)
            )
            .padding(10)
            .style(container::rounded_box);
            
            column![
                error_banner,
                main,
            ]
            .into()
        } else {
            main.into()
        }
    }

    /// Get the current theme
    pub fn theme(&self) -> Theme {
        self.theme.to_iced_theme()
    }
    
    /// Get subscriptions (keyboard shortcuts)
    pub fn subscription(&self) -> Subscription<Message> {
        use iced::event::{self, Event};
        use iced::keyboard;
        
        event::listen_with(|event, _status, _id| {
            match event {
                Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                    let bindings = crate::keybindings::keybindings();
                    
                    if bindings.send_message.matches(&key, modifiers) {
                        return Some(Message::SendMessage);
                    }
                    if bindings.new_session.matches(&key, modifiers) {
                        return Some(Message::NewSession);
                    }
                    if bindings.open_settings.matches(&key, modifiers) {
                        return Some(Message::OpenSettings);
                    }
                    if bindings.toggle_theme.matches(&key, modifiers) {
                        return Some(Message::ThemeToggle);
                    }
                    if bindings.close_panel.matches(&key, modifiers) {
                        return Some(Message::CloseSettings);
                    }
                    if bindings.switch_to_coding.matches(&key, modifiers) {
                        return Some(Message::SwitchAgent(AgentType::Coding));
                    }
                    if bindings.switch_to_planning.matches(&key, modifiers) {
                        return Some(Message::SwitchAgent(AgentType::Planning));
                    }
                    
                    None
                }
                _ => None,
            }
        })
    }
    
    /// Render the chat view
    fn view_chat(&self) -> Element<'_, Message> {
        let is_coding = self.current_agent == AgentType::Coding;
        let is_planning = self.current_agent == AgentType::Planning;

        // Header with agent switcher and settings
        let header = container(
            row![
                // Agent selector tabs
                row![
                    button(
                        row![
                            icon(icons::CODE_SLASH).size(14),
                            text(" Coding").size(14),
                        ]
                        .spacing(4)
                    )
                    .on_press(Message::SwitchAgent(AgentType::Coding))
                    .style(move |theme, status| styles::tab_button(theme, status, is_coding))
                    .padding([8, 12]),
                    button(
                        row![
                            icon(icons::LIST_CHECK).size(14),
                            text(" Planning").size(14),
                        ]
                        .spacing(4)
                    )
                    .on_press(Message::SwitchAgent(AgentType::Planning))
                    .style(move |theme, status| styles::tab_button(theme, status, is_planning))
                    .padding([8, 12]),
                ]
                .spacing(8),

                // Spacer
                iced::widget::horizontal_space(),

                // Actions
                row![
                    button(icon(icons::CIRCLE_HALF).size(16))
                        .on_press(Message::ThemeToggle)
                        .style(styles::icon_button)
                        .padding(8),
                    button(icon(icons::GEAR).size(16))
                        .on_press(Message::OpenSettings)
                        .style(styles::icon_button)
                        .padding(8),
                    button(icon(icons::PLUS_LG).size(16))
                        .on_press(Message::NewSession)
                        .style(styles::icon_button)
                        .padding(8),
                ]
                .spacing(4),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center)
        )
        .padding(10)
        .style(styles::header_container);

        // Message list
        let message_widgets: Vec<Element<Message>> = self.messages
            .iter()
            .map(|msg| self.render_message(msg))
            .collect();

        let messages: Element<Message> = scrollable(
            Column::with_children(message_widgets)
                .spacing(12)
                .padding(20)
        )
        .height(Length::Fill)
        .into();

        // Input area
        let input = container(
            row![
                text_input("Type a message...", &self.input_value)
                    .on_input(Message::InputChanged)
                    .on_submit(Message::SendMessage)
                    .style(styles::text_input_style)
                    .padding(12)
                    .size(14)
                    .width(Length::Fill),
                button(
                    if self.is_streaming {
                        row![icon(icons::HOURGLASS_SPLIT).size(14)]
                    } else {
                        row![icon(icons::SEND_FILL).size(14)]
                    }
                )
                .on_press_maybe(
                    if self.is_streaming || self.input_value.trim().is_empty() {
                        None
                    } else {
                        Some(Message::SendMessage)
                    }
                )
                .style(styles::primary_button)
                .padding([10, 16]),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center)
        )
        .padding(12)
        .style(styles::input_area_container);

        column![
            header,
            messages,
            input,
        ]
        .into()
    }
    
    /// Save the current session to the database
    fn save_current_session(&mut self) {
        if self.messages.is_empty() {
            return;
        }
        
        if let Ok(db) = SessionDatabase::open() {
            // Create or get session ID
            let session_id = self.current_session.as_ref()
                .map(|s| s.id.clone())
                .unwrap_or_else(|| Uuid::new_v4().to_string());
            
            // Create session name from first user message or current time
            let session_name = self.messages.iter()
                .find(|m| m.role == MessageRole::User)
                .map(|m| {
                    let preview: String = m.content.chars().take(50).collect();
                    if m.content.len() > 50 {
                        format!("{}...", preview)
                    } else {
                        preview
                    }
                })
                .unwrap_or_else(|| format!("Session {}", Local::now().format("%Y-%m-%d %H:%M")));
            
            let session = Session {
                id: session_id.clone(),
                name: session_name,
                agent_type: self.current_agent.as_str().to_string(),
                created_at: self.current_session.as_ref()
                    .and_then(|s| s.created_at.clone()),
                updated_at: Some(Utc::now().to_rfc3339()),
                total_tokens: 0,
                message_count: self.messages.len() as i64,
            };
            
            // Try to create, or update if exists
            if self.current_session.is_none() {
                if let Err(e) = db.create_session(&session) {
                    tracing::warn!("Failed to create session: {}", e);
                }
            }
            
            // Clear existing messages and re-add (simple but works)
            let _ = db.clear_session_messages(&session_id);
            
            // Save all messages
            for msg in &self.messages {
                if msg.is_streaming {
                    continue; // Skip streaming messages
                }
                let session_msg = SessionMessage {
                    id: None,
                    session_id: session_id.clone(),
                    role: msg.role,
                    content: msg.content.clone(),
                    tool_calls_json: None,
                    tool_result_json: None,
                    tokens: 0,
                    created_at: None,
                };
                if let Err(e) = db.add_message(&session_msg) {
                    tracing::warn!("Failed to save message: {}", e);
                }
            }
            
            // Update current session reference
            self.current_session = Some(session);
            
            tracing::debug!("Saved session {} with {} messages", session_id, self.messages.len());
        }
    }
    
    /// Render a single message
    fn render_message<'a>(&'a self, msg: &'a ChatMessage) -> Element<'a, Message> {
        let is_user = msg.role == MessageRole::User;
        let is_dark = matches!(self.theme, AppTheme::Dark);

        let label = match msg.role {
            MessageRole::User => "You",
            MessageRole::Assistant => "Ticca",
            MessageRole::System => "System",
            MessageRole::Tool => "Tool",
        };

        let content: Element<Message> = if msg.is_streaming && msg.content.is_empty() {
            row![
                icon(icons::CHAT_DOTS).size(14),
                text(" Thinking...").size(14),
            ]
            .spacing(6)
            .into()
        } else if msg.is_streaming {
            // Render markdown while streaming - shows content as it arrives
            // Append cursor to show streaming is active
            let content_with_cursor = format!("{}▌", msg.content);
            crate::views::components::markdown::render(&content_with_cursor, is_dark)
        } else if msg.role == MessageRole::Assistant || msg.role == MessageRole::System {
            // Render markdown for assistant/system messages
            crate::views::components::markdown::render(&msg.content, is_dark)
        } else {
            text(&msg.content).size(14).into()
        };

        container(
            column![
                text(label).size(12),
                content,
            ]
            .spacing(6)
        )
        .padding(12)
        .width(Length::FillPortion(4))
        .style(move |theme| styles::message_bubble(theme, is_user))
        .into()
    }
    
    /// Render the settings view
    fn view_settings(&self) -> Element<'_, Message> {
        crate::views::config::view(
            self.theme,
            &self.available_models,
            self.default_model.as_deref(),
            &self.agent_pinned_models,
            self.is_loading_models,
        )
    }
}

impl Default for TiccaApp {
    fn default() -> Self {
        Self::new().0
    }
}

/// Load configuration from database
fn load_config() -> (AppTheme, Option<String>, HashMap<AgentType, String>) {
    let db = match ConfigDatabase::open() {
        Ok(db) => db,
        Err(_) => return (AppTheme::Dark, None, HashMap::new()),
    };

    let theme = db.get_setting(setting_keys::THEME)
        .ok()
        .flatten()
        .map(|s| AppTheme::from_str(&s.value))
        .unwrap_or(AppTheme::Dark);

    let default_model = db.get_setting(setting_keys::DEFAULT_MODEL)
        .ok()
        .flatten()
        .map(|s| s.value)
        .filter(|s| !s.is_empty());

    // Load agent pinned models
    let pinned_map = db.get_all_agent_pinned_models()
        .ok()
        .unwrap_or_default();

    let mut agent_pinned_models = HashMap::new();
    for (agent_str, model) in pinned_map {
        if let Some(agent_type) = AgentType::from_str(&agent_str) {
            agent_pinned_models.insert(agent_type, model);
        }
    }

    (theme, default_model, agent_pinned_models)
}

/// Get the Claude OAuth token if available and valid
fn get_claude_auth_token() -> Option<String> {
    let db = ConfigDatabase::open().ok()?;
    let token = db.get_oauth_token(providers::CLAUDE).ok()??;

    if token.is_expired() {
        tracing::warn!("Claude OAuth token is expired");
        return None;
    }

    Some(token.access_token)
}

/// Run the Rig agent with streaming response
///
/// Returns a Stream that yields Message events for each chunk
fn run_rig_agent_stream(
    auth_token: String,
    system_prompt: String,
    user_message: String,
    model_name: Option<String>,
) -> impl futures::Stream<Item = Message> {
    async_stream::stream! {
        // Use provided model or fetch from API
        let model_name = match model_name {
            Some(name) => {
                tracing::info!("Using configured model: {}", name);
                name
            }
            None => {
                match fetch_best_model(&auth_token).await {
                    Ok(name) => {
                        tracing::info!("Using auto-detected model: {}", name);
                        name
                    }
                    Err(e) => {
                        yield Message::StreamError(e);
                        return;
                    }
                }
            }
        };

        // Create OAuth client with the token
        let client: anthropic::OAuthClient = match anthropic::OAuthClient::builder()
            .api_key(auth_token)
            .build()
        {
            Ok(client) => client,
            Err(e) => {
                yield Message::StreamError(format!("Failed to create OAuth client: {}", e));
                return;
            }
        };

        // Create agent with system prompt using the model
        let agent = client
            .agent(&model_name)
            .preamble(&system_prompt)
            .temperature(0.7)
            .max_tokens(4096)
            .build();

        // Use streaming prompt
        use rig::streaming::StreamingPrompt;
        use rig::agent::MultiTurnStreamItem;
        use rig::streaming::StreamedAssistantContent;

        // stream_prompt returns a StreamingPromptRequest, which we await to get the stream
        let mut stream = agent.stream_prompt(&user_message).await;

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(text_chunk))) => {
                    if !text_chunk.text.is_empty() {
                        yield Message::StreamChunk(text_chunk.text);
                    }
                }
                Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Reasoning(reasoning))) => {
                    // Also stream reasoning/thinking content
                    let text = reasoning.reasoning.join("");
                    if !text.is_empty() {
                        yield Message::StreamChunk(text);
                    }
                }
                Ok(MultiTurnStreamItem::FinalResponse(_)) => {
                    // Stream complete, we'll yield StreamComplete at the end
                }
                Ok(_) => {
                    // Other stream items (tool calls etc)
                }
                Err(e) => {
                    yield Message::StreamError(format!("Stream error: {}", e));
                    return;
                }
            }
        }
        yield Message::StreamComplete;
    }
}

/// Fetch the best available model from the Claude API
async fn fetch_best_model(auth_token: &str) -> Result<String, String> {
    // Use the ClaudeClient to fetch models
    let client = llm::ClaudeClient::new(auth_token.to_string());
    let models = client.fetch_latest_models().await
        .map_err(|e| format!("Failed to fetch models: {}", e))?;

    // Prefer sonnet, then opus, then haiku
    let preferred_order = ["sonnet", "opus", "haiku"];

    for family in preferred_order {
        if let Some(model) = models.iter().find(|m| m.contains(family)) {
            return Ok(model.clone());
        }
    }

    // If no match, return the first available model or error
    models.into_iter().next()
        .ok_or_else(|| "No models available from Claude API".to_string())
}

/// Bundled Noto Sans font for consistent text rendering
const NOTO_SANS_REGULAR: &[u8] = include_bytes!("../assets/fonts/NotoSans-Regular.ttf");

/// Bundled Noto Sans Bold font
const NOTO_SANS_BOLD: &[u8] = include_bytes!("../assets/fonts/NotoSans-Bold.ttf");

/// Bundled Noto Sans Italic font
const NOTO_SANS_ITALIC: &[u8] = include_bytes!("../assets/fonts/NotoSans-Italic.ttf");

/// Bundled Noto Sans Bold Italic font
const NOTO_SANS_BOLD_ITALIC: &[u8] = include_bytes!("../assets/fonts/NotoSans-BoldItalic.ttf");

/// Bundled Noto Sans Mono font for code blocks
const NOTO_SANS_MONO: &[u8] = include_bytes!("../assets/fonts/NotoSansMono-Regular.ttf");

/// Bundled Noto Sans Symbols font for symbols and special characters
const NOTO_SANS_SYMBOLS: &[u8] = include_bytes!("../assets/fonts/NotoSansSymbols-Regular.ttf");

/// Bundled Noto Sans Symbols 2 font for emoji and extended symbols
/// Note: cosmic-text doesn't support color emoji fonts (CBDT/CBLC), so we use monochrome
const NOTO_SANS_SYMBOLS2: &[u8] = include_bytes!("../assets/fonts/NotoSansSymbols2-Regular.ttf");

/// Run the application with default settings
pub fn run() -> anyhow::Result<()> {
    iced::application(TiccaApp::title, TiccaApp::update, TiccaApp::view)
        .subscription(TiccaApp::subscription)
        .theme(TiccaApp::theme)
        // Load bundled fonts - order matters for fallback chain
        .font(NOTO_SANS_REGULAR)
        .font(NOTO_SANS_BOLD)
        .font(NOTO_SANS_ITALIC)
        .font(NOTO_SANS_BOLD_ITALIC)
        .font(NOTO_SANS_MONO)
        .font(NOTO_SANS_SYMBOLS)
        .font(NOTO_SANS_SYMBOLS2)
        .font(iced_fonts::BOOTSTRAP_FONT_BYTES)
        // Set Noto Sans as the default font for consistent rendering
        .default_font(iced::Font::with_name("Noto Sans"))
        .window_size(iced::Size::new(900.0, 700.0))
        .antialiasing(true)
        .run_with(TiccaApp::new)?;
    Ok(())
}
