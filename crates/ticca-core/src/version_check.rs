//! Version check functionality for checking GitHub releases.
//!
//! This module provides functionality to:
//! - Fetch the latest release from GitHub
//! - Compare versions using semver
//! - Determine if an update is available

use anyhow::{Context, Result};
use serde::Deserialize;

/// GitHub API endpoint for latest release
const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/janfeddersen-wq/ticca-desktop/releases/latest";

/// Current application version (from Cargo.toml)
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Information about a GitHub release
#[derive(Debug, Clone)]
pub struct LatestRelease {
    /// Version string without 'v' prefix (e.g., "0.16.0")
    pub version: String,
    /// Original tag name (e.g., "v0.16.0")
    pub tag_name: String,
    /// URL to the release page
    pub html_url: String,
}

/// GitHub API response structure
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
}

/// Fetches the latest release information from GitHub.
///
/// Returns `None` if the request fails or the response is invalid.
/// This is designed to fail silently - version checks should not
/// disrupt the user experience.
pub async fn fetch_latest_release() -> Result<LatestRelease> {
    let client = reqwest::Client::new();
    let response = client
        .get(GITHUB_RELEASES_URL)
        .header("User-Agent", "ticca-desktop")
        .header("Accept", "application/vnd.github+json")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .context("Failed to fetch releases from GitHub")?;

    if !response.status().is_success() {
        anyhow::bail!("GitHub API returned status {}", response.status());
    }

    let release: GitHubRelease = response
        .json()
        .await
        .context("Failed to parse GitHub release response")?;

    // Strip 'v' prefix from tag name if present
    let version = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name)
        .to_string();

    Ok(LatestRelease {
        version,
        tag_name: release.tag_name,
        html_url: release.html_url,
    })
}

/// Compares two version strings and returns true if `latest` is newer than `current`.
///
/// Uses semver parsing for accurate comparison. Returns `false` if either
/// version string is invalid (fail-safe behavior).
pub fn is_newer_version(current: &str, latest: &str) -> bool {
    match (
        semver::Version::parse(current),
        semver::Version::parse(latest),
    ) {
        (Ok(curr), Ok(lat)) => lat > curr,
        _ => {
            tracing::warn!(
                "Failed to parse versions for comparison: current={}, latest={}",
                current,
                latest
            );
            false
        }
    }
}

/// Checks if an update is available and returns the release info if so.
///
/// This function:
/// 1. Fetches the latest release from GitHub
/// 2. Compares it to the current version
/// 3. Returns the release info only if a newer version exists
///
/// Returns `None` if:
/// - The fetch fails (network error, API error, etc.)
/// - The current version is up to date
/// - The dismissed version matches the latest version
pub async fn check_for_update(dismissed_version: Option<&str>) -> Option<LatestRelease> {
    match fetch_latest_release().await {
        Ok(release) => {
            // Check if this version was dismissed
            if dismissed_version.is_some_and(|dismissed| dismissed == release.version) {
                tracing::debug!(
                    "Skipping update notification: version {} was dismissed",
                    release.version
                );
                return None;
            }

            // Check if it's actually newer
            if is_newer_version(CURRENT_VERSION, &release.version) {
                tracing::info!(
                    "Update available: {} -> {}",
                    CURRENT_VERSION,
                    release.version
                );
                Some(release)
            } else {
                tracing::debug!(
                    "No update available: current={}, latest={}",
                    CURRENT_VERSION,
                    release.version
                );
                None
            }
        }
        Err(e) => {
            tracing::debug!("Failed to check for updates: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("0.15.0", "0.16.0"));
        assert!(is_newer_version("0.15.0", "1.0.0"));
        assert!(is_newer_version("0.15.0", "0.15.1"));
        assert!(!is_newer_version("0.16.0", "0.15.0"));
        assert!(!is_newer_version("0.15.0", "0.15.0"));
        assert!(!is_newer_version("1.0.0", "0.99.99"));
    }

    #[test]
    fn test_is_newer_version_invalid() {
        // Invalid versions should return false
        assert!(!is_newer_version("invalid", "0.16.0"));
        assert!(!is_newer_version("0.15.0", "invalid"));
        assert!(!is_newer_version("invalid", "also-invalid"));
    }

    #[test]
    fn test_current_version_is_valid() {
        // Ensure the embedded version is a valid semver
        let version = semver::Version::parse(CURRENT_VERSION);
        assert!(
            version.is_ok(),
            "CURRENT_VERSION '{}' should be valid semver",
            CURRENT_VERSION
        );
    }
}
