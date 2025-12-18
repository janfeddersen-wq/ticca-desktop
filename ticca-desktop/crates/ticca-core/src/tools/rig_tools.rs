//! Rig-compatible tool wrappers for the native Rust tools
//!
//! These wrappers implement rig's `Tool` trait to enable the ReAct loop.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

/// Shared context for all tools - primarily the working directory
#[derive(Clone)]
pub struct ToolContext {
    pub working_directory: PathBuf,
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

// ============================================================================
// Shell Tool
// ============================================================================

#[derive(Debug, thiserror::Error)]
#[error("Shell error: {0}")]
pub struct ShellError(String);

#[derive(Debug, Clone, Deserialize)]
pub struct ShellArgs {
    /// The shell command to execute
    pub command: String,
    /// Working directory override (optional)
    pub cwd: Option<String>,
    /// Timeout in seconds (default: 60)
    pub timeout: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ShellTool {
    #[serde(skip)]
    context: Option<Arc<ToolContext>>,
}

impl ShellTool {
    pub fn new(context: Arc<ToolContext>) -> Self {
        Self {
            context: Some(context),
        }
    }
}

impl Tool for ShellTool {
    const NAME: &'static str = "shell";

    type Error = ShellError;
    type Args = ShellArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "shell".to_string(),
            description: "Execute a shell command. Use this to run builds, tests, git commands, or any CLI tool. Output is captured and returned.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory for command execution (optional, defaults to project root)"
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Timeout in seconds (default: 60)"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let cwd = args.cwd.unwrap_or_else(|| {
            self.context
                .as_ref()
                .map(|c| c.working_directory.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string())
        });
        let timeout = args.timeout.unwrap_or(60);

        let result = super::shell::shell_impl(&args.command, Some(&cwd), timeout)
            .await
            .map_err(|e| ShellError(e.to_string()))?;

        if result.success {
            Ok(result.content)
        } else {
            Ok(format!(
                "{}\n\nError: {}",
                result.content,
                result.error.unwrap_or_default()
            ))
        }
    }
}

// ============================================================================
// Read File Tool
// ============================================================================

#[derive(Debug, thiserror::Error)]
#[error("Read file error: {0}")]
pub struct ReadFileError(String);

#[derive(Debug, Clone, Deserialize)]
pub struct ReadFileArgs {
    /// Path to the file to read (relative to working directory or absolute)
    pub path: String,
    /// Starting line number (1-based, optional)
    pub start_line: Option<usize>,
    /// Number of lines to read (optional)
    pub num_lines: Option<usize>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ReadFileTool {
    #[serde(skip)]
    context: Option<Arc<ToolContext>>,
}

impl ReadFileTool {
    pub fn new(context: Arc<ToolContext>) -> Self {
        Self {
            context: Some(context),
        }
    }
}

impl Tool for ReadFileTool {
    const NAME: &'static str = "read_file";

