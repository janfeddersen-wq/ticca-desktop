//! Right Sidebar - Todo List, Agent Flow, System Executions
//!
//! Collapsible panel showing task lists, agent call graphs, and command outputs.

use gpui::{
    div, prelude::FluentBuilder as _, px, AnyElement, Context, FontWeight,
    InteractiveElement as _, IntoElement, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window,
};
use gpui_component::{
    h_flex,
    tab::{Tab, TabBar},
    v_flex, ActiveTheme, Sizable as _,
};

use crate::actions::*;
use crate::app::{TiccaApp, AgentStatus, ExecutionStatus as AppExecutionStatus};

// =============================================================================
// Main Sidebar
// =============================================================================

/// Render the right sidebar with tabs for Todo, Agents, and Executions
pub fn render_sidebar(
    app: &TiccaApp,
    _window: &mut Window,
    cx: &mut Context<TiccaApp>,
) -> AnyElement {
    let theme = cx.theme();
    let sidebar_tab = app.chat.sidebar_tab;

    // Map current tab to index
    let selected_index = match sidebar_tab {
        RightSidebarTab::TodoList => 0,
        RightSidebarTab::AgentsFlow => 1,
        RightSidebarTab::SystemExecutions => 2,
    };

    v_flex()
        .w(px(280.))
        .h_full()
        .border_l_1()
        .border_color(theme.border)
        .bg(theme.sidebar)
        .child(
            // Sidebar header with tabs
            div()
                .w_full()
                .border_b_1()
                .border_color(theme.border)
                .child(
                    TabBar::new("sidebar-tabs")
                        .small()
                        .selected_index(selected_index)
                        .on_click(cx.listener(|this, idx: &usize, _, cx| {
                            this.chat.sidebar_tab = match idx {
                                0 => RightSidebarTab::TodoList,
                                1 => RightSidebarTab::AgentsFlow,
                                2 => RightSidebarTab::SystemExecutions,
                                _ => RightSidebarTab::TodoList,
                            };
                            cx.notify();
                        }))
                        .child(Tab::new().label("Todo"))
                        .child(Tab::new().label("Agents"))
                        .child(Tab::new().label("Exec")),
                ),
        )
        .child(
            // Tab content
            div()
                .id("sidebar-content")
                .flex_1()
                .overflow_y_scroll()
                .p_3()
                .child(match sidebar_tab {
                    RightSidebarTab::TodoList => render_todo_list(app, cx).into_any_element(),
                    RightSidebarTab::AgentsFlow => render_agent_flow(app, cx).into_any_element(),
                    RightSidebarTab::SystemExecutions => {
                        render_system_executions(app, cx).into_any_element()
                    }
                }),
        )
        .into_any_element()
}

// =============================================================================
// Todo List Tab
// =============================================================================

/// Render the todo list tab content
fn render_todo_list(app: &TiccaApp, cx: &mut Context<TiccaApp>) -> impl IntoElement {
    use ticca_core::tools::TodoStatus as CoreTodoStatus;
    
    let theme = cx.theme();
    let todo_state = app.chat.todo_state.clone().unwrap_or_default();
    
    let total = todo_state.items.len();
    let completed = todo_state.items.iter()
        .filter(|item| item.status == CoreTodoStatus::Completed)
        .count();
    let in_progress = todo_state.items.iter()
        .filter(|item| item.status == CoreTodoStatus::InProgress)
        .count();
    
    let all_completed = !todo_state.items.is_empty() 
        && todo_state.items.iter().all(|item| item.status == CoreTodoStatus::Completed);
    
    let status_text = if all_completed {
        "All complete ✓"
    } else if in_progress > 0 {
        "In progress..."
    } else {
        "Pending"
    };

    v_flex()
        .gap_3()
        .child(
            h_flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Todo List"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(if all_completed { theme.success } else { theme.muted_foreground })
                        .child(status_text.to_string()),
                ),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(format!("{} items • {} completed{}",
                    total,
                    completed,
                    if in_progress > 0 { format!(" • {} in progress", in_progress) } else { String::new() }
                )),
        )
        .when(todo_state.items.is_empty(), |el| {
            el.child(
                v_flex()
                    .gap_1()
                    .mt_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("No items yet."),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("The agent can populate this via the todo_list tool."),
                    ),
            )
        })
        .when(!todo_state.items.is_empty(), |el| {
            let items: Vec<_> = todo_state.items.iter().map(|item| {
                let status = match item.status {
                    CoreTodoStatus::Pending => TodoStatus::Pending,
                    CoreTodoStatus::InProgress => TodoStatus::InProgress,
                    CoreTodoStatus::Completed => TodoStatus::Done,
                };
                (item.content.clone(), status)
            }).collect();
            
            el.child(
                v_flex()
                    .gap_2()
                    .mt_2()
                    .children(items.into_iter().map(|(content, status)| {
                        render_todo_item_simple(content, status)
                    }))
            )
        })
}

