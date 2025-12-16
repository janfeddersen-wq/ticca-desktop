//! Model pinning system for agent-specific model overrides
//!
//! Provides a hierarchical model resolution system:
//! 1. Agent-specific pinned model (highest priority)
//! 2. Global default model
//! 3. Fallback default (lowest priority)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A pinned model configuration for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedModel {
    /// The model identifier to use.
    pub model: String,

    /// Optional reason for the pin (for documentation).
    pub reason: Option<String>,
}

impl PinnedModel {
    /// Create a new pinned model.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            reason: None,
        }
    }

    /// Create a pinned model with a reason.
    pub fn with_reason(model: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            reason: Some(reason.into()),
        }
    }
}

/// Registry of pinned models for agents.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ModelPinRegistry {
    /// Agent-specific model pins, keyed by agent name.
    pins: HashMap<String, PinnedModel>,
}

impl ModelPinRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pin a model for a specific agent.
    pub fn pin(&mut self, agent_name: &str, model: impl Into<String>) {
        self.pins.insert(agent_name.to_string(), PinnedModel::new(model));
    }

    /// Pin a model for a specific agent with a reason.
    pub fn pin_with_reason(
        &mut self,
        agent_name: &str,
        model: impl Into<String>,
        reason: impl Into<String>,
    ) {
        self.pins.insert(
            agent_name.to_string(),
            PinnedModel::with_reason(model, reason),
        );
    }

    /// Unpin a model for a specific agent.
    pub fn unpin(&mut self, agent_name: &str) -> Option<PinnedModel> {
        self.pins.remove(agent_name)
    }

    /// Get the pinned model for an agent.
    pub fn get_pin(&self, agent_name: &str) -> Option<&PinnedModel> {
        self.pins.get(agent_name)
    }

    /// Check if an agent has a pinned model.
    pub fn is_pinned(&self, agent_name: &str) -> bool {
        self.pins.contains_key(agent_name)
    }

    /// List all pinned agents.
    pub fn pinned_agents(&self) -> impl Iterator<Item = &str> {
        self.pins.keys().map(|s| s.as_str())
    }

    /// Clear all pins.
    pub fn clear(&mut self) {
        self.pins.clear();
    }
}

/// Model resolver with hierarchical resolution.
///
/// Resolution order:
/// 1. Agent-specific pinned model
/// 2. Global default model
/// 3. Fallback default
#[derive(Debug, Clone)]
pub struct ModelResolver {
    /// Registry of pinned models.
    pins: ModelPinRegistry,

    /// Global default model.
    global_default: Option<String>,

    /// Ultimate fallback if nothing else is set.
    fallback: String,
}

impl ModelResolver {
    /// Create a new resolver with a fallback model.
    pub fn new(fallback: impl Into<String>) -> Self {
        Self {
            pins: ModelPinRegistry::new(),
            global_default: None,
            fallback: fallback.into(),
        }
    }

    /// Create a resolver with an existing pin registry.
    pub fn with_pins(mut self, pins: ModelPinRegistry) -> Self {
        self.pins = pins;
        self
    }

    /// Set the global default model.
    pub fn with_global_default(mut self, model: impl Into<String>) -> Self {
        self.global_default = Some(model.into());
        self
    }

    /// Resolve the model to use for a given agent.
    ///
    /// Returns the model according to the resolution hierarchy:
    /// 1. Pinned model for the agent
    /// 2. Global default
    /// 3. Fallback
    pub fn resolve(&self, agent_name: &str) -> &str {
        // 1. Check for pinned model
        if let Some(pinned) = self.pins.get_pin(agent_name) {
            return &pinned.model;
        }

        // 2. Check for global default
        if let Some(ref global) = self.global_default {
            return global;
        }

        // 3. Return fallback
        &self.fallback
    }

    /// Resolve with information about where the model came from.
    pub fn resolve_with_source(&self, agent_name: &str) -> (&str, ModelSource) {
        if let Some(pinned) = self.pins.get_pin(agent_name) {
            return (&pinned.model, ModelSource::Pinned);
        }

        if let Some(ref global) = self.global_default {
            return (global, ModelSource::GlobalDefault);
        }

        (&self.fallback, ModelSource::Fallback)
    }

