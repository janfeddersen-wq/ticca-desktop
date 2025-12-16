//! Python-based agent controller implementation
//!
//! This module implements the `AgentController` trait by calling into
//! Python code. It handles GIL management, async bridging, and
//! serialization between Rust and Python types.

use async_trait::async_trait;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyCFunction};

use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};

use crate::agent_controller::AgentController;
use crate::callback::StreamSender;
use crate::error::{BridgeError, BridgeResult};
use crate::runtime::{get_or_init_python, PythonConfig, PythonRuntime};
use crate::types::{AgentInfo, AgentRequest, AgentResponse, StreamChunk};

/// Python agent controller configuration
#[derive(Debug, Clone)]
pub struct PythonControllerConfig {
    /// Python module containing the agent implementation
    pub module_name: String,
    /// Class name of the agent controller in Python
    pub class_name: String,
    /// Whether to use async Python calls
    pub use_async: bool,
}

impl Default for PythonControllerConfig {
    fn default() -> Self {
        Self {
            module_name: "ticca_agent".to_string(),
            class_name: "AgentController".to_string(),
            use_async: true,
        }
    }
}

impl PythonControllerConfig {
    /// Create a new config with the specified module and class
    pub fn new(module_name: impl Into<String>, class_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
            class_name: class_name.into(),
            use_async: true,
        }
    }
}

/// Agent controller that delegates to Python
///
/// This implementation calls into a Python agent controller class
/// to execute requests. It manages GIL acquisition and ensures
/// Python calls don't block the Tokio runtime.
pub struct PythonAgentController {
    /// Configuration for the controller
    config: PythonControllerConfig,
    /// Cached list of agents (updated periodically)
    agents_cache: RwLock<Vec<AgentInfo>>,
    /// Whether the controller has been initialized
    initialized: RwLock<bool>,
    /// Tokio runtime handle for blocking operations
    #[allow(dead_code)]
    runtime_handle: tokio::runtime::Handle,
}

impl PythonAgentController {
    /// Create a new Python agent controller
    ///
    /// This will initialize the Python runtime if not already done.
    pub fn new(config: PythonControllerConfig) -> BridgeResult<Self> {
        // Ensure Python is initialized
        get_or_init_python()?;

        Ok(Self {
            config,
            agents_cache: RwLock::new(Vec::new()),
            initialized: RwLock::new(false),
            runtime_handle: tokio::runtime::Handle::current(),
        })
    }

    /// Create with default configuration
    pub fn with_defaults() -> BridgeResult<Self> {
        Self::new(PythonControllerConfig::default())
    }

    /// Create with custom Python config
    pub fn with_python_config(
        controller_config: PythonControllerConfig,
        python_config: PythonConfig,
    ) -> BridgeResult<Self> {
        // Initialize Python with custom config
        PythonRuntime::init(python_config)?;

        Self::new(controller_config)
    }

