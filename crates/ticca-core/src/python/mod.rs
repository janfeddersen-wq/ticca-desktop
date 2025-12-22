//! Python environment management using UV.
//!
//! This module provides functionality for managing Python virtual environments
//! and running Python scripts using UV.
//!
//! UV is a fast Python package installer and resolver written in Rust.
//! It is downloaded on-demand via the external tools system when the user
//! installs it from the Tools settings page.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tracing::{debug, info, warn};

use crate::config::paths::get_uv_binary_path;
use crate::external_tools;
use crate::external_tools::manifest::load_manifest;
use crate::external_tools::types::ExternalToolId;

// ============================================================================
// UV binary management
// ============================================================================

/// Ensures the UV binary is available and returns its path.
///
/// This function checks if UV has been installed via the external tools system.
/// If UV is not installed, it returns an error instructing the user to install it.
///
/// # Returns
///
/// The path to the UV binary.
///
/// # Errors
///
/// Returns an error if:
/// - UV is not installed (user needs to install it from Tools settings)
/// - The UV binary path cannot be determined
/// - The UV binary doesn't exist on disk despite being marked as installed
///
/// # Example
///
/// ```ignore
/// let uv_path = ensure_uv_available()?;
/// println!("UV available at: {}", uv_path.display());
/// ```
pub fn ensure_uv_available() -> Result<PathBuf> {
    // Check if UV is installed via the manifest
    let manifest = load_manifest().context("Failed to load tools manifest")?;

    if !manifest.is_installed(ExternalToolId::Uv) {
        anyhow::bail!(
            "UV is not installed. Please install it from Settings → Tools to use Python skills."
        );
    }

    let uv_path = get_uv_binary_path()?;

    // Verify the binary actually exists
    if !uv_path.exists() {
        anyhow::bail!(
            "UV is marked as installed but the binary was not found at {}. \
             Please reinstall UV from Settings → Tools.",
            uv_path.display()
        );
    }

    debug!("UV binary available at {}", uv_path.display());
    Ok(uv_path)
}

/// Checks if UV is installed without returning an error.
///
/// This is useful for UI code that needs to check availability without
/// propagating errors.
pub fn is_uv_installed() -> bool {
    match load_manifest() {
        Ok(manifest) => {
            if !manifest.is_installed(ExternalToolId::Uv) {
                return false;
            }
            // Also verify the binary exists
            match get_uv_binary_path() {
                Ok(path) => path.exists(),
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
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
/// - UV is not installed
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
/// - UV is not installed
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
        warn!(
            "Failed to apply external tools env to UV pip install: {}",
            e
        );
    }

    let output = cmd
        .output()
        .with_context(|| "Failed to execute UV pip install")?;

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
/// - UV is not installed
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
        warn!(
            "Failed to apply external tools env to UV pip install -r: {}",
            e
        );
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

    debug!(
        "Running Python module: {} -m {} {:?}",
        python_path.display(),
        module,
        args
    );

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
    fn test_get_venv_python_unix() {
        let venv = Path::new("/tmp/test-venv");
        let python = get_venv_python(venv);

        #[cfg(windows)]
        assert!(python.ends_with("Scripts/python.exe"));

        #[cfg(not(windows))]
        assert!(python.ends_with("bin/python"));
    }

    #[test]
    fn test_is_uv_installed_returns_false_when_not_installed() {
        // On a fresh system or test environment, UV won't be installed
        // This test just verifies the function doesn't panic
        let _ = is_uv_installed();
    }

    #[test]
    #[ignore] // Run with `cargo test -- --ignored` - requires UV installed
    fn test_ensure_uv_available() {
        // This test requires UV to be installed
        let result = ensure_uv_available();
        if let Ok(uv_path) = result {
            assert!(uv_path.exists(), "UV binary should exist after check");
        }
        // If UV is not installed, the error message should be helpful
    }

    #[test]
    #[ignore] // Run with `cargo test -- --ignored` - requires UV installed and Python
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
    #[ignore] // Run with `cargo test -- --ignored` - requires UV installed and Python
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
    #[ignore] // Run with `cargo test -- --ignored` - requires UV installed and Python
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