#[derive(Clone, Copy, PartialEq)]
enum TodoStatus {
    Pending,
    InProgress,
    Done,
}

/// Simplified todo item that doesn't need context
fn render_todo_item_simple(
    label: String,
    status: TodoStatus,
) -> gpui::AnyElement {
    let (icon, is_done) = match status {
        TodoStatus::Pending => ("○", false),
        TodoStatus::InProgress => ("◐", false),
        TodoStatus::Done => ("●", true),
    };

    h_flex()
        .gap_2()
        .items_center()
        .py_1()
        .child(
            div()
                .text_sm()
                .child(icon),
        )
        .child(
            div()
                .text_sm()
                .when(is_done, |d| d.line_through())
                .child(label),
        )
        .into_any_element()
}

#[allow(dead_code)]
fn render_todo_item(
    label: &str,
    status: TodoStatus,
    cx: &mut Context<TiccaApp>,
) -> impl IntoElement {
    let theme = cx.theme();

    let (icon, color) = match status {
        TodoStatus::Pending => ("○", theme.muted_foreground),
        TodoStatus::InProgress => ("◐", theme.link),
        TodoStatus::Done => ("●", theme.success),
    };

    h_flex()
        .gap_2()
        .items_center()
        .py_1()
        .child(
            div()
                .text_sm()
                .text_color(color)
                .child(icon),
        )
        .child(
            div()
                .text_sm()
                .when(status == TodoStatus::Done, |d| {
                    d.text_color(theme.muted_foreground)
                        .line_through()
                })
                .child(label.to_string()),
        )
}

// =============================================================================
// Agent Flow Tab
// =============================================================================

/// Render the agent flow tab content
fn render_agent_flow(app: &TiccaApp, cx: &mut Context<TiccaApp>) -> impl IntoElement {
    let theme = cx.theme();
    let graph = &app.chat.agent_graph;
    
    let is_running = graph.is_any_running();
    let node_count = graph.nodes.len();
    let run_count = graph.current_run;
    
    let status_text = if is_running {
        "Running..."
    } else if node_count > 0 {
        "Complete"
    } else {
        "Idle"
    };

    v_flex()
        .gap_3()
        .child(
            h_flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Agent Flow"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(if is_running { theme.link } else { theme.muted_foreground })
                        .child(status_text.to_string()),
                ),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(format!("{} agents • {} runs", node_count, run_count)),
        )
        // Empty state
        .when(node_count == 0, |el| {
            el.child(
                v_flex()
                    .gap_1()
                    .mt_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("No agent executions yet."),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("Send a message to start the agent flow."),
                    ),
            )
        })
        // Real agent nodes (collect data before rendering to avoid borrow issues)
        .children({
            if node_count > 0 {
                // Collect nodes in order for rendering
                let nodes: Vec<_> = graph.order.iter()
                    .filter_map(|id| graph.nodes.get(id))
                    .map(|node| {
                        let elapsed = node.elapsed().map(|d| format!("{:.1}s", d.as_secs_f64()));
                        (node.label.clone(), node.status, elapsed)
                    })
                    .collect();
                
                vec![v_flex()
                    .gap_1()
                    .mt_2()
                    .p_2()
                    .rounded_md()
                    .bg(theme.secondary)
                    .children(nodes.into_iter().enumerate().map(|(idx, (label, status, elapsed))| {
                        let is_first = idx == 0;
                        let indent = if is_first { 0 } else { 1 };
                        
                        v_flex()
                            .when(!is_first, |el| {
                                el.child(render_agent_connector())
                            })
                            .child(render_agent_node_simple(&label, indent, status, elapsed))
                            .into_any_element()
                    }))
                    .into_any_element()]
            } else {
                vec![]
            }
        })
}

