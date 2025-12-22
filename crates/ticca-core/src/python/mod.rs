//! Python environment management using UV.
//!
//! This module provides functionality for managing Python virtual environments
//! and running Python scripts using the embedded UV binary.
//!
//! UV is a fast Python package installer and resolver written in Rust.
//! We embed it in the Ticca binary for each supported platform.
//!
//! Security: The embedded binary is verified against a SHA256 checksum at runtime.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tracing::{debug, info, warn};

use crate::config::paths::{get_bin_dir, get_uv_binary_path};
use crate::external_tools;

// ============================================================================
// Platform-specific UV binary embedding
// ============================================================================

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
static UV_BINARY: &[u8] = include_bytes!("../../../../vendor/uv/x86_64-unknown-linux-gnu/uv");

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
static UV_BINARY: &[u8] = include_bytes!("../../../../vendor/uv/aarch64-unknown-linux-gnu/uv");

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
static UV_BINARY: &[u8] = include_bytes!("../../../../vendor/uv/x86_64-apple-darwin/uv");

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
static UV_BINARY: &[u8] = include_bytes!("../../../../vendor/uv/aarch64-apple-darwin/uv");

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
static UV_BINARY: &[u8] = include_bytes!("../../../../vendor/uv/x86_64-pc-windows-msvc/uv.exe");

// Provide a helpful error for unsupported platforms
#[cfg(not(any(
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "linux", target_arch = "aarch64"),
    all(target_os = "macos", target_arch = "x86_64"),
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "windows", target_arch = "x86_64"),
)))]
compile_error!(
    "Unsupported platform for UV binary. \
     Supported platforms: Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (x86_64). \
     Please open an issue if you need support for another platform."
);

// ============================================================================
// Platform-specific SHA256 checksums for UV binary integrity verification
// These are updated by running: ./scripts/download-uv.sh --print-checksums
// ============================================================================

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const UV_EXPECTED_SHA256: &str = "0e05d828b5708e8a927724124db3746396afddad6273c47283d7c562dc795bd6";

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
const UV_EXPECTED_SHA256: &str = "b3d9f8a55c56ead9b6facf8f00a9f809a45ad9c27b3ec85faecab4a3e8252fa4";

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const UV_EXPECTED_SHA256: &str = "f3e45a01e92788435f98ed7fb84f410e77acbfa8bb03eb84c4399f573e1f05b9";

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const UV_EXPECTED_SHA256: &str = "415f73cab3771902db58f6a9ce4d9cf3e664a3eed26ee7e48a051453a79bd015";

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const UV_EXPECTED_SHA256: &str = "055d55eec85a91cfb5e9c8bc7f6463f9883866796c5bcb205fbcdfed9c088c88";

// ============================================================================
// UV binary management
// ============================================================================

/// Verifies the integrity of the embedded UV binary against its expected SHA256 hash.
///
/// This provides defense-in-depth against binary tampering - even if someone
/// modifies the binary after download but before compilation, this check will catch it.
///
/// # Errors
///
/// Returns an error if the computed hash doesn't match the expected hash.
fn verify_uv_integrity() -> Result<()> {
    let mut hasher = Sha256::new();
    hasher.update(UV_BINARY);
    let computed_hash = format!("{:x}", hasher.finalize());

    if computed_hash != UV_EXPECTED_SHA256 {
        anyhow::bail!(
            "UV binary integrity check failed!\n\
             Expected SHA256: {}\n\
             Computed SHA256: {}\n\
             This could indicate binary tampering. Please re-download UV binaries.",
            UV_EXPECTED_SHA256,
            computed_hash
        );
    }

    debug!("UV binary integrity verified (SHA256: {})", &computed_hash[..16]);
    Ok(())
}

