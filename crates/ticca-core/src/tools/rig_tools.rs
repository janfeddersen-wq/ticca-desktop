//! Rig-compatible tool wrappers for the native Rust tools
//!
//! These wrappers implement rig's `Tool` trait to enable the ReAct loop.

use super::policy::ToolPolicy;
use super::todo::{TodoItem, TodoListEvent, TodoStatus, TodoStore};
use crate::agents::{AgentType, get_all_agents};
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;

use super::system_exec::{SystemExecRequest, SystemExecResponse, SystemExecStore};

#[derive(Debug, Clone)]
pub struct AgentCallEvent {
    pub parent_id: usize,
    pub child_id: usize,
    pub parent: AgentType,
    pub child: AgentType,
    pub prompt: String,
}

#[derive(Debug, Clone)]
pub enum AgentStreamEvent {
    Start {
        node_id: usize,
        agent_type: AgentType,
    },
    Chunk {
        node_id: usize,
        text: String,
    },
    Reasoning {
        node_id: usize,
        text: String,
    },
    ToolCall {
        node_id: usize,
        name: String,
        args: String,
    },
    Complete {
        node_id: usize,
    },
}

#[derive(Clone)]
pub struct AgentInvokeRequest {
    pub agent_type: AgentType,
    pub prompt: String,
    pub parent_context: Arc<ToolContext>,
    pub node_id: usize,
}

pub type AgentInvoker = dyn Fn(AgentInvokeRequest) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>
    + Send
    + Sync;

/// Shared context for all tools - primarily the working directory
#[derive(Clone)]
pub struct ToolContext {
    pub working_directory: PathBuf,
    pub approval_gate: Option<Arc<super::approval::ToolApprovalGate>>,
    pub yolo_mode_enabled: bool,
    pub policy: ToolPolicy,
    pub current_agent: AgentType,
    pub current_model: Option<String>,
    pub max_tool_rounds: u32,
    pub call_graph_tx: Option<mpsc::UnboundedSender<AgentCallEvent>>,
    pub agent_stream_tx: Option<mpsc::UnboundedSender<AgentStreamEvent>>,
    pub call_graph_counter: Option<Arc<AtomicUsize>>,
    pub node_id: usize,
    pub agent_invoker: Option<Arc<AgentInvoker>>,
    pub todo_store: Option<Arc<TodoStore>>,
    pub todo_tx: Option<mpsc::UnboundedSender<TodoListEvent>>,
    pub system_exec_store: Option<Arc<SystemExecStore>>,
    pub system_exec_tx: Option<mpsc::UnboundedSender<SystemExecRequest>>,
    pub process_output_offsets: Arc<Mutex<std::collections::HashMap<String, usize>>>,
}

impl Default for ToolContext {
    fn default() -> Self {
        let working_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            policy: ToolPolicy::allow_root(working_directory.clone()),
            working_directory,
            approval_gate: None,
            yolo_mode_enabled: true,
            current_agent: AgentType::Coding,
            current_model: None,
            max_tool_rounds: 0,
            call_graph_tx: None,
            agent_stream_tx: None,
            call_graph_counter: None,
            node_id: 0,
            agent_invoker: None,
            todo_store: None,
            todo_tx: None,
            system_exec_store: None,
            system_exec_tx: None,
            process_output_offsets: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }
}

impl ToolContext {
    async fn require_approval(&self, tool_name: &str, args: String) -> Result<(), String> {
        if self.yolo_mode_enabled {
            return Ok(());
        }

        match &self.approval_gate {
            Some(gate) => {
                let approved = gate.request(tool_name, args).await;
                if approved {
                    Ok(())
                } else {
                    Err("Tool execution denied by user".to_string())
                }
            }
            None => Err("Approval gate not available".to_string()),
        }
    }

