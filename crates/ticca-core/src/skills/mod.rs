//! Skills bundle management and discovery.
//!
//! This module handles:
//! - Extracting the embedded skills bundle to the data directory
//! - Discovering available skills and parsing their metadata
//!
//! Skills are Python scripts and other assets that extend Ticca's capabilities.
//! The skills bundle is embedded at compile time and extracted on first run or
//! when the bundle version changes.
//!
//! Each skill lives in its own subdirectory under `{config_dir}/skills/` and
//! contains a `SKILL.md` file with YAML frontmatter describing the skill.

pub mod discovery;

pub use discovery::{SkillMetadata, discover_skills, parse_skill_frontmatter};

use anyhow::{Context, Result};
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use tracing::{debug, info, warn};
use zip::ZipArchive;

use crate::config::paths::get_skills_dir;

/// The embedded skills bundle (skills.zip from project root).
static SKILLS_BUNDLE: &[u8] = include_bytes!("../../../../skills.zip");

/// Version file name used to track the current bundle version.
const VERSION_FILE: &str = ".version";

/// Computes a simple version marker for the embedded bundle.
///
/// This uses the bundle length and a hash of the first 1KB to create
/// a version string that changes when the bundle content changes.
fn compute_bundle_version() -> String {
    let len = SKILLS_BUNDLE.len();

    // Simple hash of first 1KB (or less if bundle is smaller)
    let sample_size = std::cmp::min(1024, len);
    let sample = &SKILLS_BUNDLE[..sample_size];

    // Simple checksum: sum of bytes with position weighting
    let checksum: u64 = sample
        .iter()
        .enumerate()
        .map(|(i, &b)| (b as u64).wrapping_mul((i + 1) as u64))
        .fold(0u64, |acc, x| acc.wrapping_add(x));

    format!("v1-{}-{:016x}", len, checksum)
}

/// Extracts the skills bundle if needed and returns the skills directory path.
///
/// This function:
/// 1. Gets the skills directory from the centralized paths module
/// 2. Checks if a `.version` file exists with the current checksum
/// 3. If missing or checksum differs, extracts the ZIP contents
/// 4. Writes the new checksum to `.version`
/// 5. Returns the skills directory path
///
/// # Errors
///
/// Returns an error if:
/// - The skills directory cannot be created
/// - The ZIP archive cannot be read or extracted
/// - File I/O operations fail
///
/// # Example
///
/// ```ignore
/// // Call at application startup
/// let skills_dir = extract_skills_if_needed()?;
/// println!("Skills available at: {}", skills_dir.display());
/// ```
pub fn extract_skills_if_needed() -> Result<PathBuf> {
    let skills_dir = get_skills_dir()?;
    let version_file = skills_dir.join(VERSION_FILE);
    let current_version = compute_bundle_version();

    // Check if extraction is needed
    let needs_extraction = if version_file.exists() {
        match fs::read_to_string(&version_file) {
            Ok(existing_version) => {
                let existing = existing_version.trim();
                if existing == current_version {
                    debug!("Skills bundle up-to-date (version: {})", current_version);
                    false
                } else {
                    info!(
                        "Skills bundle version changed: {} -> {}",
                        existing, current_version
                    );
                    true
                }
            }
            Err(e) => {
                warn!("Failed to read version file, will re-extract: {}", e);
                true
            }
        }
    } else {
        info!("Skills bundle not found, extracting...");
        true
    };

    if needs_extraction {
        extract_bundle(&skills_dir)?;
        fs::write(&version_file, &current_version)
            .with_context(|| format!("Failed to write version file: {}", version_file.display()))?;
        info!(
            "Skills bundle extracted to {} (version: {})",
            skills_dir.display(),
            current_version
        );
    }

    Ok(skills_dir)
}

