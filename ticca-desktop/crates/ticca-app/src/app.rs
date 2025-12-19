//! Iced Application state and main loop

use iced::widget::{button, column, container, row, text, text_editor};
use iced::widget::scrollable::AbsoluteOffset;
use iced::{Element, Length, Subscription, Task, Theme, widget};

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use base64::Engine as _;

use ticca_core::agents::{AgentConfig as CoreAgentConfig, AgentType, get_agent};
use ticca_core::config::{ConfigDatabase, setting_keys};
use ticca_core::llm;
use ticca_core::session::Session;
use crate::oauth_handler;
use crate::session_manager;

use crate::app_config::load_config;
use crate::chat_message::ChatMessage;
use crate::helpers::format_tool_call_oneliner;
use crate::image_handler;
use crate::llm_stream::{self, get_claude_auth_token, get_gemini_auth_token, get_chatgpt_auth_token, get_chatgpt_id_token};
use crate::messages::{Message, ImageAttachment};
use crate::theme::AppTheme;
use crate::views::chat::CHAT_SCROLLABLE_ID;
use crate::views::config::ProviderAuthStatus;

/// Main application state
pub struct TiccaApp {
    // UI State
    current_view: View,
    theme: AppTheme,

    // Chat state
    input_value: String,
    messages: Vec<ChatMessage>,
    is_streaming: bool,

    // Image attachments pending to be sent
    pending_attachments: Vec<ImageAttachment>,

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

    // Working directory for tools
    working_directory: PathBuf,

    // Agent configuration
    max_tool_rounds: u32,

    // Raw view toggle state (message indices showing raw text)
    raw_view_messages: HashSet<usize>,
    // Text editor content for raw view (created on demand)
    raw_view_editors: HashMap<usize, text_editor::Content>,

    // Error display
    error_message: Option<String>,

    // Provider authentication status
    provider_auth_status: ProviderAuthStatus,

    // Auto-scroll tracking: true if user is at/near bottom of chat
    user_at_bottom: bool,

    // Streaming stats for TPS indicator
    stream_start_time: Option<std::time::Instant>,
    stream_chars_received: usize,
    current_tps: f64,
    /// Toggles on each chunk for pulse effect
    stream_pulse: bool,
    /// Rolling window of TPS samples for 1-minute average (one sample per second)
    tps_samples: std::collections::VecDeque<f64>,
    /// Time when we last received bytes (for detecting "waiting" state)
    last_bytes_time: Option<std::time::Instant>,
    /// Counter for spinner animation frames
    spinner_frame: usize,
}

/// Views in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Chat,
    Settings,
}

/// Check if all providers have valid (non-expired) tokens
fn check_provider_auth_status() -> ProviderAuthStatus {
    ProviderAuthStatus {
        claude: get_claude_auth_token().is_some(),
        gemini: get_gemini_auth_token().is_some(),
        chatgpt: get_chatgpt_auth_token().is_some(),
    }
}