    /// Initialize the Python controller
    ///
    /// This imports the Python module and creates the controller instance.
    pub async fn initialize(&self) -> BridgeResult<()> {
        let mut initialized = self.initialized.write().await;
        if *initialized {
            return Ok(());
        }

        let module_name = self.config.module_name.clone();
        let class_name = self.config.class_name.clone();

        // Run initialization in a blocking task to avoid GIL issues
        let result = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                // Try to import the module
                let module = py.import_bound(&*module_name).map_err(|e| {
                    BridgeError::ModuleNotLoaded(format!(
                        "Failed to import '{}': {}",
                        module_name, e
                    ))
                })?;

                // Verify the class exists
                module.getattr(&*class_name).map_err(|e| {
                    BridgeError::PythonException(format!(
                        "Class '{}' not found in '{}': {}",
                        class_name, module_name, e
                    ))
                })?;

                info!(
                    "Python controller initialized: {}.{}",
                    module_name, class_name
                );
                Ok::<(), BridgeError>(())
            })
        })
        .await?;

        result?;
        *initialized = true;

        // Refresh the agents cache
        drop(initialized);
        self.refresh_agents_cache().await?;

        Ok(())
    }

    /// Refresh the cached list of agents
    async fn refresh_agents_cache(&self) -> BridgeResult<()> {
        let module_name = self.config.module_name.clone();
        let class_name = self.config.class_name.clone();

        let agents = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> BridgeResult<Vec<AgentInfo>> {
                let module = py.import_bound(&*module_name)?;
                let controller_class = module.getattr(&*class_name)?;
                let controller = controller_class.call0()?;

                let agents_py = controller.call_method0("list_agents")?;
                let agents_json: String = agents_py
                    .call_method0("model_dump_json")
                    .and_then(|r: Bound<'_, pyo3::PyAny>| r.extract())
                    .or_else(|_| {
                        // Fallback: maybe it's already a string or list
                        py.import_bound("json")?
                            .call_method1("dumps", (&agents_py,))?
                            .extract()
                    })?;

                let agents: Vec<AgentInfo> = serde_json::from_str(&agents_json)?;
                Ok(agents)
            })
        })
        .await??;

        let mut cache = self.agents_cache.write().await;
        *cache = agents;

        Ok(())
    }

    /// Call a Python method and deserialize the result
    async fn call_python_method<T>(
        &self,
        method: &str,
        request_json: String,
    ) -> BridgeResult<T>
    where
        T: for<'de> serde::Deserialize<'de> + Send + 'static,
    {
        let module_name = self.config.module_name.clone();
        let class_name = self.config.class_name.clone();
        let method_name = method.to_string();

        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> BridgeResult<T> {
                let module = py.import_bound(&*module_name)?;
                let controller_class = module.getattr(&*class_name)?;
                let controller = controller_class.call0()?;

                // Pass request as JSON string
                let result = controller.call_method1(&*method_name, (&request_json,))?;

                // Get result as JSON
                let result_json: String = result
                    .call_method0("model_dump_json")
                    .and_then(|r: Bound<'_, pyo3::PyAny>| r.extract())
                    .or_else(|_| {
                        // Fallback for non-pydantic objects
                        py.import_bound("json")?
                            .call_method1("dumps", (&result,))?
                            .extract()
                    })?;

                let parsed: T = serde_json::from_str(&result_json)?;
                Ok(parsed)
            })
        })
        .await?
    }

    /// Execute a streaming request with callback
    #[instrument(skip(self, callback), fields(agent = %request.agent_name))]
    async fn do_execute_streaming(
        &self,
        request: AgentRequest,
        callback: StreamSender,
    ) -> BridgeResult<AgentResponse> {
        let module_name = self.config.module_name.clone();
        let class_name = self.config.class_name.clone();
        let request_json = serde_json::to_string(&request)?;

        // Spawn blocking task for Python execution
        let result = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> BridgeResult<AgentResponse> {
                let module = py.import_bound(&*module_name)?;
                let controller_class = module.getattr(&*class_name)?;
                let controller = controller_class.call0()?;

                // Create a Python callback wrapper
                let callback_clone = callback.clone();

                // Create Python function for streaming callback
                let py_callback = PyCFunction::new_closure_bound(
                    py,
                    None,
                    None,
                    move |args: &Bound<'_, pyo3::types::PyTuple>,
                          _kwargs: Option<&Bound<'_, PyDict>>|
                          -> PyResult<()> {
                        let chunk_json: String = args.get_item(0)?.extract()?;

                        let chunk: StreamChunk = serde_json::from_str(&chunk_json)
                            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

                        // Send chunk through the Rust callback
                        // Use try_send to avoid blocking
                        if let Err(e) = callback_clone.try_send(chunk) {
                            warn!("Failed to send chunk: {}", e);
                        }

                        Ok(())
                    },
                )?;

                // Call the streaming method
                let result = controller.call_method1(
                    "execute_streaming",
                    (&request_json, py_callback),
                )?;

                // Get result as JSON
                let result_json: String = result
                    .call_method0("model_dump_json")
                    .and_then(|r: Bound<'_, pyo3::PyAny>| r.extract())
                    .or_else(|_| {
                        py.import_bound("json")?
                            .call_method1("dumps", (&result,))?
                            .extract()
                    })?;

                let response: AgentResponse = serde_json::from_str(&result_json)?;

                // Send done chunk
                let _ = callback.try_send(StreamChunk::done(response.clone()));

                Ok(response)
            })
        })
        .await??;

        Ok(result)
    }
}

