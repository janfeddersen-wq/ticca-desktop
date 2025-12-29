//! MCP (Model Context Protocol) server connection handling.
//!
//! Uses serdes-ai-mcp for MCP integration.

use super::types::MCP_CONNECT_TIMEOUT;
use crate::agents::AgentType;
use crate::config::{ConfigDatabase, McpServer, McpTransport};
use serdes_ai_mcp::{McpClient, McpToolset};
use serdes_ai_toolsets::DynamicToolset;
use std::sync::Arc;

/// List active MCP servers for an agent type.
pub fn list_active_mcp_servers(agent: AgentType) -> Vec<McpServer> {
    let Ok(db) = ConfigDatabase::open() else {
        return Vec::new();
    };

    let ids = db
        .get_agent_mcp_server_ids(agent.as_str())
        .unwrap_or_default();
    if ids.is_empty() {
        return Vec::new();
    }

    let servers = db.list_mcp_servers().unwrap_or_default();
    servers
        .into_iter()
        .filter(|s| s.is_enabled && ids.contains(&s.id))
        .collect()
}

/// Connect to an MCP server and retrieve its toolset.
pub async fn connect_mcp_server<Deps: Send + Sync + 'static>(
    server: &McpServer,
) -> anyhow::Result<McpToolset<Deps>> {
    match server.transport {
        McpTransport::StreamableHttp => {
            let url = server
                .endpoint_url
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Missing endpoint_url for HTTP MCP server"))?;

            let client = McpClient::http(url);
            client.initialize().await?;
            Ok(McpToolset::new(client).with_id(&server.name))
        }
        McpTransport::Stdio => {
            let command = server
                .command
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Missing command for stdio MCP server"))?;

            let args: Vec<&str> = server.args.iter().map(|s| s.as_str()).collect();
            let toolset = McpToolset::stdio(command, &args).await?;
            Ok(toolset.with_id(&server.name))
        }
    }
}

/// Load MCP toolsets for an agent type.
/// 
/// Returns a vector of McpToolsets that can be added to an agent.
pub async fn load_mcp_toolsets<Deps: Send + Sync + 'static>(
    agent: AgentType,
) -> Vec<McpToolset<Deps>> {
    let servers = list_active_mcp_servers(agent);
    if servers.is_empty() {
        return Vec::new();
    }

    let mut toolsets = Vec::new();

    for server in servers {
        let res = tokio::time::timeout(MCP_CONNECT_TIMEOUT, connect_mcp_server(&server)).await;
        match res {
            Ok(Ok(toolset)) => {
                tracing::info!("Loaded MCP toolset from '{}'", server.name);
                toolsets.push(toolset);
            }
            Ok(Err(e)) => {
                tracing::warn!("Failed to connect to MCP server '{}': {}", server.name, e);
            }
            Err(_) => {
                tracing::warn!(
                    "Timed out connecting to MCP server '{}' ({}s)",
                    server.name,
                    MCP_CONNECT_TIMEOUT.as_secs()
                );
            }
        }
    }

    toolsets
}
