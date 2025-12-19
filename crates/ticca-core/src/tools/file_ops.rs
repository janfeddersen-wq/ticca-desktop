//! File operation tools: list_files, read_file

use crate::tools::registry::{ToolDefinition, ToolResult, ToolExecutor};
use crate::tools::spec;
use anyhow::Result;
use ignore::WalkBuilder;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Patterns to ignore when listing files
const IGNORE_PATTERNS: &[&str] = &[
    ".git",
    "node_modules",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    "target",
    "dist",
    "build",
    ".venv",
    "venv",
    ".env",
    "*.pyc",
    "*.pyo",
    ".DS_Store",
    "Thumbs.db",
    "*.egg-info",
    ".tox",
    ".nox",
    ".coverage",
    "htmlcov",
    ".gradle",
    ".idea",
    ".vscode",
];

/// Check if a directory is likely a home directory
fn is_home_directory(path: &Path) -> bool {
    let home = dirs::home_dir();
    if let Some(home_path) = home {
        if path == home_path {
            return true;
        }
        // Check common home subdirectories
        let common_subdirs = ["Documents", "Desktop", "Downloads", "Pictures", "Music", "Videos"];
        if let Some(parent) = path.parent() {
            if parent == home_path {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    return common_subdirs.contains(&name);
                }
            }
        }
    }
    false
}

/// Check if a directory looks like a project directory
fn is_project_directory(path: &Path) -> bool {
    let indicators = [
        "package.json",
        "pyproject.toml",
        "Cargo.toml",
        "pom.xml",
        "build.gradle",
        "CMakeLists.txt",
        ".git",
        "requirements.txt",
        "composer.json",
        "Gemfile",
        "go.mod",
        "Makefile",
        "setup.py",
    ];
    
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if indicators.contains(&name) {
                    return true;
                }
            }
        }
    }
    false
}

#[derive(Debug, Serialize)]
struct FileEntry {
    path: String,
    file_type: String,
    size: u64,
}

/// Check if a filename matches any ignore pattern
fn matches_ignore_pattern(name: &str) -> bool {
    IGNORE_PATTERNS.iter().any(|p| {
        if p.starts_with("*.") {
            name.ends_with(&p[1..])
        } else {
            name == *p
        }
    })
}

/// List files in a directory
pub fn list_files_impl(directory: &str, recursive: bool) -> Result<ToolResult> {
    let path = PathBuf::from(directory);
    let abs_path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(&path)
    };
    
    if !abs_path.exists() {
        return Ok(ToolResult::error(format!("Directory does not exist: {}", directory)));
    }
    
    if !abs_path.is_dir() {
        return Ok(ToolResult::error(format!("Not a directory: {}", directory)));
    }
    
    // Auto-disable recursion for home directories (unless it's a project)
    let effective_recursive = if recursive && is_home_directory(&abs_path) && !is_project_directory(&abs_path) {
        false
    } else {
        recursive
    };
    
    let mut output_lines = vec![format!(
        "DIRECTORY LISTING: {} (recursive={})",
        abs_path.display(),
        effective_recursive
    )];
    
    let mut entries: Vec<FileEntry> = Vec::new();
    
    if effective_recursive {
        // Use ignore crate for efficient recursive listing
        let walker = WalkBuilder::new(&abs_path)
            .hidden(false)
            .git_ignore(true)
            .git_global(false)
            .git_exclude(false)
            .max_depth(Some(10))  // Limit depth for safety
            .build();
        
        for entry in walker.flatten() {
            let entry_path = entry.path();
            if entry_path == abs_path {
                continue;  // Skip root directory itself
            }
            
            // Skip ignored patterns
            if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
                if matches_ignore_pattern(name) {
                    continue;
                }
            }
            
            let relative_path = entry_path.strip_prefix(&abs_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| entry_path.to_string_lossy().to_string());
            
            let file_type = if entry_path.is_dir() {
                "dir"
            } else if entry_path.is_symlink() {
                "symlink"
            } else {
                "file"
            };
            
            let size = entry_path.metadata().map(|m| m.len()).unwrap_or(0);
            
            entries.push(FileEntry {
                path: relative_path,
                file_type: file_type.to_string(),
                size,
            });
        }
    } else {
        // Non-recursive: just list immediate children
        for entry in fs::read_dir(&abs_path)? {
            let entry = entry?;
            let entry_path = entry.path();
            
            // Skip ignored patterns
            if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
                if matches_ignore_pattern(name) {
                    continue;
                }
            }
            
            let relative_path = entry_path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| entry_path.to_string_lossy().to_string());
            
            let file_type = if entry_path.is_dir() {
                "dir"
            } else if entry_path.is_symlink() {
                "symlink"
            } else {
                "file"
            };
            
            let size = entry_path.metadata().map(|m| m.len()).unwrap_or(0);
            
            entries.push(FileEntry {
                path: relative_path,
                file_type: file_type.to_string(),
                size,
            });
        }
    }
    
    // Sort entries: directories first, then files, alphabetically
    entries.sort_by(|a, b| {
        match (a.file_type.as_str(), b.file_type.as_str()) {
            ("dir", "dir") | ("file", "file") | ("symlink", "symlink") => a.path.cmp(&b.path),
            ("dir", _) => std::cmp::Ordering::Less,
            (_, "dir") => std::cmp::Ordering::Greater,
            _ => a.path.cmp(&b.path),
        }
    });
    
    // Format output
    for entry in &entries {
        let size_str = format_size(entry.size);
        let type_indicator = match entry.file_type.as_str() {
            "dir" => "/",
            "symlink" => "@",
            _ => "",
        };
        output_lines.push(format!("{}{} ({})", entry.path, type_indicator, size_str));
    }
    
    output_lines.push(format!("\nTotal: {} entries", entries.len()));
    
    Ok(ToolResult::success(output_lines.join("\n")))
}

