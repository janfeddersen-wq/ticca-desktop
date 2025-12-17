//! Keyboard shortcut definitions and handling
//!
//! Note: Some keybindings are defined but not yet wired up in the UI.

#![allow(dead_code)]

use iced::keyboard::{Key, Modifiers, key::Named};

/// A keyboard shortcut definition
#[derive(Debug, Clone)]
pub struct Keybinding {
    pub key: Key,
    pub modifiers: Modifiers,
    pub description: &'static str,
}

impl Keybinding {
    pub fn new(key: Key, modifiers: Modifiers, description: &'static str) -> Self {
        Self { key, modifiers, description }
    }
    
    /// Check if a key event matches this keybinding
    pub fn matches(&self, key: &Key, modifiers: Modifiers) -> bool {
        *key == self.key && modifiers == self.modifiers
    }
}

/// All keybindings in the application
pub struct Keybindings {
    pub send_message: Keybinding,
    pub new_session: Keybinding,
    pub open_settings: Keybinding,
    pub toggle_theme: Keybinding,
    pub close_panel: Keybinding,
    pub switch_to_coding: Keybinding,
    pub switch_to_planning: Keybinding,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            send_message: Keybinding::new(
                Key::Named(Named::Enter),
                Modifiers::CTRL,
                "Send message (Ctrl+Enter)",
            ),
            new_session: Keybinding::new(
                Key::Character("n".into()),
                Modifiers::CTRL,
                "New session (Ctrl+N)",
            ),
            open_settings: Keybinding::new(
                Key::Character(",".into()),
                Modifiers::CTRL,
                "Open settings (Ctrl+,)",
            ),
            toggle_theme: Keybinding::new(
                Key::Character("t".into()),
                Modifiers::CTRL,
                "Toggle theme (Ctrl+T)",
            ),
            close_panel: Keybinding::new(
                Key::Named(Named::Escape),
                Modifiers::empty(),
                "Close panel (Escape)",
            ),
            switch_to_coding: Keybinding::new(
                Key::Character("1".into()),
                Modifiers::CTRL,
                "Switch to Coding agent (Ctrl+1)",
            ),
            switch_to_planning: Keybinding::new(
                Key::Character("2".into()),
                Modifiers::CTRL,
                "Switch to Planning agent (Ctrl+2)",
            ),
        }
    }
}

impl Keybindings {
    /// Get all keybindings as a list
    pub fn all(&self) -> Vec<&Keybinding> {
        vec![
            &self.send_message,
            &self.new_session,
            &self.open_settings,
            &self.toggle_theme,
            &self.close_panel,
            &self.switch_to_coding,
            &self.switch_to_planning,
        ]
    }
}

/// Global keybindings instance
pub fn keybindings() -> &'static Keybindings {
    static KEYBINDINGS: std::sync::OnceLock<Keybindings> = std::sync::OnceLock::new();
    KEYBINDINGS.get_or_init(Keybindings::default)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_keybinding_matches() {
        let binding = Keybinding::new(
            Key::Character("n".into()),
            Modifiers::CTRL,
            "Test",
        );
        
        assert!(binding.matches(&Key::Character("n".into()), Modifiers::CTRL));
        assert!(!binding.matches(&Key::Character("n".into()), Modifiers::empty()));
        assert!(!binding.matches(&Key::Character("m".into()), Modifiers::CTRL));
    }
    
    #[test]
    fn test_keybindings_all() {
        let bindings = Keybindings::default();
        let all = bindings.all();
        assert_eq!(all.len(), 7);
    }
}
