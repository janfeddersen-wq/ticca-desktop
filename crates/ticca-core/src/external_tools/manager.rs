//! External tool manager for coordinating downloads and installations.
//!
//! The `ExternalToolManager` is the main entry point for managing external tools.
//! It coordinates between the catalog, manifest, downloader, and extractor.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::catalog::{get_all_tool_definitions, get_tool_definition};
use super::downloader::{DownloadProgress, download_file};
use super::extractor::{extract_archive, make_executable};
use super::manifest::{ToolsManifest, load_manifest, save_manifest};
use super::types::{ExternalToolId, Platform, ToolDefinition, ToolStatus};
use crate::config::paths::get_tools_dir;

// ============================================================================
// Tool Info
// ============================================================================

/// Combined information about a tool (definition + status).
#[derive(Debug, Clone)]
pub struct ToolInfo {
    /// The tool's static definition.
    pub definition: &'static ToolDefinition,
    /// Current installation status.
    pub status: ToolStatus,
    /// Path to the installed executable (if installed).
    pub executable_path: Option<PathBuf>,
}

// ============================================================================
// External Tool Manager
// ============================================================================

/// Manages external tool downloads, installations, and lifecycle.
///
/// This is the main entry point for the external tools system.
/// It's thread-safe and can be shared across async tasks.
pub struct ExternalToolManager {
    /// Base directory for all tools.
    tools_dir: PathBuf,
    /// Currently detected platform.
    platform: Option<Platform>,
    /// Cached manifest (protected by RwLock for async access).
    manifest: Arc<RwLock<ToolsManifest>>,
}

impl ExternalToolManager {
    /// Creates a new tool manager.
    ///
    /// This loads the manifest from disk and detects the current platform.
    pub fn new() -> Result<Self> {
        let tools_dir = get_tools_dir()?;
        let platform = Platform::detect();
        let manifest = load_manifest()?;

        info!(
            "ExternalToolManager initialized. Tools dir: {}, Platform: {:?}",
            tools_dir.display(),
            platform
        );

        Ok(Self {
            tools_dir,
            platform,
            manifest: Arc::new(RwLock::new(manifest)),
        })
    }

    /// Creates a tool manager with a custom tools directory (for testing).
    #[cfg(test)]
    pub fn with_tools_dir(tools_dir: PathBuf) -> Result<Self> {
        let platform = Platform::detect();

        Ok(Self {
            tools_dir,
            platform,
            manifest: Arc::new(RwLock::new(ToolsManifest::new())),
        })
    }

    /// Returns the base tools directory.
    pub fn tools_dir(&self) -> &Path {
        &self.tools_dir
    }

    /// Returns the detected platform.
    pub fn platform(&self) -> Option<Platform> {
        self.platform
    }

    // ========================================================================
    // Tool Queries
    // ========================================================================

    /// Lists all available tools with their current status.
    pub async fn list_tools(&self) -> Vec<ToolInfo> {
        let manifest = self.manifest.read().await;
        let platform = self.platform;

        get_all_tool_definitions()
            .into_iter()
            .map(|def| self.build_tool_info(def, &manifest, platform))
            .collect()
    }

    /// Gets the status of a specific tool.
    pub async fn status(&self, tool_id: ExternalToolId) -> ToolStatus {
        let manifest = self.manifest.read().await;
        self.get_tool_status(tool_id, &manifest)
    }

    /// Gets the path to a tool's executable, if installed.
    pub async fn get_executable_path(&self, tool_id: ExternalToolId) -> Option<PathBuf> {
        let platform = self.platform?;
        let manifest = self.manifest.read().await;
        if !manifest.is_installed(tool_id) {
            return None;
        }

        let def = get_tool_definition(tool_id);
        let tool_dir = self.get_tool_dir(tool_id);
        let exec_relpath = def.get_executable_path(platform);
        let exec_path = tool_dir.join(exec_relpath);

        if exec_path.exists() {
            Some(exec_path)
        } else {
            None
        }
    }

    // ========================================================================
    // Installation
    // ========================================================================