    fn enforce_path(&self, path: &Path) -> Result<(), String> {
        if self.policy.is_path_allowed(path) {
            Ok(())
        } else {
            Err(format!(
                "Tool access denied for path outside allowed roots: {}",
                path.display()
            ))
        }
    }
}

// ============================================================================
// System Execution Tools (UI-backed)
// ============================================================================

#[derive(Debug, thiserror::Error)]
#[error("execute_shell error: {0}")]
pub struct ExecuteShellError(String);

#[derive(Debug, Clone, Deserialize)]
pub struct ExecuteShellArgs {
    /// The shell command to execute
    pub command: String,
    /// Working directory override (optional)
    pub cwd: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ExecuteShellTool {
    #[serde(skip)]
    context: Option<Arc<ToolContext>>,
}

impl ExecuteShellTool {
    pub fn new(context: Arc<ToolContext>) -> Self {
        Self {
            context: Some(context),
        }
    }
}

static SYSTEM_EXEC_REQUEST_ID: AtomicUsize = AtomicUsize::new(1);

impl Tool for ExecuteShellTool {
    const NAME: &'static str = "execute_shell";

    type Error = ExecuteShellError;
    type Args = ExecuteShellArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        let spec = super::spec::execute_shell_spec(60);
        ToolDefinition {
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            parameters: spec.rig_parameters,
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(context) = &self.context else {
            return Err(ExecuteShellError("Tool context not available".to_string()));
        };
        let Some(store) = &context.system_exec_store else {
            return Err(ExecuteShellError(
                "System execution store not configured".to_string(),
            ));
        };
        let Some(tx) = &context.system_exec_tx else {
            return Err(ExecuteShellError(
                "System execution UI channel not configured".to_string(),
            ));
        };

        if let Some(cwd) = &args.cwd {
            let cwd_path = PathBuf::from(cwd);
            let resolved = if cwd_path.is_absolute() {
                cwd_path
            } else {
                context.working_directory.join(cwd_path)
            };
            if let Err(e) = context.enforce_path(&resolved) {
                return Err(ExecuteShellError(e));
            }
        }

        if let Err(e) = context
            .require_approval("execute_shell", format!("command={}", args.command))
            .await
        {
            return Err(ExecuteShellError(e));
        }

        let request_id = SYSTEM_EXEC_REQUEST_ID.fetch_add(1, Ordering::SeqCst) as u64;
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel::<SystemExecResponse>();
        store.register_pending(request_id, resp_tx);

        tx.send(SystemExecRequest::ExecuteShell {
            request_id,
            command: args.command.clone(),
            cwd: args.cwd.clone(),
        })
        .map_err(|_| ExecuteShellError("Failed to dispatch execute_shell request".to_string()))?;

        let started = resp_rx
            .await
            .map_err(|_| ExecuteShellError("execute_shell request was dropped".to_string()))?;

        let process_id = match started {
            SystemExecResponse::Started { process_id } => process_id,
            SystemExecResponse::Error { message } => return Err(ExecuteShellError(message)),
            other => {
                return Err(ExecuteShellError(format!(
                    "Unexpected response for execute_shell: {:?}",
                    other
                )));
            }
        };

        let completed = tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                if let Some(Some(code)) = store.exit_code(&process_id) {
                    let output = store.output(&process_id).unwrap_or_default();
                    return Ok::<_, String>((code, output));
                }
                store.wait_for_update(&process_id).await?;
            }
        })
        .await;

        match completed {
            Ok(Ok((exit_code, output))) => Ok(json!({
                "process_id": process_id,
                "stdout": output,
                "stderr": "",
                "exit_code": exit_code,
            })
            .to_string()),
            Ok(Err(e)) => Err(ExecuteShellError(e)),
            Err(_) => Ok(format!(
                "Process started successfully with ID: `{}`. It is still running. You can use other tools to monitor its output, send further commands, or terminate it.",
                process_id
            )),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("list_processes error: {0}")]
pub struct ListProcessesError(String);

#[derive(Debug, Clone, Deserialize)]
pub struct ListProcessesArgs {}

