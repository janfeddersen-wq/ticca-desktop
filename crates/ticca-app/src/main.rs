//! Ticca Desktop - Main entry point
//!
//! A sleek desktop application for AI-assisted coding.
//! Supports both GUI (Iced) and TUI (ratatui) modes.

mod agent_graph;
mod app;
mod app_config;
mod chat_message;
mod cli;
mod helpers;
mod icons;
mod image_handler;
mod keybindings;
mod llm_stream;
mod material_icons;
mod messages;
mod oauth_handler;
mod runner;
mod session_manager;
mod system_executions;
mod theme;
#[cfg(feature = "tui")]
mod tui;
mod views;
mod widgets;

use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn main() -> anyhow::Result<()> {
    // Parse command line arguments first
    let args = cli::Args::parse();

    // Initialize logging differently based on mode
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    if args.console {
        let log_dir = ticca_core::config::paths::get_data_dir()?;
        std::fs::create_dir_all(&log_dir).ok();
        let log_file = log_dir.join("ticca-tui.log");

        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)?;

        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_writer(std::sync::Mutex::new(file)))
            .with(filter)
            .init();

        tracing::info!("Starting Ticca Desktop (Console Mode) - logs at {:?}", log_file);
    } else {
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer())
            .with(filter)
            .init();

        tracing::info!("Starting Ticca Desktop (GUI Mode)");
    }

    // Dispatch to appropriate UI mode
    if args.console {
        #[cfg(feature = "tui")]
        {
            tui::run(args)
        }
        #[cfg(not(feature = "tui"))]
        {
            anyhow::bail!(
                "TUI support is not compiled. Rebuild with: cargo build --features tui"
            );
        }
    } else {
        #[cfg(feature = "gui")]
        {
            runner::run()
        }
        #[cfg(not(feature = "gui"))]
        {
            anyhow::bail!(
                "GUI support is not compiled. Rebuild with: cargo build --features gui"
            );
        }
    }
}
