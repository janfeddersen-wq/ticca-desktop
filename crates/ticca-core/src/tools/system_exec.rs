//! System execution orchestration (UI-backed processes).
//!
//! This module provides a shared store and request/response types used by:
//! - UI (ticca-app) to spawn/track terminal instances
//! - LLM tools (ticca-core) to request shell execution and manage processes

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{Notify, oneshot};

use crate::tools::registry::{ToolDefinition, ToolExecutor, ToolResult};
use crate::tools::spec;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessKind {
    Llm,
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSnapshot {
    pub process_id: String,
    pub kind: ProcessKind,
    pub visible: bool,
    pub output: String,
    pub exit_code: Option<i32>,
    pub started_at_ms: u64,
    pub finished_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemExecRequest {
    ExecuteShell {
        request_id: u64,
        command: String,
        cwd: Option<String>,
    },
    KillProcess {
        request_id: u64,
        process_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemExecResponse {
    Started { process_id: String },
    Killed { process_id: String },
    Error { message: String },
}

#[derive(Default)]
struct Inner {
    processes: HashMap<String, ProcessEntry>,
    pending: HashMap<u64, oneshot::Sender<SystemExecResponse>>,
}

struct ProcessEntry {
    snapshot: ProcessSnapshot,
    notify: Arc<Notify>,
}

#[derive(Default)]
pub struct SystemExecStore {
    inner: Mutex<Inner>,
}

impl std::fmt::Debug for SystemExecStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemExecStore").finish_non_exhaustive()
    }
}

impl SystemExecStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_pending(
        &self,
        request_id: u64,
        responder: oneshot::Sender<SystemExecResponse>,
    ) {
        let mut inner = self.inner.lock().expect("SystemExecStore mutex poisoned");
        inner.pending.insert(request_id, responder);
    }

    pub fn respond(&self, request_id: u64, response: SystemExecResponse) {
        let responder = {
            let mut inner = self.inner.lock().expect("SystemExecStore mutex poisoned");
            inner.pending.remove(&request_id)
        };
        if let Some(tx) = responder {
            let _ = tx.send(response);
        }
    }

    pub fn upsert_process(&self, snapshot: ProcessSnapshot) {
        let notify = {
            let mut inner = self.inner.lock().expect("SystemExecStore mutex poisoned");
            let notify = inner
                .processes
                .get(&snapshot.process_id)
                .map(|p| p.notify.clone())
                .unwrap_or_else(|| Arc::new(Notify::new()));
            inner.processes.insert(
                snapshot.process_id.clone(),
                ProcessEntry {
                    snapshot,
                    notify: notify.clone(),
                },
            );
            notify
        };
        notify.notify_waiters();
    }

    pub fn set_output(&self, process_id: &str, output: String) {
        let notify = {
            let mut inner = self.inner.lock().expect("SystemExecStore mutex poisoned");
            let Some(entry) = inner.processes.get_mut(process_id) else {
                return;
            };
            entry.snapshot.output = output;
            entry.notify.clone()
        };
        notify.notify_waiters();
    }

    pub fn mark_finished(&self, process_id: &str, exit_code: Option<i32>) {
        let now_ms = now_ms();
        let notify = {
            let mut inner = self.inner.lock().expect("SystemExecStore mutex poisoned");
            let Some(entry) = inner.processes.get_mut(process_id) else {
                return;
            };
            entry.snapshot.exit_code = exit_code;
            entry.snapshot.finished_at_ms = Some(now_ms);
            entry.notify.clone()
        };
        notify.notify_waiters();
    }

    pub fn set_visible(&self, process_id: &str, visible: bool) {
        let notify = {
            let mut inner = self.inner.lock().expect("SystemExecStore mutex poisoned");
            let Some(entry) = inner.processes.get_mut(process_id) else {
                return;
            };
            entry.snapshot.visible = visible;
            entry.notify.clone()
        };
        notify.notify_waiters();
    }

    pub fn remove_process(&self, process_id: &str) {
        let mut inner = self.inner.lock().expect("SystemExecStore mutex poisoned");
        inner.processes.remove(process_id);
    }

    pub fn list_visible(&self) -> Vec<String> {
        let inner = self.inner.lock().expect("SystemExecStore mutex poisoned");
        let mut ids: Vec<String> = inner
            .processes
            .values()
            .filter(|p| p.snapshot.visible)
            .map(|p| p.snapshot.process_id.clone())
            .collect();
        ids.sort();
        ids
    }

    pub fn snapshot(&self, process_id: &str) -> Option<ProcessSnapshot> {
        let inner = self.inner.lock().expect("SystemExecStore mutex poisoned");
        inner.processes.get(process_id).map(|p| p.snapshot.clone())
    }

    pub fn output(&self, process_id: &str) -> Option<String> {
        self.snapshot(process_id).map(|s| s.output)
    }

    pub fn exit_code(&self, process_id: &str) -> Option<Option<i32>> {
        self.snapshot(process_id).map(|s| s.exit_code)
    }

    pub async fn wait_for_update(&self, process_id: &str) -> Result<(), String> {
        let notify = {
            let inner = self.inner.lock().expect("SystemExecStore mutex poisoned");
            inner
                .processes
                .get(process_id)
                .map(|p| p.notify.clone())
                .ok_or_else(|| format!("Unknown process: {}", process_id))?
        };
        notify.notified().await;
        Ok(())
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ---------------------------------------------------------------------------
// Tool registry definitions (UI-backed tools are not directly executable here).
// These are provided to keep tool discovery consistent; actual execution is
// routed through the rig tools with a configured UI process manager.
// ---------------------------------------------------------------------------

pub fn execute_shell_definition() -> ToolDefinition {
    let spec = spec::execute_shell_spec(60);
    ToolDefinition {
        name: spec.name.to_string(),
        description: spec.description.to_string(),
        parameters: spec.registry_parameters,
    }
}

pub fn execute_shell_executor() -> ToolExecutor {
    Arc::new(|_params: Value| {
        Box::pin(async move {
            Ok(ToolResult::error(
                "execute_shell is UI-backed and not available in the standalone registry",
            ))
        })
    })
}

pub fn list_processes_definition() -> ToolDefinition {
    let spec = spec::list_processes_spec();
    ToolDefinition {
        name: spec.name.to_string(),
        description: spec.description.to_string(),
        parameters: spec.registry_parameters,
    }
}

pub fn list_processes_executor() -> ToolExecutor {
    Arc::new(|_params: Value| {
        Box::pin(async move {
            Ok(ToolResult::error(
                "list_processes is UI-backed and not available in the standalone registry",
            ))
        })
    })
}

pub fn read_process_output_definition() -> ToolDefinition {
    let spec = spec::read_process_output_spec();
    ToolDefinition {
        name: spec.name.to_string(),
        description: spec.description.to_string(),
        parameters: spec.registry_parameters,
    }
}

pub fn read_process_output_executor() -> ToolExecutor {
    Arc::new(|_params: Value| {
        Box::pin(async move {
            Ok(ToolResult::error(
                "read_process_output is UI-backed and not available in the standalone registry",
            ))
        })
    })
}

pub fn kill_process_definition() -> ToolDefinition {
    let spec = spec::kill_process_spec();
    ToolDefinition {
        name: spec.name.to_string(),
        description: spec.description.to_string(),
        parameters: spec.registry_parameters,
    }
}

pub fn kill_process_executor() -> ToolExecutor {
    Arc::new(|_params: Value| {
        Box::pin(async move {
            Ok(ToolResult::error(
                "kill_process is UI-backed and not available in the standalone registry",
            ))
        })
    })
}
