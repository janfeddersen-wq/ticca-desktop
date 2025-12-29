//! SerdesAI-compatible tool wrappers for the native Rust tools
//!
//! These wrappers implement serdes-ai-tools' `Tool` trait to enable the agent loop.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use serdes_ai_tools::{RunContext, Tool, ToolDefinition, ToolError, ToolResult, ToolReturn};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;

use super::policy::ToolPolicy;
use super::system_exec::{SystemExecRequest, SystemExecResponse, SystemExecStore};
use super::todo::{TodoItem, TodoListEvent, TodoListState, TodoStatus, TodoStore};
use crate::agents::{AgentType, get_all_agents};

/// Maximum tool output size in characters (~50KB, roughly 15K tokens)
const MAX_TOOL_OUTPUT_CHARS: usize = 50_000;

/// Truncate a string to a maximum length, adding a truncation notice if needed
fn truncate_output(output: &str, max_chars: usize) -> String {
    if output.len() <= max_chars {
        return output.to_string();
    }
    let truncated = &output[..max_chars];
    let cut_point = truncated.rfind('\n').unwrap_or(max_chars);
    format!(
        "{}\n\n[... OUTPUT TRUNCATED: {} chars total, showing first {} chars ...]",
        &output[..cut_point],
        output.len(),
        cut_point
    )
}

// ============================================================================
// Call Graph and Agent Events
// ============================================================================

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
    Start { node_id: usize, agent_type: AgentType },
    Chunk { node_id: usize, text: String },
    Reasoning { node_id: usize, text: String },
    ToolCall { node_id: usize, name: String, args: String },
    Complete { node_id: usize, output: String },
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

// ============================================================================
// Tool Context
// ============================================================================

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
                if gate.request(tool_name, args).await {
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
            Err(format!("Tool access denied for path outside allowed roots: {}", path.display()))
        }
    }
}

// ============================================================================
// TiccaDeps - Dependencies for all tools
// ============================================================================

#[derive(Clone)]
pub struct TiccaDeps {
    pub tool_context: Arc<ToolContext>,
}

impl TiccaDeps {
    pub fn new(tool_context: Arc<ToolContext>) -> Self {
        Self { tool_context }
    }
}

impl Default for TiccaDeps {
    fn default() -> Self {
        Self { tool_context: Arc::new(ToolContext::default()) }
    }
}

fn get_context(ctx: &RunContext<TiccaDeps>) -> &Arc<ToolContext> {
    &ctx.deps.tool_context
}

// ============================================================================
// 1. ExecuteShellTool
// ============================================================================

static SYSTEM_EXEC_REQUEST_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug, Clone, Deserialize)]
pub struct ExecuteShellArgs {
    pub command: String,
    pub cwd: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ExecuteShellTool;

impl ExecuteShellTool {
    pub fn new(_context: Arc<ToolContext>) -> Self { Self }
}

#[async_trait]
impl Tool<TiccaDeps> for ExecuteShellTool {
    fn definition(&self) -> ToolDefinition {
        let spec = super::spec::execute_shell_spec(60);
        ToolDefinition::new(spec.name, spec.description).with_parameters(spec.rig_parameters)
    }

