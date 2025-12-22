//! Core types for external tool management.
//!
//! This module defines the foundational types used across the external tools
//! infrastructure: tool identifiers, platform detection, tool status, and
//! tool definitions.

use serde::{Deserialize, Serialize};
use std::fmt;

// ============================================================================
// External Tool Identifiers
// ============================================================================

/// Unique identifier for each external tool.
///
/// This enum represents all supported external tools that can be downloaded
/// and managed by Ticca.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExternalToolId {
    /// Pandoc - Universal document converter for text extraction.
    Pandoc,
    /// Node.js - JavaScript runtime for document/presentation creation.
    Node,
    /// LibreOffice - Office suite for formula recalculation and validation.
    LibreOffice,
}

impl ExternalToolId {
    /// Returns all available tool IDs.
    pub fn all() -> &'static [ExternalToolId] {
        &[Self::Pandoc, Self::Node, Self::LibreOffice]
    }

    /// Returns the lowercase string identifier for this tool.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pandoc => "pandoc",
            Self::Node => "node",
            Self::LibreOffice => "libreoffice",
        }
    }
}

impl fmt::Display for ExternalToolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for ExternalToolId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pandoc" => Ok(Self::Pandoc),
            "node" | "nodejs" => Ok(Self::Node),
            "libreoffice" | "libre" | "soffice" => Ok(Self::LibreOffice),
            _ => Err(format!("Unknown tool: {}", s)),
        }
    }
}

// ============================================================================
// Platform Detection
// ============================================================================

/// Represents a supported platform (OS + architecture).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    LinuxX64,
    LinuxArm64,
    MacosX64,
    MacosArm64,
    WindowsX64,
}

impl Platform {
    /// Detects the current platform at runtime.
    ///
    /// Returns `None` if the platform is unsupported.
    pub fn detect() -> Option<Self> {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            Some(Platform::LinuxX64)
        }
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        {
            Some(Platform::LinuxArm64)
        }
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        {
            Some(Platform::MacosX64)
        }
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            Some(Platform::MacosArm64)
        }
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            Some(Platform::WindowsX64)
        }
        #[cfg(not(any(
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "windows", target_arch = "x86_64"),
        )))]
        {
            None
        }
    }

    /// Returns a human-readable description of the platform.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::LinuxX64 => "Linux (x86_64)",
            Self::LinuxArm64 => "Linux (ARM64)",
            Self::MacosX64 => "macOS (Intel)",
            Self::MacosArm64 => "macOS (Apple Silicon)",
            Self::WindowsX64 => "Windows (x86_64)",
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ============================================================================
// Tool Status
// ============================================================================

/// Current installation status of an external tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolStatus {
    /// Tool is not installed.
    NotInstalled,
    /// Tool is currently being downloaded/installed.
    Installing {
        /// Progress percentage (0.0 to 100.0).
        progress_percent: u8,
    },
    /// Tool is fully installed and ready to use.
    Installed {
        /// Installed version string.
        version: String,
    },
    /// Tool failed to install.
    Failed {
        /// Error message describing the failure.
        error: String,
    },
    /// Tool is not supported on this platform.
    UnsupportedPlatform,
}

impl ToolStatus {
    /// Returns true if the tool is ready to use.
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Installed { .. })
    }

    /// Returns true if the tool can be installed (not already installed or installing).
    pub fn can_install(&self) -> bool {
        matches!(self, Self::NotInstalled | Self::Failed { .. })
    }
}

// ============================================================================
// Tool Definition
// ============================================================================

/// Archive format for downloaded tool packages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArchiveFormat {
    /// Gzip-compressed tar archive (.tar.gz, .tgz)
    TarGz,
    /// XZ-compressed tar archive (.tar.xz)
    TarXz,
    /// ZIP archive (.zip)
    Zip,
    /// AppImage - single executable file (no extraction needed)
    AppImage,
}

impl ArchiveFormat {
    /// Infers the archive format from a URL or filename.
    pub fn from_url(url: &str) -> Option<Self> {
        let lower = url.to_lowercase();
        if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
            Some(Self::TarGz)
        } else if lower.ends_with(".tar.xz") {
            Some(Self::TarXz)
        } else if lower.ends_with(".zip") {
            Some(Self::Zip)
        } else if lower.ends_with(".appimage") {
            Some(Self::AppImage)
        } else {
            None
        }
    }

    /// Returns true if this format requires extraction.
    pub fn requires_extraction(&self) -> bool {
        !matches!(self, Self::AppImage)
    }
}

/// A platform-specific download entry with optional SHA256 checksum.
#[derive(Debug, Clone, Copy)]
pub struct PlatformDownload {
    /// The download URL.
    pub url: &'static str,
    /// Expected SHA256 hash (lowercase hex), or None to skip verification.
    pub sha256: Option<&'static str>,
}

