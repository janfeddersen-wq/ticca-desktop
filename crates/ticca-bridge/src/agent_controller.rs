//! Agent controller trait and related abstractions
//!
//! This module defines the `AgentController` trait which provides
//! the interface for executing agent requests, both synchronously
//! and with streaming support.

use async_trait::async_trait;

use crate::callback::StreamSender;
use crate::error::BridgeResult;
use crate::types::{AgentInfo, AgentRequest, AgentResponse};

/// Controller for executing agent requests
///
/// This trait defines the interface for agent execution, supporting
/// both single-response and streaming modes. Implementations may
/// wrap Python agents, mock agents for testing, or any other backend.
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync` to allow usage across
/// threads and in async contexts.
///
/// # Examples
///
/// ```ignore
/// use ticca_bridge::{AgentController, AgentRequest};
///
/// async fn run_agent(controller: &impl AgentController) {
///     let request = AgentRequest::new(
///         "session-1",
///         "conv-1", 
///         "Hello!",
///         "default"
///     );
///     
///     let response = controller.execute(request).await?;
///     println!("Response: {}", response.content);
/// }
/// ```
#[async_trait]
pub trait AgentController: Send + Sync {
    /// Execute an agent request and wait for the complete response
    ///
    /// This method blocks until the agent has fully processed the request
    /// and returns the complete response. For long-running requests,
    /// consider using `execute_streaming` instead.
    ///
    /// # Arguments
    ///
    /// * `request` - The agent request containing message and configuration
    ///
    /// # Returns
    ///
    /// The complete agent response, or an error if execution failed.
    async fn execute(&self, request: AgentRequest) -> BridgeResult<AgentResponse>;

    /// Execute an agent request with streaming updates
    ///
    /// This method sends incremental updates through the callback
    /// as the agent processes the request. The final response is
    /// returned when complete.
    ///
    /// # Arguments
    ///
    /// * `request` - The agent request containing message and configuration
    /// * `callback` - Sender for streaming updates
    ///
    /// # Returns
    ///
    /// The complete agent response, or an error if execution failed.
    /// Note that even if streaming succeeds, the final response may
    /// indicate an error through its `finish_reason`.
    async fn execute_streaming(
        &self,
        request: AgentRequest,
        callback: StreamSender,
    ) -> BridgeResult<AgentResponse>;

    /// List all available agents
    ///
    /// Returns information about all agents that can be invoked
    /// through this controller.
    async fn list_agents(&self) -> BridgeResult<Vec<AgentInfo>>;

    /// Get information about a specific agent
    ///
    /// # Arguments
    ///
    /// * `name` - The name or ID of the agent to look up
    ///
    /// # Returns
    ///
    /// Agent information if found, `None` otherwise.
    async fn get_agent_info(&self, name: &str) -> BridgeResult<Option<AgentInfo>>;

    /// Check if the controller is ready to accept requests
    ///
    /// This can be used to verify that the underlying runtime
    /// (e.g., Python) is properly initialized.
    fn is_ready(&self) -> bool {
        true
    }

    /// Get the controller's name/identifier
    fn name(&self) -> &str {
        "agent_controller"
    }
}

/// A boxed agent controller for dynamic dispatch
pub type BoxedAgentController = Box<dyn AgentController>;

