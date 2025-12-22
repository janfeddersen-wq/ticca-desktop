//! Archive extraction for downloaded tool packages.
//!
//! This module handles extracting various archive formats (zip, tar.gz, tar.xz)
//! and setting executable permissions on Unix systems.

use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::{self, BufReader, Read, Write};
use std::path::Path;
use tracing::{debug, info, warn};

use super::types::ArchiveFormat;

// ============================================================================
// Archive Extraction
// ============================================================================

/// Extracts an archive to a destination directory.
///
/// # Arguments
///
/// * `archive_path` - Path to the archive file.
/// * `dest_dir` - Directory to extract into.
/// * `format` - The archive format.
///
/// # Errors
///
/// Returns an error if:
/// - The archive cannot be opened.
/// - Extraction fails.
/// - Files cannot be written to the destination.
pub fn extract_archive(archive_path: &Path, dest_dir: &Path, format: ArchiveFormat) -> Result<()> {
    info!(
        "Extracting {:?} archive {} to {}",
        format,
        archive_path.display(),
        dest_dir.display()
    );

    // Ensure destination directory exists
    fs::create_dir_all(dest_dir)
        .with_context(|| format!("Failed to create directory: {}", dest_dir.display()))?;

    match format {
        ArchiveFormat::Zip => extract_zip(archive_path, dest_dir),
        ArchiveFormat::TarGz => extract_tar_gz(archive_path, dest_dir),
        ArchiveFormat::TarXz => extract_tar_xz(archive_path, dest_dir),
        ArchiveFormat::AppImage => {
            // AppImages don't need extraction - they're single executables
            // This should be handled by the manager before calling extract_archive
            anyhow::bail!("AppImage files do not require extraction")
        }
    }
}

// ============================================================================
// ZIP Extraction
// ============================================================================

fn extract_zip(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = File::open(archive_path)
        .with_context(|| format!("Failed to open zip: {}", archive_path.display()))?;

    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("Failed to read zip: {}", archive_path.display()))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let entry_path = match entry.enclosed_name() {
            Some(path) => path.to_owned(),
            None => {
                debug!("Skipping unsafe path in zip");
                continue;
            }
        };

        let dest_path = dest_dir.join(&entry_path);

        if entry.is_dir() {
            fs::create_dir_all(&dest_path)?;
        } else {
            // Ensure parent directory exists
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut outfile = File::create(&dest_path)
                .with_context(|| format!("Failed to create: {}", dest_path.display()))?;

            io::copy(&mut entry, &mut outfile)?;

            // Set executable permissions on Unix
            #[cfg(unix)]
            set_unix_permissions(&dest_path, entry.unix_mode())?;
        }
    }

    debug!("ZIP extraction complete");
    Ok(())
}

// ============================================================================
// TAR.GZ Extraction
// ============================================================================

fn extract_tar_gz(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = File::open(archive_path)
        .with_context(|| format!("Failed to open tar.gz: {}", archive_path.display()))?;

    let reader = BufReader::new(file);
    let decoder = flate2::read::GzDecoder::new(reader);
    extract_tar(decoder, dest_dir)
}

// ============================================================================
// TAR.XZ Extraction
// ============================================================================

fn extract_tar_xz(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = File::open(archive_path)
        .with_context(|| format!("Failed to open tar.xz: {}", archive_path.display()))?;

    let reader = BufReader::new(file);
    let decoder = xz2::read::XzDecoder::new(reader);
    extract_tar(decoder, dest_dir)
}

// ============================================================================
// Common TAR Extraction
// ============================================================================

fn extract_tar<R: Read>(reader: R, dest_dir: &Path) -> Result<()> {
    let mut archive = tar::Archive::new(reader);
    let dest_dir_canonical = dest_dir
        .canonicalize()
        .unwrap_or_else(|_| dest_dir.to_path_buf());

    for entry_result in archive.entries()? {
        let mut entry = entry_result?;
        let entry_type = entry.header().entry_type();

        // Security: Skip symlinks and hardlinks entirely to prevent escape attacks
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            warn!("Skipping symlink/hardlink in tar archive (security)");
            continue;
        }

        let path = entry.path()?;

        // Security: skip absolute paths and paths with ..
        if path.is_absolute()
            || path
                .components()
                .any(|c| c == std::path::Component::ParentDir)
        {
            warn!("Skipping unsafe path in tar: {:?}", path);
            continue;
        }

        let dest_path = dest_dir.join(&path);

        // Security: Verify destination is within dest_dir (prevent escape via path tricks)
        let dest_canonical = if dest_path.exists() {
            dest_path.canonicalize()?
        } else {
            // For new files, canonicalize parent and append filename
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
                let parent_canonical = parent.canonicalize()?;
                parent_canonical.join(dest_path.file_name().unwrap_or_default())
            } else {
                dest_path.clone()
            }
        };

        if !dest_canonical.starts_with(&dest_dir_canonical) {
            warn!(
                "Skipping path that escapes dest_dir: {:?} -> {:?}",
                path, dest_canonical
            );
            continue;
        }

        if entry_type.is_dir() {
            fs::create_dir_all(&dest_path)?;
        } else if entry_type.is_file() {
            // Create parent directories
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Manually write file contents (don't use entry.unpack which follows symlinks)
            let mut outfile = File::create(&dest_path)
                .with_context(|| format!("Failed to create: {}", dest_path.display()))?;
            io::copy(&mut entry, &mut outfile)?;
            outfile.flush()?;

            // Set executable bit for files that had it in the archive
            #[cfg(unix)]
            {
                if let Ok(mode) = entry.header().mode() {
                    set_unix_permissions(&dest_path, Some(mode))?;
                }
            }
        }
        // Skip other entry types (devices, fifos, etc.)
    }

    debug!("TAR extraction complete");
    Ok(())
}

