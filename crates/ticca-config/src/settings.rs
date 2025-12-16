//! Application settings
//!
//! Complete configuration system for Ticca Desktop with support for:
//! - Core AI settings (model, temperature, default agent)
//! - Message/context management (tokens, compaction, sessions)
//! - Display preferences (diff lines, message suppression)
//! - Safety controls (yolo mode, recursion)
//! - Provider-specific settings (OpenAI reasoning effort, verbosity)

use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, info};

use crate::{ConfigError, Result};

// =============================================================================
// Enums
// =============================================================================

/// Strategy for compacting conversation context when approaching token limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CompactionStrategy {
    /// Truncate older messages (faster, may lose context)
    #[default]
    Truncation,
    /// Summarize older messages (slower, preserves context better)
    Summarization,
}

impl std::fmt::Display for CompactionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncation => write!(f, "truncation"),
            Self::Summarization => write!(f, "summarization"),
        }
    }
}

/// Reasoning effort level for OpenAI GPT-5 models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    /// Minimal reasoning, fastest responses
    Low,
    /// Balanced reasoning (default)
    #[default]
    Medium,
    /// Maximum reasoning, most thorough
    High,
}

impl std::fmt::Display for ReasoningEffort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
        }
    }
}

/// Verbosity level for model responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Verbosity {
    /// Concise responses
    Low,
    /// Balanced verbosity (default)
    #[default]
    Medium,
    /// Detailed, verbose responses
    High,
}

impl std::fmt::Display for Verbosity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
        }
    }
}

// =============================================================================
// Main Settings Structure
// =============================================================================

/// Main application settings container.
///
/// Organized into logical groups for easy navigation and serialization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Core AI settings
    pub core: CoreSettings,
    /// Message and context management
    pub context: ContextSettings,
    /// Display preferences
    pub display: DisplaySettings,
    /// Safety controls
    pub safety: SafetySettings,
    /// Advanced settings
    pub advanced: AdvancedSettings,
    /// OpenAI-specific settings
    pub openai: OpenAiSettings,
    /// UI settings (GPUI-specific)
    pub ui: UiSettings,
}

impl Settings {
    /// Load settings from a TOML file.
    ///
    /// If the file doesn't exist, returns default settings.
    /// If the file exists but is invalid, returns an error.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            debug!("Config file not found, using defaults: {}", path.display());
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)?;
        let settings: Self = toml::from_str(&content)?;

        // Validate after loading
        settings.validate()?;

        info!("Settings loaded from: {}", path.display());
        Ok(settings)
    }

    /// Save settings to a TOML file.
    ///
    /// Creates parent directories if they don't exist.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        // Validate before saving
        self.validate()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;

        info!("Settings saved to: {}", path.display());
        Ok(())
    }

    /// Validate all settings are within acceptable ranges.
    pub fn validate(&self) -> Result<()> {
        self.context.validate()?;
        self.display.validate()?;
        Ok(())
    }
}

// =============================================================================
// Settings Groups
// =============================================================================

/// Core AI settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CoreSettings {
    /// Global default model (e.g., "gpt-4", "claude-3-opus").
    /// If None, uses provider's default.
    pub model: Option<String>,

    /// Global temperature for model responses (0.0-2.0).
    /// If None, uses model's default.
    pub temperature: Option<f64>,

    /// Default agent to use on startup.
    pub default_agent: String,
}

impl Default for CoreSettings {
    fn default() -> Self {
        Self {
            model: None,
            temperature: None,
            default_agent: "code-puppy".to_string(),
        }
    }
}

/// Message and context management settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ContextSettings {
    /// Number of tokens to always protect from compaction.
    /// Recent messages within this limit are never removed.
    pub protected_token_count: u32,

    /// Strategy for reducing context when approaching token limits.
    pub compaction_strategy: CompactionStrategy,

    /// Threshold (0.0-1.0) at which compaction triggers.
    /// E.g., 0.85 means compact when 85% of context is used.
    pub compaction_threshold: f64,

    /// Maximum number of messages to keep in a session.
    pub message_limit: u32,

    /// Whether to automatically save sessions.
    pub auto_save_session: bool,

    /// Maximum number of saved sessions to retain.
    pub max_saved_sessions: u32,
}

impl Default for ContextSettings {
    fn default() -> Self {
        Self {
            protected_token_count: 50000,
            compaction_strategy: CompactionStrategy::default(),
            compaction_threshold: 0.85,
            message_limit: 1000,
            auto_save_session: true,
            max_saved_sessions: 20,
        }
    }
}

