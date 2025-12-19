//! Shared tool specifications for registry and rig tools

use serde_json::{json, Value};

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
        ToolParameterSchema::string("Path to the directory to list. Defaults to current directory.")
            .with_default(json!(".")),
    );
    params.insert(
        "recursive".to_string(),
        ToolParameterSchema::boolean("Whether to recursively list subdirectories. Defaults to true.")
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
        ToolParameterSchema::integer("Number of lines to read from start_line. Required if start_line is set.")
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
        ToolParameterSchema::string("Array of {old_str, new_str} objects for text replacement. Optional."),
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

pub fn shell_spec(default_timeout: u64, _max_output_lines: usize) -> ToolSpec {
    let mut params = std::collections::HashMap::new();
    params.insert(
        "command".to_string(),
        ToolParameterSchema::string("The shell command to execute."),
    );
    params.insert(
        "cwd".to_string(),
        ToolParameterSchema::string("Working directory for command execution. Defaults to current directory.")
            .with_default(json!(null)),
    );
    params.insert(
        "timeout".to_string(),
        ToolParameterSchema::integer(format!(
            "Timeout in seconds. Defaults to {} seconds.",
            default_timeout
        ))
        .with_default(json!(default_timeout)),
    );

    ToolSpec {
        name: "shell",
        description: "Execute a shell command with configurable timeout and working directory.",
        rig_parameters: json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "The shell command to execute" },
                "cwd": { "type": "string", "description": "Working directory for command execution (optional)" },
                "timeout": { "type": "integer", "description": "Timeout in seconds (default: 60)" }
            },
            "required": ["command"]
        }),
        registry_parameters: ToolParameterSchema::object(params, vec!["command".to_string()]),
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

pub fn tool_specs_for_names(names: &[&str]) -> Vec<ToolSpec> {
    names
        .iter()
        .filter_map(|name| match *name {
            "list_files" => Some(list_files_spec()),
            "read_file" => Some(read_file_spec()),
            "edit_file" => Some(edit_file_spec()),
            "delete_file" => Some(delete_file_spec()),
            "grep" => Some(grep_spec(200)),
            "shell" => Some(shell_spec(60, 256)),
            "write_file" => Some(write_file_spec()),
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