impl PlatformDownload {
    /// Creates a new download entry with optional SHA256.
    pub const fn new(url: &'static str, sha256: Option<&'static str>) -> Self {
        Self { url, sha256 }
    }
}

/// Platform-specific download URLs for a tool.
#[derive(Debug, Clone)]
pub struct PlatformUrls {
    pub linux_x64: Option<PlatformDownload>,
    pub linux_arm64: Option<PlatformDownload>,
    pub macos_x64: Option<PlatformDownload>,
    pub macos_arm64: Option<PlatformDownload>,
    pub windows_x64: Option<PlatformDownload>,
}

impl PlatformUrls {
    /// Returns the download info for the given platform.
    pub fn get(&self, platform: Platform) -> Option<PlatformDownload> {
        match platform {
            Platform::LinuxX64 => self.linux_x64,
            Platform::LinuxArm64 => self.linux_arm64,
            Platform::MacosX64 => self.macos_x64,
            Platform::MacosArm64 => self.macos_arm64,
            Platform::WindowsX64 => self.windows_x64,
        }
    }

    /// Returns just the URL for the given platform (convenience method).
    pub fn get_url(&self, platform: Platform) -> Option<&'static str> {
        self.get(platform).map(|d| d.url)
    }
}

/// Platform-specific executable relative paths.
///
/// Different platforms have different executable locations and extensions:
/// - Unix (Linux/macOS): typically `bin/tool`
/// - Windows: typically `tool.exe` (often at root of extracted dir)
#[derive(Debug, Clone)]
pub struct ExecutablePaths {
    /// Path for Linux (both x64 and arm64), e.g., "bin/pandoc"
    pub linux: &'static str,
    /// Path for macOS (both x64 and arm64), e.g., "bin/pandoc"
    pub macos: &'static str,
    /// Path for Windows, e.g., "pandoc.exe"
    pub windows: &'static str,
}

impl ExecutablePaths {
    /// Returns the executable relative path for the given platform.
    pub fn get(&self, platform: Platform) -> &'static str {
        match platform {
            Platform::LinuxX64 | Platform::LinuxArm64 => self.linux,
            Platform::MacosX64 | Platform::MacosArm64 => self.macos,
            Platform::WindowsX64 => self.windows,
        }
    }
}

/// Complete definition of an external tool.
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    /// Unique identifier for this tool.
    pub id: ExternalToolId,
    /// Human-readable display name.
    pub display_name: &'static str,
    /// Brief description of what the tool does.
    pub description: &'static str,
    /// Current version being offered.
    pub version: &'static str,
    /// Approximate download size in megabytes.
    pub size_mb: u32,
    /// Features/skills that require this tool.
    pub required_by: &'static [&'static str],
    /// Platform-specific download URLs.
    pub urls: PlatformUrls,
    /// Platform-specific executable relative paths.
    pub executable_paths: ExecutablePaths,
}

impl ToolDefinition {
    /// Returns the download info for the current platform.
    pub fn get_download_for_platform(&self, platform: Platform) -> Option<PlatformDownload> {
        self.urls.get(platform)
    }

