//! Ticca Desktop - Main entry point
//!
//! A sleek, Iced-based desktop application for AI-assisted coding.

mod app;
mod app_config;
mod chat_message;
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
mod theme;
mod views;
mod widgets;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn main() -> anyhow::Result<()> {
    // Initialize logging
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();
    
    tracing::info!("Starting Ticca Desktop");

    // Run the Iced application
    runner::run()
}