/// Render a simple agent node without needing context
fn render_agent_node_simple(
    label: &str,
    indent: usize,
    status: AgentStatus,
    elapsed: Option<String>,
) -> gpui::AnyElement {
    let (status_icon, is_running, is_failed) = match status {
        AgentStatus::Running => ("●", true, false),
        AgentStatus::Completed => ("✓", false, false),
        AgentStatus::Failed => ("✗", false, true),
    };

    h_flex()
        .ml(px((indent * 16) as f32))
        .gap_2()
        .items_center()
        .child(
            div()
                .text_xs()
                .child(status_icon),
        )
        .child(
            div()
                .text_xs()
                .font_weight(if is_running {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                })
                .child(label.to_string()),
        )
        .when(elapsed.is_some(), |el| {
            el.child(
                div()
                    .text_xs()
                    .child(elapsed.unwrap_or_default()),
            )
        })
        .into_any_element()
}

/// Render a real agent node from graph data
fn render_agent_node_real(
    label: &str,
    indent: usize,
    status: AgentStatus,
    elapsed: Option<String>,
    cx: &mut Context<TiccaApp>,
) -> impl IntoElement {
    let theme = cx.theme();
    
    let (status_icon, status_color) = match status {
        AgentStatus::Running => ("●", theme.link),
        AgentStatus::Completed => ("✓", theme.success),
        AgentStatus::Failed => ("✗", theme.danger),
    };

    h_flex()
        .ml(px((indent * 16) as f32))
        .gap_2()
        .items_center()
        .child(
            div()
                .text_xs()
                .text_color(status_color)
                .child(status_icon),
        )
        .child(
            div()
                .text_xs()
                .font_weight(if status == AgentStatus::Running {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                })
                .child(label.to_string()),
        )
        .when(elapsed.is_some(), |el| {
            el.child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(elapsed.unwrap_or_default()),
            )
        })
}

#[allow(dead_code)]
fn render_agent_node(
    name: &str,
    indent: usize,
    is_active: bool,
    cx: &mut Context<TiccaApp>,
) -> impl IntoElement {
    let theme = cx.theme();

    h_flex()
        .ml(px((indent * 16) as f32))
        .gap_2()
        .items_center()
        .child(
            div()
                .w(px(8.))
                .h(px(8.))
                .rounded_full()
                .when(is_active, |d| d.bg(theme.success))
                .when(!is_active, |d| d.bg(theme.muted_foreground)),
        )
        .child(
            div()
                .text_xs()
                .font_weight(if is_active {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                })
                .child(name.to_string()),
        )
}

fn render_agent_connector() -> impl IntoElement {
    div()
        .ml(px(3.))
        .w(px(2.))
        .h(px(12.))
        .bg(gpui::rgb(0x666666))
}

// =============================================================================
// System Executions Tab
// =============================================================================