/// Format file size in human-readable format
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Read a file with optional line range
pub fn read_file_impl(file_path: &str, start_line: Option<usize>, num_lines: Option<usize>) -> Result<ToolResult> {
    let path = PathBuf::from(file_path);
    
    if !path.exists() {
        return Ok(ToolResult::error(format!("File does not exist: {}", file_path)));
    }
    
    if !path.is_file() {
        return Ok(ToolResult::error(format!("Not a file: {}", file_path)));
    }
    
    let content = fs::read_to_string(&path)?;
    
    // Apply line range if specified
    let result_content = match (start_line, num_lines) {
        (Some(start), Some(count)) => {
            let lines: Vec<&str> = content.lines().collect();
            let start_idx = start.saturating_sub(1);  // Convert to 0-based
            let end_idx = (start_idx + count).min(lines.len());
            
            if start_idx >= lines.len() {
                return Ok(ToolResult::error(format!(
                    "Start line {} is beyond file length ({} lines)",
                    start, lines.len()
                )));
            }
            
            lines[start_idx..end_idx].join("\n")
        }
        (Some(_), None) => {
            return Ok(ToolResult::error("If start_line is specified, num_lines must also be specified"));
        }
        (None, Some(_)) => {
            return Ok(ToolResult::error("If num_lines is specified, start_line must also be specified"));
        }
        (None, None) => content,
    };
    
    // Check token limit (rough estimate: ~4 chars per token)
    let estimated_tokens = result_content.len() / 4;
    if estimated_tokens > 10000 {
        return Ok(ToolResult::error(format!(
            "File content too large (~{} tokens). Use start_line and num_lines to read a portion.",
            estimated_tokens
        )));
    }
    
    Ok(ToolResult::success(result_content))
}

/// Get list_files tool definition
pub fn list_files_definition() -> ToolDefinition {
    let spec = spec::list_files_spec();
    ToolDefinition {
        name: spec.name.to_string(),
        description: spec.description.to_string(),
        parameters: spec.registry_parameters,
    }
}

/// Get list_files executor
pub fn list_files_executor() -> ToolExecutor {
    Arc::new(|params: Value| {
        Box::pin(async move {
            let directory = params.get("directory")
                .and_then(|v| v.as_str())
                .unwrap_or(".");
            let recursive = params.get("recursive")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            
            list_files_impl(directory, recursive)
        })
    })
}

/// Get read_file tool definition
pub fn read_file_definition() -> ToolDefinition {
    let spec = spec::read_file_spec();
    ToolDefinition {
        name: spec.name.to_string(),
        description: spec.description.to_string(),
        parameters: spec.registry_parameters,
    }
}

/// Get read_file executor
pub fn read_file_executor() -> ToolExecutor {
    Arc::new(|params: Value| {
        Box::pin(async move {
            let file_path = params.get("path")
                .and_then(|v| v.as_str())
                .or_else(|| params.get("file_path").and_then(|v| v.as_str()))
                .ok_or_else(|| anyhow::anyhow!("path is required"))?;
            
            let start_line = params.get("start_line")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);
            
            let num_lines = params.get("num_lines")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);
            
            read_file_impl(file_path, start_line, num_lines)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_list_files_simple() {
        let dir = TempDir::new().unwrap();
        
        // Create some test files
        fs::write(dir.path().join("file1.txt"), "content").unwrap();
        fs::write(dir.path().join("file2.rs"), "fn main() {}").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();
        
        let result = list_files_impl(dir.path().to_str().unwrap(), false).unwrap();
        assert!(result.success);
        assert!(result.content.contains("file1.txt"));
        assert!(result.content.contains("file2.rs"));
        assert!(result.content.contains("subdir/"));
    }

    #[test]
    fn test_read_file_full() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "line1\nline2\nline3").unwrap();
        
        let result = read_file_impl(file_path.to_str().unwrap(), None, None).unwrap();
        assert!(result.success);
        assert_eq!(result.content, "line1\nline2\nline3");
    }

    #[test]
    fn test_read_file_with_range() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "line1\nline2\nline3\nline4\nline5").unwrap();
        
        let result = read_file_impl(file_path.to_str().unwrap(), Some(2), Some(2)).unwrap();
        assert!(result.success);
        assert_eq!(result.content, "line2\nline3");
    }

    #[test]
    fn test_read_file_not_found() {
        let result = read_file_impl("/nonexistent/file.txt", None, None).unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("does not exist"));
    }
}