/// Ensures the UV binary is available and returns its path.
///
/// This function:
/// 1. Verifies the embedded binary integrity (SHA256 check)
/// 2. Gets the UV binary path from the centralized paths module
/// 3. Checks if the binary exists and has the correct size (quick version check)
/// 4. If not, writes the embedded binary to disk
/// 5. On Unix, sets executable permissions (0o755)
///
/// # Returns
///
/// The path to the UV binary.
///
/// # Errors
///
/// Returns an error if:
/// - The embedded binary fails integrity verification
/// - The bin directory cannot be created
/// - The binary cannot be written to disk
/// - Permissions cannot be set (Unix only)
///
/// # Example
///
/// ```ignore
/// let uv_path = ensure_uv_available()?;
/// println!("UV available at: {}", uv_path.display());
/// ```
pub fn ensure_uv_available() -> Result<PathBuf> {
    // Security: Verify embedded binary integrity before extraction
    verify_uv_integrity()?;

    let uv_path = get_uv_binary_path()?;
    let expected_size = UV_BINARY.len() as u64;

    // Check if binary exists with correct size
    let needs_extraction = if uv_path.exists() {
        match fs::metadata(&uv_path) {
            Ok(metadata) => {
                if metadata.len() == expected_size {
                    debug!("UV binary up-to-date at {}", uv_path.display());
                    false
                } else {
                    info!(
                        "UV binary size mismatch ({} vs {}), re-extracting",
                        metadata.len(),
                        expected_size
                    );
                    true
                }
            }
            Err(e) => {
                warn!("Failed to read UV binary metadata, re-extracting: {}", e);
                true
            }
        }
    } else {
        info!("UV binary not found, extracting...");
        true
    };

    if needs_extraction {
        extract_uv_binary(&uv_path)?;
    }

    Ok(uv_path)
}

/// Extracts the embedded UV binary to the target path.
fn extract_uv_binary(target_path: &PathBuf) -> Result<()> {
    // Ensure parent directory exists
    let bin_dir = get_bin_dir()?;
    fs::create_dir_all(&bin_dir)
        .with_context(|| format!("Failed to create bin directory: {}", bin_dir.display()))?;

    // Write the binary
    fs::write(target_path, UV_BINARY)
        .with_context(|| format!("Failed to write UV binary to {}", target_path.display()))?;

    // Set executable permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = fs::Permissions::from_mode(0o755);
        fs::set_permissions(target_path, permissions)
            .with_context(|| "Failed to set UV binary permissions")?;
    }

    info!("UV binary extracted to {}", target_path.display());
    Ok(())
}

// ============================================================================
// Virtual environment management
// ============================================================================

/// Returns the path to the Python executable in a virtual environment.
///
/// - Unix: `{venv_path}/bin/python`
/// - Windows: `{venv_path}/Scripts/python.exe`
pub fn get_venv_python(venv_path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        venv_path.join("Scripts").join("python.exe")
    }
    #[cfg(not(windows))]
    {
        venv_path.join("bin").join("python")
    }
}

/// Creates a Python virtual environment at the specified path.
///
/// Uses UV to create the virtual environment, which is significantly
/// faster than the standard `python -m venv` approach.
///
/// # Arguments
///
/// * `venv_path` - Path where the virtual environment should be created
///
/// # Errors
///
/// Returns an error if:
/// - UV binary cannot be extracted
/// - The UV command fails
///
/// # Example
///
/// ```ignore
/// let venv_path = get_venvs_dir()?.join("my-skill");
/// create_venv(&venv_path)?;
/// ```
pub fn create_venv(venv_path: &Path) -> Result<()> {
    let uv_path = ensure_uv_available()?;

    info!("Creating virtual environment at {}", venv_path.display());

    let mut cmd = Command::new(&uv_path);
    cmd.arg("venv").arg(venv_path);

    // Apply external tools PATH so UV can find any required binaries
    if let Err(e) = external_tools::apply_to_command(&mut cmd) {
        warn!("Failed to apply external tools env to UV venv: {}", e);
    }

    let output = cmd
        .output()
        .with_context(|| "Failed to execute UV venv command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to create virtual environment: {}", stderr);
    }

    debug!("Virtual environment created successfully");
    Ok(())
}

/// Installs Python packages into a virtual environment using UV.
///
/// # Arguments
///
/// * `venv_path` - Path to the virtual environment
/// * `packages` - List of packages to install (e.g., `["requests", "playwright==1.40.0"]`)
///
/// # Errors
///
/// Returns an error if:
/// - UV binary cannot be extracted
/// - The UV pip install command fails
///
/// # Example
///
/// ```ignore
/// pip_install(&venv_path, &["requests", "beautifulsoup4"])?;
/// ```
pub fn pip_install(venv_path: &Path, packages: &[&str]) -> Result<()> {
    if packages.is_empty() {
        return Ok(());
    }

    let uv_path = ensure_uv_available()?;
    let python_path = get_venv_python(venv_path);

    info!(
        "Installing packages into {}: {:?}",
        venv_path.display(),
        packages
    );

    let mut cmd = Command::new(&uv_path);
    cmd.arg("pip")
        .arg("install")
        .arg("--python")
        .arg(&python_path);

    for package in packages {
        cmd.arg(package);
    }

    // Apply external tools PATH so UV can find any required binaries
    if let Err(e) = external_tools::apply_to_command(&mut cmd) {
        warn!("Failed to apply external tools env to UV pip install: {}", e);
    }

    let output = cmd.output().with_context(|| "Failed to execute UV pip install")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to install packages: {}", stderr);
    }

    debug!("Packages installed successfully");
    Ok(())
}

