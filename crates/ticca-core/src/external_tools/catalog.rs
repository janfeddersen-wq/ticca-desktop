//! Tool catalog with hardcoded definitions.
//!
//! This module contains the static definitions for all supported external tools,
//! including their download URLs, versions, and metadata.

use super::types::{ExecutablePaths, ExternalToolId, PlatformDownload, PlatformUrls, ToolDefinition};

// ============================================================================
// Pandoc Definition
// ============================================================================

const PANDOC_VERSION: &str = "3.6.2";

const PANDOC_URLS: PlatformUrls = PlatformUrls {
    linux_x64: Some(PlatformDownload::new(
        "https://github.com/jgm/pandoc/releases/download/3.6.2/pandoc-3.6.2-linux-amd64.tar.gz",
        None, // TODO: Add SHA256 checksum
    )),
    linux_arm64: Some(PlatformDownload::new(
        "https://github.com/jgm/pandoc/releases/download/3.6.2/pandoc-3.6.2-linux-arm64.tar.gz",
        None,
    )),
    macos_x64: Some(PlatformDownload::new(
        "https://github.com/jgm/pandoc/releases/download/3.6.2/pandoc-3.6.2-x86_64-macOS.zip",
        None,
    )),
    macos_arm64: Some(PlatformDownload::new(
        "https://github.com/jgm/pandoc/releases/download/3.6.2/pandoc-3.6.2-arm64-macOS.zip",
        None,
    )),
    windows_x64: Some(PlatformDownload::new(
        "https://github.com/jgm/pandoc/releases/download/3.6.2/pandoc-3.6.2-windows-x86_64.zip",
        None,
    )),
};

const PANDOC_EXECUTABLE_PATHS: ExecutablePaths = ExecutablePaths {
    linux: "bin/pandoc",
    macos: "bin/pandoc",
    windows: "pandoc.exe",
};

const PANDOC_DEFINITION: ToolDefinition = ToolDefinition {
    id: ExternalToolId::Pandoc,
    display_name: "Pandoc",
    description: "Universal document converter for text extraction from docx and other formats",
    version: PANDOC_VERSION,
    size_mb: 40,
    required_by: &["docx"],
    urls: PANDOC_URLS,
    executable_paths: PANDOC_EXECUTABLE_PATHS,
};

// ============================================================================
// Node.js Definition
// ============================================================================

const NODE_VERSION: &str = "22.12.0";

const NODE_URLS: PlatformUrls = PlatformUrls {
    linux_x64: Some(PlatformDownload::new(
        "https://nodejs.org/dist/v22.12.0/node-v22.12.0-linux-x64.tar.xz",
        None, // TODO: Add SHA256 checksum
    )),
    linux_arm64: Some(PlatformDownload::new(
        "https://nodejs.org/dist/v22.12.0/node-v22.12.0-linux-arm64.tar.xz",
        None,
    )),
    macos_x64: Some(PlatformDownload::new(
        "https://nodejs.org/dist/v22.12.0/node-v22.12.0-darwin-x64.tar.gz",
        None,
    )),
    macos_arm64: Some(PlatformDownload::new(
        "https://nodejs.org/dist/v22.12.0/node-v22.12.0-darwin-arm64.tar.gz",
        None,
    )),
    windows_x64: Some(PlatformDownload::new(
        "https://nodejs.org/dist/v22.12.0/node-v22.12.0-win-x64.zip",
        None,
    )),
};

const NODE_EXECUTABLE_PATHS: ExecutablePaths = ExecutablePaths {
    linux: "bin/node",
    macos: "bin/node",
    windows: "node.exe",
};

const NODE_DEFINITION: ToolDefinition = ToolDefinition {
    id: ExternalToolId::Node,
    display_name: "Node.js",
    description: "JavaScript runtime for document and presentation creation (docx, pptx)",
    version: NODE_VERSION,
    size_mb: 25,
    required_by: &["docx", "pptx"],
    urls: NODE_URLS,
    executable_paths: NODE_EXECUTABLE_PATHS,
};

// ============================================================================
// LibreOffice Definition
// ============================================================================

const LIBREOFFICE_VERSION: &str = "fresh";

// LibreOffice URLs - using AppImage for Linux (portable single-file executable)
const LIBREOFFICE_URLS: PlatformUrls = PlatformUrls {
    // AppImage - portable, no extraction needed, just download and run
    linux_x64: Some(PlatformDownload::new(
        "https://appimages.libreitalia.org/LibreOffice-fresh.basic-x86_64.AppImage",
        None,
    )),
    linux_arm64: None, // No AppImage available for ARM64
    // DMG files require special handling - marked as unsupported for now
    macos_x64: None,
    macos_arm64: None,
    // Windows portable version
    windows_x64: None, // Needs PortableApps or MSI extraction - not simple zip
};

const LIBREOFFICE_EXECUTABLE_PATHS: ExecutablePaths = ExecutablePaths {
    linux: "soffice.AppImage",
    macos: "Contents/MacOS/soffice",
    windows: "program/soffice.exe",
};

const LIBREOFFICE_DEFINITION: ToolDefinition = ToolDefinition {
    id: ExternalToolId::LibreOffice,
    display_name: "LibreOffice",
    description: "Office suite for formula recalculation, visual validation, and PDF conversion",
    version: LIBREOFFICE_VERSION,
    size_mb: 280,
    required_by: &["xlsx", "docx", "pptx"],
    urls: LIBREOFFICE_URLS,
    executable_paths: LIBREOFFICE_EXECUTABLE_PATHS,
};

