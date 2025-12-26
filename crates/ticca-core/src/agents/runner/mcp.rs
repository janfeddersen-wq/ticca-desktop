//! MCP (Model Context Protocol) server connection handling.

use crate::agents::AgentType;
use crate::config::{ConfigDatabase, McpServer, McpTransport};
use super::types::MCP_CONNECT_TIMEOUT;

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

/// Create RMCP client info for MCP connections.
fn rmcp_client_info() -> rmcp::model::ClientInfo {
    use rmcp::model::{ClientCapabilities, ClientInfo, Implementation};
    ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "ticca-desktop".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            ..Default::default()
        },
    }
}

/// Connect to an MCP server and retrieve its tools.
pub async fn connect_mcp_server(
    server: &McpServer,
) -> anyhow::Result<(Vec<rmcp::model::Tool>, rmcp::service::ServerSink)> {
    use rmcp::ServiceExt;

    match server.transport {
        McpTransport::StreamableHttp => {
            let url = server
                .endpoint_url
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Missing endpoint_url for HTTP MCP server"))?;

            let transport = rmcp::transport::StreamableHttpClientTransport::from_uri(url);
            let client = rmcp_client_info().serve(transport).await?;

            let tools = client.list_tools(Default::default()).await?.tools;
            Ok((tools, client.peer().to_owned()))
        }
        McpTransport::Stdio => {
            let command = server
                .command
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Missing command for stdio MCP server"))?;

            let mut cmd = tokio::process::Command::new(command);
            cmd.args(&server.args);
            cmd.envs(server.env.clone());

            let transport = rmcp::transport::TokioChildProcess::new(cmd)
                .map_err(|e| anyhow::anyhow!("Failed to spawn MCP server '{}': {}", command, e))?;

            let client = rmcp_client_info().serve(transport).await?;
            let tools = client.list_tools(Default::default()).await?.tools;
            Ok((tools, client.peer().to_owned()))
        }
    }
}

/// Attach MCP tools to an agent builder.
pub async fn attach_mcp_tools_to_builder<M>(
    mut builder: rig::agent::AgentBuilderSimple<M>,
    agent: AgentType,
) -> rig::agent::AgentBuilderSimple<M>
where
    M: rig::completion::CompletionModel + 'static,
{
    let servers = list_active_mcp_servers(agent);
    if servers.is_empty() {
        return builder;
    }

    for server in servers {
        let res = tokio::time::timeout(MCP_CONNECT_TIMEOUT, connect_mcp_server(&server)).await;
        match res {
            Ok(Ok((tools, sink))) => {
                if tools.is_empty() {
                    tracing::info!("MCP server '{}' has no tools", server.name);
                    continue;
                }
                tracing::info!("Loaded {} MCP tools from '{}'", tools.len(), server.name);
                builder = builder.rmcp_tools(tools, sink);
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

    builder
}
