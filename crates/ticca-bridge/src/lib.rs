#![deny(clippy::unwrap_used)]
// Allow this because pyo3 proc macros generate code that triggers false positives
#![allow(clippy::useless_conversion)]

//! Ticca Bridge - PyO3 bridge for Python interoperability
//!
//! This crate provides Python bindings for Ticca's core functionality,
//! enabling Python code to interact with the Rust backend and vice versa.
//!
//! # Architecture
//!
//! The bridge provides two main integration patterns:
//!
//! 1. **Rust → Python**: Execute Python agents from Rust using [`AgentController`]
//! 2. **Python → Rust**: Export Rust types to Python for use in agent implementations
//!
//! # Key Components
//!
//! - [`types`]: Shared types for request/response communication
//! - [`callback`]: Streaming callback infrastructure
//! - [`runtime`]: Python runtime initialization
//! - [`AgentController`]: Trait for executing agent requests
//! - [`PythonAgentController`]: Implementation that calls Python agents
//!
//! # Example
//!
//! ```ignore
//! use ticca_bridge::{PythonAgentController, AgentRequest, AgentController};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create controller
//!     let controller = PythonAgentController::with_defaults()?;
//!     
//!     // Execute a request
//!     let request = AgentRequest::new(
//!         "session-1",
//!         "conv-1",
//!         "Hello, agent!",
//!         "default"
//!     );
//!     
//!     let response = controller.execute(request).await?;
//!     println!("Response: {}", response.content);
//!     
//!     Ok(())
//! }
//! ```

use pyo3::prelude::*;

// Public modules
pub mod agent_controller;
pub mod callback;
pub mod error;
pub mod python_controller;
pub mod runtime;
pub mod types;

// Python binding modules - exposed for re-export by ticca-core
pub mod agent;
pub mod session;

// Re-exports for convenience
pub use agent_controller::{AgentController, BoxedAgentController, SharedAgentController};
pub use callback::{PythonStreamCallback, StreamCallback, StreamReceiver, StreamSender};
pub use error::{BridgeError, BridgeResult};
pub use python_controller::{PythonAgentController, PythonControllerBuilder, PythonControllerConfig};
pub use runtime::{get_or_init_python, init_python, init_python_with_config, PythonConfig, PythonRuntime};
pub use types::{
    AgentInfo, AgentRequest, AgentResponse, AgentState, FinishReason, StreamChunk, TokenUsage,
    ToolCall,
};

/// A Python module implemented in Rust.
///
/// This module is loaded when Python imports `ticca_bridge`.
/// It exposes Rust types and functions to Python.
#[pymodule]
fn ticca_bridge(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Version info
    m.add_function(wrap_pyfunction!(version, m)?)?;

    // Core types for Python agent implementations
    m.add_class::<agent::PyAgentConfig>()?;
    m.add_class::<session::PySession>()?;
    m.add_class::<session::PyMessage>()?;

    // Add type classes
    m.add_class::<PyAgentRequest>()?;
    m.add_class::<PyAgentResponse>()?;
    m.add_class::<PyToolCall>()?;
    m.add_class::<PyTokenUsage>()?;
    m.add_class::<PyStreamChunk>()?;

    Ok(())
}

/// Get the version of the ticca-bridge crate
#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// ============================================================================
// Python wrapper types for bridge types
// ============================================================================

/// Python-exposed AgentRequest
#[pyclass(name = "AgentRequest")]
#[derive(Clone)]
pub struct PyAgentRequest {
    inner: AgentRequest,
}

#[pymethods]
impl PyAgentRequest {
    #[new]
    #[pyo3(signature = (session_id, conversation_id, message, agent_name, model=None, temperature=None, max_tokens=None))]
    fn new(
        session_id: String,
        conversation_id: String,
        message: String,
        agent_name: String,
        model: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Self {
        let mut request = AgentRequest::new(session_id, conversation_id, message, agent_name);
        if let Some(m) = model {
            request = request.with_model(m);
        }
        if let Some(t) = temperature {
            request = request.with_temperature(t);
        }
        if let Some(mt) = max_tokens {
            request = request.with_max_tokens(mt);
        }
        Self { inner: request }
    }

    #[getter]
    fn session_id(&self) -> &str {
        &self.inner.session_id
    }

    #[getter]
    fn conversation_id(&self) -> &str {
        &self.inner.conversation_id
    }

    #[getter]
    fn message(&self) -> &str {
        &self.inner.message
    }

    #[getter]
    fn agent_name(&self) -> &str {
        &self.inner.agent_name
    }

    #[getter]
    fn model(&self) -> Option<&str> {
        self.inner.model.as_deref()
    }

    #[getter]
    fn temperature(&self) -> Option<f32> {
        self.inner.temperature
    }

    #[getter]
    fn max_tokens(&self) -> Option<u32> {
        self.inner.max_tokens
    }

    /// Serialize to JSON
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Deserialize from JSON
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: AgentRequest = serde_json::from_str(json)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    fn __repr__(&self) -> String {
        format!(
            "AgentRequest(session_id='{}', agent_name='{}', message='{}')",
            self.inner.session_id,
            self.inner.agent_name,
            if self.inner.message.len() > 50 {
                format!("{}...", &self.inner.message[..50])
            } else {
                self.inner.message.clone()
            }
        )
    }
}

impl PyAgentRequest {
    /// Get the inner request
    pub fn into_inner(self) -> AgentRequest {
        self.inner
    }
}

/// Python-exposed AgentResponse
#[pyclass(name = "AgentResponse")]
#[derive(Clone)]
pub struct PyAgentResponse {
    inner: AgentResponse,
}

#[pymethods]
impl PyAgentResponse {
    #[new]
    #[pyo3(signature = (message_id, content, role="assistant".to_string()))]
    fn new(message_id: String, content: String, role: String) -> Self {
        Self {
            inner: AgentResponse {
                message_id,
                content,
                role,
                tool_calls: None,
                usage: TokenUsage::default(),
                finish_reason: FinishReason::Stop,
            },
        }
    }

