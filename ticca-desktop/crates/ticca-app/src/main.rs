//! Ticca Desktop - Main entry point
//!
//! A sleek, Iced-based desktop application for AI-assisted coding.

mod app;
mod icons;
mod messages;
mod theme;
mod views;
mod keybindings;

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
    app::run()
}
