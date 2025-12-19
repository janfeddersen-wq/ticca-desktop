//! Shell command execution tool

use crate::tools::registry::{ToolDefinition, ToolParameterSchema, ToolResult, ToolExecutor};
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

/// Default timeout for shell commands (60 seconds)
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Maximum number of lines to capture in output
const MAX_OUTPUT_LINES: usize = 256;

/// Result of a shell command execution
#[derive(Debug)]
pub struct ShellCommandResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub execution_time_secs: f64,
    pub timed_out: bool,
}

/// Execute a shell command asynchronously
pub async fn shell_impl(
    command: &str,
    cwd: Option<&str>,
    timeout_secs: u64,
) -> Result<ToolResult> {
    if command.trim().is_empty() {
        return Ok(ToolResult::error("Command cannot be empty"));
    }

    // Determine working directory
    let working_dir = match cwd {
        Some(dir) => {
            let path = PathBuf::from(dir);
            if !path.exists() {
                return Ok(ToolResult::error(format!("Working directory does not exist: {}", dir)));
            }
            if !path.is_dir() {
                return Ok(ToolResult::error(format!("Not a directory: {}", dir)));
            }
            path
        }
        None => std::env::current_dir()?,
    };

    let start_time = std::time::Instant::now();

    // Create the command
    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg(command)
        .current_dir(&working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Spawn the process
    let mut child = cmd.spawn()?;

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let mut stdout_lines: Vec<String> = Vec::new();
    let mut stderr_lines: Vec<String> = Vec::new();

    // Read output with timeout
    let timeout_duration = Duration::from_secs(timeout_secs);
    let mut timed_out = false;

    let read_output = async {
        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(l)) => {
                            if stdout_lines.len() < MAX_OUTPUT_LINES {
                                stdout_lines.push(l);
                            }
                        }
                        Ok(None) => {}
                        Err(_) => break,
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(l)) => {
                            if stderr_lines.len() < MAX_OUTPUT_LINES {
                                stderr_lines.push(l);
                            }
                        }
                        Ok(None) => {}
                        Err(_) => break,
                    }
                }
                status = child.wait() => {
                    return status;
                }
            }
        }
        child.wait().await
    };

    let exit_status = match timeout(timeout_duration, read_output).await {
        Ok(result) => result,
        Err(_) => {
            // Timeout occurred, try to kill the process
            timed_out = true;
            let _ = child.kill().await;
            Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Command timed out",
            ))
        }
    };

    let execution_time = start_time.elapsed().as_secs_f64();

    let exit_code = exit_status.ok().and_then(|s| s.code());
    let success = exit_code == Some(0) && !timed_out;

    // Format output
    let stdout_str = stdout_lines.join("\n");
    let stderr_str = stderr_lines.join("\n");

    let mut output_parts = vec![format!(
        "Command: {}\nWorking Directory: {}\nExit Code: {}\nExecution Time: {:.2}s{}",
        command,
        working_dir.display(),
        exit_code.map(|c| c.to_string()).unwrap_or_else(|| "N/A".to_string()),
        execution_time,
        if timed_out { " (TIMED OUT)" } else { "" }
    )];

    if !stdout_str.is_empty() {
        output_parts.push(format!("\n--- STDOUT ---\n{}", stdout_str));
        if stdout_lines.len() >= MAX_OUTPUT_LINES {
            output_parts.push(format!("\n... (truncated to {} lines)", MAX_OUTPUT_LINES));
        }
    }

    if !stderr_str.is_empty() {
        output_parts.push(format!("\n--- STDERR ---\n{}", stderr_str));
        if stderr_lines.len() >= MAX_OUTPUT_LINES {
            output_parts.push(format!("\n... (truncated to {} lines)", MAX_OUTPUT_LINES));
        }
    }

    let output = output_parts.join("");

    if success {
        Ok(ToolResult::success(output))
    } else {
        Ok(ToolResult {
            success: false,
            content: output,
            error: Some(if timed_out {
                format!("Command timed out after {} seconds", timeout_secs)
            } else {
                format!("Command failed with exit code: {:?}", exit_code)
            }),
        })
    }
}

/// Get shell tool definition
pub fn shell_definition() -> ToolDefinition {
    let mut params = HashMap::new();
    params.insert(
        "command".to_string(),
        ToolParameterSchema::string("The shell command to execute."),
    );
    params.insert(
        "cwd".to_string(),
        ToolParameterSchema::string("Working directory for command execution. Defaults to current directory.")
            .with_default(json!(null)),
    );
    params.insert(
        "timeout".to_string(),
        ToolParameterSchema::integer(format!(
            "Timeout in seconds. Defaults to {} seconds.",
            DEFAULT_TIMEOUT_SECS
        ))
        .with_default(json!(DEFAULT_TIMEOUT_SECS)),
    );

    ToolDefinition {
        name: "shell".to_string(),
        description: format!(
            "Execute a shell command with configurable timeout and working directory. \
            Output is limited to {} lines per stream.",
            MAX_OUTPUT_LINES
        ),
        parameters: ToolParameterSchema::object(params, vec!["command".to_string()]),
    }
}

/// Get shell executor
pub fn shell_executor() -> ToolExecutor {
    Arc::new(|params: Value| {
        Box::pin(async move {
            let command = params.get("command")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("command is required"))?;
            
            let cwd = params.get("cwd")
                .and_then(|v| v.as_str());
            
            let timeout_secs = params.get("timeout")
                .and_then(|v| v.as_u64())
                .unwrap_or(DEFAULT_TIMEOUT_SECS);
            
            shell_impl(command, cwd, timeout_secs).await
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shell_echo() {
        let result = shell_impl("echo 'Hello, World!'", None, 10).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("Hello, World!"));
        assert!(result.content.contains("Exit Code: 0"));
    }

    #[tokio::test]
    async fn test_shell_with_cwd() {
        let temp_dir = std::env::temp_dir();
        let temp_str = temp_dir.to_string_lossy();
        let result = shell_impl("pwd", Some(&temp_str), 10).await.unwrap();
        assert!(result.success);
        let normalized = temp_str.replace('\\', "/");
        assert!(result.content.contains(&normalized) || result.content.contains(temp_str.as_ref()));
    }

    #[tokio::test]
    async fn test_shell_failure() {
        let result = shell_impl("exit 1", None, 10).await.unwrap();
        assert!(!result.success);
        assert!(result.content.contains("Exit Code: 1"));
    }

    #[tokio::test]
    async fn test_shell_invalid_cwd() {
        let result = shell_impl("echo test", Some("/nonexistent/path"), 10).await.unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("does not exist") || result.content.contains("does not exist"));
    }

    #[tokio::test]
    async fn test_shell_empty_command() {
        let result = shell_impl("", None, 10).await.unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("empty"));
    }
}
