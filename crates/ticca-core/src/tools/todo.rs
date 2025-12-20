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
pub struct TodoItem {
    pub text: String,
    pub status: TodoStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoListState {
    pub items: Vec<TodoItem>,
    pub confirmed_complete: bool,
}

impl TodoListState {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            confirmed_complete: false,
        }
    }

    pub fn is_completed_and_confirmed(&self) -> bool {
        self.confirmed_complete
            && self
                .items
                .iter()
                .all(|item| item.status == TodoStatus::Completed)
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

        let status_line = if self.is_completed_and_confirmed() {
            "✅ Confirmed complete"
        } else {
            "⚠️ Not confirmed"
        };

        let mut lines = Vec::new();
        lines.push("## To Do".to_string());
        lines.push(status_line.to_string());
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
                lines.push(format!("{} {}", icon, item.text));
            }
        }

        if !self.is_completed_and_confirmed() {
            lines.push(String::new());
            lines.push(
                "When all items are completed, call todo_write (or todo_list) with confirmed_complete=true."
                    .to_string(),
            );
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

    pub async fn update_node(
        &self,
        node_id: usize,
        mut items: Vec<TodoItem>,
        confirmed_complete: Option<bool>,
    ) -> TodoListState {
        items.retain(|item| !item.text.trim().is_empty());

        let is_all_completed = items
            .iter()
            .all(|item| item.status == TodoStatus::Completed);
        let confirmed_complete = match confirmed_complete {
            Some(true) => is_all_completed,
            Some(false) | None => false,
        };

        let state = TodoListState {
            items,
            confirmed_complete,
        };

        let mut guard = self.inner.write().await;
        guard.insert(node_id, state.clone());
        state
    }

    pub async fn snapshot(&self, node_id: usize) -> TodoListState {
        let guard = self.inner.read().await;
        guard.get(&node_id).cloned().unwrap_or_default()
    }
}
