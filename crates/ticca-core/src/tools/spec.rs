//! Shared tool specifications for registry and rig tools

use serde_json::{Value, json};

use super::registry::ToolParameterSchema;

pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub rig_parameters: Value,
    pub registry_parameters: ToolParameterSchema,
}

/// Get all tool specs for estimation purposes
pub fn all_specs() -> Vec<ToolSpec> {
    vec![
        list_files_spec(),
        read_file_spec(),
        edit_file_spec(),
        delete_file_spec(),
        grep_spec(50),
        execute_shell_spec(120_000), // default timeout
        list_processes_spec(),
        read_process_output_spec(),
        kill_process_spec(),
        write_file_spec(),
        list_agents_spec(),
        invoke_agent_spec(),
        todo_read_spec(),
        todo_write_spec(),
        todo_list_spec(),
        share_reasoning_spec(),
    ]
}

/// Estimate total tokens for all tool definitions
/// Uses chars/3.4 ratio consistent with rig's estimation
pub fn estimate_all_tools_tokens() -> usize {
    let specs = all_specs();
    let mut total_chars = 0usize;

    for spec in specs {
        // Name
        total_chars += spec.name.len();
        // Description
        total_chars += spec.description.len();
        // Parameters JSON
        if let Ok(json_str) = serde_json::to_string(&spec.rig_parameters) {
            total_chars += json_str.len();
        }
    }

    // chars / 3.4 ratio (consistent with rig's estimation)
    (total_chars as f32 / 3.4).ceil() as usize
}

pub fn list_files_spec() -> ToolSpec {
    let mut params = std::collections::HashMap::new();
    params.insert(
        "directory".to_string(),
        ToolParameterSchema::string(
            "Path to the directory to list. Defaults to current directory.",
        )
        .with_default(json!(".")),
    );
    params.insert(
        "recursive".to_string(),
        ToolParameterSchema::boolean(
            "Whether to recursively list subdirectories. Defaults to true.",
        )
        .with_default(json!(true)),
    );

    ToolSpec {
        name: "list_files",
        description: "List files and directories with intelligent filtering. Automatically ignores common build artifacts, caches, and other noise.",
        rig_parameters: json!({
            "type": "object",
            "properties": {
                "directory": { "type": "string", "description": "Directory to list (optional, defaults to project root)" },
                "recursive": { "type": "boolean", "description": "Whether to list recursively (default: true)" }
            },
            "required": []
        }),
        registry_parameters: ToolParameterSchema::object(params, vec![]),
    }
}

pub fn read_file_spec() -> ToolSpec {
    let mut params = std::collections::HashMap::new();
    params.insert(
        "path".to_string(),
        ToolParameterSchema::string("Path to the file to read."),
    );
    params.insert(
        "start_line".to_string(),
        ToolParameterSchema::integer("Starting line number (1-based). Optional.")
            .with_default(json!(null)),
    );
    params.insert(
        "num_lines".to_string(),
        ToolParameterSchema::integer(
            "Number of lines to read from start_line. Required if start_line is set.",
        )
        .with_default(json!(null)),
    );

    ToolSpec {
        name: "read_file",
        description: "Read file contents with optional line-range selection.",
        rig_parameters: json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file (relative to project root or absolute)" },
                "start_line": { "type": "integer", "description": "Starting line number (1-based, optional)" },
                "num_lines": { "type": "integer", "description": "Number of lines to read (optional)" }
            },
            "required": ["path"]
        }),
        registry_parameters: ToolParameterSchema::object(params, vec!["path".to_string()]),
    }
}

