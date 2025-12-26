//! Chat feature - main conversation UI and state management.
//!
//! This module is split into focused submodules:
//! - `types`: Core types, constants, and helper functions
//! - `state`: ChatState definition and core methods
//! - `handlers`: Complex message handler implementations
//! - `update`: Message update handler (main match dispatch)
//! - `view`: View rendering
//! - `approval`: Tool approval modal
//! - `subscriptions`: Iced subscription streams

mod approval;
mod handlers;
mod state;
mod subscriptions;
#[cfg(test)]
mod tests;
mod types;
mod update;
mod view;

// Re-export for the app feature module
pub(in crate::app) use approval::wrap_with_approval_modal;
pub(crate) use types::DiffModalState;
pub(in crate::app) use state::ChatState;
pub(in crate::app) use update::update;
pub(in crate::app) use view::view;

// Re-export Effect for internal use
use super::super::effects::Effect;
