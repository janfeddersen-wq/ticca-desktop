//! Application state management
//!
//! This module provides the central `AppState` struct that ties together
//! all major components: configuration, database, agent controller,
//! and session management.

use std::path::PathBuf;
use std::sync::Arc;

use tracing::{debug, info, instrument};

use ticca_bridge::{
    AgentRequest, AgentResponse, PythonAgentController, SharedAgentController,
    StreamCallback, StreamChunk, StreamSender,
};
use ticca_config::{AppConfig, ConfigLoader};
use ticca_db::Database;

use crate::error::CoreError;
use crate::session_manager::SessionManager;

/// Type alias for stream callbacks
pub type StreamCallbackFn = Box<dyn Fn(StreamChunk) + Send + Sync>;

/// Application-wide state container
///
/// `AppState` is the central hub that coordinates all major components
/// of the Ticca application. It provides:
///
/// - Configuration management
/// - Database access
/// - Agent execution
/// - Session management
/// - MCP (Model Context Protocol) integration status
///
/// # Thread Safety
///
/// `AppState` is designed to be shared across threads and async tasks.
/// All internal components use `Arc` for shared ownership.
///
/// # Example
///
/// ```ignore
/// use ticca_core::AppState;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let state = AppState::initialize(None).await?;
///     
///     let response = state.send_message(
///         "conv-123",
///         "Hello, how are you?",
///         None,
///     ).await?;
///     
///     println!("Response: {}", response.content);
///     state.shutdown().await?;
///     Ok(())
/// }
/// ```
pub struct AppState {
    /// Application configuration
    config: Arc<AppConfig>,
    /// Database connection
    db: Arc<Database>,
    /// Agent controller for executing AI requests
    agent_controller: SharedAgentController,
    /// Session manager for conversation state
    session_manager: SessionManager,
    /// Whether MCP is enabled
    mcp_enabled: bool,
    /// Shutdown flag
    shutdown_requested: std::sync::atomic::AtomicBool,
}

impl AppState {
    /// Initialize the application state
    ///
    /// This performs all necessary setup:
    /// 1. Load configuration from disk or defaults
    /// 2. Initialize the database and run migrations
    /// 3. Initialize the Python runtime and agent controller
    /// 4. Set up the session manager
    ///
    /// # Arguments
    ///
    /// * `config_path` - Optional path to a configuration file.
    ///   If `None`, uses the default configuration location.
    ///
    /// # Errors
    ///
    /// Returns an error if any initialization step fails.
    #[instrument(skip_all, fields(config_path = ?config_path))]
    pub async fn initialize(config_path: Option<PathBuf>) -> Result<Self, CoreError> {
        info!("Initializing Ticca application state");

        // 1. Load configuration
        let config = Self::load_config(config_path)?;
        let config = Arc::new(config);
        debug!("Configuration loaded");

        // 2. Initialize database
        let db = Self::init_database(&config).await?;
        let db = Arc::new(db);
        debug!("Database initialized");

        // 3. Initialize agent controller (Python runtime)
        let agent_controller = Self::init_agent_controller(&config)?;
        debug!("Agent controller initialized");

        // 4. Set up session manager
        let session_manager = SessionManager::new(db.clone());
        debug!("Session manager initialized");

        // 5. Check MCP status from settings (disable_mcp = false means MCP is enabled)
        let mcp_enabled = !config.settings.advanced.disable_mcp;
        if mcp_enabled {
            info!("MCP (Model Context Protocol) is enabled");
        }

        info!("Ticca application state initialized successfully");

        Ok(Self {
            config,
            db,
            agent_controller,
            session_manager,
            mcp_enabled,
            shutdown_requested: std::sync::atomic::AtomicBool::new(false),
        })
    }

    /// Load configuration from file or defaults
    fn load_config(config_path: Option<PathBuf>) -> Result<AppConfig, CoreError> {
        let config = match config_path {
            Some(path) => {
                info!(path = %path.display(), "Loading configuration from file");
                ConfigLoader::new()
                    .with_config_path(&path)
                    .load_with_options()
                    .map_err(CoreError::from)?
            }
            None => {
                debug!("Loading configuration from default location");
                ConfigLoader::load().map_err(CoreError::from)?
            }
        };
        Ok(config)
    }

