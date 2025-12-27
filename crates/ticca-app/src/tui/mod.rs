//! Terminal User Interface (TUI) Module
//!
//! Provides a ratatui-based terminal UI as an alternative to the Iced GUI.
//! Supports all features including chat, settings, themes, and tool approvals.

mod app;
mod event;
pub mod theme;
mod ui;
mod views;
mod widgets;
mod streaming;

use std::io;
use std::panic;

use crossterm::{
    event::{
        DisableMouseCapture,
        EnableMouseCapture,
        KeyCode,
        KeyEventKind,
        KeyModifiers,
        MouseButton,
        MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;

use crate::cli::Args;
use app::{ChatFocus, ConfirmAction, ModalKind, SettingsFocus, TuiApp, View};
use event::{keys, Event, EventHandler};
use ticca_core::session::MessageRole;

pub use app::SettingsTab;
#[allow(unused_imports)]
pub use theme::{TuiColors, TuiStyles};

/// Run the TUI application
pub fn run(args: Args) -> anyhow::Result<()> {
    // Setup panic hook to restore terminal on panic
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Create app and event handler
    let mut app = TuiApp::new(&args)?;
    let mut events = EventHandler::new();

    // Run main loop
    let result = run_event_loop(&mut terminal, &mut app, &mut events);

    // Cleanup
    restore_terminal()?;

    result
}

/// Restore terminal to normal state
fn restore_terminal() -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

/// Main event loop
fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut TuiApp,
    events: &mut EventHandler,
) -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;

    rt.block_on(async {
        spawn_background_tasks(app, events.sender());

        loop {
            // Render current state
            terminal.draw(|frame| ui::render(frame, app))?;

            // Wait for and handle next event
            if let Some(event) = events.next().await {
                handle_event(app, event, events)?;
            }

            // Check for quit
            if app.should_quit {
                break;
            }
        }
        Ok(())
    })
}

/// Spawn background tasks (model loading, etc.)
fn spawn_background_tasks(_app: &TuiApp, event_tx: mpsc::UnboundedSender<Event>) {
    if ticca_core::llm::auth::has_any_valid_account() {
        let tx = event_tx.clone();
        tokio::spawn(async move {
            let result = load_models().await;
            let _ = tx.send(Event::ModelsLoaded(result));
        });
    }
}

async fn load_models() -> Result<Vec<String>, String> {
    ticca_core::llm::ModelService::fetch_all().await
}