    /// Installs a tool with progress reporting.
    ///
    /// # Arguments
    ///
    /// * `tool_id` - The tool to install.
    /// * `progress_cb` - A callback invoked with progress updates.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The tool is not supported on this platform.
    /// - The download fails.
    /// - Extraction fails.
    pub async fn install<F>(&self, tool_id: ExternalToolId, progress_cb: F) -> Result<()>
    where
        F: Fn(DownloadProgress) + Send + Sync,
    {
        let def = get_tool_definition(tool_id);

        // Check platform support
        let platform = self
            .platform
            .ok_or_else(|| anyhow::anyhow!("Cannot install tools: unsupported platform"))?;

        let download_info = def.get_download_for_platform(platform).ok_or_else(|| {
            anyhow::anyhow!("{} is not supported on {}", def.display_name, platform)
        })?;

        let url = download_info.url;
        let expected_sha256 = download_info.sha256;

        let format = def
            .get_archive_format(platform)
            .ok_or_else(|| anyhow::anyhow!("Unknown archive format for {}", def.display_name))?;

        info!(
            "Installing {} v{} from {}",
            def.display_name, def.version, url
        );

        // Prepare directories
        let tool_dir = self.get_tool_dir(tool_id);
        let archive_path = self.tools_dir.join(format!("{}.archive", tool_id.as_str()));

        // Clean up any previous partial install
        if tool_dir.exists() {
            tokio::fs::remove_dir_all(&tool_dir)
                .await
                .with_context(|| format!("Failed to clean up {}", tool_dir.display()))?;
        }

        // Download with optional SHA256 verification
        let bytes_downloaded =
            download_file(url, &archive_path, expected_sha256, progress_cb).await?;

        // Handle AppImage specially - no extraction needed
        if !format.requires_extraction() {
            info!("Setting up AppImage for {}", def.display_name);
            tokio::fs::create_dir_all(&tool_dir)
                .await
                .with_context(|| format!("Failed to create {}", tool_dir.display()))?;

            // Move the downloaded file to be the executable
            let exec_path = tool_dir.join(def.get_executable_path(platform));
            tokio::fs::rename(&archive_path, &exec_path)
                .await
                .with_context(|| format!("Failed to move AppImage to {}", exec_path.display()))?;

            // Make it executable
            make_executable(&exec_path)?;
        } else {
            // Extract archive
            info!("Extracting {} to {}", def.display_name, tool_dir.display());
            extract_archive(&archive_path, &tool_dir, format)?;

            // Find and make executable the main binary
            self.setup_executable(tool_id, &tool_dir, def).await?;

            // Clean up archive
            if let Err(e) = tokio::fs::remove_file(&archive_path).await {
                warn!("Failed to clean up archive: {}", e);
            }
        }

        // Update manifest
        {
            let mut manifest = self.manifest.write().await;
            manifest.mark_installed(tool_id, def.version.to_string(), bytes_downloaded);
            save_manifest(&manifest)?;
        }

        info!(
            "{} v{} installed successfully",
            def.display_name, def.version
        );
        Ok(())
    }

