//! Iced Application state and main loop

use iced::widget::{button, column, container, row, scrollable, text, text_editor, text_input, markdown, Column, Space};
use iced::widget::scrollable::AbsoluteOffset;
use iced::{Element, Length, Subscription, Task, Theme, widget};

/// Create horizontal space that fills available width (iced 0.14 helper)
fn horizontal_space() -> Space {
    Space::new().width(Length::Fill)
}

/// Format a tool call as a concise one-liner for display
fn format_tool_call_oneliner(name: &str, args: &str) -> String {
    // Try to parse the args as JSON to extract relevant fields
    let parsed: serde_json::Value = serde_json::from_str(args).unwrap_or(serde_json::Value::Null);

    let param_display = match name {
        "list_files" => {
            // Show directory, limited to 80 chars
            if let Some(dir) = parsed.get("directory").or(parsed.get("path")).and_then(|v| v.as_str()) {
                let display = if dir.len() > 80 {
                    format!("...{}", &dir[dir.len()-77..])
                } else {
                    dir.to_string()
                };
                format!("Directory: {}", display)
            } else {
                String::new()
            }
        }
        "read_file" => {
            // Show just the path
            if let Some(path) = parsed.get("path").and_then(|v| v.as_str()) {
                let display = if path.len() > 80 {
                    format!("...{}", &path[path.len()-77..])
                } else {
                    path.to_string()
                };
                display
            } else {
                String::new()
            }
        }
        "edit_file" => {
            // Show just the filename
            if let Some(path) = parsed.get("path").and_then(|v| v.as_str()) {
                std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(path)
                    .to_string()
            } else {
                String::new()
            }
        }
        "write_file" => {
            // Show just the filename
            if let Some(path) = parsed.get("path").and_then(|v| v.as_str()) {
                std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(path)
                    .to_string()
            } else {
                String::new()
            }
        }
        "shell" => {
            // Show command, limited to 80 chars
            if let Some(cmd) = parsed.get("command").and_then(|v| v.as_str()) {
                let display = if cmd.len() > 80 {
                    format!("{}...", &cmd[..77])
                } else {
                    cmd.to_string()
                };
                format!("`{}`", display)
            } else {
                String::new()
            }
        }
        "grep" => {
            // Show pattern and optionally path
            let pattern = parsed.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
            let path = parsed.get("path").and_then(|v| v.as_str());
            let mut display = format!("\"{}\"", pattern);
            if let Some(p) = path {
                let short_path = if p.len() > 40 {
                    format!("...{}", &p[p.len()-37..])
                } else {
                    p.to_string()
                };
                display.push_str(&format!(" in {}", short_path));
            }
            if display.len() > 80 {
                format!("{}...", &display[..77])
            } else {
                display
            }
        }
        _ => {
            // Generic: show first key-value pair, limited
            if let Some(obj) = parsed.as_object() {
                if let Some((key, val)) = obj.iter().next() {
                    let val_str = match val {
                        serde_json::Value::String(s) => s.clone(),
                        _ => val.to_string(),
                    };
                    let display = format!("{}: {}", key, val_str);
                    if display.len() > 80 {
                        format!("{}...", &display[..77])
                    } else {
                        display
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        }
    };

    if param_display.is_empty() {
        format!("🔧 **{}**", name)
    } else {
        format!("🔧 **{}** {}", name, param_display)
    }
}

/// ID for the chat messages scrollable
const CHAT_SCROLLABLE_ID: &str = "chat_messages";

use crate::image_handler;
use crate::llm_stream::{self, get_claude_auth_token};
use crate::material_icons::{self as mi, icon, icons};
use crate::messages::{Message, ImageAttachment};
use crate::theme::{styles, AppTheme};

use ticca_core::agents::{AgentConfig as CoreAgentConfig, AgentType, get_agent};
use ticca_core::config::{ConfigDatabase, setting_keys};
use ticca_core::llm;
use ticca_core::session::{Session, SessionDatabase, SessionMessage, MessageRole};
use ticca_core::OAuthToken;
use ticca_oauth::{ClaudeOAuth, ChatGptOAuth};

use uuid::Uuid;
use chrono::{Local, Utc};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use base64::Engine as _;

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

    // Auto-scroll tracking: true if user is at/near bottom of chat
    user_at_bottom: bool,
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
    /// Reasoning/thinking content (collapsible)
    pub reasoning: Option<String>,
    /// Parsed markdown items (cached for rendering)
    pub parsed_items: Vec<markdown::Item>,
    /// Track if last content added was a tool call (for formatting)
    pub last_was_tool_call: bool,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        let content = content.into();
        let parsed_items = markdown::parse(&content).collect();
        Self {
            role: MessageRole::User,
            content,
            is_streaming: false,
            reasoning: None,
            parsed_items,
            last_was_tool_call: false,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        let content = content.into();
        let parsed_items = markdown::parse(&content).collect();
        Self {
            role: MessageRole::Assistant,
            content,
            is_streaming: false,
            reasoning: None,
            parsed_items,
            last_was_tool_call: false,
        }
    }

    pub fn assistant_streaming() -> Self {
        Self {
            role: MessageRole::Assistant,
            content: String::new(),
            is_streaming: true,
            reasoning: None,
            parsed_items: Vec::new(),
            last_was_tool_call: false,
        }
    }

    /// Update parsed items when content changes
    pub fn update_parsed_items(&mut self) {
        self.parsed_items = markdown::parse(&self.content).collect();
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
            user_at_bottom: true, // Start at bottom
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
                    .take(self.messages.len().saturating_sub(1)) // Exclude the message we just added
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

            Message::StreamComplete => {
                self.is_streaming = false;
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
                self.raw_view_messages.clear();
                self.raw_view_editors.clear();
                self.messages.push(ChatMessage::assistant("New session started. How can I help you?"));
                self.current_session = None;
                Task::none()
            }
            
            Message::LoadSession(session_id) => {
                tracing::info!("Loading session: {}", session_id);
                if let Ok(db) = SessionDatabase::open() {
                    if let Ok(Some(session)) = db.get_session(&session_id) {
                        if let Ok(messages) = db.get_messages(&session_id) {
                            // Convert session messages to chat messages
                            self.messages = messages.iter()
                                .map(|m| {
                                    let parsed_items = markdown::parse(&m.content).collect();
                                    ChatMessage {
                                        role: m.role.clone(),
                                        content: m.content.clone(),
                                        is_streaming: false,
                                        reasoning: None,
                                        parsed_items,
                                        last_was_tool_call: false,
                                    }
                                })
                                .collect();

                            // Clear raw view state for new session
                            self.raw_view_messages.clear();
                            self.raw_view_editors.clear();

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
    
    /// Get subscriptions (keyboard shortcuts and file drop events)
    pub fn subscription(&self) -> Subscription<Message> {
        use iced::event::{self, Event};
        use iced::keyboard;
        use iced::window;

        event::listen_with(|event, _status, _id| {
            match event {
                // Handle file drops for drag & drop images (X11 only - not implemented on Wayland)
                Event::Window(window::Event::FileDropped(path)) => {
                    if image_handler::is_image_file(&path) {
                        Some(Message::FileDropped(path))
                    } else {
                        None
                    }
                }
                Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                    let bindings = crate::keybindings::keybindings();

                    // Ctrl+V to paste image from clipboard
                    if modifiers.command() {
                        if let iced::keyboard::Key::Character(c) = &key {
                            if c.as_str() == "v" {
                                return Some(Message::PasteImage);
                            }
                        }
                    }

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
                            icon(icons::CODE).size(14),
                            text(" Coding").size(14),
                        ]
                        .spacing(4)
                    )
                    .on_press(Message::SwitchAgent(AgentType::Coding))
                    .style(move |theme, status| styles::tab_button(theme, status, is_coding))
                    .padding([8, 12]),
                    button(
                        row![
                            icon(icons::CHECKLIST).size(14),
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
                horizontal_space(),

                // Actions
                row![
                    button(icon(icons::CONTRAST).size(18))
                        .on_press(Message::ThemeToggle)
                        .style(styles::icon_button)
                        .padding(8),
                    button(icon(icons::SETTINGS).size(18))
                        .on_press(Message::OpenSettings)
                        .style(styles::icon_button)
                        .padding(8),
                    button(icon(icons::ADD).size(18))
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

        // Working directory selector bar
        let dir_display = self.working_directory.to_string_lossy();
        let dir_bar = container(
            row![
                icon(icons::FOLDER).size(16),
                text(format!(" {}", dir_display)).size(12),
                horizontal_space(),
                button(text("Change").size(12))
                    .on_press(Message::SelectWorkingDirectory)
                    .style(styles::secondary_button)
                    .padding([4, 8]),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
        )
        .padding([6, 12])
        .style(styles::dir_bar_container);

        // Message list
        let message_widgets: Vec<Element<Message>> = self.messages
            .iter()
            .enumerate()
            .map(|(idx, msg)| self.render_message(idx, msg))
            .collect();

        let messages: Element<Message> = scrollable(
            Column::with_children(message_widgets)
                .spacing(12)
                .padding(20)
        )
        .id(widget::Id::new(CHAT_SCROLLABLE_ID))
        .on_scroll(Message::ChatScrolled)
        .height(Length::Fill)
        .into();

        // Build attachment previews if any
        let attachment_preview: Option<Element<Message>> = if self.pending_attachments.is_empty() {
            None
        } else {
            let previews: Vec<Element<Message>> = self.pending_attachments
                .iter()
                .enumerate()
                .map(|(idx, attachment)| {
                    // Create thumbnail from PNG data
                    let handle = iced::widget::image::Handle::from_bytes((*attachment.data).clone());
                    let thumbnail = iced::widget::image(handle)
                        .width(Length::Fixed(60.0))
                        .height(Length::Fixed(60.0));

                    let filename = attachment.filename.as_deref().unwrap_or("image");
                    let size_kb = attachment.data.len() / 1024;

                    container(
                        column![
                            // Thumbnail with remove button overlay
                            iced::widget::stack![
                                container(thumbnail)
                                    .style(styles::image_thumbnail_container),
                                container(
                                    button(icon(icons::CLOSE).size(12))
                                        .on_press(Message::RemoveAttachment(idx))
                                        .style(styles::remove_attachment_button)
                                        .padding(2)
                                )
                                .align_x(iced::alignment::Horizontal::Right)
                                .width(Length::Fill),
                            ]
                            .width(Length::Fixed(60.0))
                            .height(Length::Fixed(60.0)),
                            // Filename and size
                            text(format!("{}KB", size_kb)).size(10),
                        ]
                        .spacing(2)
                        .align_x(iced::Alignment::Center)
                    )
                    .padding(4)
                    .into()
                })
                .collect();

            Some(
                container(
                    row![
                        icon(icons::ATTACH_FILE).size(14),
                        iced::widget::Row::with_children(previews).spacing(8),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                )
                .padding([8, 12])
                .width(Length::Fill)
                .style(styles::attachment_bar_container)
                .into()
            )
        };

        // Check if we can send (has text or attachments)
        let can_send = !self.is_streaming &&
            (!self.input_value.trim().is_empty() || !self.pending_attachments.is_empty());

        // Input area
        let input_row = row![
            // Add image button (works on Wayland via xdg-portal)
            button(icon(icons::ATTACH_FILE).size(20))
                .on_press(Message::SelectImageFile)
                .style(styles::icon_button)
                .padding([8, 8]),
            text_input("Type a message...", &self.input_value)
                .on_input(Message::InputChanged)
                .on_submit(Message::SendMessage)
                .style(styles::text_input_style)
                .padding(12)
                .size(14)
                .width(Length::Fill),
            button(
                if self.is_streaming {
                    icon(icons::HOURGLASS_EMPTY).size(20)
                } else {
                    icon(icons::ARROW_UPWARD).size(20)
                }
            )
            .on_press_maybe(if can_send { Some(Message::SendMessage) } else { None })
            .style(styles::send_button)
            .padding([8, 8]),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);

        let input_content: Element<'_, Message> = if let Some(preview) = attachment_preview {
            column![preview, input_row].spacing(0).into()
        } else {
            input_row.into()
        };
        let input = container(input_content)
        .padding(12)
        .style(styles::input_area_container);

        column![
            header,
            dir_bar,
            messages,
            input,
        ]
        .into()
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
    fn render_message<'a>(&'a self, index: usize, msg: &'a ChatMessage) -> Element<'a, Message> {
        let is_user = msg.role == MessageRole::User;
        let is_dark = matches!(self.theme, AppTheme::Dark);
        let is_raw_view = self.raw_view_messages.contains(&index);

        let label = match msg.role {
            MessageRole::User => "You",
            MessageRole::Assistant => "Assistant",
            MessageRole::System => "System",
            MessageRole::Tool => "Tool",
        };

        let content: Element<Message> = if msg.is_streaming && msg.content.is_empty() {
            row![
                icon(icons::PENDING).size(16),
                text(" Thinking...").size(14),
            ]
            .spacing(6)
            .into()
        } else if msg.is_streaming {
            // Render markdown while streaming - use parsed items with cursor
            // Note: For streaming, we use the cached items since they're updated on each chunk
            markdown::view(&msg.parsed_items, markdown::Settings::with_text_size(14, self.theme.to_iced_theme()))
                .map(Message::LinkClicked)
                .into()
        } else if is_raw_view {
            // Raw view: show selectable plain text
            if let Some(editor_content) = self.raw_view_editors.get(&index) {
                text_editor(editor_content)
                    .on_action(move |action| Message::RawViewEditorAction(index, action))
                    .style(move |theme, _status| styles::raw_text_editor(theme, is_dark))
                    .into()
            } else {
                // Fallback if editor not yet created
                text(&msg.content).size(14).into()
            }
        } else {
            // Render markdown for completed messages using built-in renderer
            markdown::view(&msg.parsed_items, markdown::Settings::with_text_size(14, self.theme.to_iced_theme()))
                .map(Message::LinkClicked)
                .into()
        };

        // Toggle icon: CODE for raw view, DESCRIPTION for markdown view
        let toggle_icon = if is_raw_view { icons::DESCRIPTION } else { icons::CODE };

        // Header row with label, toggle button, and copy button
        let header = row![
            text(label).size(12),
            horizontal_space(),
            // Raw/Markdown toggle button
            button(icon(toggle_icon).size(16))
                .on_press(Message::ToggleRawView(index))
                .style(styles::icon_button)
                .padding([4, 6]),
            // Copy button
            button(icon(icons::CONTENT_COPY).size(16))
                .on_press(Message::CopyMessage(index))
                .style(styles::icon_button)
                .padding([4, 6]),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center);

        // Build the message column
        let mut msg_column = column![header].spacing(6);

        // Add reasoning section if present
        if let Some(ref reasoning) = msg.reasoning {
            let reasoning_content = container(
                column![
                    row![
                        icon(icons::PSYCHOLOGY).size(14),
                        text(" Thinking").size(12),
                    ]
                    .spacing(4),
                    container(
                        text(reasoning).size(12)
                    )
                    .padding([4, 8])
                ]
                .spacing(4)
            )
            .padding(8)
            .width(Length::Fill)
            .style(move |theme| styles::reasoning_container(theme, is_dark));

            msg_column = msg_column.push(reasoning_content);
        }

        msg_column = msg_column.push(content);

        container(msg_column)
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

/// Configuration loaded from database
struct AppConfig {
    theme: AppTheme,
    default_model: Option<String>,
    agent_pinned_models: HashMap<AgentType, String>,
    max_tool_rounds: u32,
}

/// Load configuration from database
fn load_config() -> AppConfig {
    use ticca_core::config::defaults;

    let db = match ConfigDatabase::open() {
        Ok(db) => db,
        Err(_) => return AppConfig {
            theme: AppTheme::Dark,
            default_model: None,
            agent_pinned_models: HashMap::new(),
            max_tool_rounds: defaults::MAX_TOOL_ROUNDS,
        },
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

    let max_tool_rounds = db.get_setting(setting_keys::MAX_TOOL_ROUNDS)
        .ok()
        .flatten()
        .and_then(|s| s.value.parse().ok())
        .unwrap_or(defaults::MAX_TOOL_ROUNDS);

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

    AppConfig {
        theme,
        default_model,
        agent_pinned_models,
        max_tool_rounds,
    }
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
    iced::application(TiccaApp::new, TiccaApp::update, TiccaApp::view)
        .title(TiccaApp::title)
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
        // Material Icons font (replaces Bootstrap Icons)
        .font(mi::FONT_BYTES)
        // Set Noto Sans as the default font for consistent rendering
        .default_font(iced::Font::with_name("Noto Sans"))
        .window_size(iced::Size::new(900.0, 700.0))
        .antialiasing(true)
        .run()?;
    Ok(())
}