    /// Get a mutable reference to the pin registry.
    pub fn pins_mut(&mut self) -> &mut ModelPinRegistry {
        &mut self.pins
    }

    /// Get the pin registry.
    pub fn pins(&self) -> &ModelPinRegistry {
        &self.pins
    }

    /// Set the global default.
    pub fn set_global_default(&mut self, model: Option<String>) {
        self.global_default = model;
    }
}

impl Default for ModelResolver {
    fn default() -> Self {
        Self::new("gpt-4")
    }
}

/// Source of a resolved model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSource {
    /// Model was pinned for the specific agent.
    Pinned,
    /// Model came from global default.
    GlobalDefault,
    /// Model is the ultimate fallback.
    Fallback,
}

impl std::fmt::Display for ModelSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pinned => write!(f, "pinned"),
            Self::GlobalDefault => write!(f, "global default"),
            Self::Fallback => write!(f, "fallback"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pinned_model() {
        let pin = PinnedModel::new("gpt-4-turbo");
        assert_eq!(pin.model, "gpt-4-turbo");
        assert!(pin.reason.is_none());

        let pin = PinnedModel::with_reason("claude-3-opus", "Better at code");
        assert_eq!(pin.model, "claude-3-opus");
        assert_eq!(pin.reason, Some("Better at code".to_string()));
    }

    #[test]
    fn test_pin_registry() {
        let mut registry = ModelPinRegistry::new();

        registry.pin("code-puppy", "gpt-4-turbo");
        registry.pin_with_reason("review-agent", "claude-3-opus", "Best for reviews");

        assert!(registry.is_pinned("code-puppy"));
        assert!(registry.is_pinned("review-agent"));
        assert!(!registry.is_pinned("unknown-agent"));

        let pin = registry.get_pin("review-agent").expect("should exist");
        assert_eq!(pin.model, "claude-3-opus");
        assert_eq!(pin.reason, Some("Best for reviews".to_string()));

        registry.unpin("code-puppy");
        assert!(!registry.is_pinned("code-puppy"));
    }

    #[test]
    fn test_resolver_hierarchy() {
        let mut pins = ModelPinRegistry::new();
        pins.pin("special-agent", "claude-3-opus");

        let resolver = ModelResolver::new("fallback-model")
            .with_global_default("gpt-4")
            .with_pins(pins);

        // Pinned model takes precedence
        assert_eq!(resolver.resolve("special-agent"), "claude-3-opus");

        // Non-pinned agent gets global default
        assert_eq!(resolver.resolve("regular-agent"), "gpt-4");
    }

    #[test]
    fn test_resolver_fallback() {
        let resolver = ModelResolver::new("fallback-model");

        // No global default, falls back to fallback
        assert_eq!(resolver.resolve("any-agent"), "fallback-model");
    }

    #[test]
    fn test_resolve_with_source() {
        let mut pins = ModelPinRegistry::new();
        pins.pin("pinned-agent", "pinned-model");

        let resolver = ModelResolver::new("fallback")
            .with_global_default("global")
            .with_pins(pins);

        let (model, source) = resolver.resolve_with_source("pinned-agent");
        assert_eq!(model, "pinned-model");
        assert_eq!(source, ModelSource::Pinned);

        let (model, source) = resolver.resolve_with_source("regular-agent");
        assert_eq!(model, "global");
        assert_eq!(source, ModelSource::GlobalDefault);

        let resolver = ModelResolver::new("fallback");
        let (model, source) = resolver.resolve_with_source("any-agent");
        assert_eq!(model, "fallback");
        assert_eq!(source, ModelSource::Fallback);
    }

    #[test]
    fn test_model_source_display() {
        assert_eq!(ModelSource::Pinned.to_string(), "pinned");
        assert_eq!(ModelSource::GlobalDefault.to_string(), "global default");
        assert_eq!(ModelSource::Fallback.to_string(), "fallback");
    }
}
