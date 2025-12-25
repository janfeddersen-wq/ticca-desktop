//! Grep tool using ripgrep libraries

use crate::tools::registry::{ToolDefinition, ToolExecutor, ToolResult};
use crate::tools::spec;
use anyhow::Result;
use grep_regex::RegexMatcher;
use grep_searcher::{Searcher, Sink, SinkMatch};
use ignore::WalkBuilder;
use serde::Serialize;
use serde_json::Value;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

/// Maximum number of matches to return (reduced to prevent context blowup)
const MAX_MATCHES: usize = 50;

/// Maximum matches per file to avoid one file dominating results
const MAX_MATCHES_PER_FILE: usize = 10;

/// Maximum line content length before truncation (chars)
const MAX_LINE_LENGTH: usize = 512;

/// Maximum file size to search (5MB) - skip larger files
const MAX_FILE_SIZE: u64 = 5 * 1024 * 1024;

/// A single match found by grep
#[derive(Debug, Clone, Serialize)]
pub struct GrepMatch {
    pub file_path: String,
    pub line_number: u64,
    pub line_content: String,
    /// Number of characters truncated from line content (0 if not truncated)
    #[serde(skip_serializing_if = "is_zero")]
    pub truncated_chars: usize,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// Truncate a string to max_chars, returning (truncated_string, chars_removed)
fn truncate_line(s: &str, max_chars: usize) -> (String, usize) {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        (s.to_string(), 0)
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        (truncated, char_count - max_chars)
    }
}

/// Sink implementation to collect matches
struct MatchCollector {
    matches: Vec<GrepMatch>,
    file_path: String,
    max_matches: usize,
    max_per_file: usize,
    file_match_count: usize,
}

impl Sink for MatchCollector {
    type Error = io::Error;

    fn matched(&mut self, _searcher: &Searcher, mat: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        // Stop if we've hit the global limit
        if self.matches.len() >= self.max_matches {
            return Ok(false);
        }

        // Stop searching this file if we've hit the per-file limit
        if self.file_match_count >= self.max_per_file {
            return Ok(false);
        }

        let raw_content = String::from_utf8_lossy(mat.bytes()).trim().to_string();
        let (line_content, truncated_chars) = truncate_line(&raw_content, MAX_LINE_LENGTH);

        self.matches.push(GrepMatch {
            file_path: self.file_path.clone(),
            line_number: mat.line_number().unwrap_or(0),
            line_content,
            truncated_chars,
        });

        self.file_match_count += 1;
        Ok(true)
    }
}