/// Installs Python packages from a requirements file into a virtual environment.
///
/// # Arguments
///
/// * `venv_path` - Path to the virtual environment
/// * `requirements_file` - Path to the requirements.txt file
///
/// # Errors
///
/// Returns an error if:
/// - UV binary cannot be extracted
/// - The requirements file doesn't exist
/// - The UV pip install command fails
pub fn pip_install_requirements(venv_path: &Path, requirements_file: &Path) -> Result<()> {
    let uv_path = ensure_uv_available()?;
    let python_path = get_venv_python(venv_path);

    info!(
        "Installing requirements from {} into {}",
        requirements_file.display(),
        venv_path.display()
    );

    let mut cmd = Command::new(&uv_path);
    cmd.arg("pip")
        .arg("install")
        .arg("--python")
        .arg(&python_path)
        .arg("-r")
        .arg(requirements_file);

    // Apply external tools PATH so UV can find any required binaries
    if let Err(e) = external_tools::apply_to_command(&mut cmd) {
        warn!("Failed to apply external tools env to UV pip install -r: {}", e);
    }

    let output = cmd
        .output()
        .with_context(|| "Failed to execute UV pip install -r")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to install requirements: {}", stderr);
    }

    debug!("Requirements installed successfully");
    Ok(())
}

// ============================================================================
// Python script execution
// ============================================================================

/// Runs a Python script using the Python interpreter from a virtual environment.
///
/// # Arguments
///
/// * `venv_path` - Path to the virtual environment
/// * `script` - Path to the Python script to run
/// * `args` - Arguments to pass to the script
///
/// # Returns
///
/// The output of the command (stdout, stderr, exit status).
///
/// # Errors
///
/// Returns an error if:
/// - The Python executable cannot be found
/// - The command fails to execute
///
/// # Example
///
/// ```ignore
/// let output = run_python_script(
///     &venv_path,
///     Path::new("scripts/my_script.py"),
///     &["--input", "data.json"]
/// )?;
/// println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
/// ```
pub fn run_python_script(venv_path: &Path, script: &Path, args: &[&str]) -> Result<Output> {
    let python_path = get_venv_python(venv_path);

    if !python_path.exists() {
        anyhow::bail!(
            "Python executable not found at {}. Was the venv created?",
            python_path.display()
        );
    }

    debug!(
        "Running Python script: {} {:?} with args {:?}",
        python_path.display(),
        script.display(),
        args
    );

    let mut cmd = Command::new(&python_path);
    cmd.arg(script);
    for arg in args {
        cmd.arg(arg);
    }

    // Apply external tools PATH so Python subprocess can find pandoc/node etc.
    if let Err(e) = external_tools::apply_to_command(&mut cmd) {
        warn!("Failed to apply external tools env to Python script: {}", e);
    }

    let output = cmd
        .output()
        .with_context(|| format!("Failed to execute Python script: {}", script.display()))?;

    Ok(output)
}