    async fn call(&self, ctx: &RunContext<TiccaDeps>, args: JsonValue) -> ToolResult {
        let context = get_context(ctx);
        let args: ExecuteShellArgs = serde_json::from_value(args)?;

        let Some(store) = &context.system_exec_store else {
            return Err(ToolError::execution_failed("System execution store not configured"));
        };
        let Some(tx) = &context.system_exec_tx else {
            return Err(ToolError::execution_failed("System execution UI channel not configured"));
        };

        if let Some(cwd) = &args.cwd {
            let cwd_path = PathBuf::from(cwd);
            let resolved = if cwd_path.is_absolute() { cwd_path } else { context.working_directory.join(cwd_path) };
            context.enforce_path(&resolved).map_err(ToolError::execution_failed)?;
        }

        context.require_approval("execute_shell", format!("command={}", args.command)).await.map_err(ToolError::execution_failed)?;

        let request_id = SYSTEM_EXEC_REQUEST_ID.fetch_add(1, Ordering::SeqCst) as u64;
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel::<SystemExecResponse>();
        store.register_pending(request_id, resp_tx);

        tx.send(SystemExecRequest::ExecuteShell { request_id, command: args.command.clone(), cwd: args.cwd.clone() })
            .map_err(|_| ToolError::execution_failed("Failed to dispatch execute_shell request"))?;

        let started = resp_rx.await.map_err(|_| ToolError::execution_failed("execute_shell request was dropped"))?;

        let process_id = match started {
            SystemExecResponse::Started { process_id } => process_id,
            SystemExecResponse::Error { message } => return Err(ToolError::execution_failed(message)),
            other => return Err(ToolError::execution_failed(format!("Unexpected response: {:?}", other))),
        };

        let completed = tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                if let Some(Some(code)) = store.exit_code(&process_id) {
                    let output = store.output(&process_id).unwrap_or_default();
                    return Ok::<_, String>((code, output));
                }
                store.wait_for_update(&process_id).await?;
            }
        }).await;

        match completed {
            Ok(Ok((exit_code, output))) => Ok(ToolReturn::json(json!({
                "process_id": process_id, "stdout": truncate_output(&output, MAX_TOOL_OUTPUT_CHARS), "stderr": "", "exit_code": exit_code,
            }))),
            Ok(Err(e)) => Err(ToolError::execution_failed(e)),
            Err(_) => Ok(ToolReturn::text(format!("Process started with ID: `{}`. Still running.", process_id))),
        }
    }
}

// ============================================================================
// 2. ListProcessesTool
// ============================================================================

#[derive(Clone, Serialize, Deserialize)]
pub struct ListProcessesTool;

impl ListProcessesTool {
    pub fn new(_context: Arc<ToolContext>) -> Self { Self }
}

#[async_trait]
impl Tool<TiccaDeps> for ListProcessesTool {
    fn definition(&self) -> ToolDefinition {
        let spec = super::spec::list_processes_spec();
        ToolDefinition::new(spec.name, spec.description).with_parameters(spec.rig_parameters)
    }

    async fn call(&self, ctx: &RunContext<TiccaDeps>, _args: JsonValue) -> ToolResult {
        let context = get_context(ctx);
        let Some(store) = &context.system_exec_store else {
            return Err(ToolError::execution_failed("System execution store not configured"));
        };
        Ok(ToolReturn::json(json!(store.list_visible())))
    }
}

