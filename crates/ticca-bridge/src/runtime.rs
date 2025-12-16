//! Python runtime initialization and management
//!
//! This module handles the embedded Python interpreter setup,
//! ensuring thread-safe initialization and proper GIL management.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tracing::{debug, error, info, warn};

use crate::error::{BridgeError, BridgeResult};

/// Global Python runtime state
static PYTHON_RUNTIME: OnceLock<PythonRuntime> = OnceLock::new();
/// Initialization lock to prevent race conditions
static INIT_LOCK: Mutex<()> = Mutex::new(());

/// Configuration for Python runtime initialization
#[derive(Debug, Clone)]
pub struct PythonConfig {
    /// Path to the Python home directory
    pub python_home: Option<PathBuf>,
    /// Additional paths to add to sys.path
    pub extra_paths: Vec<PathBuf>,
    /// Whether to initialize with a virtual environment
    pub venv_path: Option<PathBuf>,
    /// The main Python module to load (e.g., "ticca_agent")
    pub main_module: Option<String>,
}

impl Default for PythonConfig {
    fn default() -> Self {
        Self {
            python_home: None,
            extra_paths: Vec::new(),
            venv_path: None,
            main_module: Some("ticca_agent".to_string()),
        }
    }
}

impl PythonConfig {
    /// Create a new Python config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the Python home directory
    pub fn with_python_home(mut self, path: impl Into<PathBuf>) -> Self {
        self.python_home = Some(path.into());
        self
    }

    /// Add an extra path to sys.path
    pub fn with_extra_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.extra_paths.push(path.into());
        self
    }

    /// Set the virtual environment path
    pub fn with_venv(mut self, path: impl Into<PathBuf>) -> Self {
        self.venv_path = Some(path.into());
        self
    }

    /// Set the main module to load
    pub fn with_main_module(mut self, module: impl Into<String>) -> Self {
        self.main_module = Some(module.into());
        self
    }
}

/// Manages the Python runtime lifecycle
pub struct PythonRuntime {
    /// Whether the runtime was successfully initialized
    initialized: bool,
    /// Configuration used for initialization
    config: PythonConfig,
}

impl PythonRuntime {
    /// Initialize the Python runtime with the given configuration
    ///
    /// This should be called once at application startup.
    /// Subsequent calls will return the existing runtime.
    pub fn init(config: PythonConfig) -> BridgeResult<&'static Self> {
        // Check if already initialized
        if let Some(runtime) = PYTHON_RUNTIME.get() {
            return Ok(runtime);
        }

        // Acquire lock to prevent race conditions during initialization
        let _guard = INIT_LOCK
            .lock()
            .map_err(|e| BridgeError::Runtime(format!("Init lock poisoned: {e}")))?;

        // Double-check after acquiring lock
        if let Some(runtime) = PYTHON_RUNTIME.get() {
            return Ok(runtime);
        }

        // Perform initialization
        let runtime = Self::do_init(config)?;