/// Runs a Python module using the `-m` flag.
///
/// # Arguments
///
/// * `venv_path` - Path to the virtual environment
/// * `module` - The module to run (e.g., "playwright", "http.server")
/// * `args` - Arguments to pass to the module
///
/// # Returns
///
/// The output of the command.
///
/// # Example
///
/// ```ignore
/// let output = run_python_module(&venv_path, "playwright", &["install", "chromium"])?;
/// ```
pub fn run_python_module(venv_path: &Path, module: &str, args: &[&str]) -> Result<Output> {
    let python_path = get_venv_python(venv_path);

    if !python_path.exists() {
        anyhow::bail!(
            "Python executable not found at {}. Was the venv created?",
            python_path.display()
        );
    }

    debug!("Running Python module: {} -m {} {:?}", python_path.display(), module, args);

    let mut cmd = Command::new(&python_path);
    cmd.arg("-m").arg(module);
    for arg in args {
        cmd.arg(arg);
    }

    // Apply external tools PATH so Python subprocess can find pandoc/node etc.
    if let Err(e) = external_tools::apply_to_command(&mut cmd) {
        warn!("Failed to apply external tools env to Python module: {}", e);
    }

    let output = cmd
        .output()
        .with_context(|| format!("Failed to execute Python module: {}", module))?;

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uv_binary_is_embedded() {
        // Just verify the binary data is present and non-empty
        assert!(!UV_BINARY.is_empty(), "UV binary should be embedded");
        // UV binaries are typically > 20MB
        assert!(
            UV_BINARY.len() > 20_000_000,
            "UV binary should be larger than 20MB, got {} bytes",
            UV_BINARY.len()
        );
    }

    #[test]
    fn test_uv_binary_integrity() {
        // Verify the embedded binary passes integrity check
        let result = verify_uv_integrity();
        assert!(
            result.is_ok(),
            "UV binary integrity check should pass: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_get_venv_python_unix() {
        let venv = Path::new("/tmp/test-venv");
        let python = get_venv_python(venv);

        #[cfg(windows)]
        assert!(python.ends_with("Scripts/python.exe"));

        #[cfg(not(windows))]
        assert!(python.ends_with("bin/python"));
    }

    #[test]
    fn test_ensure_uv_available() {
        // This test actually extracts the binary, which is fine for integration testing
        let result = ensure_uv_available();
        assert!(result.is_ok(), "ensure_uv_available should succeed");

        let uv_path = result.unwrap();
        assert!(uv_path.exists(), "UV binary should exist after extraction");

        // Verify it's executable by checking the file
        let metadata = fs::metadata(&uv_path).unwrap();
        assert!(metadata.len() > 0, "UV binary should not be empty");
    }

    #[test]
    fn test_uv_binary_runs() {
        // Extract UV and verify it can execute
        let uv_path = ensure_uv_available().expect("Failed to extract UV");

        // Run `uv --version`
        let output = Command::new(&uv_path)
            .arg("--version")
            .output()
            .expect("Failed to execute UV");

        assert!(output.status.success(), "UV --version should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("uv"),
            "UV version output should contain 'uv', got: {}",
            stdout
        );
    }

    #[test]
    #[ignore] // Run with `cargo test -- --ignored` - requires Python installed
    fn test_create_venv_and_check_python() {
        use tempfile::TempDir;

        // Create a temporary directory for the venv
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let venv_path = temp_dir.path().join("test-venv");

        // Create the virtual environment
        create_venv(&venv_path).expect("Failed to create venv");

        // Verify the Python binary exists
        let python_path = get_venv_python(&venv_path);
        assert!(
            python_path.exists(),
            "Python binary should exist at {}",
            python_path.display()
        );

        // Verify Python runs
        let output = Command::new(&python_path)
            .arg("--version")
            .output()
            .expect("Failed to run Python");

        assert!(output.status.success(), "Python --version should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let version_output = format!("{}{}", stdout, stderr);
        assert!(
            version_output.contains("Python"),
            "Python version output should contain 'Python', got: {}",
            version_output
        );

        // Temp dir is automatically cleaned up when dropped
    }

    #[test]
    #[ignore] // Run with `cargo test -- --ignored` - requires Python installed
    fn test_pip_install_package() {
        use tempfile::TempDir;

        // Create a temporary directory for the venv
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let venv_path = temp_dir.path().join("test-venv");

        // Create the virtual environment
        create_venv(&venv_path).expect("Failed to create venv");

        // Install a small package (pip is a good test since it's always available)
        pip_install(&venv_path, &["pip", "--upgrade"]).expect("Failed to upgrade pip");

        // Verify pip is available and runs
        let python_path = get_venv_python(&venv_path);
        let output = Command::new(&python_path)
            .args(["-m", "pip", "--version"])
            .output()
            .expect("Failed to run pip");

        assert!(output.status.success(), "pip --version should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("pip"),
            "pip version output should contain 'pip', got: {}",
            stdout
        );
    }

    #[test]
    #[ignore] // Run with `cargo test -- --ignored` - requires Python installed
    fn test_run_python_script() {
        use std::io::Write;
        use tempfile::{NamedTempFile, TempDir};

        // Create a temporary directory for the venv
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let venv_path = temp_dir.path().join("test-venv");

        // Create the virtual environment
        create_venv(&venv_path).expect("Failed to create venv");

        // Create a simple Python script
        let mut script_file = NamedTempFile::new().expect("Failed to create temp file");
        writeln!(script_file, "import sys").unwrap();
        writeln!(script_file, "print('Hello from Python!')").unwrap();
        writeln!(script_file, "print('Args:', sys.argv[1:])").unwrap();
        script_file.flush().unwrap();

        // Run the script
        let output = run_python_script(&venv_path, script_file.path(), &["arg1", "arg2"])
            .expect("Failed to run Python script");

        assert!(output.status.success(), "Python script should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Hello from Python!"),
            "Script output should contain greeting, got: {}",
            stdout
        );
        assert!(
            stdout.contains("arg1") && stdout.contains("arg2"),
            "Script output should contain args, got: {}",
            stdout
        );
    }
}
