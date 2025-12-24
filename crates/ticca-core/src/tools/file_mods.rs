//! File modification tools: edit_file, delete_file

use crate::tools::registry::{ToolDefinition, ToolExecutor, ToolResult};
use crate::tools::spec;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

/// Payload types for edit_file
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EditPayload {
    Content(ContentPayload),
    Replacements(ReplacementsPayload),
    DeleteSnippet(DeleteSnippetPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPayload {
    pub file_path: String,
    pub content: String,
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplacementsPayload {
    pub file_path: String,
    pub replacements: Vec<Replacement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replacement {
    pub old_str: String,
    pub new_str: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteSnippetPayload {
    pub file_path: String,
    pub delete_snippet: String,
}

/// Generate a unified diff between old and new content
fn generate_diff(old_content: &str, new_content: &str, file_path: &str) -> String {
    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();

    let mut diff_lines = vec![
        format!("--- a/{}", file_path),
        format!("+++  b/{}", file_path),
    ];

    // Simple diff: show removed and added lines
    // For a proper diff, you'd use a diff library like `similar`
    let mut i = 0;
    let mut j = 0;

    while i < old_lines.len() || j < new_lines.len() {
        if i < old_lines.len() && j < new_lines.len() && old_lines[i] == new_lines[j] {
            diff_lines.push(format!(" {}", old_lines[i]));
            i += 1;
            j += 1;
        } else if i < old_lines.len()
            && (j >= new_lines.len() || !new_lines.contains(&old_lines[i]))
        {
            diff_lines.push(format!("-{}", old_lines[i]));
            i += 1;
        } else if j < new_lines.len() {
            diff_lines.push(format!("+{}", new_lines[j]));
            j += 1;
        }
    }

    diff_lines.join("\n")
}

/// Edit a file using content replacement
fn edit_with_content(payload: &ContentPayload) -> Result<ToolResult> {
    let path = PathBuf::from(&payload.file_path);

    // Check if file exists and we're not allowing overwrite
    if path.exists() && !payload.overwrite {
        return Ok(ToolResult::error(format!(
            "File already exists: {}. Set overwrite=true to replace it.",
            payload.file_path
        )));
    }

    // Ensure parent directory exists
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent)?;
    }

    let old_content = if path.exists() {
        fs::read_to_string(&path).unwrap_or_default()
    } else {
        String::new()
    };

    // Write the new content
    fs::write(&path, &payload.content)?;

    let diff = if old_content.is_empty() {
        format!(
            "Created new file: {} ({} bytes)",
            payload.file_path,
            payload.content.len()
        )
    } else {
        generate_diff(&old_content, &payload.content, &payload.file_path)
    };

    Ok(ToolResult::success(format!(
        "File '{}' {} successfully.\n\n{}",
        payload.file_path,
        if old_content.is_empty() {
            "created"
        } else {
            "overwritten"
        },
        diff
    )))
}

/// Edit a file using text replacements
fn edit_with_replacements(payload: &ReplacementsPayload) -> Result<ToolResult> {
    let path = PathBuf::from(&payload.file_path);

    if !path.exists() {
        return Ok(ToolResult::error(format!(
            "File does not exist: {}",
            payload.file_path
        )));
    }

    let old_content = fs::read_to_string(&path)?;
    let mut new_content = old_content.clone();
    let mut applied_count = 0;
    let mut errors = Vec::new();

    for (idx, replacement) in payload.replacements.iter().enumerate() {
        if new_content.contains(&replacement.old_str) {
            new_content = new_content.replace(&replacement.old_str, &replacement.new_str);
            applied_count += 1;
        } else {
            errors.push(format!(
                "Replacement #{}: Could not find '{}'",
                idx + 1,
                truncate_string(&replacement.old_str, 50)
            ));
        }
    }

    if applied_count == 0 {
        return Ok(ToolResult::error(format!(
            "No replacements applied. Errors:\n{}",
            errors.join("\n")
        )));
    }

    // Write the updated content
    fs::write(&path, &new_content)?;

    let diff = generate_diff(&old_content, &new_content, &payload.file_path);

    let mut result_msg = format!(
        "Applied {}/{} replacements to '{}'.\n\n{}",
        applied_count,
        payload.replacements.len(),
        payload.file_path,
        diff
    );

    if !errors.is_empty() {
        result_msg.push_str(&format!("\n\nWarnings:\n{}", errors.join("\n")));
    }

    Ok(ToolResult::success(result_msg))
}

/// Edit a file by deleting a snippet
fn edit_with_delete_snippet(payload: &DeleteSnippetPayload) -> Result<ToolResult> {
    let path = PathBuf::from(&payload.file_path);

    if !path.exists() {
        return Ok(ToolResult::error(format!(
            "File does not exist: {}",
            payload.file_path
        )));
    }

    let old_content = fs::read_to_string(&path)?;

    if !old_content.contains(&payload.delete_snippet) {
        return Ok(ToolResult::error(format!(
            "Snippet not found in file: '{}'",
            truncate_string(&payload.delete_snippet, 50)
        )));
    }

    let new_content = old_content.replace(&payload.delete_snippet, "");

    // Write the updated content
    fs::write(&path, &new_content)?;

    let diff = generate_diff(&old_content, &new_content, &payload.file_path);

    Ok(ToolResult::success(format!(
        "Deleted snippet from '{}'.\n\n{}",
        payload.file_path, diff
    )))
}

/// Truncate a string for display purposes (char-safe, not byte-safe)
fn truncate_string(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

/// Main edit_file implementation
pub fn edit_file_impl(params: Value) -> Result<ToolResult> {
    // Try to parse as different payload types

    // Check for content payload
    if params.get("content").is_some() {
        let payload: ContentPayload = serde_json::from_value(params)?;
        return edit_with_content(&payload);
    }

    // Check for replacements payload
    if params.get("replacements").is_some() {
        let payload: ReplacementsPayload = serde_json::from_value(params)?;
        return edit_with_replacements(&payload);
    }

    // Check for delete_snippet payload
    if params.get("delete_snippet").is_some() {
        let payload: DeleteSnippetPayload = serde_json::from_value(params)?;
        return edit_with_delete_snippet(&payload);
    }

    Ok(ToolResult::error(
        "Invalid edit_file payload. Must contain either 'content', 'replacements', or 'delete_snippet'.",
    ))
}

/// Delete a file
pub fn delete_file_impl(file_path: &str) -> Result<ToolResult> {
    let path = PathBuf::from(file_path);

    if !path.exists() {
        return Ok(ToolResult::error(format!(
            "File does not exist: {}",
            file_path
        )));
    }

    if !path.is_file() {
        return Ok(ToolResult::error(format!(
            "Not a regular file: {}",
            file_path
        )));
    }

    // Read content before deletion for diff
    let old_content = fs::read_to_string(&path).unwrap_or_default();

    // Delete the file
    fs::remove_file(&path)?;

    let diff = generate_diff(&old_content, "", file_path);

    Ok(ToolResult::success(format!(
        "File '{}' deleted successfully.\n\n{}",
        file_path, diff
    )))
}

/// Get edit_file tool definition
pub fn edit_file_definition() -> ToolDefinition {
    let spec = spec::edit_file_spec();
    ToolDefinition {
        name: spec.name.to_string(),
        description: spec.description.to_string(),
        parameters: spec.registry_parameters,
    }
}

/// Get edit_file executor
pub fn edit_file_executor() -> ToolExecutor {
    Arc::new(|params: Value| {
        Box::pin(async move {
            let mut params = params;
            if params.get("file_path").is_none()
                && let Some(path) = params.get("path").cloned()
            {
                params["file_path"] = path;
            }
            edit_file_impl(params)
        })
    })
}

/// Get delete_file tool definition
pub fn delete_file_definition() -> ToolDefinition {
    let spec = spec::delete_file_spec();
    ToolDefinition {
        name: spec.name.to_string(),
        description: spec.description.to_string(),
        parameters: spec.registry_parameters,
    }
}

/// Get delete_file executor
pub fn delete_file_executor() -> ToolExecutor {
    Arc::new(|params: Value| {
        Box::pin(async move {
            let file_path = params
                .get("path")
                .and_then(|v| v.as_str())
                .or_else(|| params.get("file_path").and_then(|v| v.as_str()))
                .ok_or_else(|| anyhow::anyhow!("path is required"))?;

            delete_file_impl(file_path)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_new_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("new_file.txt");

        let payload = ContentPayload {
            file_path: file_path.to_str().unwrap().to_string(),
            content: "Hello, World!".to_string(),
            overwrite: false,
        };

        let result = edit_with_content(&payload).unwrap();
        assert!(result.success);
        assert!(file_path.exists());
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "Hello, World!");
    }

    #[test]
    fn test_overwrite_existing_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("existing.txt");
        fs::write(&file_path, "old content").unwrap();

        // Should fail without overwrite flag
        let payload = ContentPayload {
            file_path: file_path.to_str().unwrap().to_string(),
            content: "new content".to_string(),
            overwrite: false,
        };
        let result = edit_with_content(&payload).unwrap();
        assert!(!result.success);

        // Should succeed with overwrite flag
        let payload = ContentPayload {
            file_path: file_path.to_str().unwrap().to_string(),
            content: "new content".to_string(),
            overwrite: true,
        };
        let result = edit_with_content(&payload).unwrap();
        assert!(result.success);
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "new content");
    }

    #[test]
    fn test_replacements() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("replace.txt");
        fs::write(&file_path, "Hello, World! Hello, Rust!").unwrap();

        let payload = ReplacementsPayload {
            file_path: file_path.to_str().unwrap().to_string(),
            replacements: vec![Replacement {
                old_str: "Hello".to_string(),
                new_str: "Hi".to_string(),
            }],
        };

        let result = edit_with_replacements(&payload).unwrap();
        assert!(result.success);
        assert_eq!(
            fs::read_to_string(&file_path).unwrap(),
            "Hi, World! Hi, Rust!"
        );
    }

    #[test]
    fn test_delete_snippet() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("delete.txt");
        fs::write(&file_path, "Keep this. Delete this. Keep this too.").unwrap();

        let payload = DeleteSnippetPayload {
            file_path: file_path.to_str().unwrap().to_string(),
            delete_snippet: "Delete this. ".to_string(),
        };

        let result = edit_with_delete_snippet(&payload).unwrap();
        assert!(result.success);
        assert_eq!(
            fs::read_to_string(&file_path).unwrap(),
            "Keep this. Keep this too."
        );
    }

    #[test]
    fn test_delete_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("to_delete.txt");
        fs::write(&file_path, "content").unwrap();

        let result = delete_file_impl(file_path.to_str().unwrap()).unwrap();
        assert!(result.success);
        assert!(!file_path.exists());
    }
}
