//! GPUI Views for Ticca Desktop
//!
//! These are the main view components rendered by the app.

pub mod chat;
pub mod settings;
pub mod sidebar;

pub use chat::render_chat_view;
pub use settings::render_settings_view;
// sidebar is used internally by chat view
