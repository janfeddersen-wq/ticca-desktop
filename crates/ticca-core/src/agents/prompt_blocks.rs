//! Shared prompt building blocks for agents

use crate::tools::ToolParameterSchema;
use crate::tools::spec::ToolSpec;

use super::profile::ToolUsagePolicy;

pub struct PromptBlocks;

impl PromptBlocks {
    pub fn tool_docs(tool_specs: &[ToolSpec]) -> String {
        let mut out = String::from("## Available Tools\n\n");

        for spec in tool_specs {
            out.push_str(&format!("### {}\n{}\n", spec.name, spec.description));
            if let Some(lines) = Self::format_params(&spec.registry_parameters) {
                out.push_str(&lines);
            }
            out.push('\n');
        }

        out
    }

    pub fn agent_guidelines(policy: &ToolUsagePolicy, extra: &[&str]) -> String {
        let mut lines: Vec<String> = Vec::new();

        if policy.read_before_write {
            lines.push("Always read files before editing them".to_string());
        }

        if policy.prefer_edit_over_write {
            lines.push("Prefer editing existing files over creating new ones".to_string());
        }

        if policy.max_file_lines > 0 {
            lines.push(format!(
                "Keep files under {} lines; split larger files into smaller modules",
                policy.max_file_lines
            ));
        }

        lines.extend(extra.iter().map(|s| s.to_string()));

        let mut out = String::from("## Guidelines\n\n");
        for (idx, line) in lines.iter().enumerate() {
            out.push_str(&format!("{}. {}\n", idx + 1, line));
        }

        out
    }

    fn format_params(schema: &ToolParameterSchema) -> Option<String> {
        let props = schema.properties.as_ref()?;
        let mut keys: Vec<&String> = props.keys().collect();
        keys.sort();

        let required = schema.required.as_deref().unwrap_or(&[]);
        let mut out = String::new();

        for key in keys {
            let param = &props[key];
            let required_label = if required.iter().any(|name| name == key) {
                "required"
            } else {
                "optional"
            };
            let description = param.description.as_deref().unwrap_or("No description.");
            let mut line = format!(
                "- `{}` ({}, {}): {}",
                key, param.param_type, required_label, description
            );

            if let Some(default) = &param.default
                && !default.is_null()
            {
                line.push_str(&format!(" Default: {}.", default));
            }

            out.push_str(&line);
            out.push('\n');
        }

        Some(out)
    }
}