/// Handle an event
fn handle_event(app: &mut TuiApp, event: Event, events: &mut EventHandler) -> anyhow::Result<()> {
    match event {
        Event::Tick => {
            app.tick_animation();
            app.tick_error();
        }

        Event::Key(key) => {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }

            // Handle modal first if present
            if app.active_modal.is_some() {
                handle_modal_key(app, key)?;
                return Ok(());
            }

            let in_input = app.view == View::Chat && app.chat_focus == ChatFocus::Input;

            if keys::ESCAPE.matches(&key) {
                match app.view {
                    View::Help => app.view = View::Chat,
                    View::Settings => app.view = View::Chat,
                    View::Chat => {
                        if app.is_streaming {
                            if let Some(tx) = app.stream_cancel_tx.take() {
                                let _ = tx.send(());
                            }
                        }
                    }
                }
                return Ok(());
            }

            if keys::SETTINGS.matches(&key)
                || keys::SETTINGS_F10.matches(&key)
                || keys::SETTINGS_F9.matches(&key)
            {
                if app.view == View::Settings {
                    app.view = View::Chat;
                } else {
                    app.view = View::Settings;
                    app.settings.refresh_all();
                }
                return Ok(());
            }

            if in_input {
                // Handle Shift+Enter to insert newline
                if key.code == KeyCode::Enter && key.modifiers.contains(KeyModifiers::SHIFT) {
                    app.input.insert_newline();
                    return Ok(());
                }

                if (keys::SEND.matches(&key) || keys::SEND_ALT.matches(&key)) && !app.is_streaming {
                    app.send_message(events.sender());
                    return Ok(());
                }

                if key.code == KeyCode::Enter
                    && key.modifiers == KeyModifiers::NONE
                    && !app.is_streaming
                {
                    let input_text = app.input.lines().join("\n");
                    if !input_text.trim().is_empty() {
                        app.send_message(events.sender());
                        return Ok(());
                    }
                }

                if keys::NEW_SESSION.matches(&key) && !app.is_streaming {
                    app.new_session();
                    return Ok(());
                }

                if keys::STOP_STREAM.matches(&key) && app.is_streaming {
                    if let Some(tx) = app.stream_cancel_tx.take() {
                        let _ = tx.send(());
                    }
                    return Ok(());
                }

                if keys::TAB.matches(&key) {
                    app.chat_focus = ChatFocus::Messages;
                    return Ok(());
                }

                if keys::PICK_AGENT.matches(&key) {
                    app.active_modal = Some(ModalKind::AgentPicker);
                    app.modal_list_index = 0;
                    return Ok(());
                }

                if keys::PICK_MODEL.matches(&key) {
                    app.active_modal = Some(ModalKind::ModelPicker);
                    app.modal_list_index = 0;
                    return Ok(());
                }

                if keys::TOGGLE_YOLO.matches(&key) {
                    app.toggle_yolo();
                    return Ok(());
                }

                if keys::WORKING_DIR.matches(&key) {
                    app.open_directory_picker();
                    return Ok(());
                }

                if keys::AGENT_PLANNING.matches(&key) {
                    app.switch_agent(ticca_core::agents::AgentType::Planning);
                    return Ok(());
                }

                if keys::AGENT_CODING.matches(&key) {
                    app.switch_agent(ticca_core::agents::AgentType::Coding);
                    return Ok(());
                }

                if keys::AGENT_SKILLS.matches(&key) {
                    app.switch_agent(ticca_core::agents::AgentType::Skills);
                    return Ok(());
                }

                app.input.input(key);
                return Ok(());
            }

            if keys::QUIT.matches(&key) || keys::QUIT_CTRL.matches(&key) {
                if app.view == View::Chat && !app.is_streaming {
                    app.should_quit = true;
                } else if app.view != View::Chat {
                    app.view = View::Chat;
                }
                return Ok(());
            }

            if keys::HELP.matches(&key) {
                app.view = View::Help;
                return Ok(());
            }

            match app.view {
                View::Chat => handle_chat_key_not_input(app, key, events)?,
                View::Settings => handle_settings_key(app, key)?,
                View::Help => {
                    app.view = View::Chat;
                }
            }
        }

        Event::Mouse(mouse) => {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    if let Some(modal) = app.active_modal.as_ref() {
                        match modal {
                            ModalKind::AgentPicker
                            | ModalKind::ModelPicker
                            | ModalKind::ThemePicker
                            | ModalKind::FilePicker { .. } => {
                                app.modal_list_index = app.modal_list_index.saturating_sub(3);
                            }
                            _ => {}
                        }
                    } else {
                        match app.view {
                            View::Chat => {
                                app.message_scroll = app.message_scroll.saturating_sub(1);
                            }
                            View::Settings => {
                                app.settings.list_index = app.settings.list_index.saturating_sub(3);
                            }
                            View::Help => {}
                        }
                    }
                }
                MouseEventKind::ScrollDown => {
                    if let Some(modal) = app.active_modal.as_ref() {
                        match modal {
                            ModalKind::AgentPicker
                            | ModalKind::ModelPicker
                            | ModalKind::ThemePicker
                            | ModalKind::FilePicker { .. } => {
                                app.modal_list_index = app.modal_list_index.saturating_add(3);
                            }
                            _ => {}
                        }
                    } else {
                        match app.view {
                            View::Chat => {
                                app.message_scroll = app.message_scroll.saturating_add(1);
                            }
                            View::Settings => {
                                app.settings.list_index = app.settings.list_index.saturating_add(3);
                            }
                            View::Help => {}
                        }
                    }
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    if app.view == View::Chat && !app.is_streaming {
                        if let Some(area) = app.send_button_area.get() {
                            let within_x = mouse.column >= area.x
                                && mouse.column < area.x.saturating_add(area.width);
                            let within_y = mouse.row >= area.y
                                && mouse.row < area.y.saturating_add(area.height);

                            if within_x && within_y {
                                app.send_message(events.sender());
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Event::Resize(_w, _h) => {
            // Terminal handles resize automatically
        }

        // Streaming events
        Event::StreamChunk(chunk) => {
            app.stream_buffer.push_str(&chunk);
            if let Some(msg) = app.messages.last_mut() {
                if msg.role == MessageRole::Assistant {
                    msg.current_text_block_mut().push_str(&chunk);
                    msg.content.push_str(&chunk);
                    msg.update_parsed_items();
                    // Auto-scroll to bottom during streaming
                    app.message_scroll = usize::MAX;
                }
            }
        }

        Event::StreamComplete => {
            app.is_streaming = false;
            app.stream_cancel_tx = None;
            app.approval_tx = None;
            if let Some(msg) = app.messages.last_mut() {
                if msg.role == MessageRole::Assistant {
                    msg.is_streaming = false;
                    msg.update_parsed_items();
                }
            }
        }

        Event::StreamStopped => {
            app.is_streaming = false;
            app.stream_cancel_tx = None;
            app.approval_tx = None;
            if let Some(msg) = app.messages.last_mut() {
                if msg.role == MessageRole::Assistant {
                    msg.is_streaming = false;
                    msg.update_parsed_items();
                }
            }
        }

        Event::StreamError(err) => {
            app.is_streaming = false;
            app.stream_cancel_tx = None;
            app.approval_tx = None;
            if let Some(msg) = app.messages.last_mut() {
                if msg.role == MessageRole::Assistant {
                    msg.is_streaming = false;
                    msg.update_parsed_items();
                }
            }
            app.show_error(err);
        }

        Event::ToolCall { name, args } => {
            if let Some(msg) = app.messages.last_mut() {
                if msg.role == MessageRole::Assistant {
                    let tool_text = format!("\n\n🔧 Tool: {}\n{}\n", name, args);
                    msg.current_text_block_mut().push_str(&tool_text);
                    msg.content.push_str(&tool_text);
                    msg.last_was_tool_call = true;
                    msg.update_parsed_items();
                }
            }
        }

        Event::ToolApprovalRequested { id, name, args } => {
            app.pending_approval = Some(app::ToolApprovalPrompt {
                id,
                tool_name: name,
                args,
            });
            app.active_modal = Some(ModalKind::ToolApproval(
                app.pending_approval.clone().unwrap(),
            ));
        }

        Event::Reasoning { text, signature } => {
            if let Some(msg) = app.messages.last_mut() {
                if msg.role == MessageRole::Assistant {
                    if let Some(existing) = &mut msg.reasoning {
                        existing.push_str(&text);
                    } else {
                        msg.reasoning = Some(text);
                    }
                    if msg.reasoning_signature.is_none() {
                        msg.reasoning_signature = signature;
                    }
                }
            }
        }

        Event::Usage {
            input_tokens,
            output_tokens,
        } => {
            app.input_tokens = input_tokens;
            app.output_tokens = output_tokens;
        }

        Event::ModelsLoaded(result) => match result {
            Ok(models) => {
                app.available_models = models;
                app.settings.is_loading_models = false;
            }
            Err(err) => {
                app.show_error(format!("Failed to load models: {}", err));
                app.settings.is_loading_models = false;
            }
        },

        _ => {
            // Handle other events as needed
        }
    }

    Ok(())
}

/// Handle key events in chat view when NOT focused on input
fn handle_chat_key_not_input(
    app: &mut TuiApp,
    key: crossterm::event::KeyEvent,
    events: &mut EventHandler,
) -> anyhow::Result<()> {
    if keys::SEND.matches(&key) && !app.is_streaming {
        app.send_message(events.sender());
        return Ok(());
    }

    if keys::NEW_SESSION.matches(&key) && !app.is_streaming {
        app.new_session();
        return Ok(());
    }

    if keys::STOP_STREAM.matches(&key) && app.is_streaming {
        if let Some(tx) = app.stream_cancel_tx.take() {
            let _ = tx.send(());
        }
        return Ok(());
    }

    if keys::TOGGLE_YOLO.matches(&key) {
        app.toggle_yolo();
        return Ok(());
    }

    if keys::PICK_AGENT.matches(&key) {
        app.active_modal = Some(ModalKind::AgentPicker);
        app.modal_list_index = 0;
        return Ok(());
    }

    if keys::PICK_MODEL.matches(&key) {
        app.active_modal = Some(ModalKind::ModelPicker);
        app.modal_list_index = 0;
        return Ok(());
    }

    if keys::WORKING_DIR.matches(&key) {
        app.open_directory_picker();
        return Ok(());
    }

    if keys::AGENT_PLANNING.matches(&key) {
        app.switch_agent(ticca_core::agents::AgentType::Planning);
        return Ok(());
    }
    if keys::AGENT_CODING.matches(&key) {
        app.switch_agent(ticca_core::agents::AgentType::Coding);
        return Ok(());
    }
    if keys::AGENT_SKILLS.matches(&key) {
        app.switch_agent(ticca_core::agents::AgentType::Skills);
        return Ok(());
    }

    if keys::TAB.matches(&key) {
        app.chat_focus = ChatFocus::Input;
        return Ok(());
    }

    if app.chat_focus == ChatFocus::Messages {
        if keys::UP.matches(&key) {
            app.message_scroll = app.message_scroll.saturating_sub(1);
        } else if keys::DOWN.matches(&key) {
            app.message_scroll = app.message_scroll.saturating_add(1);
        } else if keys::PAGE_UP.matches(&key) {
            app.message_scroll = app.message_scroll.saturating_sub(10);
        } else if keys::PAGE_DOWN.matches(&key) {
            app.message_scroll = app.message_scroll.saturating_add(10);
        } else if keys::HOME.matches(&key) {
            app.message_scroll = 0;
        } else if keys::END.matches(&key) {
            app.message_scroll = usize::MAX;
        }
    }

    Ok(())
}

/// Handle key events in settings view
fn handle_settings_key(app: &mut TuiApp, key: crossterm::event::KeyEvent) -> anyhow::Result<()> {
    // Tab switching with number keys
    if keys::TAB_1.matches(&key) {
        app.settings.active_tab = SettingsTab::Accounts;
        return Ok(());
    }
    if keys::TAB_2.matches(&key) {
        app.settings.active_tab = SettingsTab::Models;
        return Ok(());
    }
    if keys::TAB_3.matches(&key) {
        app.settings.active_tab = SettingsTab::Agents;
        return Ok(());
    }
    if keys::TAB_4.matches(&key) {
        app.settings.active_tab = SettingsTab::McpServers;
        return Ok(());
    }
    if keys::TAB_5.matches(&key) {
        app.settings.active_tab = SettingsTab::Tools;
        return Ok(());
    }
    if keys::TAB_6.matches(&key) {
        app.settings.active_tab = SettingsTab::Appearance;
        return Ok(());
    }
    if keys::TAB_7.matches(&key) {
        app.settings.active_tab = SettingsTab::Sessions;
        return Ok(());
    }

    // Tab cycling with arrow keys
    if keys::LEFT.matches(&key) {
        app.settings.active_tab = app.settings.active_tab.prev();
        return Ok(());
    }
    if keys::RIGHT.matches(&key) {
        app.settings.active_tab = app.settings.active_tab.next();
        return Ok(());
    }

    // Focus cycling
    if keys::TAB.matches(&key) {
        app.settings.focus = match app.settings.focus {
            SettingsFocus::TabBar => SettingsFocus::Content,
            SettingsFocus::Content => SettingsFocus::List,
            SettingsFocus::List => SettingsFocus::TabBar,
            SettingsFocus::Form => SettingsFocus::List,
        };
        return Ok(());
    }

    // List navigation
    if keys::UP.matches(&key) {
        app.settings.list_index = app.settings.list_index.saturating_sub(1);
        return Ok(());
    }
    if keys::DOWN.matches(&key) {
        app.settings.list_index = app.settings.list_index.saturating_add(1);
        return Ok(());
    }

    // Appearance tab - theme picker
    if app.settings.active_tab == SettingsTab::Appearance && keys::ENTER.matches(&key) {
        app.active_modal = Some(ModalKind::ThemePicker);
        app.modal_list_index = 0;
        return Ok(());
    }

    Ok(())
}

/// Handle key events when a modal is active
fn handle_modal_key(app: &mut TuiApp, key: crossterm::event::KeyEvent) -> anyhow::Result<()> {
    let modal = app.active_modal.take();

    match modal {
        Some(ModalKind::ToolApproval(prompt)) => {
            if keys::APPROVE.matches(&key) {
                // Send approval
                if let Some(tx) = &app.approval_tx {
                    let _ = tx.send(ticca_core::tools::ToolApprovalDecision {
                        id: prompt.id,
                        approved: true,
                    });
                }
                app.pending_approval = None;
            } else if keys::DENY.matches(&key) {
                // Send denial
                if let Some(tx) = &app.approval_tx {
                    let _ = tx.send(ticca_core::tools::ToolApprovalDecision {
                        id: prompt.id,
                        approved: false,
                    });
                }
                app.pending_approval = None;
            } else if keys::ALWAYS_APPROVE.matches(&key) {
                // Send always approve
                app.yolo_mode = true;
                let _ = ticca_core::config::ConfigService::set_setting(
                    ticca_core::config::setting_keys::YOLO_MODE,
                    "true",
                );
                if let Some(tx) = &app.approval_tx {
                    let _ = tx.send(ticca_core::tools::ToolApprovalDecision {
                        id: prompt.id,
                        approved: true,
                    });
                }
                app.pending_approval = None;
            } else if keys::ESCAPE.matches(&key) {
                app.active_modal = Some(ModalKind::ToolApproval(prompt));
            } else {
                app.active_modal = Some(ModalKind::ToolApproval(prompt));
            }
        }

        Some(ModalKind::AgentPicker) => {
            if keys::ESCAPE.matches(&key) {
                // Close modal
            } else if keys::ENTER.matches(&key) {
                let agents = [
                    ticca_core::agents::AgentType::Planning,
                    ticca_core::agents::AgentType::Coding,
                    ticca_core::agents::AgentType::Skills,
                ];
                if let Some(&agent) = agents.get(app.modal_list_index) {
                    app.switch_agent(agent);
                }
            } else if keys::UP.matches(&key) {
                app.modal_list_index = app.modal_list_index.saturating_sub(1);
                app.active_modal = Some(ModalKind::AgentPicker);
            } else if keys::DOWN.matches(&key) {
                app.modal_list_index = (app.modal_list_index + 1).min(2);
                app.active_modal = Some(ModalKind::AgentPicker);
            } else {
                app.active_modal = Some(ModalKind::AgentPicker);
            }
        }

        Some(ModalKind::ModelPicker) => {
            if keys::ESCAPE.matches(&key) {
                // Close modal
            } else if keys::ENTER.matches(&key) {
                if let Some(model) = app.available_models.get(app.modal_list_index) {
                    app.current_model = Some(model.clone());
                }
            } else if keys::UP.matches(&key) {
                app.modal_list_index = app.modal_list_index.saturating_sub(1);
                app.active_modal = Some(ModalKind::ModelPicker);
            } else if keys::DOWN.matches(&key) {
                app.modal_list_index =
                    (app.modal_list_index + 1).min(app.available_models.len().saturating_sub(1));
                app.active_modal = Some(ModalKind::ModelPicker);
            } else {
                app.active_modal = Some(ModalKind::ModelPicker);
            }
        }

        Some(ModalKind::ThemePicker) => {
            use crate::theme::ALL_THEMES;

            if keys::ESCAPE.matches(&key) {
                // Close modal
            } else if keys::ENTER.matches(&key) {
                if let Some(&theme) = ALL_THEMES.get(app.modal_list_index) {
                    app.set_theme(theme);
                }
            } else if keys::UP.matches(&key) {
                app.modal_list_index = app.modal_list_index.saturating_sub(1);
                app.active_modal = Some(ModalKind::ThemePicker);
            } else if keys::DOWN.matches(&key) {
                app.modal_list_index =
                    (app.modal_list_index + 1).min(ALL_THEMES.len().saturating_sub(1));
                app.active_modal = Some(ModalKind::ThemePicker);
            } else {
                app.active_modal = Some(ModalKind::ThemePicker);
            }
        }

        Some(ModalKind::ApiKeyForm(form)) => {
            if keys::ESCAPE.matches(&key) || keys::ENTER.matches(&key) {
                // Close modal or save placeholder
            } else if keys::TAB.matches(&key) {
                let mut form = form;
                form.next_field();
                app.active_modal = Some(ModalKind::ApiKeyForm(form));
            } else {
                app.active_modal = Some(ModalKind::ApiKeyForm(form));
            }
        }

        Some(ModalKind::McpServerForm(form)) => {
            if keys::ESCAPE.matches(&key) || keys::ENTER.matches(&key) {
                // Close modal or save placeholder
            } else if keys::TAB.matches(&key) {
                let mut form = form;
                form.next_field();
                app.active_modal = Some(ModalKind::McpServerForm(form));
            } else {
                app.active_modal = Some(ModalKind::McpServerForm(form));
            }
        }

        Some(ModalKind::Confirmation {
            title,
            message,
            action,
        }) => {
            if keys::APPROVE.matches(&key) {
                // Execute action
                match action {
                    ConfirmAction::DeleteSession(id) => {
                        if let Ok(db) = ticca_core::session::SessionDatabase::open() {
                            let _ = db.delete_session(&id);
                        }
                        app.settings.refresh_sessions();
                    }
                    ConfirmAction::DeleteMcpServer(id) => {
                        let _ = ticca_core::config::ConfigService::delete_mcp_server(&id);
                        app.settings.refresh_mcp();
                    }
                    ConfirmAction::DeleteAccount(id) => {
                        let _ = ticca_core::config::ConfigService::delete_oauth_account(&id);
                        app.settings.refresh_accounts();
                    }
                    ConfirmAction::NewSession => {
                        app.new_session();
                    }
                }
            } else if keys::DENY.matches(&key) || keys::ESCAPE.matches(&key) {
                // Cancel
            } else {
                app.active_modal = Some(ModalKind::Confirmation {
                    title,
                    message,
                    action,
                });
            }
        }

        Some(ModalKind::Error(_)) => {
            // Any key closes error modal
        }

        Some(ModalKind::FilePicker {
            path,
            entries,
            selected,
        }) => {
            if keys::ESCAPE.matches(&key) {
                // Close modal
            } else if keys::ENTER.matches(&key) {
                if let Some((name, is_dir)) = entries.get(selected) {
                    if *is_dir {
                        let new_path = if name == ".." {
                            path.parent().unwrap_or(&path).to_path_buf()
                        } else {
                            path.join(name)
                        };

                        if name == ".." || new_path.is_dir() {
                            app.working_directory = new_path.clone();
                            let _ = ticca_core::config::ConfigService::set_setting(
                                ticca_core::config::setting_keys::WORKING_DIRECTORY,
                                &new_path.display().to_string(),
                            );
                            let new_entries = app::read_directory_entries(&new_path);
                            app.active_modal = Some(ModalKind::FilePicker {
                                path: new_path,
                                entries: new_entries,
                                selected: 0,
                            });
                        }
                    }
                }
            } else if keys::UP.matches(&key) {
                let new_selected = selected.saturating_sub(1);
                app.active_modal = Some(ModalKind::FilePicker {
                    path,
                    entries,
                    selected: new_selected,
                });
            } else if keys::DOWN.matches(&key) {
                let new_selected = (selected + 1).min(entries.len().saturating_sub(1));
                app.active_modal = Some(ModalKind::FilePicker {
                    path,
                    entries,
                    selected: new_selected,
                });
            } else if key.code == crossterm::event::KeyCode::Backspace {
                if let Some(parent) = path.parent() {
                    let new_entries = app::read_directory_entries(parent);
                    app.active_modal = Some(ModalKind::FilePicker {
                        path: parent.to_path_buf(),
                        entries: new_entries,
                        selected: 0,
                    });
                } else {
                    app.active_modal = Some(ModalKind::FilePicker {
                        path,
                        entries,
                        selected,
                    });
                }
            } else {
                app.active_modal = Some(ModalKind::FilePicker {
                    path,
                    entries,
                    selected,
                });
            }
        }

        None => {}
    }

    Ok(())
}
