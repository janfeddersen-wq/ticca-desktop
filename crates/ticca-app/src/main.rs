//! Ticca Desktop - Main entry point
//!
//! A sleek, GPUI-based desktop application for AI-assisted coding.
//! Supports both GUI mode (default) and TUI/console mode (--console).

use std::path::PathBuf;

use clap::Parser;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

mod actions;
mod app;
mod chat_message;
mod keybindings;
mod llm_stream;
mod modals;
mod notifications;
mod runner;
mod theme;
mod tui;
mod views;

/// AI-assisted coding assistant with both GUI and TUI modes
#[derive(Parser)]
#[command(name = "ticca", about = "AI-assisted coding assistant", version)]
struct Args {
    /// Run in console/TUI mode instead of GUI
    #[arg(long, short = 'c')]
    console: bool,

    /// Initial prompt to send (console mode only)
    #[arg(long, short = 'p')]
    prompt: Option<String>,

    /// Working directory
    #[arg(long, short = 'd')]
    directory: Option<PathBuf>,

    /// Model to use (overrides default from settings)
    #[arg(long, short = 'm')]
    model: Option<String>,

    /// Enable YOLO mode (auto-approve all tools)
    #[arg(long)]
    yolo: bool,

    /// Enable verbose logging (writes to ~/.local/share/ticca-desktop/ticca.log in console mode)
    #[arg(long, short = 'v')]
    verbose: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.console {
        // In console mode, redirect logs to a file to avoid corrupting the TUI
        init_file_logging(args.verbose);

        tracing::info!("Starting Ticca in console/TUI mode");
        tui::run(tui::TuiConfig {
            initial_prompt: args.prompt,
            working_directory: args.directory,
            model_override: args.model,
            yolo_override: args.yolo,
        })
    } else {
        // In GUI mode, use stdout logging
        init_stdout_logging();

        tracing::info!("Starting Ticca Desktop (GUI mode)");
        runner::run()
    }
}

/// Initialize logging to stdout (for GUI mode)
fn init_stdout_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();
}

/// Initialize logging to a file (for console/TUI mode)
fn init_file_logging(verbose: bool) {
    use std::fs::{self, File};
    use tracing_subscriber::fmt::writer::MakeWriterExt;

    // Get log file path in data directory
    let log_path = get_log_path();

    // Create parent directories if needed
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Open log file (truncate on each run to avoid huge files)
    let log_file = match File::create(&log_path) {
        Ok(f) => f,
        Err(_e) => {
            // If we can't create the log file, just disable logging entirely
            // Don't print to stderr as it would also corrupt the TUI
            let filter = EnvFilter::new("off");
            tracing_subscriber::registry().with(filter).init();
            return;
        }
    };

    let filter = if verbose {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"))
    } else {
        // In non-verbose mode, only log warnings and errors
        EnvFilter::new("warn")
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(log_file.with_max_level(tracing::Level::TRACE))
                .with_ansi(false), // No ANSI colors in log file
        )
        .with(filter)
        .init();
}

/// Get the path for the log file
fn get_log_path() -> PathBuf {
    // Use platform-specific data directory
    if let Some(data_dir) = directories::ProjectDirs::from("", "", "ticca-desktop") {
        data_dir.data_dir().join("ticca.log")
    } else {
        // Fallback to current directory
        PathBuf::from("ticca.log")
    }
}
