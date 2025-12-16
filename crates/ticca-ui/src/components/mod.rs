//! UI Components for Ticca Desktop.
//!
//! This module contains all reusable UI components for the GPUI-based
//! frontend, designed for 120FPS performance.

pub mod chat;
pub mod input;
pub mod markdown;
pub mod scroll;
pub mod sidebar;
pub mod toolbar;

// Re-export commonly used types
pub use chat::{ChatMessage, ChatView, MessageRole, ToolCallDisplay};
pub use input::{Attachment, AttachmentKind, InputBar};
pub use markdown::{render_markdown, MarkdownRenderer, RenderedContent};
pub use scroll::{ScrollDirection, SmoothScroll};
pub use sidebar::{ConversationPreview, Sidebar};
pub use toolbar::{ConnectionStatus, Toolbar};
