use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

use super::{McpServer, McpTransport};

pub fn parse_mcp_servers_json(input: &str, existing: &[McpServer]) -> Result<Vec<McpServer>> {
    let value: Value = serde_json::from_str(input).context("Invalid JSON")?;

    let mut existing_by_name: HashMap<&str, &McpServer> = HashMap::new();
    for server in existing {
        existing_by_name
            .entry(server.name.as_str())
            .or_insert(server);
    }

    let mut servers = Vec::new();
    extract_servers_from_value(&value, &existing_by_name, &mut servers)?;
    if servers.is_empty() {
        bail!("No MCP servers found in JSON");
    }
    Ok(servers)
}

fn extract_servers_from_value(
    value: &Value,
    existing_by_name: &HashMap<&str, &McpServer>,
    out: &mut Vec<McpServer>,
) -> Result<()> {
    match value {
        Value::Array(items) => {
            for item in items {
                out.push(parse_server(item, None, existing_by_name)?);
            }
        }
        Value::Object(map) => {
            if let Some(inner) = map
                .get("mcpServers")
                .or_else(|| map.get("mcp_servers"))
                .or_else(|| map.get("servers"))
            {
                match inner {
                    Value::Object(servers) => {
                        for (name, server_cfg) in servers {
                            out.push(parse_server(server_cfg, Some(name), existing_by_name)?);
                        }
                    }
                    Value::Array(items) => {
                        for item in items {
                            out.push(parse_server(item, None, existing_by_name)?);
                        }
                    }
                    _ => bail!("Expected mcpServers/mcp_servers/servers to be an object or array"),
                }
                return Ok(());
            }

            out.push(parse_server(value, None, existing_by_name)?);
        }
        _ => bail!("Expected a JSON object or array"),
    }

    Ok(())
}

fn parse_server(
    value: &Value,
    name_hint: Option<&str>,
    existing_by_name: &HashMap<&str, &McpServer>,
) -> Result<McpServer> {
    let obj = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Each MCP server must be a JSON object"))?;

    let name = obj
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| name_hint.map(str::to_string))
        .ok_or_else(|| anyhow::anyhow!("MCP server is missing a name"))?;

    let is_enabled = obj
        .get("is_enabled")
        .or_else(|| obj.get("enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let command = obj
        .get("command")
        .and_then(Value::as_str)
        .map(str::to_string);

    let endpoint_url = obj
        .get("endpoint_url")
        .or_else(|| obj.get("endpointUrl"))
        .or_else(|| obj.get("url"))
        .and_then(Value::as_str)
        .map(str::to_string);

    let args = match obj.get("args") {
        None => Vec::new(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| anyhow::anyhow!("args must be an array of strings"))
            })
            .collect::<Result<Vec<_>>>()?,
        Some(_) => bail!("args must be an array of strings"),
    };

    let env = match obj.get("env") {
        None => BTreeMap::new(),
        Some(Value::Object(items)) => {
            let mut env = BTreeMap::new();
            for (k, v) in items {
                let value = v
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| v.to_string());
                env.insert(k.clone(), value);
            }
            env
        }
        Some(_) => bail!("env must be an object of string values"),
    };

    let transport = if let Some(t) = obj.get("transport").and_then(Value::as_str) {
        parse_transport(t)?
    } else if endpoint_url.is_some() && command.is_none() {
        McpTransport::StreamableHttp
    } else {
        McpTransport::Stdio
    };

    let id = obj
        .get("id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| existing_by_name.get(name.as_str()).map(|s| s.id.clone()))
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let mut server = McpServer::new(id, name).set_enabled(is_enabled);
    server = server.with_args(args).with_env(env);

    match transport {
        McpTransport::Stdio => {
            let command = command
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow::anyhow!("Stdio MCP server requires a non-empty command"))?;
            Ok(server.with_stdio_command(command))
        }
        McpTransport::StreamableHttp => {
            let url = endpoint_url
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow::anyhow!("HTTP MCP server requires a non-empty url"))?;
            Ok(server.with_streamable_http(url))
        }
    }
}

fn parse_transport(raw: &str) -> Result<McpTransport> {
    let normalized = raw.trim().to_lowercase();
    match normalized.as_str() {
        "stdio" => Ok(McpTransport::Stdio),
        "http" | "streamable_http" | "streamable-http" | "sse" => Ok(McpTransport::StreamableHttp),
        other => bail!("Unsupported transport: {}", other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_claude_desktop_mcp_servers_map() {
        let input = r#"
        {
          "mcpServers": {
            "filesystem": {
              "command": "mcp-filesystem",
              "args": ["--root", "/tmp"],
              "env": {"FOO": "bar"}
            },
            "web": {
              "url": "http://localhost:7777"
            }
          }
        }
        "#;

        let existing = vec![McpServer::new("fixed-id", "filesystem").with_stdio_command("old")];
        let parsed = parse_mcp_servers_json(input, &existing).unwrap();
        assert_eq!(parsed.len(), 2);

        let fs = parsed.iter().find(|s| s.name == "filesystem").unwrap();
        assert_eq!(fs.id, "fixed-id");
        assert_eq!(fs.transport, McpTransport::Stdio);
        assert_eq!(fs.command.as_deref(), Some("mcp-filesystem"));
        assert_eq!(fs.args, vec!["--root".to_string(), "/tmp".to_string()]);
        assert_eq!(fs.env.get("FOO").map(String::as_str), Some("bar"));

        let web = parsed.iter().find(|s| s.name == "web").unwrap();
        assert_eq!(web.transport, McpTransport::StreamableHttp);
        assert_eq!(web.endpoint_url.as_deref(), Some("http://localhost:7777"));
    }
}
