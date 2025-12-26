//! Helper functions for the application

/// Truncate a string to a maximum number of characters (char-safe, not byte-safe)
fn truncate_end(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

/// Truncate a string from the start, keeping the end (char-safe)
fn truncate_start(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let skip = char_count - max_chars.saturating_sub(3);
        let truncated: String = s.chars().skip(skip).collect();
        format!("...{}", truncated)
    }
}

/// Format a tool call as a concise one-liner for display
pub fn format_tool_call_oneliner(
    name: &str,
    args: &str,
    working_directory: Option<&std::path::Path>,
) -> String {
    // Try to parse the args as JSON to extract relevant fields
    let parsed: serde_json::Value = serde_json::from_str(args).unwrap_or(serde_json::Value::Null);

    let resolve_path = |path: &str| {
        let path = std::path::Path::new(path);
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            working_directory
                .unwrap_or_else(|| std::path::Path::new("."))
                .join(path)
        };
        full_path.to_string_lossy().replace('\\', "/")
    };

    let link_path = |display: String, full_path: &str| {
        if full_path.is_empty() {
            display
        } else {
            format!("[{}](<{}>)", display, full_path)
        }
    };

    let param_display = match name {
        "list_files" => {
            // Show directory, limited to 80 chars
            if let Some(dir) = parsed
                .get("directory")
                .or(parsed.get("path"))
                .and_then(|v| v.as_str())
            {
                let display = truncate_start(dir, 80);
                format!("Directory: {}", display)
            } else {
                String::new()
            }
        }
        "read_file" => {
            // Show just the path (clickable)
            if let Some(path) = parsed.get("path").and_then(|v| v.as_str()) {
                let display = truncate_start(path, 80);
                let full_path = resolve_path(path);
                link_path(display, &full_path)
            } else {
                String::new()
            }
        }
        "edit_file" | "write_file" => {
            // Show just the filename (clickable)
            if let Some(path) = parsed.get("path").and_then(|v| v.as_str()) {
                let filename = std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(path)
                    .to_string();
                let full_path = resolve_path(path);
                link_path(filename, &full_path)
            } else {
                String::new()
            }
        }
        "execute_shell" => {
            // Show command, limited to 80 chars
            if let Some(cmd) = parsed.get("command").and_then(|v| v.as_str()) {
                let display = truncate_end(cmd, 80);
                format!("`{}`", display)
            } else {
                String::new()
            }
        }
        "read_process_output" | "kill_process" => parsed
            .get("process_id")
            .and_then(|v| v.as_str())
            .map(|id| format!("Process: {}", id))
            .unwrap_or_default(),
        "list_processes" => "Show active processes".to_string(),
        "grep" => {
            // Show search_string and optionally directory
            let pattern = parsed
                .get("search_string")
                .or(parsed.get("pattern"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let path = parsed
                .get("directory")
                .or(parsed.get("path"))
                .and_then(|v| v.as_str());
            if pattern.is_empty() {
                String::new()
            } else {
                let mut display = format!("\"{}\"", truncate_end(pattern, 50));
                if let Some(p) = path {
                    let short_path = truncate_start(p, 30);
                    display.push_str(&format!(" in {}", short_path));
                }
                truncate_end(&display, 80)
            }
        }
        "invoke_agent" => {
            let agent = parsed.get("agent").and_then(|v| v.as_str()).unwrap_or("");
            let prompt = parsed.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
            let mut display = if agent.is_empty() {
                String::new()
            } else {
                format!("Agent: {}", agent)
            };

            if !prompt.is_empty() {
                let preview: String = prompt.chars().take(60).collect();
                let suffix = if prompt.chars().count() > 60 {
                    "..."
                } else {
                    ""
                };
                if !display.is_empty() {
                    display.push_str(" • ");
                }
                display.push_str(&format!("Prompt: \"{}{}\"", preview, suffix));
            }
            display
        }
        "list_agents" => "Show available agents".to_string(),
        _ => {
            // Generic: show first key-value pair, limited
            if let Some(obj) = parsed.as_object() {
                if let Some((key, val)) = obj.iter().next() {
                    let val_str = match val {
                        serde_json::Value::String(s) => s.clone(),
                        _ => val.to_string(),
                    };
                    let display = format!("{}: {}", key, val_str);
                    truncate_end(&display, 80)
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        }
    };

    if param_display.is_empty() {
        format!("🔧 **{}**", name)
    } else {
        format!("🔧 **{}** {}", name, param_display)
    }
}