/// Search for a pattern in files
pub fn grep_impl(search_string: &str, directory: &str) -> Result<ToolResult> {
    if search_string.is_empty() {
        return Ok(ToolResult::error("Search string cannot be empty"));
    }

    let path = PathBuf::from(directory);
    let abs_path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(&path)
    };

    if !abs_path.exists() {
        return Ok(ToolResult::error(format!(
            "Directory does not exist: {}",
            directory
        )));
    }

    if !abs_path.is_dir() {
        return Ok(ToolResult::error(format!("Not a directory: {}", directory)));
    }

    // Build the regex matcher
    // Support for common ripgrep flags in the search string
    let (pattern, case_insensitive) =
        if search_string.starts_with("--ignore-case ") || search_string.starts_with("-i ") {
            let pattern = search_string
                .trim_start_matches("--ignore-case ")
                .trim_start_matches("-i ");
            (pattern.to_string(), true)
        } else {
            (search_string.to_string(), false)
        };

    // Build regex pattern with case sensitivity option
    let regex_pattern = if case_insensitive {
        format!("(?i){}", pattern)
    } else {
        pattern.clone()
    };

    let matcher = RegexMatcher::new_line_matcher(&regex_pattern).or_else(|_| {
        // If regex fails, try as literal
        let escaped = regex::escape(&pattern);
        let escaped_pattern = if case_insensitive {
            format!("(?i){}", escaped)
        } else {
            escaped
        };
        RegexMatcher::new_line_matcher(&escaped_pattern)
    })?;

    let mut all_matches: Vec<GrepMatch> = Vec::new();

    // Walk the directory tree
    let walker = WalkBuilder::new(&abs_path)
        .hidden(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .max_depth(Some(10))
        .build();

    let mut searcher = Searcher::new();

    let mut skipped_large_files = 0usize;
    let mut files_with_more_matches = 0usize;

    for entry in walker.flatten() {
        if all_matches.len() >= MAX_MATCHES {
            break;
        }

        let entry_path = entry.path();

        // Skip directories and non-text files
        if entry_path.is_dir() {
            continue;
        }

        // Skip large files (> 5MB) to avoid slow searches
        if let Ok(metadata) = entry_path.metadata()
            && metadata.len() > MAX_FILE_SIZE
        {
            skipped_large_files += 1;
            continue;
        }

        // Skip binary files by extension
        if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
            let binary_extensions = [
                "exe", "dll", "so", "dylib", "bin", "obj", "o", "a", "png", "jpg", "jpeg", "gif",
                "bmp", "ico", "svg", "mp3", "mp4", "wav", "avi", "mov", "zip", "tar", "gz", "bz2",
                "xz", "7z", "rar", "pdf", "doc", "docx", "xls", "xlsx", "db", "sqlite", "sqlite3",
                "wasm", "pyc", "pyo", "class", "jar", "war", "ear",
            ];
            if binary_extensions.contains(&ext.to_lowercase().as_str()) {
                continue;
            }
        }

        let relative_path = entry_path
            .strip_prefix(&abs_path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| entry_path.to_string_lossy().to_string());

        let mut collector = MatchCollector {
            matches: Vec::new(),
            file_path: relative_path,
            max_matches: MAX_MATCHES - all_matches.len(),
            max_per_file: MAX_MATCHES_PER_FILE,
            file_match_count: 0,
        };

        // Search the file
        let search_result = searcher.search_path(&matcher, entry_path, &mut collector);

        if search_result.is_ok() {
            // Track if we hit the per-file limit (meaning there were more matches)
            if collector.file_match_count >= MAX_MATCHES_PER_FILE {
                files_with_more_matches += 1;
            }
            all_matches.extend(collector.matches);
        }
        // Silently ignore files that can't be read (binary, permission issues, etc.)
    }

    if all_matches.is_empty() {
        return Ok(ToolResult::success(format!(
            "No matches found for '{}' in {}",
            pattern, directory
        )));
    }

    // Format output
    let mut notes = Vec::new();

    if all_matches.len() >= MAX_MATCHES {
        notes.push(format!("limited to {} results", MAX_MATCHES));
    }
    if files_with_more_matches > 0 {
        notes.push(format!(
            "{} file(s) had more matches (capped at {} per file)",
            files_with_more_matches, MAX_MATCHES_PER_FILE
        ));
    }
    if skipped_large_files > 0 {
        notes.push(format!(
            "skipped {} file(s) larger than 5MB",
            skipped_large_files
        ));
    }

    let notes_str = if notes.is_empty() {
        String::new()
    } else {
        format!(" ({})", notes.join(", "))
    };

    let mut output_lines = vec![format!(
        "Found {} matches for '{}' in {}{}:",
        all_matches.len(),
        pattern,
        directory,
        notes_str
    )];

    for m in &all_matches {
        let truncation_note = if m.truncated_chars > 0 {
            format!(" [...{} more chars]", m.truncated_chars)
        } else {
            String::new()
        };
        output_lines.push(format!(
            "{}:{}:{}{}",
            m.file_path, m.line_number, m.line_content, truncation_note
        ));
    }

    Ok(ToolResult::success(output_lines.join("\n")))
}

/// Get grep tool definition
pub fn grep_definition() -> ToolDefinition {
    let spec = spec::grep_spec(MAX_MATCHES);
    ToolDefinition {
        name: spec.name.to_string(),
        description: format!(
            "{} Returns up to {} matches ({} per file max). Lines over {} chars are truncated. Files over 5MB are skipped.",
            spec.description, MAX_MATCHES, MAX_MATCHES_PER_FILE, MAX_LINE_LENGTH
        ),
        parameters: spec.registry_parameters,
    }
}

/// Get grep executor
pub fn grep_executor() -> ToolExecutor {
    Arc::new(|params: Value| {
        Box::pin(async move {
            let search_string = params
                .get("search_string")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("search_string is required"))?;

            let directory = params
                .get("directory")
                .and_then(|v| v.as_str())
                .unwrap_or(".");

            grep_impl(search_string, directory)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_grep_simple() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("test.txt"),
            "Hello, World!\nHello, Rust!\nGoodbye!",
        )
        .unwrap();
        fs::write(dir.path().join("other.txt"), "No match here").unwrap();

        let result = grep_impl("Hello", dir.path().to_str().unwrap()).unwrap();
        assert!(result.success);
        assert!(result.content.contains("2 matches"));
        assert!(result.content.contains("test.txt"));
    }

    #[test]
    fn test_grep_no_matches() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("test.txt"), "Hello, World!").unwrap();

        let result = grep_impl("NotFound", dir.path().to_str().unwrap()).unwrap();
        assert!(result.success);
        assert!(result.content.contains("No matches found"));
    }

    #[test]
    fn test_grep_case_insensitive() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("test.txt"), "Hello, WORLD!\nhello, world!").unwrap();

        let result = grep_impl("-i hello", dir.path().to_str().unwrap()).unwrap();
        assert!(result.success);
        assert!(result.content.contains("2 matches"));
    }

    #[test]
    fn test_grep_regex() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("test.txt"), "foo123\nfoo456\nbar789").unwrap();

        let result = grep_impl(r"foo\d+", dir.path().to_str().unwrap()).unwrap();
        assert!(result.success);
        assert!(result.content.contains("2 matches"));
    }
}