    #[getter]
    fn message_id(&self) -> &str {
        &self.inner.message_id
    }

    #[getter]
    fn content(&self) -> &str {
        &self.inner.content
    }

    #[getter]
    fn role(&self) -> &str {
        &self.inner.role
    }

    /// Serialize to JSON
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Deserialize from JSON
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: AgentResponse = serde_json::from_str(json)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    fn __repr__(&self) -> String {
        format!(
            "AgentResponse(message_id='{}', content='{}')",
            self.inner.message_id,
            if self.inner.content.len() > 50 {
                format!("{}...", &self.inner.content[..50])
            } else {
                self.inner.content.clone()
            }
        )
    }
}

impl PyAgentResponse {
    /// Create from inner response
    pub fn from_inner(inner: AgentResponse) -> Self {
        Self { inner }
    }

    /// Get the inner response
    pub fn into_inner(self) -> AgentResponse {
        self.inner
    }
}

/// Python-exposed ToolCall
#[pyclass(name = "ToolCall")]
#[derive(Clone)]
pub struct PyToolCall {
    inner: ToolCall,
}

#[pymethods]
impl PyToolCall {
    #[new]
    fn new(id: String, name: String, arguments: String) -> PyResult<Self> {
        let args: serde_json::Value = serde_json::from_str(&arguments)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: ToolCall::new(id, name, args),
        })
    }

    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }

    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    #[getter]
    fn arguments(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner.arguments)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("ToolCall(id='{}', name='{}')", self.inner.id, self.inner.name)
    }
}

/// Python-exposed TokenUsage
#[pyclass(name = "TokenUsage")]
#[derive(Clone)]
pub struct PyTokenUsage {
    inner: TokenUsage,
}

#[pymethods]
impl PyTokenUsage {
    #[new]
    fn new(prompt_tokens: u32, completion_tokens: u32) -> Self {
        Self {
            inner: TokenUsage::new(prompt_tokens, completion_tokens),
        }
    }

    #[getter]
    fn prompt_tokens(&self) -> u32 {
        self.inner.prompt_tokens
    }

    #[getter]
    fn completion_tokens(&self) -> u32 {
        self.inner.completion_tokens
    }

    #[getter]
    fn total_tokens(&self) -> u32 {
        self.inner.total_tokens
    }

    fn __repr__(&self) -> String {
        format!(
            "TokenUsage(prompt={}, completion={}, total={})",
            self.inner.prompt_tokens, self.inner.completion_tokens, self.inner.total_tokens
        )
    }
}

/// Python-exposed StreamChunk
#[pyclass(name = "StreamChunk")]
#[derive(Clone)]
pub struct PyStreamChunk {
    inner: StreamChunk,
}

#[pymethods]
impl PyStreamChunk {
    /// Create a text delta chunk
    #[staticmethod]
    fn text_delta(content: String) -> Self {
        Self {
            inner: StreamChunk::text(content),
        }
    }

    /// Create a thinking delta chunk
    #[staticmethod]
    fn thinking_delta(content: String) -> Self {
        Self {
            inner: StreamChunk::thinking(content),
        }
    }

    /// Create an error chunk
    #[staticmethod]
    fn error(message: String) -> Self {
        Self {
            inner: StreamChunk::error(message),
        }
    }

    /// Serialize to JSON
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Deserialize from JSON
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: StreamChunk = serde_json::from_str(json)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            StreamChunk::TextDelta { content } => {
                format!("StreamChunk::TextDelta('{}')", content)
            }
            StreamChunk::ThinkingDelta { content } => {
                format!("StreamChunk::ThinkingDelta('{}')", content)
            }
            StreamChunk::ToolStart { tool_call } => {
                format!("StreamChunk::ToolStart({})", tool_call.name)
            }
            StreamChunk::ToolResult { tool_call_id, .. } => {
                format!("StreamChunk::ToolResult({})", tool_call_id)
            }
            StreamChunk::StateChange { from, to } => {
                format!("StreamChunk::StateChange({} -> {})", from, to)
            }
            StreamChunk::Usage { usage } => {
                format!("StreamChunk::Usage({})", usage.total_tokens)
            }
            StreamChunk::Done { .. } => "StreamChunk::Done".to_string(),
            StreamChunk::Error { message } => {
                format!("StreamChunk::Error('{}')", message)
            }
        }
    }
}

impl PyStreamChunk {
    /// Get the inner chunk
    pub fn into_inner(self) -> StreamChunk {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }

    #[test]
    fn test_agent_request_creation() {
        let request = AgentRequest::new("s1", "c1", "Hello", "agent1");
        assert_eq!(request.session_id, "s1");
        assert_eq!(request.agent_name, "agent1");
    }

    #[test]
    fn test_stream_chunk_json_roundtrip() {
        let chunk = StreamChunk::text("Hello, world!");
        let json = serde_json::to_string(&chunk).expect("serialize");
        let parsed: StreamChunk = serde_json::from_str(&json).expect("deserialize");

        match parsed {
            StreamChunk::TextDelta { content } => assert_eq!(content, "Hello, world!"),
            _ => panic!("Wrong chunk type"),
        }
    }
}