pub fn edit_file_spec() -> ToolSpec {
    let mut params = std::collections::HashMap::new();
    params.insert(
        "path".to_string(),
        ToolParameterSchema::string("Path to the file to edit."),
    );
    params.insert(
        "content".to_string(),
        ToolParameterSchema::string("Full content to write (for ContentPayload). Optional."),
    );
    params.insert(
        "overwrite".to_string(),
        ToolParameterSchema::boolean("Whether to overwrite existing file. Default: false.")
            .with_default(json!(false)),
    );
    params.insert(
        "replacements".to_string(),
        ToolParameterSchema::string(
            "Array of {old_str, new_str} objects for text replacement. Optional.",
        ),
    );
    params.insert(
        "delete_snippet".to_string(),
        ToolParameterSchema::string("Exact text snippet to delete from file. Optional."),
    );

    ToolSpec {
        name: "edit_file",
        description: "Edit a file by replacing text. Supports multiple replacements in one call. Uses fuzzy matching (Jaro-Winkler ≥95%) to recover from minor whitespace/indent errors.",
        rig_parameters: json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to edit" },
                "old_text": { "type": "string", "description": "Text to find and replace (for single replacement)" },
                "new_text": { "type": "string", "description": "Replacement text (for single replacement)" },
                "replacements": {
                    "type": "array",
                    "description": "Array of replacements for batch editing (alternative to old_text/new_text)",
                    "items": {
                        "type": "object",
                        "properties": {
                            "old_text": { "type": "string", "description": "Text to find" },
                            "new_text": { "type": "string", "description": "Replacement text" }
                        },
                        "required": ["old_text", "new_text"]
                    }
                }
            },
            "required": ["path"]
        }),
        registry_parameters: ToolParameterSchema::object(params, vec!["path".to_string()]),
    }
}

pub fn delete_file_spec() -> ToolSpec {
    let mut params = std::collections::HashMap::new();
    params.insert(
        "path".to_string(),
        ToolParameterSchema::string("Path to the file to delete."),
    );

    ToolSpec {
        name: "delete_file",
        description: "Delete a file with diff generation showing what was removed.",
        rig_parameters: json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to delete" }
            },
            "required": ["path"]
        }),
        registry_parameters: ToolParameterSchema::object(params, vec!["path".to_string()]),
    }
}

pub fn grep_spec(_max_matches: usize) -> ToolSpec {
    let mut params = std::collections::HashMap::new();
    params.insert(
        "search_string".to_string(),
        ToolParameterSchema::string(
            "The text pattern to search for. Supports regex. Can include ripgrep flags like '--ignore-case' or '-i' at the start."
        ),
    );
    params.insert(
        "directory".to_string(),
        ToolParameterSchema::string("Root directory to search in. Defaults to current directory.")
            .with_default(json!(".")),
    );

    ToolSpec {
        name: "grep",
        description: "Recursively search for text patterns across files using ripgrep.",
        rig_parameters: json!({
            "type": "object",
            "properties": {
                "search_string": { "type": "string", "description": "Pattern to search for (regex supported)" },
                "directory": { "type": "string", "description": "Root directory to search in (optional)" },
                "case_insensitive": { "type": "boolean", "description": "Whether to search case-insensitively (default: false)" }
            },
            "required": ["search_string"]
        }),
        registry_parameters: ToolParameterSchema::object(params, vec!["search_string".to_string()]),
    }
}

pub fn execute_shell_spec(_default_timeout: u64) -> ToolSpec {
    let mut params = std::collections::HashMap::new();
    params.insert(
        "command".to_string(),
        ToolParameterSchema::string("The shell command to execute in a visible UI terminal."),
    );
    params.insert(
        "cwd".to_string(),
        ToolParameterSchema::string("Working directory for command execution (optional).")
            .with_default(json!(null)),
    );

    ToolSpec {
        name: "execute_shell",
        description: "Execute a shell command in a UI terminal instance (shown in System Executions). Short commands may return completed output; long-running commands return immediately with a process ID.",
        rig_parameters: json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "The shell command to execute" },
                "cwd": { "type": "string", "description": "Working directory for command execution (optional)" }
            },
            "required": ["command"]
        }),
        registry_parameters: ToolParameterSchema::object(params, vec!["command".to_string()]),
    }
}

pub fn list_processes_spec() -> ToolSpec {
    ToolSpec {
        name: "list_processes",
        description: "List all active terminal process IDs in the System Executions tab.",
        rig_parameters: json!({
            "type": "object",
            "properties": {}
        }),
        registry_parameters: ToolParameterSchema::object(std::collections::HashMap::new(), vec![]),
    }
}

pub fn read_process_output_spec() -> ToolSpec {
    let mut params = std::collections::HashMap::new();
    params.insert(
        "process_id".to_string(),
        ToolParameterSchema::string("Process ID to read output from."),
    );

    ToolSpec {
        name: "read_process_output",
        description: "Read only new output from a terminal process since the last read for that process.",
        rig_parameters: json!({
            "type": "object",
            "properties": {
                "process_id": { "type": "string", "description": "The process ID to read from" }
            },
            "required": ["process_id"]
        }),
        registry_parameters: ToolParameterSchema::object(params, vec!["process_id".to_string()]),
    }
}

