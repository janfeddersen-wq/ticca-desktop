//! Python bindings for agent types
//!
//! This module provides Python-accessible agent configuration types.
//! These are standalone definitions to avoid circular dependencies with ticca-core.

use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

/// Agent ID wrapper
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Agent configuration (Rust side)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: AgentId,
    pub name: String,
    pub description: Option<String>,
    pub model: String,
    pub system_prompt: Option<String>,
}

impl AgentConfig {
    pub fn new(id: impl Into<String>, name: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            id: AgentId::new(id),
            name: name.into(),
            description: None,
            model: model.into(),
            system_prompt: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }
}

/// Python-exposed agent configuration
#[pyclass(name = "AgentConfig")]
#[derive(Clone)]
pub struct PyAgentConfig {
    inner: AgentConfig,
}

#[pymethods]
impl PyAgentConfig {
    #[new]
    #[pyo3(signature = (id, name, model, description=None, system_prompt=None))]
    fn new(
        id: String,
        name: String,
        model: String,
        description: Option<String>,
        system_prompt: Option<String>,
    ) -> Self {
        let mut config = AgentConfig::new(id, name, model);
        if let Some(desc) = description {
            config = config.with_description(desc);
        }
        if let Some(prompt) = system_prompt {
            config = config.with_system_prompt(prompt);
        }
        Self { inner: config }
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.0.clone()
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[getter]
    fn model(&self) -> String {
        self.inner.model.clone()
    }

    #[getter]
    fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }

    #[getter]
    fn system_prompt(&self) -> Option<String> {
        self.inner.system_prompt.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "AgentConfig(id='{}', name='{}', model='{}')",
            self.inner.id.0, self.inner.name, self.inner.model
        )
    }
}

impl PyAgentConfig {
    pub fn into_inner(self) -> AgentConfig {
        self.inner
    }
}
