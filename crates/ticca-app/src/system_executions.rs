//! System Executions state and helpers (UI terminal instances).

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use iced::Size;
use iced_term::Terminal;
use iced_term::settings::{BackendSettings, Settings};
use tracing::warn;

use ticca_core::external_tools;
use ticca_core::tools::{ProcessKind, ProcessSnapshot, SystemExecStore};

pub struct TerminalInstance {
    #[allow(dead_code)]
    pub process_id: String,
    #[allow(dead_code)]
    pub kind: ProcessKind,
    pub started_at: Instant,
    pub terminal_id: u64,
    pub terminal: Terminal,
    pub auto_close_if_fast: bool,
}

pub struct SystemExecutionsState {
    pub store: std::sync::Arc<SystemExecStore>,
    pub terminals: HashMap<String, TerminalInstance>,
    pub terminal_index: HashMap<u64, String>,
    pub next_terminal_id: u64,
    pub new_terminal_name: String,
    pub ui_error: Option<String>,
    last_ts_ms: u64,
    last_ts_bump: u32,
}

impl SystemExecutionsState {
    pub fn new(store: std::sync::Arc<SystemExecStore>) -> Self {
        Self {
            store,
            terminals: HashMap::new(),
            terminal_index: HashMap::new(),
            next_terminal_id: 1,
            new_terminal_name: String::new(),
            ui_error: None,
            last_ts_ms: 0,
            last_ts_bump: 0,
        }
    }

    pub fn generate_llm_process_id(&mut self) -> String {
        let now = chrono::Utc::now();
        let ts_ms = now.timestamp_millis() as u64;

        if ts_ms == self.last_ts_ms {
            self.last_ts_bump = self.last_ts_bump.saturating_add(1);
        } else {
            self.last_ts_ms = ts_ms;
            self.last_ts_bump = 0;
        }

        let base = now.format("%Y%m%d_%H%M%S").to_string();
        let ms = (ts_ms % 1000) as u32;
        if self.last_ts_bump == 0 {
            format!("process_{}_{:03}", base, ms)
        } else {
            format!("process_{}_{:03}_{:02}", base, ms, self.last_ts_bump)
        }
    }

    pub fn create_llm_terminal(
        &mut self,
        process_id: String,
        command: String,
        cwd: Option<PathBuf>,
    ) -> Result<(), String> {
        let (program, args) = platform_command_args(command);
        self.create_terminal(process_id, ProcessKind::Llm, program, args, cwd, true)
    }

    pub fn create_user_terminal(
        &mut self,
        name: String,
        cwd: Option<PathBuf>,
    ) -> Result<(), String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("Terminal name cannot be empty".to_string());
        }
        if self.terminals.contains_key(trimmed) {
            return Err(format!("Terminal name already exists: {}", trimmed));
        }
        let (program, args) = platform_default_shell();
        self.create_terminal(
            trimmed.to_string(),
            ProcessKind::User,
            program,
            args,
            cwd,
            false,
        )
    }

    fn create_terminal(
        &mut self,
        process_id: String,
        kind: ProcessKind,
        program: String,
        args: Vec<String>,
        cwd: Option<PathBuf>,
        auto_close_if_fast: bool,
    ) -> Result<(), String> {
        if self.terminals.contains_key(&process_id) {
            return Err(format!("Process already exists: {}", process_id));
        }

        let terminal_id = self.next_terminal_id;
        self.next_terminal_id = self.next_terminal_id.saturating_add(1);

        // Get environment overrides for external tools (PATH injection)
        let mut env = external_tools::env_overrides().unwrap_or_else(|e| {
            warn!("Failed to get external tools env: {}", e);
            HashMap::new()
        });

        // Add headless environment variables to disable interactive pagers and prompts.
        // This prevents tools like `git log`, `less`, `man`, etc. from waiting for user input.
        // Works across all platforms (Linux, macOS, Windows).
        env.insert("TERM".to_string(), "dumb".to_string());
        env.insert("PAGER".to_string(), "cat".to_string());
        env.insert("GIT_PAGER".to_string(), "cat".to_string());
        env.insert("BAT_PAGER".to_string(), String::new());
        env.insert("SYSTEMD_PAGER".to_string(), String::new());
        env.insert("LESS".to_string(), "-FRX".to_string());
        env.insert("GIT_TERMINAL_PROMPT".to_string(), "0".to_string());

        let backend = BackendSettings {
            program,
            args,
            env,
            working_directory: cwd,
        };
        let settings = Settings {
            backend,
            ..Default::default()
        };

        let mut terminal = Terminal::new(terminal_id, settings).map_err(|e| e.to_string())?;

        // Ensure a sensible initial PTY size even before the widget has processed
        // any layout-driven resize events.
        terminal.handle(iced_term::Command::ProxyToBackend(
            iced_term::backend::Command::Resize(Some(Size::new(240.0, 220.0)), None),
        ));
        let started_at = Instant::now();

        self.store.upsert_process(ProcessSnapshot {
            process_id: process_id.clone(),
            kind,
            visible: true,
            output: String::new(),
            exit_code: None,
            started_at_ms: now_ms(),
            finished_at_ms: None,
        });

        let instance = TerminalInstance {
            process_id: process_id.clone(),
            kind,
            started_at,
            terminal_id,
            terminal,
            auto_close_if_fast,
        };

        self.terminal_index.insert(terminal_id, process_id.clone());
        self.terminals.insert(process_id, instance);
        Ok(())
    }

    pub fn remove_terminal(&mut self, process_id: &str) -> bool {
        if let Some(instance) = self.terminals.remove(process_id) {
            self.terminal_index.remove(&instance.terminal_id);
            self.store.set_visible(process_id, false);
            true
        } else {
            false
        }
    }

    pub fn shutdown_terminal(&mut self, process_id: &str) -> Result<(), String> {
        let Some(instance) = self.terminals.get_mut(process_id) else {
            return Err(format!("Unknown process: {}", process_id));
        };
        instance.terminal.shutdown();
        Ok(())
    }

    #[allow(dead_code)]
    pub fn update_output_from_terminal(&mut self, process_id: &str) {
        let Some(instance) = self.terminals.get_mut(process_id) else {
            return;
        };
        let output = instance.terminal.dump_text();
        self.store.set_output(process_id, output);
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(target_os = "windows")]
fn platform_default_shell() -> (String, Vec<String>) {
    ("powershell.exe".to_string(), vec!["-NoLogo".to_string()])
}

#[cfg(not(target_os = "windows"))]
fn platform_default_shell() -> (String, Vec<String>) {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    (shell, Vec::new())
}

#[cfg(target_os = "windows")]
fn platform_command_args(command: String) -> (String, Vec<String>) {
    (
        "powershell.exe".to_string(),
        vec![
            "-NoLogo".to_string(),
            "-NoProfile".to_string(),
            "-Command".to_string(),
            command,
        ],
    )
}

#[cfg(not(target_os = "windows"))]
fn platform_command_args(command: String) -> (String, Vec<String>) {
    ("sh".to_string(), vec!["-lc".to_string(), command])
}