impl TiccaApp {
    /// Create a new application instance
    pub fn new() -> (Self, Task<Message>) {
        // Load configuration
        let config = load_config();

        // Load working directory from config or use current directory
        let working_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        let app = Self {
            current_view: View::Chat,
            theme: config.theme,
            input_value: String::new(),
            messages: vec![
                ChatMessage::assistant("Welcome to Ticca. How can I assist you?"),
            ],
            is_streaming: false,
            pending_attachments: Vec::new(),
            current_agent: AgentType::Coding,
            agent_config: CoreAgentConfig::coding(),
            available_models: Vec::new(),
            default_model: config.default_model,
            agent_pinned_models: config.agent_pinned_models,
            is_loading_models: false,
            current_session: None,
            working_directory,
            max_tool_rounds: config.max_tool_rounds,
            raw_view_messages: HashSet::new(),
            raw_view_editors: HashMap::new(),
            error_message: None,
            provider_auth_status: check_provider_auth_status(),
            user_at_bottom: true, // Start at bottom
            stream_start_time: None,
            stream_chars_received: 0,
            current_tps: 0.0,
            stream_pulse: false,
            tps_samples: std::collections::VecDeque::with_capacity(60),
            last_bytes_time: None,
            spinner_frame: 0,
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

            Message::ChatScrolled(viewport) => {
                // Check if user is at or near the bottom of the chat
                // We consider "at bottom" if within 50 pixels of the end
                let content_height = viewport.content_bounds().height;
                let viewport_height = viewport.bounds().height;
                let scroll_offset = viewport.absolute_offset().y;
                let max_scroll = (content_height - viewport_height).max(0.0);
                let distance_from_bottom = max_scroll - scroll_offset;
                self.user_at_bottom = distance_from_bottom < 50.0;
                Task::none()
            }

            Message::SendMessage => {
                // Check if we have something to send (text or images)
                let has_text = !self.input_value.trim().is_empty();
                let has_images = !self.pending_attachments.is_empty();

                if (!has_text && !has_images) || self.is_streaming {
                    return Task::none();
                }

                let user_message = self.input_value.clone();
                self.input_value.clear();

                // Take the pending attachments
                let attachments = std::mem::take(&mut self.pending_attachments);

                // Build display message with image indicators
                let display_message = if attachments.is_empty() {
                    user_message.clone()
                } else {
                    let img_count = attachments.len();
                    let img_text = if img_count == 1 {
                        "[1 image attached]".to_string()
                    } else {
                        format!("[{} images attached]", img_count)
                    };
                    if user_message.is_empty() {
                        img_text
                    } else {
                        format!("{}\n\n{}", img_text, user_message)
                    }
                };

                // Add user message
                self.messages.push(ChatMessage::user(&display_message));

                // User just sent a message, so scroll to bottom and track as at bottom
                self.user_at_bottom = true;

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

                // Initialize streaming stats
                self.stream_start_time = Some(std::time::Instant::now());
                self.stream_chars_received = 0;
                self.current_tps = 0.0;
                self.stream_pulse = false;

                // Get the system prompt based on current agent
                let agent = get_agent(self.current_agent);
                let system_prompt = agent.system_prompt();

                // Get the auth token for Claude Code
                let auth_token = match llm_stream::get_claude_auth_token() {
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

                // Send to Claude via Rig OAuthClient with streaming (ReAct loop enabled)
                let working_dir = self.working_directory.clone();
                let max_tool_rounds = self.max_tool_rounds;
                // Build conversation history (exclude the last user message we just added)
                let history: Vec<_> = self.messages.iter()
                    .take(self.messages.len().saturating_sub(2)) // Exclude the user message + streaming placeholder
                    .filter(|m| !m.is_streaming) // Exclude streaming messages
                    .cloned()
                    .collect();

                // Convert attachments to base64 for the API
                let image_data: Vec<(String, String)> = attachments.iter()
                    .map(|att| {
                        let base64_data = base64::engine::general_purpose::STANDARD.encode(&*att.data);
                        ("image/png".to_string(), base64_data)
                    })
                    .collect();

                let stream_task = Task::run(
                    llm_stream::run_rig_agent_stream(auth_token, system_prompt, user_message, model_name, working_dir, max_tool_rounds, history, image_data),
                    |event| event,
                );

                // Scroll to bottom immediately after sending
                let scroll_task = widget::operation::scroll_to(
                    widget::Id::new(CHAT_SCROLLABLE_ID),
                    AbsoluteOffset { x: 0.0, y: f32::MAX },
                );

                Task::batch([scroll_task, stream_task])
            }
            
            Message::CopyMessage(index) => {
                // Copy message content to clipboard
                if let Some(msg) = self.messages.get(index) {
                    let content = msg.content.clone();
                    return Task::perform(
                        async move {
                            use arboard::Clipboard;
                            match Clipboard::new() {
                                Ok(mut clipboard) => {
                                    if let Err(e) = clipboard.set_text(&content) {
                                        tracing::error!("Failed to copy to clipboard: {}", e);
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Failed to access clipboard: {}", e);
                                }
                            }
                        },
                        |_| Message::DismissError, // No-op message after clipboard operation
                    );
                }
                Task::none()
            }

            Message::ToggleRawView(index) => {
                // Toggle between markdown and raw text view
                if self.raw_view_messages.contains(&index) {
                    self.raw_view_messages.remove(&index);
                    self.raw_view_editors.remove(&index);
                } else {
                    // Create editor content from message
                    if let Some(msg) = self.messages.get(index) {
                        let content = text_editor::Content::with_text(&msg.content);
                        self.raw_view_editors.insert(index, content);
                    }
                    self.raw_view_messages.insert(index);
                }
                Task::none()
            }

            Message::RawViewEditorAction(index, action) => {
                // Handle text selection/copy in raw view editor
                if let Some(editor) = self.raw_view_editors.get_mut(&index) {
                    editor.perform(action);
                }
                Task::none()
            }

            Message::StreamChunk(chunk) => {
                if let Some(last) = self.messages.last_mut() {
                    if last.is_streaming {
                        // If previous content was a tool call, prefix with lightbulb
                        if last.last_was_tool_call && !chunk.trim().is_empty() {
                            last.content.push_str("\n\n💡 ");
                            last.last_was_tool_call = false;
                        }
                        last.content.push_str(&chunk);
                        // Update parsed markdown items for rendering
                        last.update_parsed_items();
                    }
                }
                self.scroll_to_bottom_if_needed()
            }

            Message::Reasoning(reasoning) => {
                if let Some(last) = self.messages.last_mut() {
                    if last.is_streaming {
                        // Append to existing reasoning or create new
                        if let Some(ref mut existing) = last.reasoning {
                            existing.push_str(&reasoning);
                        } else {
                            last.reasoning = Some(reasoning);
                        }
                    }
                }
                self.scroll_to_bottom_if_needed()
            }

            Message::StreamStats { chars_in_window, window_ms } => {
                // Update stats from the stream - this is emitted ~every second
                self.stream_chars_received = chars_in_window;
                self.stream_pulse = !self.stream_pulse;

                // Calculate tokens for this window: chars / 4
                if window_ms > 0 {
                    let seconds = window_ms as f64 / 1000.0;
                    let sample_tps = (chars_in_window as f64) / seconds / 4.0;

                    // Add to rolling window (keep last 60 samples = ~1 minute)
                    self.tps_samples.push_back(sample_tps);
                    while self.tps_samples.len() > 60 {
                        self.tps_samples.pop_front();
                    }

                    // Calculate rolling average
                    if !self.tps_samples.is_empty() {
                        let sum: f64 = self.tps_samples.iter().sum();
                        self.current_tps = sum / self.tps_samples.len() as f64;
                    }
                }
                Task::none()
            }

            Message::AnimationTick => {
                // Fast animation tick (~60 FPS) for smooth spinner
                self.spinner_frame = self.spinner_frame.wrapping_add(1);
                Task::none()
            }

            Message::PollStreamStats => {
                // Stream stats polling (formerly used fork-specific byte counter)
                // TODO: Could be reimplemented with custom byte tracking if needed
                self.stream_pulse = !self.stream_pulse;
                Task::none()
            }

            Message::StreamComplete => {
                self.is_streaming = false;
                // Reset streaming stats
                self.stream_start_time = None;
                self.stream_chars_received = 0;
                self.current_tps = 0.0;
                self.tps_samples.clear();
                self.last_bytes_time = None;
                self.spinner_frame = 0;

                if let Some(last) = self.messages.last_mut() {
                    if last.is_streaming {
                        last.is_streaming = false;
                        // Final parse of markdown after streaming completes
                        last.update_parsed_items();
                    }
                }
                // Auto-save session after streaming completes
                self.save_current_session();
                Task::none()
            }

            Message::StreamError(error) => {
                self.is_streaming = false;
                // Reset streaming stats
                self.stream_start_time = None;
                self.stream_chars_received = 0;
                self.current_tps = 0.0;
                self.tps_samples.clear();
                self.last_bytes_time = None;
                self.spinner_frame = 0;

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
                // Start OAuth flow in background
                Task::perform(
                    oauth_handler::start_oauth(provider),
                    move |result| Message::OAuthComplete(provider, result)
                )
            }
            
            Message::OAuthComplete(provider, result) => {
                match result {
                    Ok(()) => {
                        self.error_message = None;
                        // Update auth status after successful OAuth
                        self.provider_auth_status = check_provider_auth_status();
                        // Trigger model loading for the provider that just authenticated
                        return Task::done(Message::RefreshModelsForProvider(provider));
                    }
                    Err(e) => {
                        self.error_message = Some(e);
                    }
                }
                Task::none()
            }
            
            Message::NewSession => {
                self.messages.clear();
                self.raw_view_messages.clear();
                self.raw_view_editors.clear();
                self.messages.push(ChatMessage::assistant("New session started. How can I help you?"));
                self.current_session = None;
                Task::none()
            }
            
            Message::LoadSession(session_id) => {
                tracing::info!("Loading session: {}", session_id);
                if let Some(loaded) = session_manager::load_session(&session_id) {
                    self.messages = loaded.messages;
                    self.current_session = Some(loaded.session);

                    // Clear raw view state for new session
                    self.raw_view_messages.clear();
                    self.raw_view_editors.clear();

                    // Set agent type if it matches
                    if let Some(agent_type) = loaded.agent_type {
                        self.current_agent = agent_type;
                        self.agent_config = CoreAgentConfig::new(agent_type);
                    }

                    tracing::info!("Loaded session with {} messages", self.messages.len());
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

                // Get all available auth tokens
                let claude_token = get_claude_auth_token();
                let gemini_token = get_gemini_auth_token();
                let chatgpt_token = get_chatgpt_auth_token();
                let chatgpt_id_token = get_chatgpt_id_token();

                if claude_token.is_none() && gemini_token.is_none() && chatgpt_token.is_none() {
                    self.is_loading_models = false;
                    return Task::none();
                }

                // Fetch models from all authenticated providers
                Task::perform(
                    async move {
                        let mut all_models: Vec<String> = Vec::new();

                        // Fetch Claude models
                        if let Some(token) = claude_token {
                            let client = llm::ClaudeClient::new(token);
                            if let Ok(models) = client.fetch_latest_models().await {
                                all_models.extend(models);
                            }
                        }

                        // Fetch Gemini models
                        if let Some(token) = gemini_token {
                            use ticca_oauth::GeminiOAuth;
                            let oauth = GeminiOAuth::new();
                            match oauth.fetch_models(&token, None).await {
                                Ok(models) => all_models.extend(models.into_iter().map(|m| m.name)),
                                Err(e) => {
                                    tracing::warn!("Could not fetch Gemini models: {}", e);
                                    // Add default Gemini models
                                    all_models.extend(vec![
                                        "gemini-2.0-flash-exp".to_string(),
                                        "gemini-1.5-pro".to_string(),
                                        "gemini-1.5-flash".to_string(),
                                    ]);
                                }
                            }
                        }

                        // Fetch ChatGPT models
                        if let Some(token) = chatgpt_token {
                            use ticca_oauth::ChatGptOAuth;
                            let oauth = ChatGptOAuth::new();
                            match oauth.fetch_models(&token, chatgpt_id_token.as_deref()).await {
                                Ok(models) => all_models.extend(models.into_iter().map(|m| m.id)),
                                Err(e) => {
                                    tracing::warn!("Could not fetch ChatGPT models: {}", e);
                                    // Add default ChatGPT models
                                    all_models.extend(vec![
                                        "gpt-4o".to_string(),
                                        "gpt-4o-mini".to_string(),
                                        "o1-preview".to_string(),
                                    ]);
                                }
                            }
                        }

                        if all_models.is_empty() {
                            Err("No models found from any provider".to_string())
                        } else {
                            Ok(all_models)
                        }
                    },
                    Message::ModelsLoaded
                )
            }

            Message::RefreshModelsForProvider(provider) => {
                use crate::messages::OAuthProvider;

                if self.is_loading_models {
                    return Task::none();
                }
                self.is_loading_models = true;

                match provider {
                    OAuthProvider::Claude => {
                        let auth_token = match get_claude_auth_token() {
                            Some(token) => token,
                            None => {
                                self.is_loading_models = false;
                                return Task::none();
                            }
                        };
                        Task::perform(
                            async move {
                                let client = llm::ClaudeClient::new(auth_token);
                                client.fetch_latest_models().await
                                    .map_err(|e| e.to_string())
                            },
                            Message::ModelsLoaded
                        )
                    }
                    OAuthProvider::Gemini => {
                        let auth_token = match get_gemini_auth_token() {
                            Some(token) => token,
                            None => {
                                self.is_loading_models = false;
                                return Task::none();
                            }
                        };
                        Task::perform(
                            async move {
                                use ticca_oauth::GeminiOAuth;
                                let oauth = GeminiOAuth::new();
                                match oauth.fetch_models(&auth_token, None).await {
                                    Ok(models) => Ok(models.into_iter().map(|m| m.name).collect()),
                                    Err(e) => {
                                        // Fall back to hardcoded list of popular Gemini models
                                        tracing::warn!("Could not fetch Gemini models ({}), using defaults", e);
                                        Ok(vec![
                                            "gemini-2.0-flash-exp".to_string(),
                                            "gemini-1.5-pro".to_string(),
                                            "gemini-1.5-flash".to_string(),
                                            "gemini-1.0-pro".to_string(),
                                        ])
                                    }
                                }
                            },
                            Message::ModelsLoaded
                        )
                    }
                    OAuthProvider::ChatGpt => {
                        let auth_token = match get_chatgpt_auth_token() {
                            Some(token) => token,
                            None => {
                                self.is_loading_models = false;
                                return Task::none();
                            }
                        };
                        let id_token = get_chatgpt_id_token();
                        Task::perform(
                            async move {
                                use ticca_oauth::ChatGptOAuth;
                                let oauth = ChatGptOAuth::new();
                                match oauth.fetch_models(&auth_token, id_token.as_deref()).await {
                                    Ok(models) => Ok(models.into_iter().map(|m| m.id).collect()),
                                    Err(e) => {
                                        // OAuth token may not have model listing permissions
                                        // Fall back to hardcoded list of popular models
                                        tracing::warn!("Could not fetch ChatGPT models ({}), using defaults", e);
                                        Ok(vec![
                                            "gpt-4o".to_string(),
                                            "gpt-4o-mini".to_string(),
                                            "gpt-4-turbo".to_string(),
                                            "gpt-4".to_string(),
                                            "gpt-3.5-turbo".to_string(),
                                            "o1-preview".to_string(),
                                            "o1-mini".to_string(),
                                        ])
                                    }
                                }
                            },
                            Message::ModelsLoaded
                        )
                    }
                }
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

            Message::ToolCall { name, args } => {
                // Add a one-liner visual indicator for the tool call
                if let Some(last) = self.messages.last_mut() {
                    if last.is_streaming {
                        let tool_line = format_tool_call_oneliner(&name, &args);
                        // Use double newline for proper markdown paragraph break
                        last.content.push_str(&format!("\n\n{}", tool_line));
                        // Mark that we just added a tool call
                        last.last_was_tool_call = true;
                    }
                }
                Task::none()
            }

            Message::ToolResult { name: _, result: _ } => {
                // Don't count tool results - they're local execution, not LLM output
                // Don't display tool results - keep the UI clean
                Task::none()
            }

            Message::SelectWorkingDirectory => {
                // Open native directory picker dialog
                Task::perform(
                    async {
                        let dialog = rfd::AsyncFileDialog::new()
                            .set_title("Select Working Directory")
                            .pick_folder()
                            .await;

                        dialog.map(|handle| handle.path().to_path_buf())
                    },
                    |result| {
                        match result {
                            Some(path) => Message::WorkingDirectoryChanged(path),
                            None => Message::DismissError, // User cancelled, do nothing
                        }
                    }
                )
            }

            Message::WorkingDirectoryChanged(path) => {
                self.working_directory = path;
                Task::none()
            }

            Message::SelectImageFile => {
                // Open native file picker for images (Wayland-compatible via xdg-portal)
                Task::perform(
                    async {
                        let dialog = rfd::AsyncFileDialog::new()
                            .set_title("Select Image")
                            .add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp", "bmp"])
                            .pick_file()
                            .await;

                        match dialog {
                            Some(handle) => {
                                let path = handle.path().to_path_buf();
                                image_handler::load_image_from_path(&path).await
                            }
                            None => Err("No file selected".to_string()),
                        }
                    },
                    Message::ImageLoaded
                )
            }

            Message::FileDropped(path) => {
                // Load image from dropped file (X11 only - Wayland DnD not implemented in winit)
                Task::perform(
                    async move {
                        image_handler::load_image_from_path(&path).await
                    },
                    Message::ImageLoaded
                )
            }

            Message::ImageLoaded(result) => {
                match result {
                    Ok(attachment) => {
                        self.pending_attachments.push(attachment);
                    }
                    Err(e) => {
                        // Don't show error for cancelled file dialog
                        if !e.contains("No file selected") {
                            self.error_message = Some(format!("Failed to load image: {}", e));
                        }
                    }
                }
                Task::none()
            }

            Message::PasteImage => {
                // Try to paste image from clipboard
                Task::perform(
                    async {
                        image_handler::paste_image_from_clipboard().await
                    },
                    Message::ImagePasted
                )
            }

            Message::ImagePasted(result) => {
                match result {
                    Ok(attachment) => {
                        self.pending_attachments.push(attachment);
                    }
                    Err(e) => {
                        // Silently ignore if no image in clipboard (user might have pressed Ctrl+V for text)
                        if !e.contains("No image") {
                            self.error_message = Some(format!("Failed to paste image: {}", e));
                        }
                    }
                }
                Task::none()
            }

            Message::RemoveAttachment(index) => {
                if index < self.pending_attachments.len() {
                    self.pending_attachments.remove(index);
                }
                Task::none()
            }

            Message::LinkClicked(url) => {
                // Open the URL in the default browser
                if let Err(e) = open::that(url.as_str()) {
                    tracing::warn!("Failed to open URL {}: {}", url, e);
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
    
    /// Get subscriptions (keyboard shortcuts, file drop events, and streaming stats timer)
    pub fn subscription(&self) -> Subscription<Message> {
        use iced::time;

        let keybindings = crate::keybindings::subscription();

        // Add timer subscriptions while streaming
        if self.is_streaming {
            // Stats polling every 1 second
            let stats_timer = time::every(std::time::Duration::from_secs(1))
                .map(|_| Message::PollStreamStats);

            // Fast animation timer (~60 FPS) for smooth spinner when waiting
            let is_waiting = self.last_bytes_time
                .map(|t| t.elapsed().as_secs() >= 2)
                .unwrap_or(false);

            if is_waiting {
                let animation_timer = time::every(std::time::Duration::from_millis(16))
                    .map(|_| Message::AnimationTick);
                Subscription::batch([keybindings, stats_timer, animation_timer])
            } else {
                Subscription::batch([keybindings, stats_timer])
            }
        } else {
            keybindings
        }
    }
    
    /// Render the chat view
    fn view_chat(&self) -> Element<'_, Message> {
        // Calculate seconds since last bytes received (for "waiting" indicator)
        let secs_since_bytes = self.last_bytes_time
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0);

        crate::views::chat::view(
            self.current_agent,
            &self.working_directory,
            &self.messages,
            &self.pending_attachments,
            &self.input_value,
            self.is_streaming,
            self.theme,
            &self.raw_view_messages,
            &self.raw_view_editors,
            self.stream_chars_received,
            self.current_tps,
            self.stream_pulse,
            secs_since_bytes,
            self.spinner_frame,
        )
    }

    /// Scroll to bottom of chat if user was at bottom
    fn scroll_to_bottom_if_needed(&self) -> Task<Message> {
        if self.user_at_bottom {
            widget::operation::scroll_to(
                widget::Id::new(CHAT_SCROLLABLE_ID),
                AbsoluteOffset { x: 0.0, y: f32::MAX },
            )
        } else {
            Task::none()
        }
    }

    /// Save the current session to the database
    fn save_current_session(&mut self) {
        if let Some(session) = session_manager::save_session(
            self.current_session.as_ref(),
            &self.messages,
            self.current_agent,
        ) {
            self.current_session = Some(session);
        }
    }
    
    /// Render the settings view
    fn view_settings(&self) -> Element<'_, Message> {
        crate::views::config::view(
            self.theme,
            &self.available_models,
            self.default_model.as_deref(),
            &self.agent_pinned_models,
            self.is_loading_models,
            &self.provider_auth_status,
        )
    }
}

impl Default for TiccaApp {
    fn default() -> Self {
        Self::new().0
    }
}