pub fn kill_process_spec() -> ToolSpec {
    let mut params = std::collections::HashMap::new();
    params.insert(
        "process_id".to_string(),
        ToolParameterSchema::string("Process ID to terminate."),
    );

    ToolSpec {
        name: "kill_process",
        description: "Terminate a terminal process and remove it from the System Executions tab.",
        rig_parameters: json!({
            "type": "object",
            "properties": {
                "process_id": { "type": "string", "description": "The process ID to terminate" }
            },
            "required": ["process_id"]
        }),
        registry_parameters: ToolParameterSchema::object(params, vec!["process_id".to_string()]),
    }
}

pub fn write_file_spec() -> ToolSpec {
    ToolSpec {
        name: "write_file",
        description: "Write content to a file, creating it if needed.",
        rig_parameters: json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to write" },
                "content": { "type": "string", "description": "Content to write to the file" }
            },
            "required": ["path", "content"]
        }),
        registry_parameters: ToolParameterSchema::object(
            {
                let mut params = std::collections::HashMap::new();
                params.insert(
                    "path".to_string(),
                    ToolParameterSchema::string("Path to the file to write."),
                );
                params.insert(
                    "content".to_string(),
                    ToolParameterSchema::string("Content to write to the file."),
                );
                params
            },
            vec!["path".to_string(), "content".to_string()],
        ),
    }
}

pub fn list_agents_spec() -> ToolSpec {
    ToolSpec {
        name: "list_agents",
        description: "List all available agents with their identifiers and descriptions.",
        rig_parameters: json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
        registry_parameters: ToolParameterSchema::object(std::collections::HashMap::new(), vec![]),
    }
}

pub fn invoke_agent_spec() -> ToolSpec {
    let mut params = std::collections::HashMap::new();
    params.insert(
        "agent".to_string(),
        ToolParameterSchema::string("Agent identifier to invoke (e.g., 'coding' or 'planning')."),
    );
    params.insert(
        "prompt".to_string(),
        ToolParameterSchema::string("Prompt to send to the invoked agent."),
    );

    ToolSpec {
        name: "invoke_agent",
        description: "Invoke another agent with its own isolated message history.",
        rig_parameters: json!({
            "type": "object",
            "properties": {
                "agent": { "type": "string", "description": "Agent identifier to invoke (e.g., 'coding' or 'planning')" },
                "prompt": { "type": "string", "description": "Prompt to send to the invoked agent" }
            },
            "required": ["agent", "prompt"]
        }),
        registry_parameters: ToolParameterSchema::object(
            params,
            vec!["agent".to_string(), "prompt".to_string()],
        ),
    }
}

fn todo_write_schema() -> (Value, ToolParameterSchema) {
    let mut item_props = std::collections::HashMap::new();
    item_props.insert(
        "content".to_string(),
        ToolParameterSchema::string("Task description."),
    );
    item_props.insert(
        "status".to_string(),
        ToolParameterSchema::string("One of: pending | in_progress | completed."),
    );
    item_props.insert(
        "activeForm".to_string(),
        ToolParameterSchema::string(
            "Present continuous form shown during execution (e.g., 'Running tests').",
        ),
    );

    let item_schema = ToolParameterSchema::object(
        item_props,
        vec![
            "content".to_string(),
            "status".to_string(),
            "activeForm".to_string(),
        ],
    );

    let mut params = std::collections::HashMap::new();
    params.insert(
        "todos".to_string(),
        ToolParameterSchema {
            param_type: "array".to_string(),
            description: Some("The updated todo list.".to_string()),
            default: None,
            properties: None,
            required: None,
            items: Some(Box::new(item_schema)),
        },
    );

    let rig_parameters = json!({
        "type": "object",
        "properties": {
            "todos": {
                "type": "array",
                "description": "The updated todo list",
                "items": {
                    "type": "object",
                    "properties": {
                        "content": { "type": "string", "description": "Task description" },
                        "status": { "type": "string", "enum": ["pending", "in_progress", "completed"], "description": "Task status" },
                        "activeForm": { "type": "string", "description": "Present continuous form (e.g., 'Running tests')" }
                    },
                    "required": ["content", "status", "activeForm"]
                }
            }
        },
        "required": ["todos"]
    });

    let registry_parameters = ToolParameterSchema::object(params, vec!["todos".to_string()]);

    (rig_parameters, registry_parameters)
}

