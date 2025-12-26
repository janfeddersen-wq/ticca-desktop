//! Chat types and constants.

use crate::messages::RightSidebarTab;

/// Internal pane types for the chat view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ChatPane {
    Chat,
    Flow,
}

/// Starting node ID for todo lists.
pub(super) const STARTING_TODO_NODE_ID: usize = 0;

/// Enforce sidebar tab based on UI mode.
pub(super) fn enforce_sidebar_tab(shows_expert_ui: bool, tab: RightSidebarTab) -> RightSidebarTab {
    if shows_expert_ui {
        return tab;
    }

    match tab {
        RightSidebarTab::AgentsFlow | RightSidebarTab::SystemExecutions => {
            RightSidebarTab::TodoList
        }
        RightSidebarTab::TodoList => RightSidebarTab::TodoList,
    }
}

/// Enforce todo node selection based on UI mode.
pub(super) fn enforce_todo_selected_node(shows_internal_agents: bool, node_id: usize) -> usize {
    if shows_internal_agents {
        node_id
    } else {
        STARTING_TODO_NODE_ID
    }
}

/// Tool approval prompt data.
#[derive(Debug, Clone)]
pub(in crate::app) struct ToolApprovalPrompt {
    pub id: u64,
    pub name: String,
    pub args: String,
}
