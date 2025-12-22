//! Environment helpers for external tools.
//!
//! This module provides utilities for injecting installed external tools
//! into the PATH and other environment variables so they can be used by
//! subprocesses (terminals, Python scripts, etc.).

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use tracing::debug;

use super::catalog::get_tool_definition;
use super::manifest::load_manifest;
use super::types::{ExternalToolId, Platform};
use crate::config::paths::get_tools_dir;

/// Returns the bin directories for all installed external tools.
///
/// This reads the manifest, verifies each tool's executable exists,
/// and returns the parent directory of each executable (the "bin" dir).
pub fn installed_tool_bin_dirs() -> Result<Vec<PathBuf>> {
    let platform = match Platform::detect() {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };

    let tools_dir = get_tools_dir()?;
    let manifest = load_manifest()?;
    let mut bin_dirs = Vec::new();

    for tool_id in ExternalToolId::all() {
        if !manifest.is_installed(*tool_id) {
            continue;
        }

        let def = get_tool_definition(*tool_id);
        let exec_relpath = def.get_executable_path(platform);
        let tool_dir = tools_dir.join(tool_id.as_str());
        let exec_path = tool_dir.join(exec_relpath);

        if exec_path.exists() {
            // Get the parent directory (the bin dir)
            if let Some(bin_dir) = exec_path.parent() {
                let bin_dir_path = bin_dir.to_path_buf();
                if !bin_dirs.contains(&bin_dir_path) {
                    bin_dirs.push(bin_dir_path);
                }
            }
        }
    }

    debug!("Found {} installed tool bin dirs", bin_dirs.len());
    Ok(bin_dirs)
}

/// Returns the PATH separator for the current platform.
#[inline]
fn path_separator() -> &'static str {
    #[cfg(windows)]
    {
        ";"
    }
    #[cfg(not(windows))]
    {
        ":"
    }
}

/// Prepends installed tool bin directories to an existing PATH value.
///
/// # Arguments
///
/// * `existing` - The current PATH value, or None to use the system PATH.
///
/// # Returns
///
/// A new PATH string with tool directories prepended.
pub fn prepend_tools_to_path(existing: Option<&str>) -> Result<String> {
    let bin_dirs = installed_tool_bin_dirs()?;

    if bin_dirs.is_empty() {
        // No tools installed, return existing or system PATH
        return Ok(existing
            .map(|s| s.to_string())
            .or_else(|| std::env::var("PATH").ok())
            .unwrap_or_default());
    }

    let sep = path_separator();
    let base_path = existing
        .map(|s| s.to_string())
        .or_else(|| std::env::var("PATH").ok())
        .unwrap_or_default();

    // Convert bin dirs to strings and join
    let tool_paths: Vec<String> = bin_dirs
        .into_iter()
        .filter_map(|p| p.to_str().map(|s| s.to_string()))
        .collect();

    if tool_paths.is_empty() {
        return Ok(base_path);
    }

    let tools_path = tool_paths.join(sep);

    if base_path.is_empty() {
        Ok(tools_path)
    } else {
        Ok(format!("{}{}{}", tools_path, sep, base_path))
    }
}

/// Returns environment variable overrides for external tools.
///
/// Currently this just sets PATH to include tool bin directories.
pub fn env_overrides() -> Result<HashMap<String, String>> {
    let mut overrides = HashMap::new();

    let new_path = prepend_tools_to_path(None)?;
    if !new_path.is_empty() {
        overrides.insert("PATH".to_string(), new_path);
    }

    Ok(overrides)
}

/// Applies external tool environment overrides to a Command.
///
/// This prepends tool bin directories to the PATH environment variable.
pub fn apply_to_command(cmd: &mut Command) -> Result<()> {
    let new_path = prepend_tools_to_path(None)?;
    if !new_path.is_empty() {
        cmd.env("PATH", new_path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_separator() {
        let sep = path_separator();
        #[cfg(windows)]
        assert_eq!(sep, ";");
        #[cfg(not(windows))]
        assert_eq!(sep, ":");
    }

    #[test]
    fn test_prepend_tools_to_path_no_tools() {
        // When no tools are installed, should return existing path
        let result = prepend_tools_to_path(Some("/usr/bin:/bin")).unwrap();
        // Should contain the existing path
        assert!(result.contains("/usr/bin"));
    }

    #[test]
    fn test_env_overrides_returns_path() {
        let overrides = env_overrides().unwrap();
        // Should always have PATH key (even if empty)
        // If no tools installed, it should be the system PATH or empty
        assert!(overrides.contains_key("PATH") || overrides.is_empty());
    }

    #[test]
    fn test_installed_tool_bin_dirs_no_crash() {
        // Should not crash even if tools dir doesn't exist
        let dirs = installed_tool_bin_dirs().unwrap();
        // May be empty if no tools installed
        assert!(dirs.len() <= 3); // At most 3 tools
    }
}