pub fn todo_read_spec() -> ToolSpec {
    ToolSpec {
        name: "todo_read",
        description: "Read the current todo list.",
        rig_parameters: json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
        registry_parameters: ToolParameterSchema::object(std::collections::HashMap::new(), vec![]),
    }
}

const TODO_WRITE_DESCRIPTION: &str = r#"Create and manage a structured task list for your current session.

## When to Use
Use proactively for:
- Complex multi-step tasks (3+ distinct steps)
- Non-trivial work needing careful planning
- User provides multiple tasks (numbered/comma-separated)
- After receiving new instructions - capture as todos
- Before starting work on a task - mark in_progress
- After completing a task - mark completed

## When NOT to Use
Skip when:
- Single straightforward task
- Trivial task with no organizational benefit
- Less than 3 trivial steps
- Purely conversational or informational

## Task States
- pending: Not yet started
- in_progress: Currently working (limit to ONE at a time)
- completed: Finished successfully

## Management
- Update status in real-time as you work
- Mark complete IMMEDIATELY after finishing (don't batch)
- Exactly ONE task in_progress at any time
- Only mark complete when FULLY accomplished
- Keep as in_progress if encountering errors/blockers"#;

pub fn todo_write_spec() -> ToolSpec {
    let (rig_parameters, registry_parameters) = todo_write_schema();
    ToolSpec {
        name: "todo_write",
        description: TODO_WRITE_DESCRIPTION,
        rig_parameters,
        registry_parameters,
    }
}

pub fn todo_list_spec() -> ToolSpec {
    let (rig_parameters, registry_parameters) = todo_write_schema();
    ToolSpec {
        name: "todo_list",
        description: TODO_WRITE_DESCRIPTION,
        rig_parameters,
        registry_parameters,
    }
}

pub fn share_reasoning_spec() -> ToolSpec {
    let mut params = std::collections::HashMap::new();
    params.insert(
        "insight".to_string(),
        ToolParameterSchema::string("A concise summary of an important discovery or non-obvious finding that should be remembered."),
    );

    ToolSpec {
        name: "share_reasoning",
        description: "Share important reasoning or discoveries with the user. Use this tool when you discover something non-obvious, make an important connection, or find something that should be remembered during the session. Keep insights concise and actionable.",
        rig_parameters: json!({
            "type": "object",
            "properties": {
                "insight": { "type": "string", "description": "A concise summary of an important discovery or non-obvious finding" }
            },
            "required": ["insight"]
        }),
        registry_parameters: ToolParameterSchema::object(params, vec!["insight".to_string()]),
    }
}

pub fn tool_specs_for_names(names: &[&str]) -> Vec<ToolSpec> {
    names
        .iter()
        .filter_map(|name| match *name {
            "list_files" => Some(list_files_spec()),
            "read_file" => Some(read_file_spec()),
            "edit_file" => Some(edit_file_spec()),
            "delete_file" => Some(delete_file_spec()),
            "grep" => Some(grep_spec(200)),
            "execute_shell" => Some(execute_shell_spec(60)),
            "list_processes" => Some(list_processes_spec()),
            "read_process_output" => Some(read_process_output_spec()),
            "kill_process" => Some(kill_process_spec()),
            "write_file" => Some(write_file_spec()),
            "list_agents" => Some(list_agents_spec()),
            "invoke_agent" => Some(invoke_agent_spec()),
            "todo_read" => Some(todo_read_spec()),
            "todo_write" => Some(todo_write_spec()),
            "todo_list" => Some(todo_list_spec()),
            "share_reasoning" => Some(share_reasoning_spec()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_files_spec_contains_schema() {
        let spec = list_files_spec();
        let properties = spec
            .rig_parameters
            .get("properties")
            .and_then(|v| v.as_object())
            .expect("properties should exist");

        assert!(properties.contains_key("directory"));
        assert!(properties.contains_key("recursive"));
    }

    #[test]
    fn tool_specs_for_names_filters_unknown() {
        let specs = tool_specs_for_names(&["list_files", "nope", "read_file"]);
        let names: Vec<&str> = specs.iter().map(|s| s.name).collect();
        assert_eq!(names, vec!["list_files", "read_file"]);
    }
}
