//! Async file downloader with progress reporting and security validation.
//!
//! This module provides streaming download functionality using reqwest,
//! with progress callbacks for UI integration, URL validation, and
//! optional SHA256 checksum verification.

use anyhow::{Context, Result};
use futures::StreamExt;
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info};
use url::Url;

// ============================================================================
// URL Security Validation
// ============================================================================

/// Allowed domains for downloading external tools.
const ALLOWED_DOMAINS: &[&str] = &[
    "github.com",
    "nodejs.org",
    "download.documentfoundation.org",
    "libreitalia.org", // Official LibreOffice AppImage mirror
];

/// Validates that a URL is safe for downloading.
///
/// Checks:
/// - URL scheme must be HTTPS
/// - Host must be in the allowed domain list
fn validate_url(url_str: &str) -> Result<()> {
    let url = Url::parse(url_str).with_context(|| format!("Invalid URL: {}", url_str))?;

    // Must be HTTPS
    if url.scheme() != "https" {
        anyhow::bail!("URL must use HTTPS: {}", url_str);
    }

    // Must have a valid host
    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("URL must have a host: {}", url_str))?;

    // Check if host matches allowed domains (including subdomains)
    let is_allowed = ALLOWED_DOMAINS.iter().any(|domain| {
        host == *domain || host.ends_with(&format!(".{}", domain))
    });

    if !is_allowed {
        anyhow::bail!(
            "Download domain not allowed: {}. Allowed: {:?}",
            host,
            ALLOWED_DOMAINS
        );
    }

    Ok(())
}

// ============================================================================
// Download Progress
// ============================================================================

/// Progress information during a download.
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// Bytes downloaded so far.
    pub bytes_downloaded: u64,
    /// Total bytes expected (if known from Content-Length header).
    pub total_bytes: Option<u64>,
    /// Progress percentage (0.0 to 100.0), or None if total is unknown.
    pub percent: Option<f32>,
}

impl DownloadProgress {
    fn new(bytes_downloaded: u64, total_bytes: Option<u64>) -> Self {
        let percent = total_bytes.map(|total| {
            if total > 0 {
                (bytes_downloaded as f32 / total as f32) * 100.0
            } else {
                0.0
            }
        });

        Self {
            bytes_downloaded,
            total_bytes,
            percent,
        }
    }
}

// ============================================================================
// Download Function
// ============================================================================

/// Downloads a file from a URL with streaming and progress reporting.
///
/// # Arguments
///
/// * `url` - The URL to download from (must be HTTPS from an allowed domain).
/// * `dest` - The destination file path.
/// * `expected_sha256` - Optional SHA256 hash to verify (lowercase hex string).
/// * `progress_cb` - A callback invoked with progress updates.
///
/// # Returns
///
/// The total number of bytes downloaded.
///
/// # Errors
///
/// Returns an error if:
/// - The URL is not HTTPS or from an allowed domain.
/// - The network request fails.
/// - The server returns a non-success status code.
/// - The file cannot be created or written.
/// - The SHA256 checksum does not match (if provided).
pub async fn download_file<F>(
    url: &str,
    dest: &Path,
    expected_sha256: Option<&str>,
    progress_cb: F,
) -> Result<u64>
where
    F: Fn(DownloadProgress),
{
    info!("Downloading {} to {}", url, dest.display());

    // Security: Validate URL before downloading
    validate_url(url)?;

    // Ensure parent directory exists
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    // Start the request
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to start download from {}", url))?;

    // Check for success status
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!(
            "Download failed with status {}: {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("Unknown error")
        );
    }

    // Get content length if available
    let total_bytes = response.content_length();
    debug!("Content-Length: {:?}", total_bytes);

    // Create the destination file
    let mut file = File::create(dest)
        .await
        .with_context(|| format!("Failed to create file: {}", dest.display()))?;

    // Stream the response body while computing SHA256
    let mut stream = response.bytes_stream();
    let mut bytes_downloaded: u64 = 0;
    let mut hasher = Sha256::new();

    // Report initial progress
    progress_cb(DownloadProgress::new(0, total_bytes));

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result
            .with_context(|| "Failed to read chunk from response stream")?;

        // Update hash
        hasher.update(&chunk);

        file.write_all(&chunk)
            .await
            .with_context(|| "Failed to write chunk to file")?;

        bytes_downloaded += chunk.len() as u64;
        progress_cb(DownloadProgress::new(bytes_downloaded, total_bytes));
    }

    // Ensure all data is flushed to disk
    file.flush().await.context("Failed to flush file")?;

    // Verify SHA256 if expected
    if let Some(expected) = expected_sha256 {
        let actual_hash = hasher.finalize();
        let actual_hex = format_sha256_hex(&actual_hash);

        if actual_hex != expected.to_lowercase() {
            // Delete the corrupted file
            let _ = tokio::fs::remove_file(dest).await;
            anyhow::bail!(
                "SHA256 checksum mismatch!\nExpected: {}\nActual: {}",
                expected,
                actual_hex
            );
        }
        debug!("SHA256 verified: {}", actual_hex);
    }

    info!(
        "Download complete: {} bytes written to {}",
        bytes_downloaded,
        dest.display()
    );

    Ok(bytes_downloaded)
}