    type Error = ReadFileError;
    type Args = ReadFileArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "read_file".to_string(),
            description: "Read the contents of a file. Use start_line and num_lines to read specific portions of large files.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file (relative to project root or absolute)"
                    },
                    "start_line": {
                        "type": "integer",
                        "description": "Starting line number (1-based, optional)"
                    },
                    "num_lines": {
                        "type": "integer",
                        "description": "Number of lines to read (optional)"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let base_path = self
            .context
            .as_ref()
            .map(|c| c.working_directory.clone())
            .unwrap_or_else(|| PathBuf::from("."));

        let full_path = if PathBuf::from(&args.path).is_absolute() {
            PathBuf::from(&args.path)
        } else {
            base_path.join(&args.path)
        };

        let path_str = full_path.to_string_lossy().to_string();
        let start = args.start_line;
        let num = args.num_lines;

        // Run synchronous file read in blocking task
        let result = tokio::task::spawn_blocking(move || {
            super::file_ops::read_file_impl(&path_str, start, num)
        })
        .await
        .map_err(|e| ReadFileError(format!("Task join error: {}", e)))?
        .map_err(|e| ReadFileError(e.to_string()))?;

        if result.success {
            Ok(result.content)
        } else {
            Err(ReadFileError(
                result.error.unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }
}

// ============================================================================
// List Files Tool
// ============================================================================

#[derive(Debug, thiserror::Error)]
#[error("List files error: {0}")]
pub struct ListFilesError(String);

#[derive(Debug, Clone, Deserialize)]
pub struct ListFilesArgs {
    /// Directory to list (relative to working directory or absolute)
    pub directory: Option<String>,
    /// Whether to list recursively
    pub recursive: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ListFilesTool {
    #[serde(skip)]
    context: Option<Arc<ToolContext>>,
}

impl ListFilesTool {
    pub fn new(context: Arc<ToolContext>) -> Self {
        Self {
            context: Some(context),
        }
    }
}

impl Tool for ListFilesTool {
    const NAME: &'static str = "list_files";

    type Error = ListFilesError;
    type Args = ListFilesArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "list_files".to_string(),
            description: "List files and directories. Use this to explore project structure and find files.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "directory": {
                        "type": "string",
                        "description": "Directory to list (relative to project root or absolute, defaults to project root)"
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "Whether to list recursively (default: false)"
                    }
                },
                "required": []
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let base_path = self
            .context
            .as_ref()
            .map(|c| c.working_directory.clone())
            .unwrap_or_else(|| PathBuf::from("."));

        let dir = args.directory.unwrap_or_else(|| ".".to_string());
        let full_path = if PathBuf::from(&dir).is_absolute() {
            PathBuf::from(&dir)
        } else {
            base_path.join(&dir)
        };

        let path_str = full_path.to_string_lossy().to_string();
        let recursive = args.recursive.unwrap_or(false);

        // Run synchronous list in blocking task
        let result = tokio::task::spawn_blocking(move || {
            super::file_ops::list_files_impl(&path_str, recursive)
        })
        .await
        .map_err(|e| ListFilesError(format!("Task join error: {}", e)))?
        .map_err(|e| ListFilesError(e.to_string()))?;

        if result.success {
            Ok(result.content)
        } else {
            Err(ListFilesError(
                result.error.unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }
}

// ============================================================================
// Edit File Tool
// ============================================================================

#[derive(Debug, thiserror::Error)]
#[error("Edit file error: {0}")]
pub struct EditFileError(String);

#[derive(Debug, Clone, Deserialize)]
pub struct EditFileArgs {
    /// Path to the file to edit
    pub path: String,
    /// The text to find and replace
    pub old_text: String,
    /// The new text to insert
    pub new_text: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EditFileTool {
    #[serde(skip)]
    context: Option<Arc<ToolContext>>,
}

impl EditFileTool {
    pub fn new(context: Arc<ToolContext>) -> Self {
        Self {
            context: Some(context),
        }
    }
}

impl Tool for EditFileTool {
    const NAME: &'static str = "edit_file";

    type Error = EditFileError;
    type Args = EditFileArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "edit_file".to_string(),
            description: "Edit a file by replacing specific text. The old_text must match exactly (including whitespace and indentation). Use read_file first to see the exact content.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to edit"
                    },
                    "old_text": {
                        "type": "string",
                        "description": "The exact text to find and replace (must match exactly)"
                    },
                    "new_text": {
                        "type": "string",
                        "description": "The new text to insert in place of old_text"
                    }
                },
                "required": ["path", "old_text", "new_text"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let base_path = self
            .context
            .as_ref()
            .map(|c| c.working_directory.clone())
            .unwrap_or_else(|| PathBuf::from("."));

        let full_path = if PathBuf::from(&args.path).is_absolute() {
            PathBuf::from(&args.path)
        } else {
            base_path.join(&args.path)
        };

        let path_str = full_path.to_string_lossy().to_string();

        // Build the JSON params expected by edit_file_impl
        let params = json!({
            "file_path": path_str,
            "replacements": [{
                "old_text": args.old_text,
                "new_text": args.new_text
            }]
        });

        // Run synchronous edit in blocking task
        let result = tokio::task::spawn_blocking(move || {
            super::file_mods::edit_file_impl(params)
        })
        .await
        .map_err(|e| EditFileError(format!("Task join error: {}", e)))?
        .map_err(|e| EditFileError(e.to_string()))?;

        if result.success {
            Ok(result.content)
        } else {
            Err(EditFileError(
                result.error.unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }
}

// ============================================================================
// Grep Tool
// ============================================================================

#[derive(Debug, thiserror::Error)]
#[error("Grep error: {0}")]
pub struct GrepError(String);

#[derive(Debug, Clone, Deserialize)]
pub struct GrepArgs {
    /// The pattern to search for (regex supported)
    pub pattern: String,
    /// Directory or file to search in (defaults to project root)
    pub path: Option<String>,
    /// Whether to search case-insensitively
    pub case_insensitive: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GrepTool {
    #[serde(skip)]
    context: Option<Arc<ToolContext>>,
}

impl GrepTool {
    pub fn new(context: Arc<ToolContext>) -> Self {
        Self {
            context: Some(context),
        }
    }
}

impl Tool for GrepTool {
    const NAME: &'static str = "grep";

    type Error = GrepError;
    type Args = GrepArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "grep".to_string(),
            description: "Search for text patterns in files using regex. Returns matching lines with file paths and line numbers.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The regex pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory or file to search in (defaults to project root)"
                    },
                    "case_insensitive": {
                        "type": "boolean",
                        "description": "Whether to search case-insensitively (default: false)"
                    }
                },
                "required": ["pattern"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let base_path = self
            .context
            .as_ref()
            .map(|c| c.working_directory.clone())
            .unwrap_or_else(|| PathBuf::from("."));

        let search_path = args.path.unwrap_or_else(|| ".".to_string());
        let full_path = if PathBuf::from(&search_path).is_absolute() {
            PathBuf::from(&search_path)
        } else {
            base_path.join(&search_path)
        };

        let path_str = full_path.to_string_lossy().to_string();

        // Prepend -i flag if case insensitive
        let pattern = if args.case_insensitive.unwrap_or(false) {
            format!("-i {}", args.pattern)
        } else {
            args.pattern
        };

        // Run synchronous grep in blocking task
        let result = tokio::task::spawn_blocking(move || {
            super::grep::grep_impl(&pattern, &path_str)
        })
        .await
        .map_err(|e| GrepError(format!("Task join error: {}", e)))?
        .map_err(|e| GrepError(e.to_string()))?;

        if result.success {
            Ok(result.content)
        } else {
            Err(GrepError(
                result.error.unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }
}

// ============================================================================
// Write File Tool (for creating new files)
// ============================================================================

#[derive(Debug, thiserror::Error)]
#[error("Write file error: {0}")]
pub struct WriteFileError(String);

#[derive(Debug, Clone, Deserialize)]
pub struct WriteFileArgs {
    /// Path to the file to write
    pub path: String,
    /// Content to write to the file
    pub content: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WriteFileTool {
    #[serde(skip)]
    context: Option<Arc<ToolContext>>,
}

impl WriteFileTool {
    pub fn new(context: Arc<ToolContext>) -> Self {
        Self {
            context: Some(context),
        }
    }
}

impl Tool for WriteFileTool {
    const NAME: &'static str = "write_file";

    type Error = WriteFileError;
    type Args = WriteFileArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "write_file".to_string(),
            description: "Write content to a file, creating it if it doesn't exist or overwriting if it does. Use edit_file for modifying existing files.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    }
                },
                "required": ["path", "content"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let base_path = self
            .context
            .as_ref()
            .map(|c| c.working_directory.clone())
            .unwrap_or_else(|| PathBuf::from("."));

        let full_path = if PathBuf::from(&args.path).is_absolute() {
            PathBuf::from(&args.path)
        } else {
            base_path.join(&args.path)
        };

        // Create parent directories if they don't exist
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| WriteFileError(format!("Failed to create directories: {}", e)))?;
        }

        tokio::fs::write(&full_path, &args.content)
            .await
            .map_err(|e| WriteFileError(format!("Failed to write file: {}", e)))?;

        Ok(format!(
            "Successfully wrote {} bytes to {}",
            args.content.len(),
            full_path.display()
        ))
    }
}

/// Create all tools with the given context
pub fn create_tools(context: Arc<ToolContext>) -> (
    ShellTool,
    ReadFileTool,
    ListFilesTool,
    EditFileTool,
    GrepTool,
    WriteFileTool,
) {
    (
        ShellTool::new(context.clone()),
        ReadFileTool::new(context.clone()),
        ListFilesTool::new(context.clone()),
        EditFileTool::new(context.clone()),
        GrepTool::new(context.clone()),
        WriteFileTool::new(context),
    )
}