#[derive(Clone, Serialize, Deserialize)]
pub struct ListProcessesTool {
    #[serde(skip)]
    context: Option<Arc<ToolContext>>,
}

impl ListProcessesTool {
    pub fn new(context: Arc<ToolContext>) -> Self {
        Self {
            context: Some(context),
        }
    }
}

impl Tool for ListProcessesTool {
    const NAME: &'static str = "list_processes";

    type Error = ListProcessesError;
    type Args = ListProcessesArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        let spec = super::spec::list_processes_spec();
        ToolDefinition {
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            parameters: spec.rig_parameters,
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(context) = &self.context else {
            return Err(ListProcessesError("Tool context not available".to_string()));
        };
        let Some(store) = &context.system_exec_store else {
            return Err(ListProcessesError(
                "System execution store not configured".to_string(),
            ));
        };
        Ok(json!(store.list_visible()).to_string())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("read_process_output error: {0}")]
pub struct ReadProcessOutputError(String);

#[derive(Debug, Clone, Deserialize)]
pub struct ReadProcessOutputArgs {
    pub process_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ReadProcessOutputTool {
    #[serde(skip)]
    context: Option<Arc<ToolContext>>,
}

impl ReadProcessOutputTool {
    pub fn new(context: Arc<ToolContext>) -> Self {
        Self {
            context: Some(context),
        }
    }
}

impl Tool for ReadProcessOutputTool {
    const NAME: &'static str = "read_process_output";

    type Error = ReadProcessOutputError;
    type Args = ReadProcessOutputArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        let spec = super::spec::read_process_output_spec();
        ToolDefinition {
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            parameters: spec.rig_parameters,
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(context) = &self.context else {
            return Err(ReadProcessOutputError(
                "Tool context not available".to_string(),
            ));
        };
        let Some(store) = &context.system_exec_store else {
            return Err(ReadProcessOutputError(
                "System execution store not configured".to_string(),
            ));
        };

        let Some(output) = store.output(&args.process_id) else {
            return Err(ReadProcessOutputError(format!(
                "Unknown process: {}",
                args.process_id
            )));
        };

        let mut offsets = context
            .process_output_offsets
            .lock()
            .map_err(|_| ReadProcessOutputError("Output offset mutex poisoned".to_string()))?;
        let start = offsets.get(&args.process_id).copied().unwrap_or(0);

        let (start, new_output) = if start <= output.len() && output.is_char_boundary(start) {
            (start, output[start..].to_string())
        } else {
            (0, output.clone())
        };

        offsets.insert(args.process_id, start + new_output.len());

        Ok(json!({
            "stdout": new_output,
            "stderr": "",
        })
        .to_string())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("kill_process error: {0}")]
pub struct KillProcessError(String);

#[derive(Debug, Clone, Deserialize)]
pub struct KillProcessArgs {
    pub process_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct KillProcessTool {
    #[serde(skip)]
    context: Option<Arc<ToolContext>>,
}

impl KillProcessTool {
    pub fn new(context: Arc<ToolContext>) -> Self {
        Self {
            context: Some(context),
        }
    }
}

impl Tool for KillProcessTool {
    const NAME: &'static str = "kill_process";