        // Store in global
        match PYTHON_RUNTIME.set(runtime) {
            Ok(()) => PYTHON_RUNTIME
                .get()
                .ok_or_else(|| BridgeError::Runtime("Failed to get runtime after set".to_string())),
            Err(_) => {
                // Another thread beat us - return their runtime
                PYTHON_RUNTIME
                    .get()
                    .ok_or_else(|| BridgeError::Runtime("Runtime not available".to_string()))
            }
        }
    }

    /// Get the global runtime, if initialized
    pub fn get() -> Option<&'static Self> {
        PYTHON_RUNTIME.get()
    }

    /// Check if the runtime is initialized
    pub fn is_initialized() -> bool {
        PYTHON_RUNTIME.get().is_some()
    }

    /// Perform the actual initialization
    fn do_init(config: PythonConfig) -> BridgeResult<Self> {
        info!("Initializing Python runtime");

        // Prepare freethreaded Python - this is safe to call multiple times
        // but should ideally only be called once
        pyo3::prepare_freethreaded_python();

        // Configure the Python environment
        Python::with_gil(|py| {
            Self::configure_python(py, &config)?;
            Ok::<(), BridgeError>(())
        })?;

        info!("Python runtime initialized successfully");

        Ok(Self {
            initialized: true,
            config,
        })
    }

    /// Configure Python environment (sys.path, etc.)
    fn configure_python(py: Python<'_>, config: &PythonConfig) -> BridgeResult<()> {
        let sys = py.import_bound("sys").map_err(|e| {
            error!("Failed to import sys: {}", e);
            BridgeError::Runtime(format!("Failed to import sys: {e}"))
        })?;

        let path = sys.getattr("path").map_err(|e| {
            BridgeError::Runtime(format!("Failed to get sys.path: {e}"))
        })?;

        // Add virtual environment site-packages if configured
        if let Some(venv) = &config.venv_path {
            let site_packages = venv.join("lib").join("python3.12").join("site-packages");
            if site_packages.exists() {
                let path_str = site_packages.to_string_lossy();
                path.call_method1("insert", (0, path_str.as_ref()))
                    .map_err(|e| BridgeError::Runtime(format!("Failed to add venv path: {e}")))?;
                debug!("Added venv site-packages: {}", site_packages.display());
            } else {
                warn!("Venv site-packages not found: {}", site_packages.display());
            }
        }

        // Add extra paths
        for extra_path in &config.extra_paths {
            let path_str = extra_path.to_string_lossy();
            path.call_method1("insert", (0, path_str.as_ref()))
                .map_err(|e| {
                    BridgeError::Runtime(format!("Failed to add extra path: {e}"))
                })?;
            debug!("Added extra path: {}", extra_path.display());
        }

        // Log Python version
        let version: String = sys
            .getattr("version")
            .and_then(|v: Bound<'_, pyo3::PyAny>| v.extract::<String>())
            .unwrap_or_else(|_| "unknown".to_string());
        info!("Python version: {}", version.lines().next().unwrap_or(&version));

        Ok(())
    }

    /// Execute Python code with the GIL
    ///
    /// This is a convenience method that acquires the GIL and runs the closure.
    pub fn with_gil<F, R>(&self, f: F) -> R
    where
        F: FnOnce(Python<'_>) -> R,
    {
        Python::with_gil(f)
    }

    /// Import a Python module
    pub fn import_module<'py>(&self, py: Python<'py>, name: &str) -> BridgeResult<Bound<'py, PyModule>> {
        py.import_bound(name).map_err(|e| {
            error!("Failed to import module '{}': {}", name, e);
            BridgeError::ModuleNotLoaded(format!("Failed to import '{}': {}", name, e))
        })
    }

    /// Call a Python function by module and function name
    pub fn call_function<'py>(
        &self,
        py: Python<'py>,
        module: &str,
        function: &str,
        args: impl pyo3::IntoPy<pyo3::Py<pyo3::PyAny>>,
        kwargs: Option<&Bound<'py, PyDict>>,
    ) -> BridgeResult<Bound<'py, PyAny>> {
        let mod_ = self.import_module(py, module)?;
        let func = mod_.getattr(function).map_err(|e| {
            BridgeError::PythonException(format!(
                "Function '{}' not found in module '{}': {}",
                function, module, e
            ))
        })?;

        let result = if let Some(kw) = kwargs {
            func.call((args,), Some(kw))
        } else {
            func.call1((args,))
        };

        result.map_err(|e| {
            error!("Python function call failed: {}", e);
            BridgeError::PythonException(e.to_string())
        })
    }

    /// Get the configuration
    pub fn config(&self) -> &PythonConfig {
        &self.config
    }

    /// Check if initialized
    pub fn is_ready(&self) -> bool {
        self.initialized
    }
}

/// Initialize the Python runtime with default configuration
pub fn init_python() -> BridgeResult<&'static PythonRuntime> {
    PythonRuntime::init(PythonConfig::default())
}

/// Initialize the Python runtime with custom configuration
pub fn init_python_with_config(config: PythonConfig) -> BridgeResult<&'static PythonRuntime> {
    PythonRuntime::init(config)
}

/// Get the Python runtime, initializing with defaults if necessary
pub fn get_or_init_python() -> BridgeResult<&'static PythonRuntime> {
    match PythonRuntime::get() {
        Some(runtime) => Ok(runtime),
        None => PythonRuntime::init(PythonConfig::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_config_builder() {
        let config = PythonConfig::new()
            .with_python_home("/usr/bin/python3")
            .with_extra_path("/custom/path")
            .with_venv("/home/user/.venv")
            .with_main_module("my_module");

        assert_eq!(
            config.python_home,
            Some(PathBuf::from("/usr/bin/python3"))
        );
        assert_eq!(config.extra_paths, vec![PathBuf::from("/custom/path")]);
        assert_eq!(config.venv_path, Some(PathBuf::from("/home/user/.venv")));
        assert_eq!(config.main_module, Some("my_module".to_string()));
    }

    // Note: Full initialization tests require a Python environment
    // and are better suited for integration tests
}
