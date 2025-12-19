//! Tool approval gating for destructive actions

use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{mpsc, oneshot};

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub struct ToolApprovalRequest {
    pub id: u64,
    pub tool_name: String,
    pub args: String,
    pub responder: oneshot::Sender<bool>,
}

#[derive(Debug, Clone)]
pub struct ToolApprovalDecision {
    pub id: u64,
    pub approved: bool,
}

#[derive(Debug, Clone)]
pub struct ToolApprovalGate {
    request_tx: mpsc::UnboundedSender<ToolApprovalRequest>,
}

impl ToolApprovalGate {
    pub fn new(request_tx: mpsc::UnboundedSender<ToolApprovalRequest>) -> Self {
        Self { request_tx }
    }

    pub async fn request(&self, tool_name: &str, args: String) -> bool {
        let id = REQUEST_ID.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        let request = ToolApprovalRequest {
            id,
            tool_name: tool_name.to_string(),
            args,
            responder: tx,
        };

        if self.request_tx.send(request).is_err() {
            return false;
        }

        rx.await.unwrap_or(false)
    }
}