// ============================================================================
// Unix Permissions
// ============================================================================

#[cfg(unix)]
fn set_unix_permissions(path: &Path, mode: Option<u32>) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    if let Some(mode) = mode {
        // Preserve executable bits if set
        if mode & 0o111 != 0 {
            let permissions = fs::Permissions::from_mode(mode | 0o755);
            fs::set_permissions(path, permissions)
                .with_context(|| format!("Failed to set permissions on {}", path.display()))?;
        }
    }

    Ok(())
}

/// Sets executable permission on a file (Unix only).
///
/// On Windows, this is a no-op.
#[allow(unused_variables)]
pub fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let metadata = fs::metadata(path)
            .with_context(|| format!("Failed to get metadata for {}", path.display()))?;

        let mut permissions = metadata.permissions();
        let current_mode = permissions.mode();
        permissions.set_mode(current_mode | 0o755);

        fs::set_permissions(path, permissions).with_context(|| {
            format!("Failed to set executable permission on {}", path.display())
        })?;

        debug!("Set executable permission on {}", path.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_extract_zip_simple() {
        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("test.zip");
        let extract_dir = temp_dir.path().join("extracted");

        // Create a simple zip file
        {
            let file = File::create(&archive_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);

            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);

            zip.start_file("hello.txt", options).unwrap();
            zip.write_all(b"Hello, World!").unwrap();

            zip.start_file("subdir/nested.txt", options).unwrap();
            zip.write_all(b"Nested content").unwrap();

            zip.finish().unwrap();
        }

        // Extract it
        extract_archive(&archive_path, &extract_dir, ArchiveFormat::Zip).unwrap();

        // Verify
        assert!(extract_dir.join("hello.txt").exists());
        assert!(extract_dir.join("subdir/nested.txt").exists());

        let content = fs::read_to_string(extract_dir.join("hello.txt")).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_extract_tar_gz_simple() {
        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("test.tar.gz");
        let extract_dir = temp_dir.path().join("extracted");

        // Create a simple tar.gz file
        {
            let file = File::create(&archive_path).unwrap();
            let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
            let mut builder = tar::Builder::new(encoder);

            // Add a file
            let data = b"Hello from tar.gz!";
            let mut header = tar::Header::new_gnu();
            header.set_path("greetings.txt").unwrap();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();

            builder.append(&header, &data[..]).unwrap();
            builder.finish().unwrap();
        }

        // Extract it
        extract_archive(&archive_path, &extract_dir, ArchiveFormat::TarGz).unwrap();

        // Verify
        assert!(extract_dir.join("greetings.txt").exists());
        let content = fs::read_to_string(extract_dir.join("greetings.txt")).unwrap();
        assert_eq!(content, "Hello from tar.gz!");
    }

    #[cfg(unix)]
    #[test]
    fn test_make_executable() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("script.sh");

        // Create a file without executable permission
        {
            let mut file = File::create(&file_path).unwrap();
            file.write_all(b"#!/bin/bash\necho hello").unwrap();

            let permissions = fs::Permissions::from_mode(0o644);
            fs::set_permissions(&file_path, permissions).unwrap();
        }

        // Verify it's not executable
        let metadata = fs::metadata(&file_path).unwrap();
        assert_eq!(metadata.permissions().mode() & 0o111, 0);

        // Make it executable
        make_executable(&file_path).unwrap();

        // Verify it's now executable
        let metadata = fs::metadata(&file_path).unwrap();
        assert_ne!(metadata.permissions().mode() & 0o111, 0);
    }

    #[test]
    fn test_archive_format_detection() {
        // This is tested in types.rs, but let's double-check integration
        assert_eq!(
            ArchiveFormat::from_url("https://example.com/file.tar.gz"),
            Some(ArchiveFormat::TarGz)
        );
        assert_eq!(
            ArchiveFormat::from_url("https://example.com/file.tar.xz"),
            Some(ArchiveFormat::TarXz)
        );
        assert_eq!(
            ArchiveFormat::from_url("https://example.com/file.zip"),
            Some(ArchiveFormat::Zip)
        );
    }

    #[test]
    fn test_tar_symlink_escape_blocked() {
        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("malicious.tar.gz");
        let extract_dir = temp_dir.path().join("extracted");
        let escape_target = temp_dir.path().join("escaped_file.txt");

        // Create a malicious tar.gz with a symlink that tries to escape
        {
            let file = File::create(&archive_path).unwrap();
            let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
            let mut builder = tar::Builder::new(encoder);

            // Add a symlink pointing outside the extraction directory
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_path("escape_link").unwrap();
            header.set_size(0);
            header.set_mode(0o777);
            header.set_cksum();

            // Link target tries to escape to parent directory
            builder
                .append_link(&mut header, "escape_link", "../escaped_file.txt")
                .unwrap();

            // Add a regular file that would be written through the symlink if vulnerable
            let data = b"This should NOT appear outside extraction dir!";
            let mut file_header = tar::Header::new_gnu();
            file_header.set_path("escape_link").unwrap();
            file_header.set_size(data.len() as u64);
            file_header.set_mode(0o644);
            file_header.set_cksum();

            builder.append(&file_header, &data[..]).unwrap();
            builder.finish().unwrap();
        }

        // Extract it - should succeed but skip symlink
        extract_archive(&archive_path, &extract_dir, ArchiveFormat::TarGz).unwrap();

        // The escape target file should NOT exist
        assert!(
            !escape_target.exists(),
            "Symlink escape attack succeeded - file was written outside extraction dir!"
        );

        // The extraction directory should exist
        assert!(extract_dir.exists());
    }

    #[test]
    fn test_tar_parent_dir_escape_blocked() {
        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("malicious2.tar.gz");
        let extract_dir = temp_dir.path().join("extracted");
        let escape_target = temp_dir.path().join("escaped.txt");

        // Create a tar with a path containing .. using raw header manipulation
        // The tar crate's set_path rejects .., so we build the header manually
        {
            use std::io::Cursor;

            let file = File::create(&archive_path).unwrap();
            let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());

            // Build a raw tar archive with malicious path
            let malicious_path = b"subdir/../../../escaped.txt";
            let data = b"Escaped content";

            // Create 512-byte header block
            let mut header_block = [0u8; 512];
            // Path at offset 0 (100 bytes max)
            header_block[..malicious_path.len()].copy_from_slice(malicious_path);
            // Mode at offset 100 (8 bytes, octal)
            header_block[100..107].copy_from_slice(b"0000644");
            // UID/GID at offset 108/116 (8 bytes each)
            header_block[108..115].copy_from_slice(b"0001000");
            header_block[116..123].copy_from_slice(b"0001000");
            // Size at offset 124 (12 bytes, octal) - 15 bytes
            header_block[124..135].copy_from_slice(b"00000000017");
            // Mtime at offset 136 (12 bytes)
            header_block[136..147].copy_from_slice(b"00000000000");
            // Checksum placeholder at offset 148 (8 bytes) - fill with spaces initially
            header_block[148..156].copy_from_slice(b"        ");
            // Type flag at offset 156 - '0' for regular file
            header_block[156] = b'0';

            // Calculate checksum (sum of all bytes with checksum field as spaces)
            let checksum: u32 = header_block.iter().map(|&b| b as u32).sum();
            let checksum_str = format!("{:06o}\0 ", checksum);
            header_block[148..156].copy_from_slice(checksum_str.as_bytes());

            // Data block (padded to 512 bytes)
            let mut data_block = [0u8; 512];
            data_block[..data.len()].copy_from_slice(data);

            // End of archive (two zero blocks)
            let end_blocks = [0u8; 1024];

            // Write it all
            let mut cursor = Cursor::new(Vec::new());
            cursor.write_all(&header_block).unwrap();
            cursor.write_all(&data_block).unwrap();
            cursor.write_all(&end_blocks).unwrap();

            let mut gz = encoder;
            gz.write_all(&cursor.into_inner()).unwrap();
            gz.finish().unwrap();
        }

        // Extract it - should succeed but skip the malicious entry
        extract_archive(&archive_path, &extract_dir, ArchiveFormat::TarGz).unwrap();

        // The escape target file should NOT exist
        assert!(
            !escape_target.exists(),
            "Parent dir escape attack succeeded!"
        );
    }
}
