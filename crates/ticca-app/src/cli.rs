//! Command-line argument parsing for Ticca Desktop
//!
//! Supports both GUI and TUI modes with various startup options.

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

/// AI-assisted coding desktop application
#[derive(Parser, Debug)]
#[command(name = "ticca")]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Run in console/terminal UI mode instead of GUI
    #[arg(long, short = 'c')]
    pub console: bool,

    /// Working directory for the session
    #[arg(long, short = 'w', value_name = "DIR")]
    pub working_dir: Option<PathBuf>,

    /// Start with a specific agent
    #[arg(long, short = 'a', value_enum)]
    pub agent: Option<AgentArg>,

    /// Start with a specific theme
    #[arg(long, short = 't', value_name = "THEME")]
    pub theme: Option<String>,

    /// Load a specific session by ID
    #[arg(long, short = 's', value_name = "SESSION_ID")]
    pub session: Option<String>,

    /// Enable YOLO mode (auto-approve all tool executions)
    #[arg(long)]
    pub yolo: bool,
}

/// Agent type argument for CLI
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AgentArg {
    /// Planning agent for architecture and design
    Planning,
    /// Coding agent for implementation
    Coding,
    /// Skills agent for specialized tasks
    Skills,
}

impl AgentArg {
    /// Convert to core AgentType
    pub fn to_agent_type(self) -> ticca_core::agents::AgentType {
        match self {
            AgentArg::Planning => ticca_core::agents::AgentType::Planning,
            AgentArg::Coding => ticca_core::agents::AgentType::Coding,
            AgentArg::Skills => ticca_core::agents::AgentType::Skills,
        }
    }
}

impl Args {
    /// Parse command line arguments
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Get the working directory, defaulting to current directory
    pub fn working_directory(&self) -> PathBuf {
        self.working_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    /// Get the agent type if specified
    pub fn agent_type(&self) -> Option<ticca_core::agents::AgentType> {
        self.agent.map(|a| a.to_agent_type())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_args() {
        let args = Args::parse_from(["ticca"]);
        assert!(!args.console);
        assert!(args.working_dir.is_none());
        assert!(args.agent.is_none());
        assert!(args.theme.is_none());
        assert!(!args.yolo);
    }

    #[test]
    fn test_console_flag() {
        let args = Args::parse_from(["ticca", "--console"]);
        assert!(args.console);

        let args = Args::parse_from(["ticca", "-c"]);
        assert!(args.console);
    }

    #[test]
    fn test_agent_arg() {
        let args = Args::parse_from(["ticca", "--agent", "planning"]);
        assert!(matches!(args.agent, Some(AgentArg::Planning)));

        let args = Args::parse_from(["ticca", "-a", "coding"]);
        assert!(matches!(args.agent, Some(AgentArg::Coding)));
    }

    #[test]
    fn test_working_dir() {
        let args = Args::parse_from(["ticca", "-w", "/tmp/test"]);
        assert_eq!(args.working_dir, Some(PathBuf::from("/tmp/test")));
    }

    #[test]
    fn test_combined_args() {
        let args = Args::parse_from([
            "ticca",
            "--console",
            "--agent",
            "coding",
            "--theme",
            "dracula",
            "--yolo",
            "-w",
            "/home/user/project",
        ]);
        assert!(args.console);
        assert!(matches!(args.agent, Some(AgentArg::Coding)));
        assert_eq!(args.theme, Some("dracula".to_string()));
        assert!(args.yolo);
        assert_eq!(args.working_dir, Some(PathBuf::from("/home/user/project")));
    }
}