/// Arc-wrapped agent controller for shared ownership
pub type SharedAgentController = std::sync::Arc<dyn AgentController>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::callback::StreamCallback;
    use crate::types::{FinishReason, StreamChunk, TokenUsage};
    use std::sync::Arc;

    /// Mock implementation for testing
    struct MockController {
        agents: Vec<AgentInfo>,
    }

    impl MockController {
        fn new() -> Self {
            Self {
                agents: vec![AgentInfo::new("test", "Test Agent", "gpt-4")
                    .with_description("A test agent")],
            }
        }
    }

    #[async_trait]
    impl AgentController for MockController {
        async fn execute(&self, request: AgentRequest) -> BridgeResult<AgentResponse> {
            Ok(AgentResponse {
                message_id: "msg-1".to_string(),
                content: format!("Echo: {}", request.message),
                role: "assistant".to_string(),
                tool_calls: None,
                usage: TokenUsage::new(10, 5),
                finish_reason: FinishReason::Stop,
            })
        }

        async fn execute_streaming(
            &self,
            request: AgentRequest,
            callback: StreamSender,
        ) -> BridgeResult<AgentResponse> {
            // Send some chunks
            callback.send(StreamChunk::text("Echo: ")).await?;
            callback.send(StreamChunk::text(&request.message)).await?;

            let response = AgentResponse {
                message_id: "msg-1".to_string(),
                content: format!("Echo: {}", request.message),
                role: "assistant".to_string(),
                tool_calls: None,
                usage: TokenUsage::new(10, 5),
                finish_reason: FinishReason::Stop,
            };

            callback.send(StreamChunk::done(response.clone())).await?;

            Ok(response)
        }

        async fn list_agents(&self) -> BridgeResult<Vec<AgentInfo>> {
            Ok(self.agents.clone())
        }

        async fn get_agent_info(&self, name: &str) -> BridgeResult<Option<AgentInfo>> {
            Ok(self.agents.iter().find(|a| a.id == name || a.name == name).cloned())
        }
    }

    #[tokio::test]
    async fn test_mock_controller_execute() {
        let controller = MockController::new();
        let request = AgentRequest::new("s1", "c1", "Hello", "test");

        let response = controller.execute(request).await.unwrap();
        assert_eq!(response.content, "Echo: Hello");
        assert_eq!(response.finish_reason, FinishReason::Stop);
    }

    #[tokio::test]
    async fn test_mock_controller_streaming() {
        let controller = MockController::new();
        let request = AgentRequest::new("s1", "c1", "World", "test");

        let (sender, mut receiver) = StreamCallback::new().split();

        // Run controller in background
        let handle = tokio::spawn(async move { controller.execute_streaming(request, sender).await });

        // Collect chunks
        let mut chunks = Vec::new();
        while let Some(chunk) = receiver.recv().await {
            chunks.push(chunk);
        }

        // Verify response
        let response = handle.await.unwrap().unwrap();
        assert_eq!(response.content, "Echo: World");

        // Verify we got streaming chunks
        assert!(chunks.len() >= 3); // At least: text, text, done
    }

    #[tokio::test]
    async fn test_mock_controller_list_agents() {
        let controller = MockController::new();
        let agents = controller.list_agents().await.unwrap();

        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].id, "test");
    }

    #[tokio::test]
    async fn test_mock_controller_get_agent_info() {
        let controller = MockController::new();

        let agent = controller.get_agent_info("test").await.unwrap();
        assert!(agent.is_some());
        assert_eq!(agent.as_ref().unwrap().name, "Test Agent");

        let missing = controller.get_agent_info("nonexistent").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_boxed_controller() {
        let controller: BoxedAgentController = Box::new(MockController::new());
        let request = AgentRequest::new("s1", "c1", "Test", "test");

        let response = controller.execute(request).await.unwrap();
        assert!(response.content.contains("Test"));
    }

    #[tokio::test]
    async fn test_shared_controller() {
        let controller: SharedAgentController = Arc::new(MockController::new());

        // Clone and use from multiple tasks
        let c1 = controller.clone();
        let c2 = controller.clone();

        let h1 = tokio::spawn(async move {
            c1.execute(AgentRequest::new("s1", "c1", "Task1", "test"))
                .await
        });

        let h2 = tokio::spawn(async move {
            c2.execute(AgentRequest::new("s2", "c2", "Task2", "test"))
                .await
        });

        let (r1, r2) = tokio::join!(h1, h2);
        assert!(r1.unwrap().unwrap().content.contains("Task1"));
        assert!(r2.unwrap().unwrap().content.contains("Task2"));
    }
}
