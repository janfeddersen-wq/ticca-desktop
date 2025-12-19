//! Material Design Icons support
//!
//! This module provides Material Icons font and helper functions for rendering
//! Material Design icons in the Iced UI using the `material-icons` crate.

use iced::widget::Text;
use iced::Font;
use material_icons::{Icon, icon_to_char};

/// Material Icons font - embedded from material-icons crate
pub const FONT: Font = Font::with_name("Material Icons");

/// Font bytes for loading into Iced
pub const FONT_BYTES: &[u8] = material_icons::FONT;

/// Create a text widget displaying a Material Icon
pub fn icon<'a>(icon: Icon) -> Text<'a> {
    Text::new(icon_to_char(icon).to_string()).font(FONT)
}

/// Create a Material Icon with a specific size
pub fn icon_sized<'a>(i: Icon, size: f32) -> Text<'a> {
    icon(i).size(size)
}

// Re-export commonly used icons for convenience
// Note: Using material-icons v0.2, some icons are substituted with similar alternatives
pub mod icons {
    use material_icons::Icon;

    // Navigation & Actions
    pub const ARROW_BACK: Icon = Icon::ArrowBack;
    pub const ARROW_FORWARD: Icon = Icon::ArrowForward;
    pub const REFRESH: Icon = Icon::Refresh;
    pub const ADD: Icon = Icon::Add;
    pub const CLOSE: Icon = Icon::Close;
    pub const CHECK: Icon = Icon::Check;
    pub const DONE: Icon = Icon::Done;

    // UI Controls
    pub const SETTINGS: Icon = Icon::Settings;
    pub const TUNE: Icon = Icon::Tune;
    pub const MORE_VERT: Icon = Icon::MoreVert;
    pub const MORE_HORIZ: Icon = Icon::MoreHoriz;
    pub const MENU: Icon = Icon::Menu;

    // Theme (using Brightness6 as fallback for dark/light mode icons)
    pub const DARK_MODE: Icon = Icon::Brightness6;
    pub const LIGHT_MODE: Icon = Icon::Brightness6;
    pub const BRIGHTNESS_6: Icon = Icon::Brightness6;
    pub const CONTRAST: Icon = Icon::Tune;

    // Code & Development
    pub const CODE: Icon = Icon::Code;
    pub const TERMINAL: Icon = Icon::Code;
    pub const DATA_OBJECT: Icon = Icon::Code;
    pub const DESCRIPTION: Icon = Icon::Description;
    pub const INTEGRATION_INSTRUCTIONS: Icon = Icon::Description;

    // Planning & Tasks
    pub const CHECKLIST: Icon = Icon::Assignment;
    pub const TASK_ALT: Icon = Icon::Done;
    pub const ASSIGNMENT: Icon = Icon::Assignment;
    pub const CONTENT_PASTE: Icon = Icon::ContentPaste;
    pub const VIEW_KANBAN: Icon = Icon::Assignment;

    // Chat & Communication
    pub const CHAT: Icon = Icon::Chat;
    pub const CHAT_BUBBLE: Icon = Icon::ChatBubble;
    pub const FORUM: Icon = Icon::Forum;
    pub const SEND: Icon = Icon::Send;
    pub const ARROW_UPWARD: Icon = Icon::ArrowUpward;
    pub const NORTH: Icon = Icon::ArrowUpward;

    // Security
    pub const LOCK: Icon = Icon::Lock;
    pub const LOCK_OPEN: Icon = Icon::LockOpen;
    pub const SECURITY: Icon = Icon::Security;
    pub const KEY: Icon = Icon::VpnKey;
    pub const VPN_KEY: Icon = Icon::VpnKey;

    // Status & Feedback
    pub const INFO: Icon = Icon::Info;
    pub const WARNING: Icon = Icon::Warning;
    pub const ERROR: Icon = Icon::Error;
    pub const CHECK_CIRCLE: Icon = Icon::CheckCircle;
    pub const CANCEL: Icon = Icon::Cancel;
    pub const HELP: Icon = Icon::Help;
    pub const HOURGLASS_EMPTY: Icon = Icon::HourglassEmpty;
    pub const PENDING: Icon = Icon::HourglassEmpty;

    // User & People
    pub const PERSON: Icon = Icon::Person;
    pub const PEOPLE: Icon = Icon::People;
    pub const SMART_TOY: Icon = Icon::Person;
    pub const PSYCHOLOGY: Icon = Icon::Person;

    // File Operations
    pub const FOLDER: Icon = Icon::Folder;
    pub const FOLDER_OPEN: Icon = Icon::FolderOpen;
    pub const INSERT_DRIVE_FILE: Icon = Icon::InsertDriveFile;
    pub const FILE_COPY: Icon = Icon::ContentCopy;
    pub const SAVE: Icon = Icon::Save;
    pub const DOWNLOAD: Icon = Icon::Save;
    pub const UPLOAD: Icon = Icon::ArrowUpward;
    pub const ATTACH_FILE: Icon = Icon::AttachFile;

    // Edit Actions
    pub const EDIT: Icon = Icon::Edit;
    pub const DELETE: Icon = Icon::Delete;
    pub const CONTENT_COPY: Icon = Icon::ContentCopy;
    pub const COPY_ALL: Icon = Icon::ContentCopy;

    // Images & Media
    pub const IMAGE: Icon = Icon::Image;
    pub const PHOTO: Icon = Icon::Photo;
    pub const ADD_PHOTO_ALTERNATE: Icon = Icon::Image;
    pub const SCREENSHOT: Icon = Icon::Image;
    pub const CROP_ORIGINAL: Icon = Icon::CropOriginal;

    // Misc
    pub const BOLT: Icon = Icon::Stars;
    pub const LIGHTBULB: Icon = Icon::Stars;
    pub const AUTO_AWESOME: Icon = Icon::Stars;
    pub const STARS: Icon = Icon::Stars;
    pub const INVENTORY_2: Icon = Icon::Archive;
    pub const ARCHIVE: Icon = Icon::Archive;
    pub const BUILD: Icon = Icon::Build;
    pub const CONSTRUCTION: Icon = Icon::Build;

    // Streaming / Activity indicators
    pub const RADIO_BUTTON_CHECKED: Icon = Icon::RadioButtonChecked;
    pub const RADIO_BUTTON_UNCHECKED: Icon = Icon::RadioButtonUnchecked;
    pub const FIBER_MANUAL_RECORD: Icon = Icon::FiberManualRecord;
}