// ============================================================================
// Catalog Access
// ============================================================================

/// Returns the definition for a specific tool.
pub fn get_tool_definition(id: ExternalToolId) -> &'static ToolDefinition {
    match id {
        ExternalToolId::Pandoc => &PANDOC_DEFINITION,
        ExternalToolId::Node => &NODE_DEFINITION,
        ExternalToolId::LibreOffice => &LIBREOFFICE_DEFINITION,
    }
}

/// Returns definitions for all available tools.
pub fn get_all_tool_definitions() -> Vec<&'static ToolDefinition> {
    vec![
        &PANDOC_DEFINITION,
        &NODE_DEFINITION,
        &LIBREOFFICE_DEFINITION,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external_tools::types::Platform;

    #[test]
    fn test_get_tool_definition() {
        let pandoc = get_tool_definition(ExternalToolId::Pandoc);
        assert_eq!(pandoc.id, ExternalToolId::Pandoc);
        assert_eq!(pandoc.display_name, "Pandoc");
        assert_eq!(pandoc.version, "3.6.2");
    }

    #[test]
    fn test_get_all_tool_definitions() {
        let defs = get_all_tool_definitions();
        assert_eq!(defs.len(), 3);
    }

    #[test]
    fn test_pandoc_has_all_platform_urls() {
        let pandoc = get_tool_definition(ExternalToolId::Pandoc);

        assert!(pandoc.urls.get(Platform::LinuxX64).is_some());
        assert!(pandoc.urls.get(Platform::LinuxArm64).is_some());
        assert!(pandoc.urls.get(Platform::MacosX64).is_some());
        assert!(pandoc.urls.get(Platform::MacosArm64).is_some());
        assert!(pandoc.urls.get(Platform::WindowsX64).is_some());
    }

    #[test]
    fn test_node_has_all_platform_urls() {
        let node = get_tool_definition(ExternalToolId::Node);

        assert!(node.urls.get(Platform::LinuxX64).is_some());
        assert!(node.urls.get(Platform::LinuxArm64).is_some());
        assert!(node.urls.get(Platform::MacosX64).is_some());
        assert!(node.urls.get(Platform::MacosArm64).is_some());
        assert!(node.urls.get(Platform::WindowsX64).is_some());
    }

    #[test]
    fn test_libreoffice_partial_platform_support() {
        let libre = get_tool_definition(ExternalToolId::LibreOffice);

        // Only Linux x64 is currently supported
        assert!(libre.urls.get(Platform::LinuxX64).is_some());
        assert!(libre.urls.get(Platform::LinuxArm64).is_none());
        assert!(libre.urls.get(Platform::MacosX64).is_none());
        assert!(libre.urls.get(Platform::MacosArm64).is_none());
        assert!(libre.urls.get(Platform::WindowsX64).is_none());
    }

    #[test]
    fn test_platform_url_selection() {
        let node = get_tool_definition(ExternalToolId::Node);

        let linux_url = node.get_url_for_platform(Platform::LinuxX64).unwrap();
        assert!(linux_url.contains("linux-x64"));
        assert!(linux_url.ends_with(".tar.xz"));

        let macos_arm_url = node.get_url_for_platform(Platform::MacosArm64).unwrap();
        assert!(macos_arm_url.contains("darwin-arm64"));
        assert!(macos_arm_url.ends_with(".tar.gz"));

        let windows_url = node.get_url_for_platform(Platform::WindowsX64).unwrap();
        assert!(windows_url.contains("win-x64"));
        assert!(windows_url.ends_with(".zip"));
    }

    #[test]
    fn test_pandoc_executable_path_windows_has_exe() {
        let pandoc = get_tool_definition(ExternalToolId::Pandoc);
        let windows_path = pandoc.get_executable_path(Platform::WindowsX64);
        assert!(
            windows_path.ends_with(".exe"),
            "Pandoc Windows executable path should end with .exe, got: {}",
            windows_path
        );
    }

    #[test]
    fn test_node_executable_path_windows_has_exe() {
        let node = get_tool_definition(ExternalToolId::Node);
        let windows_path = node.get_executable_path(Platform::WindowsX64);
        assert!(
            windows_path.ends_with(".exe"),
            "Node Windows executable path should end with .exe, got: {}",
            windows_path
        );
    }

    #[test]
    fn test_executable_paths_per_platform() {
        let node = get_tool_definition(ExternalToolId::Node);

        // Unix platforms should use bin/node
        assert_eq!(node.get_executable_path(Platform::LinuxX64), "bin/node");
        assert_eq!(node.get_executable_path(Platform::LinuxArm64), "bin/node");
        assert_eq!(node.get_executable_path(Platform::MacosX64), "bin/node");
        assert_eq!(node.get_executable_path(Platform::MacosArm64), "bin/node");

        // Windows should use node.exe (at root of extracted dir)
        assert_eq!(node.get_executable_path(Platform::WindowsX64), "node.exe");
    }

    #[test]
    fn test_pandoc_executable_paths_per_platform() {
        let pandoc = get_tool_definition(ExternalToolId::Pandoc);

        // Unix platforms should use bin/pandoc
        assert_eq!(pandoc.get_executable_path(Platform::LinuxX64), "bin/pandoc");
        assert_eq!(pandoc.get_executable_path(Platform::MacosArm64), "bin/pandoc");

        // Windows should use pandoc.exe
        assert_eq!(pandoc.get_executable_path(Platform::WindowsX64), "pandoc.exe");
    }
}