    /// Uninstalls a tool.
    pub async fn uninstall(&self, tool_id: ExternalToolId) -> Result<()> {
        let def = get_tool_definition(tool_id);
        let tool_dir = self.get_tool_dir(tool_id);

        info!("Uninstalling {}", def.display_name);

        // Remove the tool directory
        if tool_dir.exists() {
            tokio::fs::remove_dir_all(&tool_dir)
                .await
                .with_context(|| format!("Failed to remove {}", tool_dir.display()))?;
        }

        // Update manifest
        {
            let mut manifest = self.manifest.write().await;
            manifest.mark_uninstalled(tool_id);
            save_manifest(&manifest)?;
        }

        info!("{} uninstalled successfully", def.display_name);
        Ok(())
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    /// Gets the installation directory for a tool.
    fn get_tool_dir(&self, tool_id: ExternalToolId) -> PathBuf {
        self.tools_dir.join(tool_id.as_str())
    }

    /// Determines the tool status from the manifest and filesystem.
    fn get_tool_status(&self, tool_id: ExternalToolId, manifest: &ToolsManifest) -> ToolStatus {
        // Check platform support first
        let platform = match self.platform {
            Some(p) => p,
            None => return ToolStatus::UnsupportedPlatform,
        };

        let def = get_tool_definition(tool_id);
        if def.get_url_for_platform(platform).is_none() {
            return ToolStatus::UnsupportedPlatform;
        }

        // Check if installed in manifest
        if let Some(info) = manifest.get_tool(tool_id) {
            // Verify the executable exists using platform-aware path
            let tool_dir = self.get_tool_dir(tool_id);
            let exec_relpath = def.get_executable_path(platform);
            let exec_path = tool_dir.join(exec_relpath);

            if exec_path.exists() {
                ToolStatus::Installed {
                    version: info.version.clone(),
                }
            } else {
                // Manifest says installed but executable missing
                debug!(
                    "{} marked installed but executable not found at {}",
                    tool_id,
                    exec_path.display()
                );
                ToolStatus::NotInstalled
            }
        } else {
            ToolStatus::NotInstalled
        }
    }

    /// Builds a ToolInfo from a definition and manifest.
    fn build_tool_info(
        &self,
        definition: &'static ToolDefinition,
        manifest: &ToolsManifest,
        platform: Option<Platform>,
    ) -> ToolInfo {
        let status = match platform {
            Some(p) if definition.get_url_for_platform(p).is_some() => {
                if let Some(info) = manifest.get_tool(definition.id) {
                    // Use platform-aware executable path
                    let tool_dir = self.get_tool_dir(definition.id);
                    let exec_relpath = definition.get_executable_path(p);
                    let exec_path = tool_dir.join(exec_relpath);

                    if exec_path.exists() {
                        ToolStatus::Installed {
                            version: info.version.clone(),
                        }
                    } else {
                        ToolStatus::NotInstalled
                    }
                } else {
                    ToolStatus::NotInstalled
                }
            }
            _ => ToolStatus::UnsupportedPlatform,
        };

        let executable_path = if let (ToolStatus::Installed { .. }, Some(p)) = (&status, platform) {
            let tool_dir = self.get_tool_dir(definition.id);
            let exec_relpath = definition.get_executable_path(p);
            Some(tool_dir.join(exec_relpath))
        } else {
            None
        };

        ToolInfo {
            definition,
            status,
            executable_path,
        }
    }

    /// Sets up the executable after extraction.
    ///
    /// This finds the correct executable within the extracted directory structure
    /// (handling nested directories from archives) and ensures it's executable.
    async fn setup_executable(
        &self,
        tool_id: ExternalToolId,
        tool_dir: &Path,
        def: &ToolDefinition,
    ) -> Result<()> {
        let platform = self
            .platform
            .ok_or_else(|| anyhow::anyhow!("Cannot setup executable: unsupported platform"))?;

        // Get the platform-specific executable relative path
        let exec_relpath = def.get_executable_path(platform);

        // Many archives extract into a subdirectory (e.g., pandoc-3.6.2/bin/pandoc)
        // We need to find the actual executable
        let exec_name = Path::new(exec_relpath)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(tool_id.as_str());

        // First, check if it exists at the expected path
        let expected_path = tool_dir.join(exec_relpath);
        if expected_path.exists() {
            make_executable(&expected_path)?;
            return Ok(());
        }

        // Otherwise, look for it in subdirectories
        // Archives often extract to a versioned subdirectory
        if let Ok(mut entries) = tokio::fs::read_dir(tool_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    let nested_exec = entry_path.join(exec_relpath);
                    if nested_exec.exists() {
                        // Found it in a subdirectory - move contents up
                        // This handles archives like pandoc-3.6.2.tar.gz that extract to pandoc-3.6.2/
                        debug!(
                            "Found executable in subdirectory, relocating from {}",
                            entry_path.display()
                        );

                        // Move the nested content to the tool directory
                        self.flatten_directory(&entry_path, tool_dir).await?;

                        // Now check the expected path again
                        let final_path = tool_dir.join(exec_relpath);
                        if final_path.exists() {
                            make_executable(&final_path)?;
                            return Ok(());
                        }
                    }

                    // Also check for the executable by name directly
                    let alt_path = entry_path.join(exec_name);
                    if alt_path.exists() {
                        make_executable(&alt_path)?;
                        return Ok(());
                    }
                }
            }
        }

        // Last resort: search recursively for the executable
        if let Some(found) = self.find_executable_recursive(tool_dir, exec_name).await? {
            make_executable(&found)?;
            info!("Found {} at {}", exec_name, found.display());
            return Ok(());
        }

        anyhow::bail!(
            "Could not find {} executable in extracted files",
            def.display_name
        )
    }

    /// Moves contents from a subdirectory up to the parent.
    async fn flatten_directory(&self, from: &Path, to: &Path) -> Result<()> {
        let mut entries = tokio::fs::read_dir(from).await?;

        while let Some(entry) = entries.next_entry().await? {
            let source = entry.path();
            let dest = to.join(entry.file_name());

            if dest.exists() {
                // Skip if already exists at destination
                continue;
            }

            tokio::fs::rename(&source, &dest).await.with_context(|| {
                format!("Failed to move {} to {}", source.display(), dest.display())
            })?;
        }

        // Remove the now-empty subdirectory
        tokio::fs::remove_dir(from).await.ok();

        Ok(())
    }

    /// Recursively searches for an executable by name.
    async fn find_executable_recursive(&self, dir: &Path, name: &str) -> Result<Option<PathBuf>> {
        let mut stack = vec![dir.to_path_buf()];

        while let Some(current) = stack.pop() {
            if let Ok(mut entries) = tokio::fs::read_dir(&current).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.is_dir() {
                        stack.push(path);
                    } else if path.file_name().and_then(|s| s.to_str()) == Some(name) {
                        return Ok(Some(path));
                    }
                }
            }
        }

        Ok(None)
    }
}

