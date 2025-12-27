//! TUI UI Rendering
//!
//! Main rendering dispatcher that routes to appropriate view renderers.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

use super::app::{ModalKind, TuiApp, View};
use super::views;
use super::widgets::popup;

/// Main render function - dispatches to appropriate view
pub fn render(frame: &mut Frame, app: &TuiApp) {
    let area = frame.area();

    // Render the main view
    match app.view {
        View::Chat => render_chat_view(frame, app, area),
        View::Settings => render_settings_view(frame, app, area),
        View::Help => render_help_view(frame, app, area),
    }

    // Render modal overlay if present
    if let Some(modal) = &app.active_modal {
        render_modal(frame, app, modal, area);
    }

    // Render error message if present
    if let Some(error) = &app.error_message {
        render_error_toast(frame, app, error, area);
    }
}

/// Render the chat view (placeholder for now)
fn render_chat_view(frame: &mut Frame, app: &TuiApp, area: Rect) {
    views::chat::render(frame, app, area);
}

/// Render the settings view (placeholder for now)
fn render_settings_view(frame: &mut Frame, app: &TuiApp, area: Rect) {
    super::views::settings::render(frame, app, area);
}

/// Render the help overlay
fn render_help_view(frame: &mut Frame, app: &TuiApp, area: Rect) {
    // Semi-transparent overlay effect (just render help on top)
    let help_area = centered_rect(70, 80, area);

    frame.render_widget(Clear, help_area);

    let help_text = r#"
╔═══════════════════════════════════════════════════════════════════════════════╗
║                              KEYBOARD SHORTCUTS                               ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  GLOBAL                              CHAT                                     ║
║  ──────────────────────────────      ────────────────────────────             ║
║  q, Ctrl+Q      Quit                 Ctrl+Enter    Send message               ║
║  Ctrl+,         Open Settings        Ctrl+N        New session                ║
║  ?              Show this help       Ctrl+C        Stop streaming             ║
║  Escape         Close/Back           F1            Pick agent                 ║
║  Tab            Cycle focus          F2            Pick model                 ║
║                                      F3            Toggle YOLO mode           ║
║                                      ↑/↓           Scroll messages            ║
║  SETTINGS                            PgUp/PgDn     Page scroll                ║
║  ──────────────────────────────                                               ║
║  1-7            Switch to tab        TOOL APPROVAL                            ║
║  ←/→            Previous/Next tab    ────────────────────────────             ║
║  n              Add new item         Y             Approve                    ║
║  e              Edit selected        N             Deny                       ║
║  d              Delete selected      A             Always approve             ║
║  Space          Toggle checkbox                                               ║
║  r              Refresh                                                       ║
║                                                                               ║
║                         [Press any key to close]                              ║
╚═══════════════════════════════════════════════════════════════════════════════╝
"#;

    let paragraph = Paragraph::new(help_text)
        .style(app.styles.text())
        .block(app.styles.modal("Keyboard Shortcuts"));

    frame.render_widget(paragraph, help_area);
}

/// Render modal popup
fn render_modal(frame: &mut Frame, app: &TuiApp, modal: &ModalKind, area: Rect) {
    match modal {
        ModalKind::ToolApproval(prompt) => {
            let modal_area = centered_rect(60, 50, area);
            frame.render_widget(Clear, modal_area);
            render_tool_approval_modal(frame, app, prompt, modal_area);
        }
        ModalKind::AgentPicker => {
            let agents = [
                (
                    "Planning Agent",
                    app.current_agent == ticca_core::agents::AgentType::Planning,
                ),
                (
                    "Coding Agent",
                    app.current_agent == ticca_core::agents::AgentType::Coding,
                ),
                (
                    "Skills Agent",
                    app.current_agent == ticca_core::agents::AgentType::Skills,
                ),
            ];
            popup::render_list_picker(
                frame,
                "Select Agent",
                &agents,
                app.modal_list_index,
                &app.colors,
                area,
            );
        }
        ModalKind::ModelPicker => {
            let models: Vec<(&str, bool)> = app
                .available_models
                .iter()
                .map(|m| (m.as_str(), app.current_model.as_deref() == Some(m.as_str())))
                .collect();
            popup::render_list_picker(
                frame,
                "Select Model",
                &models,
                app.modal_list_index,
                &app.colors,
                area,
            );
        }
        ModalKind::ThemePicker => {
            let themes: Vec<(&str, bool)> = crate::theme::ALL_THEMES
                .iter()
                .map(|t| (t.display_name(), *t == app.theme))
                .collect();
            popup::render_list_picker(
                frame,
                "Select Theme",
                &themes,
                app.modal_list_index,
                &app.colors,
                area,
            );
        }
        ModalKind::FilePicker {
            path,
            entries,
            selected,
        } => {
            popup::render_file_picker(frame, path, entries, *selected, &app.colors, area);
        }
        ModalKind::Confirmation { title, message, .. } => {
            popup::render_confirmation(frame, title, message, &app.colors, area);
        }
        ModalKind::Error(msg) => {
            popup::render_confirmation(frame, "Error", msg, &app.colors, area);
        }
        ModalKind::ApiKeyForm(form) => {
            let fields: Vec<(&str, &str, bool)> = form
                .fields
                .iter()
                .enumerate()
                .map(|(i, f)| (f.label.as_str(), f.value.as_str(), i == form.focused_field))
                .collect();
            popup::render_input_form(frame, "Add API Key", &fields, &app.colors, area);
        }
        ModalKind::McpServerForm(form) => {
            let fields: Vec<(&str, &str, bool)> = form
                .fields
                .iter()
                .enumerate()
                .map(|(i, f)| (f.label.as_str(), f.value.as_str(), i == form.focused_field))
                .collect();
            popup::render_input_form(frame, "MCP Server", &fields, &app.colors, area);
        }
    }
}

fn render_tool_approval_modal(
    frame: &mut Frame,
    app: &TuiApp,
    prompt: &super::app::ToolApprovalPrompt,
    area: Rect,
) {
    let title = format!("Tool Approval: {}", prompt.tool_name);
    let block = app.styles.modal(&title);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let text = format!(
        "The agent wants to execute:\n\n  Tool: {}\n\n  Arguments:\n{}\n\n[Y] Approve  [N] Deny  [A] Always Approve",
        prompt.tool_name, prompt.args
    );
    let paragraph = Paragraph::new(text).style(app.styles.text());
    frame.render_widget(paragraph, inner);
}


fn render_error_toast(frame: &mut Frame, app: &TuiApp, error: &str, area: Rect) {
    let toast_area = Rect {
        x: area.x + 2,
        y: area.height.saturating_sub(3),
        width: area.width.saturating_sub(4).min(60),
        height: 1,
    };

    let paragraph = Paragraph::new(format!("⚠ {}", error)).style(app.styles.warning());
    frame.render_widget(paragraph, toast_area);
}

/// Calculate centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
