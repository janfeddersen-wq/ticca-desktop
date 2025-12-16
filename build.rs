//! Workspace-level build script for Ticca Desktop
//!
//! This build script handles:
//! - Python bundling configuration for PyOxidizer
//! - Compile-time checks and setup
//! - Asset embedding configuration

use std::env;
use std::path::Path;

fn main() {
    // Re-run build script if Python code changes
    println!("cargo:rerun-if-changed=python/");
    println!("cargo:rerun-if-changed=python/ticca_agent/");
    println!("cargo:rerun-if-changed=pyproject.toml");
    println!("cargo:rerun-if-changed=pyoxidizer.bzl");

    // Set up environment variables for Python bundling
    setup_python_bundling();

    // Print build info
    print_build_info();
}

/// Configure Python bundling paths and settings
fn setup_python_bundling() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let python_dir = Path::new(&manifest_dir).join("python");

    // Export Python source directory for embedded builds
    if python_dir.exists() {
        println!(
            "cargo:rustc-env=TICCA_PYTHON_DIR={}",
            python_dir.display()
        );
    }

    // Check if we're building with bundled Python
    let bundle_python = env::var("TICCA_BUNDLE_PYTHON")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false);

    if bundle_python {
        println!("cargo:rustc-cfg=bundled_python");
        println!("cargo:warning=Building with bundled Python interpreter");
    }

    // Set Python module path for development
    #[cfg(debug_assertions)]
    {
        if python_dir.exists() {
            println!(
                "cargo:rustc-env=PYTHONPATH={}",
                python_dir.display()
            );
        }
    }
}

/// Print build information for debugging
fn print_build_info() {
    let profile = env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:warning=Building Ticca Desktop");
    println!("cargo:warning=  Profile: {}", profile);
    println!("cargo:warning=  Target: {}", target);

    // Emit version info
    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());
    println!("cargo:rustc-env=TICCA_VERSION={}", version);

    // Build timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string());
    println!("cargo:rustc-env=TICCA_BUILD_TIMESTAMP={}", timestamp);
}
