//! Shared tool specifications for registry and rig tools

use serde_json::{Value, json};

use super::registry::ToolParameterSchema;

pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub rig_parameters: Value,
    pub registry_parameters: ToolParameterSchema,
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
        description: "Edit a file using one of three methods: full content replacement, targeted text replacements, or snippet deletion.",
        rig_parameters: json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to edit" },
                "old_text": { "type": "string", "description": "Exact text to find and replace" },
                "new_text": { "type": "string", "description": "Replacement text" }
            },
            "required": ["path", "old_text", "new_text"]
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
        "text".to_string(),
        ToolParameterSchema::string("Task description (keep short)."),
    );
    item_props.insert(
        "status".to_string(),
        ToolParameterSchema::string("One of: pending | in_progress | completed."),
    );

    let item_schema =
        ToolParameterSchema::object(item_props, vec!["text".to_string(), "status".to_string()]);

    let mut params = std::collections::HashMap::new();
    params.insert(
        "items".to_string(),
        ToolParameterSchema {
            param_type: "array".to_string(),
            description: Some("Ordered list of tasks for the current agent.".to_string()),
            default: None,
            properties: None,
            required: None,
            items: Some(Box::new(item_schema)),
        },
    );
    params.insert(
        "confirmed_complete".to_string(),
        ToolParameterSchema::boolean(
            "Set true to confirm all tasks are completed. Will only be accepted if every item status is completed.",
        )
        .with_default(json!(null)),
    );
    params.insert(
        "mark_all_complete".to_string(),
        ToolParameterSchema::boolean(
            "Shortcut: set true to mark ALL items as completed and confirm in one step. Overrides individual item statuses.",
        )
        .with_default(json!(null)),
    );

    let rig_parameters = json!({
        "type": "object",
        "properties": {
            "items": {
                "type": "array",
                "description": "Ordered list of tasks for the current agent",
                "items": {
                    "type": "object",
                    "properties": {
                        "text": { "type": "string", "description": "Task description (keep short)" },
                        "status": { "type": "string", "description": "One of: pending | in_progress | completed" }
                    },
                    "required": ["text", "status"]
                }
            },
            "confirmed_complete": { "type": "boolean", "description": "Set true to confirm all tasks are completed (only accepted if all items are completed)" },
            "mark_all_complete": { "type": "boolean", "description": "Shortcut: set true to mark ALL items as completed and confirm in one step. Overrides individual item statuses." }
        },
        "required": ["items"]
    });

    let registry_parameters = ToolParameterSchema::object(params, vec!["items".to_string()]);

    (rig_parameters, registry_parameters)
}

pub fn todo_read_spec() -> ToolSpec {
    ToolSpec {
        name: "todo_read",
        description: "Read the current agent-scoped To Do list (including confirmation state).",
        rig_parameters: json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
        registry_parameters: ToolParameterSchema::object(std::collections::HashMap::new(), vec![]),
    }
}

pub fn todo_write_spec() -> ToolSpec {
    let (rig_parameters, registry_parameters) = todo_write_schema();
    ToolSpec {
        name: "todo_write",
        description: "Update the agent-scoped To Do list. Replace the entire list each call; confirm completion by setting confirmed_complete=true when all items are completed.",
        rig_parameters,
        registry_parameters,
    }
}

pub fn todo_list_spec() -> ToolSpec {
    let (rig_parameters, registry_parameters) = todo_write_schema();
    ToolSpec {
        name: "todo_list",
        description: "Maintain an agent-scoped To Do list. Replace the entire list each call; confirm completion by setting confirmed_complete=true when all items are completed.",
        rig_parameters,
        registry_parameters,
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
