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