    type Error = KillProcessError;
    type Args = KillProcessArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        let spec = super::spec::kill_process_spec();
        ToolDefinition {
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            parameters: spec.rig_parameters,
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(context) = &self.context else {
            return Err(KillProcessError("Tool context not available".to_string()));
        };
        let Some(store) = &context.system_exec_store else {
            return Err(KillProcessError(
                "System execution store not configured".to_string(),
            ));
        };
        let Some(tx) = &context.system_exec_tx else {
            return Err(KillProcessError(
                "System execution UI channel not configured".to_string(),
            ));
        };

        let request_id = SYSTEM_EXEC_REQUEST_ID.fetch_add(1, Ordering::SeqCst) as u64;
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel::<SystemExecResponse>();
        store.register_pending(request_id, resp_tx);

        tx.send(SystemExecRequest::KillProcess {
            request_id,
            process_id: args.process_id.clone(),
        })
        .map_err(|_| KillProcessError("Failed to dispatch kill_process request".to_string()))?;

        let resp = resp_rx
            .await
            .map_err(|_| KillProcessError("kill_process request was dropped".to_string()))?;

        match resp {
            SystemExecResponse::Killed { process_id } => {
                Ok(format!("Process `{}` terminated successfully.", process_id))
            }
            SystemExecResponse::Error { message } => Err(KillProcessError(message)),
            other => Err(KillProcessError(format!(
                "Unexpected response for kill_process: {:?}",
                other
            ))),
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
        let spec = super::spec::read_file_spec();
        ToolDefinition {
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            parameters: spec.rig_parameters,
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

        if let Some(context) = &self.context {
            context.enforce_path(&full_path).map_err(ReadFileError)?;
        }

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
        let spec = super::spec::list_files_spec();
        ToolDefinition {
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            parameters: spec.rig_parameters,
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

        if let Some(context) = &self.context {
            context.enforce_path(&full_path).map_err(ListFilesError)?;
        }

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
        let spec = super::spec::edit_file_spec();
        ToolDefinition {
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            parameters: spec.rig_parameters,
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

        if let Some(context) = &self.context {
            context.enforce_path(&full_path).map_err(EditFileError)?;
            if let Err(e) = context
                .require_approval("edit_file", format!("path={}", args.path))
                .await
            {
                return Err(EditFileError(e));
            }
        }

        let path_str = full_path.to_string_lossy().to_string();

        // Build the JSON params expected by edit_file_impl
        let params = json!({
            "file_path": path_str,
            "replacements": [{
                "old_str": args.old_text,
                "new_str": args.new_text
            }]
        });

        // Run synchronous edit in blocking task
        let result = tokio::task::spawn_blocking(move || super::file_mods::edit_file_impl(params))
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

// ============================================================================
// Delete File Tool
// ============================================================================

#[derive(Debug, thiserror::Error)]
#[error("Delete file error: {0}")]
pub struct DeleteFileError(String);

#[derive(Debug, Clone, Deserialize)]
pub struct DeleteFileArgs {
    /// Path to the file to delete
    pub path: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DeleteFileTool {
    #[serde(skip)]
    context: Option<Arc<ToolContext>>,
}

impl DeleteFileTool {
    pub fn new(context: Arc<ToolContext>) -> Self {
        Self {
            context: Some(context),
        }
    }
}

impl Tool for DeleteFileTool {
    const NAME: &'static str = "delete_file";

    type Error = DeleteFileError;
    type Args = DeleteFileArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        let spec = super::spec::delete_file_spec();
        ToolDefinition {
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            parameters: spec.rig_parameters,
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if let Some(context) = &self.context
            && let Err(e) = context
                .require_approval("delete_file", format!("path={}", args.path))
                .await
        {
            return Err(DeleteFileError(e));
        }

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

        let result =
            tokio::task::spawn_blocking(move || super::file_mods::delete_file_impl(&path_str))
                .await
                .map_err(|e| DeleteFileError(format!("Task join error: {}", e)))?
                .map_err(|e| DeleteFileError(e.to_string()))?;

        if result.success {
            Ok(result.content)
        } else {
            Err(DeleteFileError(
                result.error.unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Grep error: {0}")]
pub struct GrepError(String);

#[derive(Debug, Clone, Deserialize)]
pub struct GrepArgs {
    /// The pattern to search for (regex supported)
    pub search_string: String,
    /// Directory or file to search in (defaults to project root)
    pub directory: Option<String>,
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
        let spec = super::spec::grep_spec(200);
        ToolDefinition {
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            parameters: spec.rig_parameters,
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let base_path = self
            .context
            .as_ref()
            .map(|c| c.working_directory.clone())
            .unwrap_or_else(|| PathBuf::from("."));

        let search_path = args.directory.unwrap_or_else(|| ".".to_string());
        let full_path = if PathBuf::from(&search_path).is_absolute() {
            PathBuf::from(&search_path)
        } else {
            base_path.join(&search_path)
        };

        if let Some(context) = &self.context {
            context.enforce_path(&full_path).map_err(GrepError)?;
        }

        let path_str = full_path.to_string_lossy().to_string();

        // Prepend -i flag if case insensitive
        let pattern = if args.case_insensitive.unwrap_or(false) {
            format!("-i {}", args.search_string)
        } else {
            args.search_string
        };

        // Run synchronous grep in blocking task
        let result =
            tokio::task::spawn_blocking(move || super::grep::grep_impl(&pattern, &path_str))
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
        let spec = super::spec::write_file_spec();
        ToolDefinition {
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            parameters: spec.rig_parameters,
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

        if let Some(context) = &self.context {
            context.enforce_path(&full_path).map_err(WriteFileError)?;
            if let Err(e) = context
                .require_approval("write_file", format!("path={}", args.path))
                .await
            {
                return Err(WriteFileError(e));
            }
        }

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

// ============================================================================
// List Agents Tool
// ============================================================================

#[derive(Debug, thiserror::Error)]
#[error("List agents error: {0}")]
pub struct ListAgentsError(String);

#[derive(Debug, Clone, Deserialize)]
pub struct ListAgentsArgs {}

#[derive(Clone, Serialize, Deserialize)]
pub struct ListAgentsTool {}

impl ListAgentsTool {
    pub fn new(_context: Arc<ToolContext>) -> Self {
        Self {}
    }
}

impl Tool for ListAgentsTool {
    const NAME: &'static str = "list_agents";

    type Error = ListAgentsError;
    type Args = ListAgentsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        let spec = super::spec::list_agents_spec();
        ToolDefinition {
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            parameters: spec.rig_parameters,
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let mut lines = Vec::new();
        for agent in get_all_agents() {
            lines.push(format!(
                "- {}: {} - {}",
                agent.agent_type().as_str(),
                agent.display_name(),
                agent.description()
            ));
        }

        if lines.is_empty() {
            lines.push("No agents available.".to_string());
        }

        Ok(lines.join("\n"))
    }
}

// ============================================================================
// Invoke Agent Tool
// ============================================================================

#[derive(Debug, thiserror::Error)]
#[error("Invoke agent error: {0}")]
pub struct InvokeAgentError(String);

#[derive(Debug, Clone, Deserialize)]
pub struct InvokeAgentArgs {
    pub agent: String,
    pub prompt: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InvokeAgentTool {
    #[serde(skip)]
    context: Option<Arc<ToolContext>>,
}

impl InvokeAgentTool {
    pub fn new(context: Arc<ToolContext>) -> Self {
        Self {
            context: Some(context),
        }
    }
}

impl Tool for InvokeAgentTool {
    const NAME: &'static str = "invoke_agent";

    type Error = InvokeAgentError;
    type Args = InvokeAgentArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        let spec = super::spec::invoke_agent_spec();
        ToolDefinition {
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            parameters: spec.rig_parameters,
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let context = self
            .context
            .as_ref()
            .ok_or_else(|| InvokeAgentError("Tool context not available".to_string()))?;

        let agent_type = AgentType::parse(&args.agent)
            .ok_or_else(|| InvokeAgentError(format!("Unknown agent: {}", args.agent)))?;

        let child_id = context
            .call_graph_counter
            .as_ref()
            .map(|counter| counter.fetch_add(1, Ordering::SeqCst))
            .unwrap_or(0);

        if let Some(tx) = &context.call_graph_tx {
            let _ = tx.send(AgentCallEvent {
                parent_id: context.node_id,
                child_id,
                parent: context.current_agent,
                child: agent_type,
                prompt: args.prompt.clone(),
            });
        }

        let invoker = context
            .agent_invoker
            .as_ref()
            .ok_or_else(|| InvokeAgentError("Agent invocation is not configured".to_string()))?;

        invoker(AgentInvokeRequest {
            agent_type,
            prompt: args.prompt,
            parent_context: context.clone(),
            node_id: child_id,
        })
        .await
        .map_err(InvokeAgentError)
    }
}

// ============================================================================
// To Do List Tool
// ============================================================================

#[derive(Debug, thiserror::Error)]
#[error("Todo list error: {0}")]
pub struct TodoListError(String);

#[derive(Debug, Clone, Deserialize)]
pub struct TodoListArgs {
    /// Array of to-do items for this agent.
    pub items: Vec<TodoListItemArgs>,
    /// Set true to confirm all items are completed.
    pub confirmed_complete: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TodoListItemArgs {
    pub text: String,
    pub status: TodoStatus,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TodoListTool {
    #[serde(skip)]
    context: Option<Arc<ToolContext>>,
}

impl TodoListTool {
    pub fn new(context: Arc<ToolContext>) -> Self {
        Self {
            context: Some(context),
        }
    }
}

impl Tool for TodoListTool {
    const NAME: &'static str = "todo_list";

    type Error = TodoListError;
    type Args = TodoListArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        let spec = super::spec::todo_list_spec();
        ToolDefinition {
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            parameters: spec.rig_parameters,
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let context = self
            .context
            .as_ref()
            .ok_or_else(|| TodoListError("Tool context is missing".to_string()))?;
        let store = context
            .todo_store
            .as_ref()
            .ok_or_else(|| TodoListError("Todo store is not configured".to_string()))?;

        let items: Vec<TodoItem> = args
            .items
            .into_iter()
            .map(|item| TodoItem {
                text: item.text,
                status: item.status,
            })
            .collect();

        let state = store
            .update_node(context.node_id, items, args.confirmed_complete)
            .await;

        if let Some(tx) = &context.todo_tx {
            let _ = tx.send(TodoListEvent::Updated {
                node_id: context.node_id,
                state: state.clone(),
            });
        }

        if state.is_completed_and_confirmed() {
            Ok("To Do list updated and confirmed complete.".to_string())
        } else {
            Ok("To Do list updated. Confirmation pending.".to_string())
        }
    }
}

/// Create all tools with the given context
pub fn create_tools(context: Arc<ToolContext>) -> RigTools {
    (
        ExecuteShellTool::new(context.clone()),
        ListProcessesTool::new(context.clone()),
        ReadProcessOutputTool::new(context.clone()),
        KillProcessTool::new(context.clone()),
        ReadFileTool::new(context.clone()),
        ListFilesTool::new(context.clone()),
        EditFileTool::new(context.clone()),
        DeleteFileTool::new(context.clone()),
        GrepTool::new(context.clone()),
        WriteFileTool::new(context.clone()),
        ListAgentsTool::new(context.clone()),
        TodoListTool::new(context.clone()),
        InvokeAgentTool::new(context),
    )
}

pub type RigTools = (
    ExecuteShellTool,
    ListProcessesTool,
    ReadProcessOutputTool,
    KillProcessTool,
    ReadFileTool,
    ListFilesTool,
    EditFileTool,
    DeleteFileTool,
    GrepTool,
    WriteFileTool,
    ListAgentsTool,
    TodoListTool,
    InvokeAgentTool,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::spec;

    #[tokio::test]
    async fn list_files_definition_matches_spec() {
        let context = Arc::new(ToolContext::default());
        let tool = ListFilesTool::new(context);
        let definition = tool.definition("".to_string()).await;
        let spec = spec::list_files_spec();

        assert_eq!(definition.name, spec.name);
        assert_eq!(definition.description, spec.description);
    }
}