// ============================================================================
// FUSE Availability Check (Linux only)
// ============================================================================

/// URL to the FUSE installation documentation on GitHub.
pub const FUSE_DOCS_URL: &str =
    "https://github.com/janfeddersen-wq/ticca-desktop/blob/main/docs/fuse-installation.md";

/// Checks if FUSE is available on the system (required for AppImage).
///
/// Returns `true` if FUSE is available or if not on Linux (not needed).
/// Returns `false` only on Linux when libfuse.so.2 cannot be found.
#[cfg(target_os = "linux")]
pub fn is_fuse_available() -> bool {
    use std::path::Path;

    // Common locations for libfuse.so.2
    let fuse_paths = [
        "/usr/lib/libfuse.so.2",
        "/usr/lib64/libfuse.so.2",
        "/usr/lib/x86_64-linux-gnu/libfuse.so.2",
        "/lib/x86_64-linux-gnu/libfuse.so.2",
        "/usr/lib/aarch64-linux-gnu/libfuse.so.2",
        "/lib/aarch64-linux-gnu/libfuse.so.2",
    ];

    fuse_paths.iter().any(|p| Path::new(p).exists())
}

#[cfg(not(target_os = "linux"))]
pub fn is_fuse_available() -> bool {
    // FUSE/AppImage is Linux-only, other platforms don't need it
    true
}

/// Returns a user-friendly error message when FUSE is not available.
pub fn fuse_missing_error() -> String {
    format!(
        "LibreOffice requires FUSE 2 to run (libfuse.so.2 not found).\n\n\
         Note: FUSE 3 is NOT compatible - you need FUSE 2 specifically.\n\n\
         Install FUSE 2 for your distribution:\n\
         - Arch/Manjaro: sudo pacman -S fuse2\n\
         - Debian/Ubuntu: sudo apt install libfuse2\n\
         - Fedora/RHEL: sudo dnf install fuse-libs\n\n\
         For detailed instructions, see:\n{}",
        FUSE_DOCS_URL
    )
}

/// Checks if an error message indicates a missing FUSE library.
pub fn is_fuse_error(error_msg: &str) -> bool {
    error_msg.contains("libfuse.so") || error_msg.contains("fuse")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_manager_list_tools() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ExternalToolManager::with_tools_dir(temp_dir.path().to_path_buf()).unwrap();

        let tools = manager.list_tools().await;
        assert_eq!(tools.len(), 4);

        // All should be either NotInstalled or UnsupportedPlatform initially
        for tool in &tools {
            assert!(
                matches!(
                    tool.status,
                    ToolStatus::NotInstalled | ToolStatus::UnsupportedPlatform
                ),
                "Tool {:?} has unexpected status: {:?}",
                tool.definition.id,
                tool.status
            );
        }
    }

    #[tokio::test]
    async fn test_manager_status() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ExternalToolManager::with_tools_dir(temp_dir.path().to_path_buf()).unwrap();

        let status = manager.status(ExternalToolId::Pandoc).await;

        // Should be NotInstalled or UnsupportedPlatform
        assert!(
            matches!(
                status,
                ToolStatus::NotInstalled | ToolStatus::UnsupportedPlatform
            ),
            "Unexpected status: {:?}",
            status
        );
    }

    #[tokio::test]
    async fn test_manager_get_executable_path_not_installed() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ExternalToolManager::with_tools_dir(temp_dir.path().to_path_buf()).unwrap();

        let path = manager.get_executable_path(ExternalToolId::Node).await;
        assert!(path.is_none());
    }

    #[test]
    fn test_get_tool_dir() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ExternalToolManager::with_tools_dir(temp_dir.path().to_path_buf()).unwrap();

        let pandoc_dir = manager.get_tool_dir(ExternalToolId::Pandoc);
        assert!(pandoc_dir.ends_with("pandoc"));

        let node_dir = manager.get_tool_dir(ExternalToolId::Node);
        assert!(node_dir.ends_with("node"));
    }
}