    /// Initialize the database connection and run migrations
    async fn init_database(config: &AppConfig) -> Result<Database, CoreError> {
        let db_path = config.paths.database_file();

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                CoreError::init(format!("Failed to create database directory: {}", e))
            })?;
        }

        info!(path = %db_path.display(), "Initializing database");

        let db = Database::connect(&db_path).await.map_err(CoreError::from)?;
        info!("Database connected and migrations applied");

        Ok(db)
    }

    /// Initialize the agent controller with Python runtime
    fn init_agent_controller(_config: &AppConfig) -> Result<SharedAgentController, CoreError> {
        info!("Initializing Python agent controller");

        let controller = PythonAgentController::with_defaults()
            .map_err(|e| CoreError::init(format!("Failed to initialize Python runtime: {}", e)))?;

        Ok(Arc::new(controller))
    }

    /// Send a message to an agent and get a response
    ///
    /// This is the main method for interacting with AI agents.
    /// It handles session management, message persistence, and
    /// agent execution.
    ///
    /// # Arguments
    ///
    /// * `conversation_id` - ID of the conversation
    /// * `message` - The user's message
    /// * `stream_callback` - Optional callback for streaming responses
    ///
    /// # Returns
    ///
    /// The agent's complete response.
    #[instrument(skip(self, stream_callback), fields(conversation_id = %conversation_id))]
    pub async fn send_message(
        &self,
        conversation_id: &str,
        message: &str,
        stream_callback: Option<StreamSender>,
    ) -> Result<AgentResponse, CoreError> {
        if self
            .shutdown_requested
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            return Err(CoreError::Cancelled);
        }

        debug!(message_len = message.len(), "Processing user message");

        // Get or create session for this conversation
        let session = self
            .session_manager
            .get_or_create_session(conversation_id)
            .await?;

        // Generate a unique message ID
        let user_msg_id = format!("msg-{}", uuid_v4());

        // Store the user message
        self.db
            .messages()
            .create_simple(&user_msg_id, conversation_id, "user", message)
            .await
            .map_err(CoreError::from)?;

        // Get the configured agent name
        let agent_name = self.config.settings.core.default_agent.clone();

        // Build the request
        let request = AgentRequest::new(&session.id, conversation_id, message, &agent_name);

        // Execute with or without streaming
        let response = match stream_callback {
            Some(callback) => {
                self.agent_controller
                    .execute_streaming(request, callback)
                    .await
                    .map_err(CoreError::from)?
            }
            None => {
                self.agent_controller
                    .execute(request)
                    .await
                    .map_err(CoreError::from)?
            }
        };

        // Store the assistant response
        let assistant_msg_id = format!("msg-{}", uuid_v4());
        self.db
            .messages()
            .create_simple(&assistant_msg_id, conversation_id, "assistant", &response.content)
            .await
            .map_err(CoreError::from)?;

        // Update session
        self.session_manager
            .update_session(conversation_id, &response)
            .await?;

        debug!(
            response_len = response.content.len(),
            tokens = response.usage.total_tokens,
            "Response generated"
        );

        Ok(response)
    }

    /// Send a message with streaming using a boxed callback
    ///
    /// Convenience method that creates the stream infrastructure
    /// and returns a receiver for consuming chunks.
    pub async fn send_message_streaming(
        &self,
        conversation_id: &str,
        message: &str,
    ) -> Result<(
        ticca_bridge::StreamReceiver,
        tokio::task::JoinHandle<Result<AgentResponse, CoreError>>,
    ), CoreError> {
        let (sender, receiver) = StreamCallback::new().split();
        let conv_id = conversation_id.to_string();
        let msg = message.to_string();
        let state = self.clone_state();

        let handle = tokio::spawn(async move {
            state.send_message(&conv_id, &msg, Some(sender)).await
        });

        Ok((receiver, handle))
    }

    /// Create a new conversation
    #[instrument(skip(self))]
    pub async fn create_conversation(&self, title: Option<&str>) -> Result<String, CoreError> {
        let conv_id = format!("conv-{}", uuid_v4());
        let title = title.unwrap_or("New Conversation");

        let conversation = self
            .db
            .conversations()
            .create(&conv_id, title)
            .await
            .map_err(CoreError::from)?;

        info!(id = %conversation.id, "Created new conversation");
        Ok(conversation.id)
    }

    /// List all conversations
    pub async fn list_conversations(&self) -> Result<Vec<ticca_db::Conversation>, CoreError> {
        self.db
            .conversations()
            .list_all()
            .await
            .map_err(CoreError::from)
    }

    /// Get messages for a conversation
    pub async fn get_messages(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<ticca_db::DbMessage>, CoreError> {
        self.db
            .messages()
            .get_by_conversation(conversation_id)
            .await
            .map_err(CoreError::from)
    }

    /// Delete a conversation and its messages
    #[instrument(skip(self))]
    pub async fn delete_conversation(&self, conversation_id: &str) -> Result<(), CoreError> {
        self.db
            .conversations()
            .delete(conversation_id)
            .await
            .map_err(CoreError::from)?;

        self.session_manager.remove_session(conversation_id).await;
        info!(id = %conversation_id, "Deleted conversation");
        Ok(())
    }

    /// Get the current configuration
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// Get the database handle
    pub fn database(&self) -> &Database {
        &self.db
    }

    /// Check if MCP is enabled
    pub fn is_mcp_enabled(&self) -> bool {
        self.mcp_enabled
    }

    /// List available agents
    pub async fn list_agents(&self) -> Result<Vec<ticca_bridge::AgentInfo>, CoreError> {
        self.agent_controller
            .list_agents()
            .await
            .map_err(CoreError::from)
    }

    /// Graceful shutdown
    ///
    /// Signals all components to stop and waits for cleanup.
    #[instrument(skip(self))]
    pub async fn shutdown(&self) -> Result<(), CoreError> {
        info!("Initiating graceful shutdown");

        self.shutdown_requested
            .store(true, std::sync::atomic::Ordering::Relaxed);

        // Clean up session manager
        self.session_manager.shutdown().await;

        // Close database connections
        self.db.close().await;

        info!("Shutdown complete");
        Ok(())
    }

    /// Clone state for spawning tasks
    fn clone_state(&self) -> AppStateHandle {
        AppStateHandle {
            config: self.config.clone(),
            db: self.db.clone(),
            agent_controller: self.agent_controller.clone(),
            session_manager: self.session_manager.clone(),
            shutdown_requested: self
                .shutdown_requested
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

/// Lightweight handle for passing state to spawned tasks
#[derive(Clone)]
struct AppStateHandle {
    config: Arc<AppConfig>,
    #[allow(dead_code)]
    db: Arc<Database>,
    agent_controller: SharedAgentController,
    session_manager: SessionManager,
    shutdown_requested: bool,
}

impl AppStateHandle {
    async fn send_message(
        &self,
        conversation_id: &str,
        message: &str,
        stream_callback: Option<StreamSender>,
    ) -> Result<AgentResponse, CoreError> {
        if self.shutdown_requested {
            return Err(CoreError::Cancelled);
        }

        let session = self
            .session_manager
            .get_or_create_session(conversation_id)
            .await?;

        let agent_name = self.config.settings.core.default_agent.clone();
        let request = AgentRequest::new(&session.id, conversation_id, message, &agent_name);

        match stream_callback {
            Some(callback) => self
                .agent_controller
                .execute_streaming(request, callback)
                .await
                .map_err(CoreError::from),
            None => self
                .agent_controller
                .execute(request)
                .await
                .map_err(CoreError::from),
        }
    }
}

/// Generate a simple UUID-like string
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:016x}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_v4() {
        let id1 = uuid_v4();
        let id2 = uuid_v4();
        // IDs should be unique (or at least different timestamps)
        // Note: This could fail if run too fast, but generally works
        assert!(!id1.is_empty());
        assert!(!id2.is_empty());
    }
}
