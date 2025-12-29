//! Toast Notifications for Ticca Desktop
//!
//! Provides helper functions for displaying notifications using gpui-component.
//! Note: These functions require access to &mut App, so they must be called from
//! global action handlers or window callbacks, not from Context methods.

use gpui::*;
use gpui_component::{
    notification::Notification,
    Root,
};

/// Show an info toast notification
/// Must be called from a context where &mut App is available (e.g., global action handler)
pub fn info(message: impl Into<SharedString>, window: &mut Window, cx: &mut App) {
    let msg = message.into();
    Root::update(window, cx, |root, window, cx| {
        root.notification.update(cx, |list, cx| {
            list.push(Notification::info(msg), window, cx);
        });
    });
}

/// Show a success toast notification
pub fn success(message: impl Into<SharedString>, window: &mut Window, cx: &mut App) {
    let msg = message.into();
    Root::update(window, cx, |root, window, cx| {
        root.notification.update(cx, |list, cx| {
            list.push(Notification::success(msg), window, cx);
        });
    });
}

/// Show a warning toast notification
pub fn warning(message: impl Into<SharedString>, window: &mut Window, cx: &mut App) {
    let msg = message.into();
    Root::update(window, cx, |root, window, cx| {
        root.notification.update(cx, |list, cx| {
            list.push(Notification::warning(msg), window, cx);
        });
    });
}

/// Show an error toast notification
pub fn error(message: impl Into<SharedString>, window: &mut Window, cx: &mut App) {
    let msg = message.into();
    Root::update(window, cx, |root, window, cx| {
        root.notification.update(cx, |list, cx| {
            list.push(Notification::error(msg), window, cx);
        });
    });
}