impl ContextSettings {
    /// Validate context settings are within acceptable ranges.
    pub fn validate(&self) -> Result<()> {
        if !(0.0..=1.0).contains(&self.compaction_threshold) {
            return Err(ConfigError::Validation(format!(
                "compaction_threshold must be between 0.0 and 1.0, got {}",
                self.compaction_threshold
            )));
        }
        Ok(())
    }
}

/// Display and UI preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplaySettings {
    /// Number of context lines to show in diffs (0-50).
    pub diff_context_lines: u8,

    /// Suppress model thinking/reasoning messages in output.
    pub suppress_thinking_messages: bool,

    /// Suppress informational messages (status updates, etc.).
    pub suppress_informational_messages: bool,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            diff_context_lines: 6,
            suppress_thinking_messages: false,
            suppress_informational_messages: false,
        }
    }
}

impl DisplaySettings {
    /// Validate display settings are within acceptable ranges.
    pub fn validate(&self) -> Result<()> {
        if self.diff_context_lines > 50 {
            return Err(ConfigError::Validation(format!(
                "diff_context_lines must be between 0 and 50, got {}",
                self.diff_context_lines
            )));
        }
        Ok(())
    }
}

/// Safety and permission settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SafetySettings {
    /// YOLO mode: skip confirmation prompts for destructive operations.
    /// When true, the agent can execute commands without asking.
    pub yolo_mode: bool,

    /// Allow recursive directory operations (listing, searching).
    pub allow_recursion: bool,
}

impl Default for SafetySettings {
    fn default() -> Self {
        Self {
            yolo_mode: true,
            allow_recursion: true,
        }
    }
}

/// Advanced settings for power users.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
#[serde(default)]
pub struct AdvancedSettings {
    /// Disable Model Context Protocol (MCP) integration.
    pub disable_mcp: bool,
}

/// OpenAI-specific settings (GPT-5 and beyond).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OpenAiSettings {
    /// Reasoning effort level for GPT-5 models.
    pub reasoning_effort: ReasoningEffort,

    /// Response verbosity level.
    pub verbosity: Verbosity,
}

/// UI-related settings (GPUI-specific).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    /// Target frame rate (default: 120 FPS).
    pub target_fps: u32,

    /// Initial window width in pixels.
    pub window_width: u32,

    /// Initial window height in pixels.
    pub window_height: u32,

    /// Color theme ("dark" or "light").
    pub theme: String,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            target_fps: 120,
            window_width: 1200,
            window_height: 800,
            theme: "dark".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.core.default_agent, "code-puppy");
        assert_eq!(settings.context.protected_token_count, 50000);
        assert_eq!(settings.context.compaction_threshold, 0.85);
        assert_eq!(settings.display.diff_context_lines, 6);
        assert!(settings.safety.yolo_mode);
        assert!(settings.safety.allow_recursion);
    }

    #[test]
    fn test_compaction_strategy_display() {
        assert_eq!(CompactionStrategy::Truncation.to_string(), "truncation");
        assert_eq!(CompactionStrategy::Summarization.to_string(), "summarization");
    }

    #[test]
    fn test_reasoning_effort_display() {
        assert_eq!(ReasoningEffort::Low.to_string(), "low");
        assert_eq!(ReasoningEffort::Medium.to_string(), "medium");
        assert_eq!(ReasoningEffort::High.to_string(), "high");
    }

    #[test]
    fn test_validation_compaction_threshold() {
        let mut settings = Settings::default();
        settings.context.compaction_threshold = 0.5;
        assert!(settings.validate().is_ok());

        settings.context.compaction_threshold = 1.5;
        assert!(settings.validate().is_err());

        settings.context.compaction_threshold = -0.1;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_validation_diff_context_lines() {
        let mut settings = Settings::default();
        settings.display.diff_context_lines = 25;
        assert!(settings.validate().is_ok());

        settings.display.diff_context_lines = 51;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let settings = Settings::default();
        let toml_str = toml::to_string_pretty(&settings).expect("serialize");
        let parsed: Settings = toml::from_str(&toml_str).expect("deserialize");

        assert_eq!(parsed.core.default_agent, settings.core.default_agent);
        assert_eq!(
            parsed.context.compaction_strategy,
            settings.context.compaction_strategy
        );
    }
}
