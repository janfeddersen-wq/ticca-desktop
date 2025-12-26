//! Agent-scoped To Do list state and events.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoItem {
    pub content: String,
    pub status: TodoStatus,
    /// Present continuous form shown during execution (e.g., "Running tests")
    pub active_form: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoListState {
    pub items: Vec<TodoItem>,
}

impl TodoListState {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn format_markdown(&self) -> String {
        let total = self.items.len();
        let completed = self
            .items
            .iter()
            .filter(|item| item.status == TodoStatus::Completed)
            .count();
        let in_progress = self
            .items
            .iter()
            .filter(|item| item.status == TodoStatus::InProgress)
            .count();
        let pending = self
            .items
            .iter()
            .filter(|item| item.status == TodoStatus::Pending)
            .count();

        let mut lines = Vec::new();
        lines.push("## To Do".to_string());
        lines.push(format!(
            "{} items: {} completed, {} in progress, {} pending",
            total, completed, in_progress, pending
        ));
        lines.push(String::new());

        if self.items.is_empty() {
            lines.push("No items.".to_string());
        } else {
            for item in &self.items {
                let icon = match item.status {
                    TodoStatus::Pending => "○",
                    TodoStatus::InProgress => "→",
                    TodoStatus::Completed => "✔",
                };
                lines.push(format!("{} {}", icon, item.content));
            }
        }

        lines.join("\n")
    }
}

impl Default for TodoListState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TodoListEvent {
    Reset {
        node_id: usize,
        state: TodoListState,
    },
    Updated {
        node_id: usize,
        state: TodoListState,
    },
}

#[derive(Debug, Default)]
pub struct TodoStore {
    inner: RwLock<HashMap<usize, TodoListState>>,
}

impl TodoStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn set_node_state(&self, node_id: usize, state: TodoListState) -> TodoListState {
        let mut guard = self.inner.write().await;
        guard.insert(node_id, state.clone());
        state
    }

    pub async fn reset_node(&self, node_id: usize) -> TodoListState {
        let mut guard = self.inner.write().await;
        let state = TodoListState::new();
        guard.insert(node_id, state.clone());
        state
    }

    pub async fn update_node(&self, node_id: usize, mut items: Vec<TodoItem>) -> TodoListState {
        items.retain(|item| !item.content.trim().is_empty());
        let state = TodoListState { items };
        let mut guard = self.inner.write().await;
        guard.insert(node_id, state.clone());
        state
    }

    pub async fn snapshot(&self, node_id: usize) -> TodoListState {
        let guard = self.inner.read().await;
        guard.get(&node_id).cloned().unwrap_or_default()
    }
}
