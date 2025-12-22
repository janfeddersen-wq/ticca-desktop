//! External tool management for on-demand downloads.
//!
//! This module provides infrastructure for downloading and managing external tools
//! like pandoc, node, and libreoffice. These tools are downloaded on-demand to keep
//! the main application binary small.
//!
//! # Architecture
//!
//! - `types`: Core types (ExternalToolId, Platform, ToolStatus, ToolDefinition)
//! - `catalog`: Static tool definitions with download URLs
//! - `manifest`: JSON persistence for installed tool state
//! - `downloader`: Async file download with progress reporting
//! - `extractor`: Archive extraction (zip, tar.gz, tar.xz)
//! - `manager`: High-level API for managing tools
//!
//! # Example
//!
//! ```ignore
//! use ticca_core::external_tools::{ExternalToolManager, ExternalToolId};
//!
//! let manager = ExternalToolManager::new()?;
//!
//! // List all tools and their status
//! for tool in manager.list_tools().await {
//!     println!("{}: {:?}", tool.definition.display_name, tool.status);
//! }
//!
//! // Install a tool with progress reporting
//! manager.install(ExternalToolId::Pandoc, |progress| {
//!     if let Some(percent) = progress.percent {
//!         println!("Progress: {:.1}%", percent);
//!     }
//! }).await?;
//!
//! // Get the executable path
//! if let Some(pandoc_path) = manager.get_executable_path(ExternalToolId::Pandoc).await {
//!     println!("Pandoc installed at: {}", pandoc_path.display());
//! }
//! ```

pub mod catalog;
pub mod downloader;
pub mod env;
pub mod extractor;
pub mod manager;
pub mod manifest;
pub mod types;

// Re-export commonly used types
pub use catalog::{get_all_tool_definitions, get_tool_definition};
pub use downloader::DownloadProgress;
pub use env::{apply_to_command, env_overrides, installed_tool_bin_dirs, prepend_tools_to_path};
pub use manager::{
    ExternalToolManager, FUSE_DOCS_URL, ToolInfo, fuse_missing_error, is_fuse_available,
    is_fuse_error,
};
pub use manifest::{InstalledToolInfo, ToolsManifest, load_manifest, save_manifest};
pub use types::{
    ArchiveFormat, ExecutablePaths, ExternalToolId, Platform, ToolDefinition, ToolStatus,
};
