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
pub fn icon_sized<'a>(icon: Icon, size: u16) -> Text<'a> {
    icon(icon).size(size)
}

// Re-export commonly used icons for convenience
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

    // Theme
    pub const DARK_MODE: Icon = Icon::DarkMode;
    pub const LIGHT_MODE: Icon = Icon::LightMode;
    pub const BRIGHTNESS_6: Icon = Icon::Brightness6;
    pub const CONTRAST: Icon = Icon::Contrast;

    // Code & Development
    pub const CODE: Icon = Icon::Code;
    pub const TERMINAL: Icon = Icon::Terminal;
    pub const DATA_OBJECT: Icon = Icon::DataObject;
    pub const DESCRIPTION: Icon = Icon::Description;
    pub const INTEGRATION_INSTRUCTIONS: Icon = Icon::IntegrationInstructions;

    // Planning & Tasks
    pub const CHECKLIST: Icon = Icon::Checklist;
    pub const TASK_ALT: Icon = Icon::TaskAlt;
    pub const ASSIGNMENT: Icon = Icon::Assignment;
    pub const CONTENT_PASTE: Icon = Icon::ContentPaste;
    pub const VIEW_KANBAN: Icon = Icon::ViewKanban;

    // Chat & Communication
    pub const CHAT: Icon = Icon::Chat;
    pub const CHAT_BUBBLE: Icon = Icon::ChatBubble;
    pub const FORUM: Icon = Icon::Forum;
    pub const SEND: Icon = Icon::Send;
    pub const ARROW_UPWARD: Icon = Icon::ArrowUpward;
    pub const NORTH: Icon = Icon::North;

    // Security
    pub const LOCK: Icon = Icon::Lock;
    pub const LOCK_OPEN: Icon = Icon::LockOpen;
    pub const SECURITY: Icon = Icon::Security;
    pub const KEY: Icon = Icon::Key;
    pub const VPN_KEY: Icon = Icon::VpnKey;

    // Status & Feedback
    pub const INFO: Icon = Icon::Info;
    pub const WARNING: Icon = Icon::Warning;
    pub const ERROR: Icon = Icon::Error;
    pub const CHECK_CIRCLE: Icon = Icon::CheckCircle;
    pub const CANCEL: Icon = Icon::Cancel;
    pub const HELP: Icon = Icon::Help;
    pub const HOURGLASS_EMPTY: Icon = Icon::HourglassEmpty;
    pub const PENDING: Icon = Icon::Pending;

    // User & People
    pub const PERSON: Icon = Icon::Person;
    pub const PEOPLE: Icon = Icon::People;
    pub const SMART_TOY: Icon = Icon::SmartToy;
    pub const PSYCHOLOGY: Icon = Icon::Psychology;

    // File Operations
    pub const FOLDER: Icon = Icon::Folder;
    pub const FOLDER_OPEN: Icon = Icon::FolderOpen;
    pub const INSERT_DRIVE_FILE: Icon = Icon::InsertDriveFile;
    pub const FILE_COPY: Icon = Icon::FileCopy;
    pub const SAVE: Icon = Icon::Save;
    pub const DOWNLOAD: Icon = Icon::Download;
    pub const UPLOAD: Icon = Icon::Upload;
    pub const ATTACH_FILE: Icon = Icon::AttachFile;

    // Edit Actions
    pub const EDIT: Icon = Icon::Edit;
    pub const DELETE: Icon = Icon::Delete;
    pub const CONTENT_COPY: Icon = Icon::ContentCopy;
    pub const COPY_ALL: Icon = Icon::CopyAll;

    // Images & Media
    pub const IMAGE: Icon = Icon::Image;
    pub const PHOTO: Icon = Icon::Photo;
    pub const ADD_PHOTO_ALTERNATE: Icon = Icon::AddPhotoAlternate;
    pub const SCREENSHOT: Icon = Icon::Screenshot;
    pub const CROP_ORIGINAL: Icon = Icon::CropOriginal;

    // Misc
    pub const BOLT: Icon = Icon::Bolt;
    pub const LIGHTBULB: Icon = Icon::Lightbulb;
    pub const AUTO_AWESOME: Icon = Icon::AutoAwesome;
    pub const STARS: Icon = Icon::Stars;
    pub const INVENTORY_2: Icon = Icon::Inventory2;
    pub const ARCHIVE: Icon = Icon::Archive;
    pub const BUILD: Icon = Icon::Build;
    pub const CONSTRUCTION: Icon = Icon::Construction;
}
