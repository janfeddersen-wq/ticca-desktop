//! Keybindings for Ticca Desktop
//!
//! Registers global and context-specific keyboard shortcuts.

use gpui::*;

use crate::actions::*;

/// Register all keybindings with the application
pub fn register(cx: &mut App) {
    // Global keybindings (available everywhere)
    cx.bind_keys([
        // Settings
        KeyBinding::new("ctrl-,", OpenSettings, None),
        KeyBinding::new("cmd-,", OpenSettings, None), // macOS
        
        // Session management
        KeyBinding::new("ctrl-n", NewSession, None),
        KeyBinding::new("cmd-n", NewSession, None), // macOS
        
        // Theme toggle
        KeyBinding::new("ctrl-shift-t", ThemeToggle, None),
        KeyBinding::new("cmd-shift-t", ThemeToggle, None), // macOS
        
        // Toggle sidebar
        KeyBinding::new("ctrl-b", ToggleFlowPanel, None),
        KeyBinding::new("cmd-b", ToggleFlowPanel, None), // macOS
        
        // Escape to close settings
        KeyBinding::new("escape", CloseSettings, None),
        
        // Agent switching with Ctrl+1/2/3/4
        KeyBinding::new("ctrl-1", SwitchAgent("coding".into()), None),
        KeyBinding::new("ctrl-2", SwitchAgent("planning".into()), None),
        KeyBinding::new("ctrl-3", SwitchAgent("skills".into()), None),
        KeyBinding::new("ctrl-4", SwitchAgent("explore".into()), None),
        KeyBinding::new("cmd-1", SwitchAgent("coding".into()), None),
        KeyBinding::new("cmd-2", SwitchAgent("planning".into()), None),
        KeyBinding::new("cmd-3", SwitchAgent("skills".into()), None),
        KeyBinding::new("cmd-4", SwitchAgent("explore".into()), None),
    ]);

    tracing::debug!("Registered keybindings");
}