/// Extracts the embedded ZIP bundle to the target directory.
fn extract_bundle(target_dir: &PathBuf) -> Result<()> {
    // Ensure target directory exists
    fs::create_dir_all(target_dir).with_context(|| {
        format!(
            "Failed to create skills directory: {}",
            target_dir.display()
        )
    })?;

    let cursor = Cursor::new(SKILLS_BUNDLE);
    let mut archive = ZipArchive::new(cursor).context("Failed to read skills bundle as ZIP")?;

    debug!("Extracting {} files from skills bundle", archive.len());

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .with_context(|| format!("Failed to read ZIP entry {}", i))?;

        // Security: Skip symlinks to prevent path traversal attacks
        if file.is_symlink() {
            warn!(
                "Skipping symlink in ZIP archive: {:?} (security restriction)",
                file.name()
            );
            continue;
        }

        let outpath = match file.enclosed_name() {
            Some(path) => target_dir.join(path),
            None => {
                warn!("Skipping ZIP entry with unsafe path: {:?}", file.name());
                continue;
            }
        };

        if file.is_dir() {
            fs::create_dir_all(&outpath)
                .with_context(|| format!("Failed to create directory: {}", outpath.display()))?;
        } else {
            // Ensure parent directory exists
            if let Some(parent) = outpath.parent()
                && !parent.exists()
            {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create parent directory: {}", parent.display())
                })?;
            }

            // Extract file
            let mut outfile = fs::File::create(&outpath)
                .with_context(|| format!("Failed to create file: {}", outpath.display()))?;

            std::io::copy(&mut file, &mut outfile)
                .with_context(|| format!("Failed to write file: {}", outpath.display()))?;

            // Set sanitized permissions on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    // Security: Strip SUID, SGID, sticky bits (keep only 0o0777)
                    // and prevent world-writable files (mask with 0o0755)
                    let sanitized_mode = (mode & 0o0777) & 0o0755;
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(sanitized_mode)).ok();
                }
            }
        }
    }

    Ok(())
}

/// Returns the path to a specific skill file.
///
/// This is a convenience function that combines the skills directory
/// with a relative path to a skill file.
///
/// # Arguments
///
/// * `relative_path` - Path relative to the skills directory
///
/// # Example
///
/// ```ignore
/// let script = get_skill_path("browser/playwright_tool.py")?;
/// ```
pub fn get_skill_path(relative_path: &str) -> Result<PathBuf> {
    Ok(get_skills_dir()?.join(relative_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_bundle_version_is_stable() {
        let v1 = compute_bundle_version();
        let v2 = compute_bundle_version();
        assert_eq!(v1, v2, "Version should be deterministic");
    }

    #[test]
    fn test_compute_bundle_version_format() {
        let version = compute_bundle_version();
        assert!(version.starts_with("v1-"), "Version should start with v1-");
        assert!(
            version.len() > 20,
            "Version should include length and checksum"
        );
    }

    #[test]
    fn test_skills_bundle_is_valid_zip() {
        let cursor = Cursor::new(SKILLS_BUNDLE);
        let archive = ZipArchive::new(cursor);
        assert!(archive.is_ok(), "Embedded bundle should be valid ZIP");

        let archive = archive.unwrap();
        assert!(!archive.is_empty(), "Bundle should contain files");
    }

    #[test]
    fn test_extract_bundle_to_temp_dir() {
        use tempfile::TempDir;

        // Create a temporary directory
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let target_path = temp_dir.path().to_path_buf();

        // Extract the bundle
        extract_bundle(&target_path).expect("Failed to extract bundle");

        // Verify some files were extracted
        let entries: Vec<_> = fs::read_dir(&target_path)
            .expect("Failed to read target dir")
            .filter_map(|e| e.ok())
            .collect();

        assert!(
            !entries.is_empty(),
            "Extracted bundle should contain files/directories"
        );

        // Temp dir is automatically cleaned up
    }

    #[test]
    fn test_skills_bundle_contains_expected_structure() {
        let cursor = Cursor::new(SKILLS_BUNDLE);
        let mut archive = ZipArchive::new(cursor).expect("Failed to read ZIP");

        // Collect all file names
        let mut file_names: Vec<String> = Vec::new();
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                file_names.push(file.name().to_string());
            }
        }

        // Verify we have some content
        assert!(!file_names.is_empty(), "Bundle should contain files");

        // Log the structure for debugging
        println!("Skills bundle contains {} entries", file_names.len());
        for name in &file_names[..std::cmp::min(10, file_names.len())] {
            println!("  - {}", name);
        }
    }

    #[test]
    fn test_extract_skills_creates_version_file() {
        // This test uses the actual skills directory
        let result = extract_skills_if_needed();
        assert!(result.is_ok(), "extract_skills_if_needed should succeed");

        let skills_dir = result.unwrap();
        let version_file = skills_dir.join(VERSION_FILE);

        assert!(
            version_file.exists(),
            "Version file should exist at {}",
            version_file.display()
        );

        let version_content = fs::read_to_string(&version_file).expect("Failed to read version");
        assert!(
            version_content.starts_with("v1-"),
            "Version file should contain valid version, got: {}",
            version_content
        );
    }

    #[test]
    fn test_get_skill_path_constructs_correct_path() {
        let path = get_skill_path("test/example.py").expect("Failed to get skill path");

        assert!(
            path.ends_with("skills/test/example.py"),
            "Path should end with skills/test/example.py, got: {}",
            path.display()
        );
    }
}