    /// Returns the download URL for the current platform.
    pub fn get_url_for_platform(&self, platform: Platform) -> Option<&'static str> {
        self.urls.get_url(platform)
    }

    /// Returns the archive format based on the URL for the given platform.
    pub fn get_archive_format(&self, platform: Platform) -> Option<ArchiveFormat> {
        self.get_url_for_platform(platform)
            .and_then(ArchiveFormat::from_url)
    }

    /// Returns the executable relative path for the given platform.
    pub fn get_executable_path(&self, platform: Platform) -> &'static str {
        self.executable_paths.get(platform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id_as_str() {
        assert_eq!(ExternalToolId::Pandoc.as_str(), "pandoc");
        assert_eq!(ExternalToolId::Node.as_str(), "node");
        assert_eq!(ExternalToolId::LibreOffice.as_str(), "libreoffice");
    }

    #[test]
    fn test_tool_id_from_str() {
        assert_eq!("pandoc".parse::<ExternalToolId>().unwrap(), ExternalToolId::Pandoc);
        assert_eq!("node".parse::<ExternalToolId>().unwrap(), ExternalToolId::Node);
        assert_eq!("nodejs".parse::<ExternalToolId>().unwrap(), ExternalToolId::Node);
        assert_eq!("libreoffice".parse::<ExternalToolId>().unwrap(), ExternalToolId::LibreOffice);
        assert!("unknown".parse::<ExternalToolId>().is_err());
    }

    #[test]
    fn test_tool_id_all() {
        let all = ExternalToolId::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&ExternalToolId::Pandoc));
        assert!(all.contains(&ExternalToolId::Node));
        assert!(all.contains(&ExternalToolId::LibreOffice));
    }

    #[test]
    fn test_platform_detect() {
        // Should return Some on supported platforms
        let platform = Platform::detect();
        // We can't assert a specific value since it depends on the build platform,
        // but on CI it should be Some
        #[cfg(any(
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "windows", target_arch = "x86_64"),
        ))]
        assert!(platform.is_some());
    }

    #[test]
    fn test_archive_format_from_url() {
        assert_eq!(
            ArchiveFormat::from_url("https://example.com/tool.tar.gz"),
            Some(ArchiveFormat::TarGz)
        );
        assert_eq!(
            ArchiveFormat::from_url("https://example.com/tool.tgz"),
            Some(ArchiveFormat::TarGz)
        );
        assert_eq!(
            ArchiveFormat::from_url("https://example.com/tool.tar.xz"),
            Some(ArchiveFormat::TarXz)
        );
        assert_eq!(
            ArchiveFormat::from_url("https://example.com/tool.zip"),
            Some(ArchiveFormat::Zip)
        );
        assert_eq!(
            ArchiveFormat::from_url("https://example.com/tool.dmg"),
            None
        );
    }

    #[test]
    fn test_tool_status_is_ready() {
        assert!(!ToolStatus::NotInstalled.is_ready());
        assert!(!ToolStatus::Installing { progress_percent: 50 }.is_ready());
        assert!(ToolStatus::Installed { version: "1.0".to_string() }.is_ready());
        assert!(!ToolStatus::Failed { error: "oops".to_string() }.is_ready());
        assert!(!ToolStatus::UnsupportedPlatform.is_ready());
    }

    #[test]
    fn test_tool_status_can_install() {
        assert!(ToolStatus::NotInstalled.can_install());
        assert!(!ToolStatus::Installing { progress_percent: 50 }.can_install());
        assert!(!ToolStatus::Installed { version: "1.0".to_string() }.can_install());
        assert!(ToolStatus::Failed { error: "oops".to_string() }.can_install());
        assert!(!ToolStatus::UnsupportedPlatform.can_install());
    }

    #[test]
    fn test_platform_urls_get() {
        let urls = PlatformUrls {
            linux_x64: Some(PlatformDownload::new("https://linux-x64", None)),
            linux_arm64: Some(PlatformDownload::new("https://linux-arm64", None)),
            macos_x64: Some(PlatformDownload::new("https://macos-x64", None)),
            macos_arm64: Some(PlatformDownload::new("https://macos-arm64", None)),
            windows_x64: Some(PlatformDownload::new("https://windows-x64", None)),
        };

        assert_eq!(urls.get_url(Platform::LinuxX64), Some("https://linux-x64"));
        assert_eq!(urls.get_url(Platform::LinuxArm64), Some("https://linux-arm64"));
        assert_eq!(urls.get_url(Platform::MacosX64), Some("https://macos-x64"));
        assert_eq!(urls.get_url(Platform::MacosArm64), Some("https://macos-arm64"));
        assert_eq!(urls.get_url(Platform::WindowsX64), Some("https://windows-x64"));
    }

    #[test]
    fn test_platform_download_with_sha256() {
        let download = PlatformDownload::new(
            "https://example.com/tool.zip",
            Some("abc123def456"),
        );
        assert_eq!(download.url, "https://example.com/tool.zip");
        assert_eq!(download.sha256, Some("abc123def456"));

        let download_no_hash = PlatformDownload::new("https://example.com/tool.zip", None);
        assert!(download_no_hash.sha256.is_none());
    }

    #[test]
    fn test_executable_paths_get() {
        let paths = ExecutablePaths {
            linux: "bin/node",
            macos: "bin/node",
            windows: "node.exe",
        };

        assert_eq!(paths.get(Platform::LinuxX64), "bin/node");
        assert_eq!(paths.get(Platform::LinuxArm64), "bin/node");
        assert_eq!(paths.get(Platform::MacosX64), "bin/node");
        assert_eq!(paths.get(Platform::MacosArm64), "bin/node");
        assert_eq!(paths.get(Platform::WindowsX64), "node.exe");
    }

    #[test]
    fn test_executable_paths_windows_has_exe_extension() {
        // This test verifies that Windows paths end with .exe
        let node_paths = ExecutablePaths {
            linux: "bin/node",
            macos: "bin/node",
            windows: "node.exe",
        };
        let pandoc_paths = ExecutablePaths {
            linux: "bin/pandoc",
            macos: "bin/pandoc",
            windows: "pandoc.exe",
        };

        assert!(
            node_paths.get(Platform::WindowsX64).ends_with(".exe"),
            "Node Windows path should end with .exe"
        );
        assert!(
            pandoc_paths.get(Platform::WindowsX64).ends_with(".exe"),
            "Pandoc Windows path should end with .exe"
        );
    }
}
