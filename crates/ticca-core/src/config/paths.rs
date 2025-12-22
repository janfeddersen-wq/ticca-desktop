//! Centralized path management for Ticca data directories.
//!
//! This module provides a single source of truth for all data directory paths,
//! eliminating duplication across the codebase. All paths are relative to the
//! platform-specific data directory:
//!
//! - Linux: `~/.local/share/ticca-desktop/`
//! - macOS: `~/Library/Application Support/ticca-desktop/`
//! - Windows: `C:\Users\<User>\AppData\Roaming\ticca-desktop\`

use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::path::PathBuf;

/// Application identifier used for `ProjectDirs`.
const APP_NAME: &str = "ticca-desktop";

/// Returns the base data directory for the application.
///
/// This is the root directory where all application data is stored.
/// The actual path is platform-dependent (see module docs).
///
/// # Errors
///
/// Returns an error if the platform-specific data directory cannot be determined
/// (e.g., on systems without a proper home directory).
///
/// # Example
///
/// ```ignore
/// let data_dir = get_data_dir()?;
/// // Linux: ~/.local/share/ticca-desktop/
/// ```
pub fn get_data_dir() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("", "", APP_NAME)
        .context("Could not determine data directory for ticca-desktop")?;

    Ok(proj_dirs.data_dir().to_path_buf())
}

/// Returns the path to the skills directory.
///
/// This directory stores user-defined skills (Python scripts, etc.).
///
/// Path: `{data_dir}/skills/`
///
/// # Errors
///
/// Returns an error if the base data directory cannot be determined.
pub fn get_skills_dir() -> Result<PathBuf> {
    Ok(get_data_dir()?.join("skills"))
}

/// Returns the path to the bin directory.
///
/// This directory stores downloaded binaries (e.g., uv).
///
/// Path: `{data_dir}/bin/`
///
/// # Errors
///
/// Returns an error if the base data directory cannot be determined.
pub fn get_bin_dir() -> Result<PathBuf> {
    Ok(get_data_dir()?.join("bin"))
}

/// Returns the path to the virtual environments directory.
///
/// This directory stores Python virtual environments created by skills.
///
/// Path: `{data_dir}/venvs/`
///
/// # Errors
///
/// Returns an error if the base data directory cannot be determined.
pub fn get_venvs_dir() -> Result<PathBuf> {
    Ok(get_data_dir()?.join("venvs"))
}

/// Returns the path to the external tools directory.
///
/// This directory stores on-demand downloaded tools (pandoc, node, libreoffice, etc.).
/// Each tool gets its own subdirectory.
///
/// Path: `{data_dir}/tools/`
///
/// # Errors
///
/// Returns an error if the base data directory cannot be determined.
pub fn get_tools_dir() -> Result<PathBuf> {
    Ok(get_data_dir()?.join("tools"))
}

/// Returns the platform-specific path to the UV binary.
///
/// UV is a fast Python package installer used for managing skill environments.
/// It is downloaded on-demand via the external tools system.
///
/// - Linux/macOS: `{data_dir}/tools/uv/uv`
/// - Windows: `{data_dir}/tools/uv/uv.exe`
///
/// # Errors
///
/// Returns an error if the base data directory cannot be determined.
pub fn get_uv_binary_path() -> Result<PathBuf> {
    let tools_dir = get_tools_dir()?;

    #[cfg(windows)]
    let uv_name = "uv.exe";

    #[cfg(not(windows))]
    let uv_name = "uv";

    Ok(tools_dir.join("uv").join(uv_name))
}

/// Ensures all required data directories exist.
///
/// This function creates the following directories if they don't exist:
/// - Base data directory
/// - Skills directory
/// - Bin directory
/// - Venvs directory
/// - Tools directory (for external tool downloads)
///
/// # Errors
///
/// Returns an error if any directory cannot be created (e.g., permission issues).
///
/// # Example
///
/// ```ignore
/// // Call at application startup
/// ensure_dirs_exist()?;
/// ```
pub fn ensure_dirs_exist() -> Result<()> {
    let dirs = [
        get_data_dir()?,
        get_skills_dir()?,
        get_bin_dir()?,
        get_venvs_dir()?,
        get_tools_dir()?,
    ];

    for dir in dirs {
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_data_dir_returns_valid_path() {
        let result = get_data_dir();
        assert!(result.is_ok());
        let path = result.unwrap();
        // On Windows, data_dir() returns {AppData}/ticca-desktop/data
        // On Unix, it returns {data_dir}/ticca-desktop
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains("ticca-desktop"),
            "Path should contain 'ticca-desktop': {}",
            path_str
        );
    }

    #[test]
    fn test_subdirs_are_under_data_dir() {
        let data_dir = get_data_dir().unwrap();
        let skills_dir = get_skills_dir().unwrap();
        let bin_dir = get_bin_dir().unwrap();
        let venvs_dir = get_venvs_dir().unwrap();
        let tools_dir = get_tools_dir().unwrap();

        assert!(skills_dir.starts_with(&data_dir));
        assert!(bin_dir.starts_with(&data_dir));
        assert!(venvs_dir.starts_with(&data_dir));
        assert!(tools_dir.starts_with(&data_dir));

        assert!(skills_dir.ends_with("skills"));
        assert!(bin_dir.ends_with("bin"));
        assert!(venvs_dir.ends_with("venvs"));
        assert!(tools_dir.ends_with("tools"));
    }

    #[test]
    fn test_uv_binary_path_has_correct_extension() {
        let uv_path = get_uv_binary_path().unwrap();
        let tools_dir = get_tools_dir().unwrap();

        // UV should be in tools/uv/
        assert!(uv_path.starts_with(&tools_dir));

        #[cfg(windows)]
        {
            assert!(uv_path.ends_with("uv.exe"));
            assert!(
                uv_path.to_string_lossy().contains("uv\\uv.exe")
                    || uv_path.to_string_lossy().contains("uv/uv.exe")
            );
        }

        #[cfg(not(windows))]
        {
            assert!(uv_path.ends_with("uv"));
            assert!(uv_path.to_string_lossy().contains("uv/uv"));
        }
    }
}