#[async_trait]
impl AgentController for PythonAgentController {
    #[instrument(skip(self), fields(agent = %request.agent_name, session = %request.session_id))]
    async fn execute(&self, request: AgentRequest) -> BridgeResult<AgentResponse> {
        // Ensure initialized
        if !*self.initialized.read().await {
            self.initialize().await?;
        }

        debug!("Executing request for agent: {}", request.agent_name);

        let request_json = serde_json::to_string(&request)?;
        self.call_python_method("execute", request_json).await
    }

    #[instrument(skip(self, callback), fields(agent = %request.agent_name, session = %request.session_id))]
    async fn execute_streaming(
        &self,
        request: AgentRequest,
        callback: StreamSender,
    ) -> BridgeResult<AgentResponse> {
        // Ensure initialized
        if !*self.initialized.read().await {
            self.initialize().await?;
        }

        debug!(
            "Executing streaming request for agent: {}",
            request.agent_name
        );

        self.do_execute_streaming(request, callback).await
    }

    async fn list_agents(&self) -> BridgeResult<Vec<AgentInfo>> {
        // Ensure initialized
        if !*self.initialized.read().await {
            self.initialize().await?;
        }

        // Return cached agents
        let cache = self.agents_cache.read().await;
        Ok(cache.clone())
    }

    async fn get_agent_info(&self, name: &str) -> BridgeResult<Option<AgentInfo>> {
        let agents = self.list_agents().await?;
        Ok(agents
            .into_iter()
            .find(|a| a.id == name || a.name == name))
    }

    fn is_ready(&self) -> bool {
        // Check if Python runtime is ready
        PythonRuntime::is_initialized()
    }

    fn name(&self) -> &str {
        "python_agent_controller"
    }
}

/// Builder for creating Python agent controllers
pub struct PythonControllerBuilder {
    controller_config: PythonControllerConfig,
    python_config: Option<PythonConfig>,
}

impl PythonControllerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            controller_config: PythonControllerConfig::default(),
            python_config: None,
        }
    }

    /// Set the Python module name
    pub fn module(mut self, name: impl Into<String>) -> Self {
        self.controller_config.module_name = name.into();
        self
    }

    /// Set the Python class name
    pub fn class(mut self, name: impl Into<String>) -> Self {
        self.controller_config.class_name = name.into();
        self
    }

    /// Enable or disable async Python calls
    pub fn use_async(mut self, enabled: bool) -> Self {
        self.controller_config.use_async = enabled;
        self
    }

    /// Set custom Python configuration
    pub fn python_config(mut self, config: PythonConfig) -> Self {
        self.python_config = Some(config);
        self
    }

    /// Build the controller
    pub fn build(self) -> BridgeResult<PythonAgentController> {
        if let Some(py_config) = self.python_config {
            PythonAgentController::with_python_config(self.controller_config, py_config)
        } else {
            PythonAgentController::new(self.controller_config)
        }
    }
}

impl Default for PythonControllerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = PythonControllerConfig::new("my_module", "MyController");
        assert_eq!(config.module_name, "my_module");
        assert_eq!(config.class_name, "MyController");
        assert!(config.use_async);
    }

    #[test]
    fn test_controller_builder() {
        let builder = PythonControllerBuilder::new()
            .module("custom_module")
            .class("CustomController")
            .use_async(false);

        assert_eq!(builder.controller_config.module_name, "custom_module");
        assert_eq!(builder.controller_config.class_name, "CustomController");
        assert!(!builder.controller_config.use_async);
    }

    // Note: Full integration tests require a Python environment
    // with the ticca_agent module installed
}