// ============================================================================
// 3. ReadProcessOutputTool
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct ReadProcessOutputArgs {
    pub process_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ReadProcessOutputTool;

impl ReadProcessOutputTool {
    pub fn new(_context: Arc<ToolContext>) -> Self { Self }
}

#[async_trait]
impl Tool<TiccaDeps> for ReadProcessOutputTool {
    fn definition(&self) -> ToolDefinition {
        let spec = super::spec::read_process_output_spec();
        ToolDefinition::new(spec.name, spec.description).with_parameters(spec.rig_parameters)
    }

    async fn call(&self, ctx: &RunContext<TiccaDeps>, args: JsonValue) -> ToolResult {
        let context = get_context(ctx);
        let args: ReadProcessOutputArgs = serde_json::from_value(args)?;

        let Some(store) = &context.system_exec_store else {
            return Err(ToolError::execution_failed("System execution store not configured"));
        };
        let Some(output) = store.output(&args.process_id) else {
            return Err(ToolError::execution_failed(format!("Unknown process: {}", args.process_id)));
        };

        let mut offsets = context.process_output_offsets.lock().map_err(|_| ToolError::execution_failed("Mutex poisoned"))?;
        let start = offsets.get(&args.process_id).copied().unwrap_or(0);
        let (start, new_output) = if start <= output.len() && output.is_char_boundary(start) {
            (start, output[start..].to_string())
        } else {
            (0, output.clone())
        };
        offsets.insert(args.process_id, start + new_output.len());
        Ok(ToolReturn::json(json!({ "stdout": truncate_output(&new_output, MAX_TOOL_OUTPUT_CHARS), "stderr": "" })))
    }
}

// ============================================================================
// 4. KillProcessTool
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct KillProcessArgs {
    pub process_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct KillProcessTool;

impl KillProcessTool {
    pub fn new(_context: Arc<ToolContext>) -> Self { Self }
}

#[async_trait]
impl Tool<TiccaDeps> for KillProcessTool {
    fn definition(&self) -> ToolDefinition {
        let spec = super::spec::kill_process_spec();
        ToolDefinition::new(spec.name, spec.description).with_parameters(spec.rig_parameters)
    }

    async fn call(&self, ctx: &RunContext<TiccaDeps>, args: JsonValue) -> ToolResult {
        let context = get_context(ctx);
        let args: KillProcessArgs = serde_json::from_value(args)?;

        let Some(store) = &context.system_exec_store else {
            return Err(ToolError::execution_failed("System execution store not configured"));
        };
        let Some(tx) = &context.system_exec_tx else {
            return Err(ToolError::execution_failed("System execution UI channel not configured"));
        };

        let request_id = SYSTEM_EXEC_REQUEST_ID.fetch_add(1, Ordering::SeqCst) as u64;
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel::<SystemExecResponse>();
        store.register_pending(request_id, resp_tx);

        tx.send(SystemExecRequest::KillProcess { request_id, process_id: args.process_id.clone() })
            .map_err(|_| ToolError::execution_failed("Failed to dispatch kill_process request"))?;

        match resp_rx.await.map_err(|_| ToolError::execution_failed("kill_process request was dropped"))? {
            SystemExecResponse::Killed { process_id } => Ok(ToolReturn::text(format!("Process `{}` terminated.", process_id))),
            SystemExecResponse::Error { message } => Err(ToolError::execution_failed(message)),
            other => Err(ToolError::execution_failed(format!("Unexpected response: {:?}", other))),
        }
    }
}

// ============================================================================
// 5. ReadFileTool
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct ReadFileArgs {
    pub path: String,
    pub start_line: Option<usize>,
    pub num_lines: Option<usize>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ReadFileTool;

impl ReadFileTool {
    pub fn new(_context: Arc<ToolContext>) -> Self { Self }
}

#[async_trait]
impl Tool<TiccaDeps> for ReadFileTool {
    fn definition(&self) -> ToolDefinition {
        let spec = super::spec::read_file_spec();
        ToolDefinition::new(spec.name, spec.description).with_parameters(spec.rig_parameters)
    }

    async fn call(&self, ctx: &RunContext<TiccaDeps>, args: JsonValue) -> ToolResult {
        let context = get_context(ctx);
        let args: ReadFileArgs = serde_json::from_value(args)?;

        let full_path = if PathBuf::from(&args.path).is_absolute() {
            PathBuf::from(&args.path)
        } else {
            context.working_directory.join(&args.path)
        };
        context.enforce_path(&full_path).map_err(ToolError::execution_failed)?;

        let path_str = full_path.to_string_lossy().to_string();
        let (start, num) = (args.start_line, args.num_lines);

        let result = tokio::task::spawn_blocking(move || super::file_ops::read_file_impl(&path_str, start, num))
            .await
            .map_err(|e| ToolError::execution_failed(format!("Task join error: {}", e)))?
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;

        if result.success {
            Ok(ToolReturn::text(truncate_output(&result.content, MAX_TOOL_OUTPUT_CHARS)))
        } else {
            Err(ToolError::execution_failed(result.error.unwrap_or_else(|| "Unknown error".to_string())))
        }
    }
}

// ============================================================================
// 6. ListFilesTool
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct ListFilesArgs {
    pub directory: Option<String>,
    pub recursive: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ListFilesTool;

impl ListFilesTool {
    pub fn new(_context: Arc<ToolContext>) -> Self { Self }
}

#[async_trait]
impl Tool<TiccaDeps> for ListFilesTool {
    fn definition(&self) -> ToolDefinition {
        let spec = super::spec::list_files_spec();
        ToolDefinition::new(spec.name, spec.description).with_parameters(spec.rig_parameters)
    }

    async fn call(&self, ctx: &RunContext<TiccaDeps>, args: JsonValue) -> ToolResult {
        let context = get_context(ctx);
        let args: ListFilesArgs = serde_json::from_value(args)?;

        let dir = args.directory.unwrap_or_else(|| ".".to_string());
        let full_path = if PathBuf::from(&dir).is_absolute() { PathBuf::from(&dir) } else { context.working_directory.join(&dir) };
        context.enforce_path(&full_path).map_err(ToolError::execution_failed)?;

        let path_str = full_path.to_string_lossy().to_string();
        let recursive = args.recursive.unwrap_or(false);

        let result = tokio::task::spawn_blocking(move || super::file_ops::list_files_impl(&path_str, recursive))
            .await
            .map_err(|e| ToolError::execution_failed(format!("Task join error: {}", e)))?
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;

        if result.success {
            Ok(ToolReturn::text(truncate_output(&result.content, MAX_TOOL_OUTPUT_CHARS)))
        } else {
            Err(ToolError::execution_failed(result.error.unwrap_or_else(|| "Unknown error".to_string())))
        }
    }
}

// ============================================================================
// 7. EditFileTool
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct EditReplacement {
    pub old_text: String,
    pub new_text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EditFileArgs {
    pub path: String,
    pub old_text: Option<String>,
    pub new_text: Option<String>,
    pub replacements: Option<Vec<EditReplacement>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EditFileTool;

impl EditFileTool {
    pub fn new(_context: Arc<ToolContext>) -> Self { Self }
}

#[async_trait]
impl Tool<TiccaDeps> for EditFileTool {
    fn definition(&self) -> ToolDefinition {
        let spec = super::spec::edit_file_spec();
        ToolDefinition::new(spec.name, spec.description).with_parameters(spec.rig_parameters)
    }

    async fn call(&self, ctx: &RunContext<TiccaDeps>, args: JsonValue) -> ToolResult {
        let context = get_context(ctx);
        let args: EditFileArgs = serde_json::from_value(args)?;

        let full_path = if PathBuf::from(&args.path).is_absolute() { PathBuf::from(&args.path) } else { context.working_directory.join(&args.path) };
        context.enforce_path(&full_path).map_err(ToolError::execution_failed)?;
        context.require_approval("edit_file", format!("path={}", args.path)).await.map_err(ToolError::execution_failed)?;

        let path_str = full_path.to_string_lossy().to_string();
        let replacements: Vec<JsonValue> = if let Some(batch) = args.replacements {
            batch.into_iter().map(|r| json!({ "old_str": r.old_text, "new_str": r.new_text })).collect()
        } else if let (Some(old_text), Some(new_text)) = (args.old_text, args.new_text) {
            vec![json!({ "old_str": old_text, "new_str": new_text })]
        } else {
            return Err(ToolError::execution_failed("Either 'replacements' or both 'old_text' and 'new_text' required"));
        };

        let params = json!({ "file_path": path_str, "replacements": replacements });
        let result = tokio::task::spawn_blocking(move || super::file_mods::edit_file_impl(params))
            .await
            .map_err(|e| ToolError::execution_failed(format!("Task join error: {}", e)))?
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;

        if result.success { Ok(ToolReturn::text(result.content)) }
        else { Err(ToolError::execution_failed(result.error.unwrap_or_else(|| "Unknown error".to_string()))) }
    }
}

// ============================================================================
// 8. WriteFileTool
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct WriteFileArgs {
    pub path: String,
    pub content: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WriteFileTool;

impl WriteFileTool {
    pub fn new(_context: Arc<ToolContext>) -> Self { Self }
}

#[async_trait]
impl Tool<TiccaDeps> for WriteFileTool {
    fn definition(&self) -> ToolDefinition {
        let spec = super::spec::write_file_spec();
        ToolDefinition::new(spec.name, spec.description).with_parameters(spec.rig_parameters)
    }

    async fn call(&self, ctx: &RunContext<TiccaDeps>, args: JsonValue) -> ToolResult {
        let context = get_context(ctx);
        let args: WriteFileArgs = serde_json::from_value(args)?;

        let full_path = if PathBuf::from(&args.path).is_absolute() { PathBuf::from(&args.path) } else { context.working_directory.join(&args.path) };
        context.enforce_path(&full_path).map_err(ToolError::execution_failed)?;
        context.require_approval("write_file", format!("path={}", args.path)).await.map_err(ToolError::execution_failed)?;

        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| ToolError::execution_failed(format!("Failed to create dirs: {}", e)))?;
        }

        let content_len = args.content.len();
        let display_path = full_path.display().to_string();
        tokio::fs::write(&full_path, &args.content).await.map_err(|e| ToolError::execution_failed(format!("Failed to write: {}", e)))?;
        Ok(ToolReturn::text(format!("Successfully wrote {} bytes to {}", content_len, display_path)))
    }
}

// ============================================================================
// 9. DeleteFileTool
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct DeleteFileArgs {
    pub path: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DeleteFileTool;

impl DeleteFileTool {
    pub fn new(_context: Arc<ToolContext>) -> Self { Self }
}

#[async_trait]
impl Tool<TiccaDeps> for DeleteFileTool {
    fn definition(&self) -> ToolDefinition {
        let spec = super::spec::delete_file_spec();
        ToolDefinition::new(spec.name, spec.description).with_parameters(spec.rig_parameters)
    }

    async fn call(&self, ctx: &RunContext<TiccaDeps>, args: JsonValue) -> ToolResult {
        let context = get_context(ctx);
        let args: DeleteFileArgs = serde_json::from_value(args)?;

        context.require_approval("delete_file", format!("path={}", args.path)).await.map_err(ToolError::execution_failed)?;

        let full_path = if PathBuf::from(&args.path).is_absolute() { PathBuf::from(&args.path) } else { context.working_directory.join(&args.path) };
        let path_str = full_path.to_string_lossy().to_string();

        let result = tokio::task::spawn_blocking(move || super::file_mods::delete_file_impl(&path_str))
            .await
            .map_err(|e| ToolError::execution_failed(format!("Task join error: {}", e)))?
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;

        if result.success { Ok(ToolReturn::text(result.content)) }
        else { Err(ToolError::execution_failed(result.error.unwrap_or_else(|| "Unknown error".to_string()))) }
    }
}

// ============================================================================
// 10. GrepTool
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct GrepArgs {
    pub search_string: String,
    pub directory: Option<String>,
    pub case_insensitive: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GrepTool;

impl GrepTool {
    pub fn new(_context: Arc<ToolContext>) -> Self { Self }
}

#[async_trait]
impl Tool<TiccaDeps> for GrepTool {
    fn definition(&self) -> ToolDefinition {
        let spec = super::spec::grep_spec(200);
        ToolDefinition::new(spec.name, spec.description).with_parameters(spec.rig_parameters)
    }

    async fn call(&self, ctx: &RunContext<TiccaDeps>, args: JsonValue) -> ToolResult {
        let context = get_context(ctx);
        let args: GrepArgs = serde_json::from_value(args)?;

        let search_path = args.directory.unwrap_or_else(|| ".".to_string());
        let full_path = if PathBuf::from(&search_path).is_absolute() { PathBuf::from(&search_path) } else { context.working_directory.join(&search_path) };
        context.enforce_path(&full_path).map_err(ToolError::execution_failed)?;

        let path_str = full_path.to_string_lossy().to_string();
        let pattern = if args.case_insensitive.unwrap_or(false) { format!("-i {}", args.search_string) } else { args.search_string };

        let result = tokio::task::spawn_blocking(move || super::grep::grep_impl(&pattern, &path_str))
            .await
            .map_err(|e| ToolError::execution_failed(format!("Task join error: {}", e)))?
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;

        if result.success { Ok(ToolReturn::text(truncate_output(&result.content, MAX_TOOL_OUTPUT_CHARS))) }
        else { Err(ToolError::execution_failed(result.error.unwrap_or_else(|| "Unknown error".to_string()))) }
    }
}

// ============================================================================
// 11. ListAgentsTool
// ============================================================================

#[derive(Clone, Serialize, Deserialize)]
pub struct ListAgentsTool;

impl ListAgentsTool {
    pub fn new(_context: Arc<ToolContext>) -> Self { Self }
}

#[async_trait]
impl Tool<TiccaDeps> for ListAgentsTool {
    fn definition(&self) -> ToolDefinition {
        let spec = super::spec::list_agents_spec();
        ToolDefinition::new(spec.name, spec.description).with_parameters(spec.rig_parameters)
    }

    async fn call(&self, _ctx: &RunContext<TiccaDeps>, _args: JsonValue) -> ToolResult {
        let mut lines = Vec::new();
        for agent in get_all_agents() {
            lines.push(format!("- {}: {} - {}", agent.agent_type().as_str(), agent.display_name(), agent.description()));
        }
        if lines.is_empty() { lines.push("No agents available.".to_string()); }
        Ok(ToolReturn::text(lines.join("\n")))
    }
}

// ============================================================================
// 12. InvokeAgentTool
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct InvokeAgentArgs {
    pub agent: String,
    pub prompt: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InvokeAgentTool;

impl InvokeAgentTool {
    pub fn new(_context: Arc<ToolContext>) -> Self { Self }
}

#[async_trait]
impl Tool<TiccaDeps> for InvokeAgentTool {
    fn definition(&self) -> ToolDefinition {
        let spec = super::spec::invoke_agent_spec();
        ToolDefinition::new(spec.name, spec.description).with_parameters(spec.rig_parameters)
    }

    async fn call(&self, ctx: &RunContext<TiccaDeps>, args: JsonValue) -> ToolResult {
        let context = get_context(ctx);
        let args: InvokeAgentArgs = serde_json::from_value(args)?;

        let agent_type = AgentType::parse(&args.agent)
            .ok_or_else(|| ToolError::execution_failed(format!("Unknown agent: {}", args.agent)))?;

        let child_id = context.call_graph_counter.as_ref().map(|c| c.fetch_add(1, Ordering::SeqCst)).unwrap_or(0);

        if let Some(tx) = &context.call_graph_tx {
            let _ = tx.send(AgentCallEvent {
                parent_id: context.node_id, child_id, parent: context.current_agent, child: agent_type, prompt: args.prompt.clone(),
            });
        }

        let invoker = context.agent_invoker.as_ref().ok_or_else(|| ToolError::execution_failed("Agent invocation not configured"))?;
        let result = invoker(AgentInvokeRequest { agent_type, prompt: args.prompt, parent_context: context.clone(), node_id: child_id })
            .await
            .map_err(ToolError::execution_failed)?;
        Ok(ToolReturn::text(result))
    }
}

// ============================================================================
// 13. ShareReasoningTool
// ============================================================================

#[derive(Debug, Deserialize, Serialize)]
pub struct ShareReasoningArgs {
    pub insight: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ShareReasoningTool;

impl ShareReasoningTool {
    pub fn new(_context: Arc<ToolContext>) -> Self { Self }
}

#[async_trait]
impl Tool<TiccaDeps> for ShareReasoningTool {
    fn definition(&self) -> ToolDefinition {
        let spec = super::spec::share_reasoning_spec();
        ToolDefinition::new(spec.name, spec.description).with_parameters(spec.rig_parameters)
    }

    async fn call(&self, _ctx: &RunContext<TiccaDeps>, args: JsonValue) -> ToolResult {
        let args: ShareReasoningArgs = serde_json::from_value(args)?;
        Ok(ToolReturn::text(format!("💡 {}", args.insight)))
    }
}

// ============================================================================
// 14. TodoReadTool
// ============================================================================

#[derive(Clone, Serialize, Deserialize)]
pub struct TodoReadTool;

impl TodoReadTool {
    pub fn new(_context: Arc<ToolContext>) -> Self { Self }
}

#[async_trait]
impl Tool<TiccaDeps> for TodoReadTool {
    fn definition(&self) -> ToolDefinition {
        let spec = super::spec::todo_read_spec();
        ToolDefinition::new(spec.name, spec.description).with_parameters(spec.rig_parameters)
    }

    async fn call(&self, ctx: &RunContext<TiccaDeps>, _args: JsonValue) -> ToolResult {
        let context = get_context(ctx);
        let store = context.todo_store.as_ref().ok_or_else(|| ToolError::execution_failed("Todo store not configured"))?;
        let state = store.snapshot(context.node_id).await;
        Ok(ToolReturn::text(state.format_markdown()))
    }
}

// ============================================================================
// 15. TodoWriteTool
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoListArgs {
    pub todos: Vec<TodoListItemArgs>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoListItemArgs {
    pub content: String,
    pub status: TodoStatus,
    pub active_form: String,
}

async fn update_todo_list(context: &Arc<ToolContext>, args: TodoListArgs) -> Result<TodoListState, ToolError> {
    let store = context.todo_store.as_ref().ok_or_else(|| ToolError::execution_failed("Todo store not configured"))?;
    let items: Vec<TodoItem> = args.todos.into_iter().map(|item| TodoItem {
        content: item.content, status: item.status, active_form: item.active_form,
    }).collect();
    let state = store.update_node(context.node_id, items).await;
    if let Some(tx) = &context.todo_tx {
        let _ = tx.send(TodoListEvent::Updated { node_id: context.node_id, state: state.clone() });
    }
    Ok(state)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TodoWriteTool;

impl TodoWriteTool {
    pub fn new(_context: Arc<ToolContext>) -> Self { Self }
}

#[async_trait]
impl Tool<TiccaDeps> for TodoWriteTool {
    fn definition(&self) -> ToolDefinition {
        let spec = super::spec::todo_write_spec();
        ToolDefinition::new(spec.name, spec.description).with_parameters(spec.rig_parameters)
    }

    async fn call(&self, ctx: &RunContext<TiccaDeps>, args: JsonValue) -> ToolResult {
        let context = get_context(ctx);
        let args: TodoListArgs = serde_json::from_value(args)?;
        let state = update_todo_list(context, args).await?;
        Ok(ToolReturn::text(state.format_markdown()))
    }
}

// ============================================================================
// 16. TodoListTool (legacy alias for TodoWriteTool)
// ============================================================================

#[derive(Clone, Serialize, Deserialize)]
pub struct TodoListTool;

impl TodoListTool {
    pub fn new(_context: Arc<ToolContext>) -> Self { Self }
}

#[async_trait]
impl Tool<TiccaDeps> for TodoListTool {
    fn definition(&self) -> ToolDefinition {
        let spec = super::spec::todo_list_spec();
        ToolDefinition::new(spec.name, spec.description).with_parameters(spec.rig_parameters)
    }

    async fn call(&self, ctx: &RunContext<TiccaDeps>, args: JsonValue) -> ToolResult {
        let context = get_context(ctx);
        let args: TodoListArgs = serde_json::from_value(args)?;
        let state = update_todo_list(context, args).await?;
        Ok(ToolReturn::text(state.format_markdown()))
    }
}

// ============================================================================
// Tool Creation Functions
// ============================================================================

pub type SerdesTools = (
    ExecuteShellTool, ListProcessesTool, ReadProcessOutputTool, KillProcessTool,
    ReadFileTool, ListFilesTool, EditFileTool, DeleteFileTool, GrepTool, WriteFileTool,
    ListAgentsTool, TodoReadTool, TodoWriteTool, TodoListTool, ShareReasoningTool, InvokeAgentTool,
);

/// Create all tools with the given context (tuple form for backward compatibility)
pub fn create_tools(context: Arc<ToolContext>) -> SerdesTools {
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
        TodoReadTool::new(context.clone()),
        TodoWriteTool::new(context.clone()),
        TodoListTool::new(context.clone()),
        ShareReasoningTool::new(context.clone()),
        InvokeAgentTool::new(context),
    )
}

/// All tools as a Vec for the registry
pub fn create_tools_vec(context: Arc<ToolContext>) -> Vec<Box<dyn Tool<TiccaDeps> + Send + Sync>> {
    vec![
        Box::new(ExecuteShellTool::new(context.clone())),
        Box::new(ListProcessesTool::new(context.clone())),
        Box::new(ReadProcessOutputTool::new(context.clone())),
        Box::new(KillProcessTool::new(context.clone())),
        Box::new(ReadFileTool::new(context.clone())),
        Box::new(ListFilesTool::new(context.clone())),
        Box::new(EditFileTool::new(context.clone())),
        Box::new(DeleteFileTool::new(context.clone())),
        Box::new(GrepTool::new(context.clone())),
        Box::new(WriteFileTool::new(context.clone())),
        Box::new(ListAgentsTool::new(context.clone())),
        Box::new(TodoReadTool::new(context.clone())),
        Box::new(TodoWriteTool::new(context.clone())),
        Box::new(TodoListTool::new(context.clone())),
        Box::new(ShareReasoningTool::new(context.clone())),
        Box::new(InvokeAgentTool::new(context)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_files_definition_matches_spec() {
        let context = Arc::new(ToolContext::default());
        let tool = ListFilesTool::new(context);
        let definition = tool.definition();
        let spec = super::super::spec::list_files_spec();
        assert_eq!(definition.name, spec.name);
        assert_eq!(definition.description, spec.description);
    }

    #[tokio::test]
    async fn test_share_reasoning_tool() {
        let tool = ShareReasoningTool;
        let ctx = RunContext::new(TiccaDeps::default(), "test-model");
        let args = json!({ "insight": "Found a bug!" });
        let result = tool.call(&ctx, args).await.unwrap();
        assert_eq!(result.as_text(), Some("💡 Found a bug!"));
    }
}
