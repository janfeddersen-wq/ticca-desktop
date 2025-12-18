//! Icon definitions using Bootstrap Icons font
//!
//! This module provides icon constants and helper functions for rendering
//! Bootstrap Icons in the Iced UI.
//!
//! Note: Many icons are defined but not yet used - they're available for future features.

#![allow(dead_code)]

use iced::widget::Text;
use iced::Font;

/// Bootstrap Icons font
pub const ICONS: Font = Font::with_name("bootstrap-icons");

/// Create a text widget displaying an icon
pub fn icon<'a>(codepoint: char) -> Text<'a> {
    Text::new(codepoint.to_string()).font(ICONS)
}

/// Create an icon with a specific size
pub fn icon_sized<'a>(codepoint: char, size: u16) -> Text<'a> {
    icon(codepoint).size(size)
}

// Icon constants - Bootstrap Icons Unicode codepoints
// See: https://icons.getbootstrap.com/

// Navigation & Actions
pub const ARROW_LEFT: char = '\u{F12F}';
pub const ARROW_RIGHT: char = '\u{F138}';
pub const ARROW_CLOCKWISE: char = '\u{F130}';
pub const PLUS: char = '\u{F4FE}';
pub const PLUS_LG: char = '\u{F64D}';
pub const X: char = '\u{F62A}';
pub const X_LG: char = '\u{F659}';
pub const CHECK: char = '\u{F26E}';
pub const CHECK_LG: char = '\u{F633}';

// Checkboxes
pub const SQUARE: char = '\u{F584}';
pub const CHECK_SQUARE_FILL: char = '\u{F26C}';

// UI Controls
pub const GEAR: char = '\u{F3E5}';
pub const GEAR_FILL: char = '\u{F3E6}';
pub const SLIDERS: char = '\u{F593}';
pub const THREE_DOTS: char = '\u{F5D4}';
pub const THREE_DOTS_VERTICAL: char = '\u{F5D5}';

// Theme
pub const MOON: char = '\u{F497}';
pub const MOON_FILL: char = '\u{F498}';
pub const SUN: char = '\u{F5A5}';
pub const SUN_FILL: char = '\u{F5A6}';
pub const CIRCLE_HALF: char = '\u{F27E}';

// Code & Development
pub const CODE: char = '\u{F2A0}';
pub const CODE_SLASH: char = '\u{F2A1}';
pub const TERMINAL: char = '\u{F5FA}';
pub const TERMINAL_FILL: char = '\u{F5FB}';
pub const BRACES: char = '\u{F1EF}';
pub const FILE_CODE: char = '\u{F343}';

// Planning & Tasks
pub const LIST_CHECK: char = '\u{F46A}';
pub const LIST_TASK: char = '\u{F46D}';
pub const CLIPBOARD: char = '\u{F29A}';
pub const CLIPBOARD_CHECK: char = '\u{F29B}';
pub const KANBAN: char = '\u{F40A}';

// Chat & Communication
pub const CHAT: char = '\u{F268}';
pub const CHAT_DOTS: char = '\u{F26A}';
pub const CHAT_DOTS_FILL: char = '\u{F26B}';
pub const CHAT_LEFT: char = '\u{F270}';
pub const CHAT_LEFT_DOTS: char = '\u{F272}';
pub const SEND: char = '\u{F571}';
pub const SEND_FILL: char = '\u{F572}';

// Security
pub const LOCK: char = '\u{F47A}';
pub const LOCK_FILL: char = '\u{F47B}';
pub const UNLOCK: char = '\u{F615}';
pub const UNLOCK_FILL: char = '\u{F616}';
pub const SHIELD: char = '\u{F577}';
pub const SHIELD_LOCK: char = '\u{F57E}';
pub const KEY: char = '\u{F40F}';
pub const KEY_FILL: char = '\u{F410}';

// Status & Feedback
pub const INFO_CIRCLE: char = '\u{F3FD}';
pub const INFO_CIRCLE_FILL: char = '\u{F3FE}';
pub const EXCLAMATION_TRIANGLE: char = '\u{F33A}';
pub const EXCLAMATION_TRIANGLE_FILL: char = '\u{F33B}';
pub const CHECK_CIRCLE: char = '\u{F26D}';
pub const CHECK_CIRCLE_FILL: char = '\u{F26F}';
pub const X_CIRCLE: char = '\u{F628}';
pub const X_CIRCLE_FILL: char = '\u{F629}';
pub const QUESTION_CIRCLE: char = '\u{F505}';
pub const HOURGLASS: char = '\u{F3F3}';
pub const HOURGLASS_SPLIT: char = '\u{F3F4}';

// User & People
pub const PERSON: char = '\u{F4DA}';
pub const PERSON_FILL: char = '\u{F4DB}';
pub const PEOPLE: char = '\u{F4CE}';
pub const ROBOT: char = '\u{F56A}';

// File Operations
pub const FOLDER: char = '\u{F3D5}';
pub const FOLDER_FILL: char = '\u{F3D6}';
pub const FOLDER_OPEN: char = '\u{F3D9}';
pub const FILE: char = '\u{F333}';
pub const FILE_EARMARK: char = '\u{F348}';
pub const FILES: char = '\u{F3C0}';
pub const SAVE: char = '\u{F56E}';
pub const DOWNLOAD: char = '\u{F30A}';
pub const UPLOAD: char = '\u{F609}';

// Edit Actions
pub const PENCIL: char = '\u{F4CC}';
pub const PENCIL_FILL: char = '\u{F4CD}';
pub const TRASH: char = '\u{F5F6}';
pub const TRASH_FILL: char = '\u{F5F7}';
pub const COPY: char = '\u{F2A8}';
pub const CLIPBOARD_PLUS: char = '\u{F2A0}';

// Misc
pub const LIGHTNING: char = '\u{F465}';
pub const LIGHTNING_FILL: char = '\u{F466}';
pub const LIGHTBULB: char = '\u{F46C}';
pub const LIGHTBULB_FILL: char = '\u{F46D}';
pub const MAGIC: char = '\u{F487}';
pub const STARS: char = '\u{F599}';
pub const SPARKLES: char = '\u{F6AE}';
pub const BOX: char = '\u{F1E8}';
pub const ARCHIVE: char = '\u{F163}';