/// Render the system executions tab content
fn render_system_executions(app: &TiccaApp, cx: &mut Context<TiccaApp>) -> impl IntoElement {
    let theme = cx.theme();
    let executions = &app.chat.system_executions;
    
    let total = executions.len();
    let running = executions.iter()
        .filter(|e| e.status == AppExecutionStatus::Running)
        .count();
    let succeeded = executions.iter()
        .filter(|e| e.status == AppExecutionStatus::Success)
        .count();
    let failed = executions.iter()
        .filter(|e| e.status == AppExecutionStatus::Failed)
        .count();

    v_flex()
        .gap_3()
        .child(
            h_flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("System Executions"),
                )
                .when(running > 0, |el| {
                    el.child(
                        div()
                            .text_xs()
                            .text_color(theme.link)
                            .child(format!("{} running", running)),
                    )
                }),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(format!("{} commands • {} ok{}",
                    total,
                    succeeded,
                    if failed > 0 { format!(" • {} failed", failed) } else { String::new() }
                )),
        )
        // Empty state
        .when(total == 0, |el| {
            el.child(
                v_flex()
                    .gap_1()
                    .mt_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("No commands executed yet."),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("Shell commands from the agent will appear here."),
                    ),
            )
        })
        // Real executions (show last 10)
        .when(total > 0, |el| {
            let recent: Vec<_> = executions.iter()
                .rev()
                .take(10)
                .map(|exec| {
                    let elapsed = format!("{:.1}s", exec.elapsed().as_secs_f64());
                    let output_preview = if exec.output.len() > 150 {
                        format!("{}...", &exec.output[..150])
                    } else {
                        exec.output.clone()
                    };
                    (exec.command.clone(), output_preview, exec.status, elapsed, exec.exit_code)
                })
                .collect();
            
            el.child(
                v_flex()
                    .gap_2()
                    .mt_2()
                    .children(recent.into_iter().map(|(cmd, output, status, elapsed, exit_code)| {
                        render_execution_item_real(&cmd, &output, status, &elapsed, exit_code, cx)
                            .into_any_element()
                    })),
            )
        })
}

/// Render a real execution item
fn render_execution_item_real(
    command: &str,
    output: &str,
    status: AppExecutionStatus,
    elapsed: &str,
    exit_code: Option<i32>,
    cx: &mut Context<TiccaApp>,
) -> impl IntoElement {
    let theme = cx.theme();

    let (status_icon, status_color) = match status {
        AppExecutionStatus::Running => ("●", theme.link),
        AppExecutionStatus::Success => ("✓", theme.success),
        AppExecutionStatus::Failed => ("✗", theme.danger),
    };

    v_flex()
        .p_2()
        .rounded_md()
        .bg(theme.secondary)
        .gap_1()
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .justify_between()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .text_xs()
                                .text_color(status_color)
                                .child(status_icon),
                        )
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::MEDIUM)
                                .overflow_hidden()
                                .child({
                                    let cmd_preview = if command.len() > 40 {
                                        format!("$ {}...", &command[..40])
                                    } else {
                                        format!("$ {}", command)
                                    };
                                    cmd_preview
                                }),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(format!("{}{}", 
                            elapsed,
                            exit_code.map(|c| format!(" ({})", c)).unwrap_or_default()
                        )),
                ),
        )
        .when(!output.is_empty(), |el| {
            el.child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .overflow_hidden()
                    .max_h(px(60.))
                    .child(output.to_string()),
            )
        })
}

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq)]
enum ExecutionStatus {
    Running,
    Success,
    Failed,
}

#[allow(dead_code)]
fn render_execution_item(
    command: &str,
    output: &str,
    status: ExecutionStatus,
    cx: &mut Context<TiccaApp>,
) -> impl IntoElement {
    let theme = cx.theme();

    let status_color = match status {
        ExecutionStatus::Running => theme.link,
        ExecutionStatus::Success => theme.success,
        ExecutionStatus::Failed => theme.danger,
    };

    v_flex()
        .p_2()
        .rounded_md()
        .bg(theme.secondary)
        .gap_1()
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .w(px(6.))
                        .h(px(6.))
                        .rounded_full()
                        .bg(status_color),
                )
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .child(format!("$ {}", command)),
                ),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(output.to_string()),
        )
}