/// Formats a SHA256 hash as lowercase hex without using the hex crate.
fn format_sha256_hex(hash: &[u8]) -> String {
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Downloads a file without progress reporting or checksum verification.
///
/// This is a convenience wrapper around `download_file` that ignores progress updates.
pub async fn download_file_simple(url: &str, dest: &Path) -> Result<u64> {
    download_file(url, dest, None, |_| {}).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_progress_calculation() {
        let progress = DownloadProgress::new(50, Some(100));
        assert_eq!(progress.bytes_downloaded, 50);
        assert_eq!(progress.total_bytes, Some(100));
        assert_eq!(progress.percent, Some(50.0));

        let progress_no_total = DownloadProgress::new(50, None);
        assert_eq!(progress_no_total.percent, None);

        let progress_zero_total = DownloadProgress::new(0, Some(0));
        assert_eq!(progress_zero_total.percent, Some(0.0));
    }

    #[test]
    fn test_download_progress_full() {
        let progress = DownloadProgress::new(100, Some(100));
        assert_eq!(progress.percent, Some(100.0));
    }

    #[test]
    fn test_validate_url_https_required() {
        // HTTP should be rejected
        assert!(validate_url("http://github.com/file.zip").is_err());

        // HTTPS should be accepted (for allowed domains)
        assert!(validate_url("https://github.com/file.zip").is_ok());
    }

    #[test]
    fn test_validate_url_allowed_domains() {
        // Allowed domains
        assert!(validate_url("https://github.com/jgm/pandoc/releases/download/file.zip").is_ok());
        assert!(validate_url("https://nodejs.org/dist/v22.12.0/node.tar.xz").is_ok());
        assert!(validate_url("https://download.documentfoundation.org/libreoffice/file.tar.gz").is_ok());

        // Subdomains of allowed domains should work
        assert!(validate_url("https://objects.githubusercontent.com/file.zip").is_err()); // Not a subdomain of github.com

        // Disallowed domains
        assert!(validate_url("https://evil.com/malware.zip").is_err());
        assert!(validate_url("https://github.com.evil.org/fake.zip").is_err());
    }

    #[test]
    fn test_validate_url_invalid() {
        assert!(validate_url("not-a-url").is_err());
        assert!(validate_url("").is_err());
        assert!(validate_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn test_format_sha256_hex() {
        // Known SHA256 of empty input
        let empty_hash = sha2::Sha256::digest(b"");
        let hex = format_sha256_hex(&empty_hash);
        assert_eq!(
            hex,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    // Integration tests would require a mock HTTP server or real network access.
    // Those are better suited for manual testing or integration test suites.
}
