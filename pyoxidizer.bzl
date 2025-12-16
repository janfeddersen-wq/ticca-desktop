# PyOxidizer configuration for Ticca Desktop
#
# This configuration bundles the Python interpreter and ticca_agent package
# into the final executable, eliminating the need for users to install Python.
#
# Usage:
#   pyoxidizer build                    # Build for current platform
#   pyoxidizer build --target-triple x86_64-apple-darwin  # Cross-compile
#   pyoxidizer run                      # Build and run

def make_exe():
    """Create the Python executable configuration."""
    
    # Get the default Python distribution for the target platform
    # This downloads a pre-built Python from GitHub releases
    dist = default_python_distribution(
        # Use Python 3.11 for best compatibility
        python_version="3.11",
    )
    
    # Configure packaging policy - how resources are bundled
    policy = dist.make_python_packaging_policy()
    
    # Prefer in-memory loading for faster startup
    # Fall back to filesystem for resources that can't be loaded in-memory
    policy.resources_location = "in-memory"
    policy.resources_location_fallback = "filesystem-relative:lib"
    
    # Include file resources (not just bytecode)
    policy.include_file_resources = True
    
    # Handle test files - exclude them in release builds
    policy.include_test = False
    
    # Configure the Python interpreter
    python_config = dist.make_python_interpreter_config()
    
    # Set the module to run when the executable starts
    # This will be overridden by the Rust code via pyo3
    python_config.run_module = "ticca_agent"
    
    # Optimize bytecode compilation
    python_config.optimization_level = 2
    
    # Don't buffer stdout/stderr for better streaming
    python_config.buffered_stdio = False
    
    # Enable UTF-8 mode
    python_config.utf8_mode = True
    
    # Create the executable
    exe = dist.to_python_executable(
        name="ticca-python",
        packaging_policy=policy,
        config=python_config,
    )
    
    # Add our ticca_agent package from the local python directory
    exe.add_python_resources(exe.pip_install(["./python"]))
    
    # Add required dependencies
    exe.add_python_resources(exe.pip_install([
        "pydantic>=2.0",
        "httpx>=0.24",
        "aiohttp>=3.8",
        "openai>=1.0",
        "anthropic>=0.18",
        "google-generativeai>=0.3",
        "tiktoken>=0.5",
    ]))
    
    return exe


def make_embedded_resources(exe):
    """Extract embedded resources for integration with Rust.
    
    This generates the oxidized_importer resources that can be
    embedded directly into the Rust binary.
    """
    return exe.to_embedded_resources()


def make_install(exe):
    """Create installation manifest for the built executable.
    
    This defines what files get installed and where.
    """
    files = FileManifest()
    
    # Add the main executable
    files.add_python_resource(".", exe)
    
    return files


def make_msi_installer(exe):
    """Create Windows MSI installer configuration.
    
    Only available on Windows targets.
    """
    return exe.to_wix_msi_builder(
        "ticca-desktop",
        "Ticca Desktop",
        "0.1.0",
        "Ticca Team",
    )


def make_macos_app_bundle(exe):
    """Create macOS application bundle configuration.
    
    Only available on macOS targets.
    """
    return exe.to_macos_application_bundle_builder(
        "Ticca",
        "io.ticca.desktop",
        "0.1.0",
    )


# Register build targets
# Use: pyoxidizer build <target_name>
register_target("exe", make_exe)
register_target("resources", make_embedded_resources, depends=["exe"])
register_target("install", make_install, depends=["exe"])

# Platform-specific installers (conditional registration would be ideal
# but PyOxidizer doesn't support that yet)
register_target("msi_installer", make_msi_installer, depends=["exe"])
register_target("macos_app_bundle", make_macos_app_bundle, depends=["exe"])

# Set default target
resolve_targets()
