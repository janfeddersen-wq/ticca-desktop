//! MCP servers CRUD operations.

use std::collections::BTreeMap;

use anyhow::Result;
use rusqlite::params;

use crate::config::models::{McpServer, McpTransport};

use super::ConfigDatabase;

impl ConfigDatabase {
    /// List all MCP servers.
    pub fn list_mcp_servers(&self) -> Result<Vec<McpServer>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, transport, command, args_json, env_json, endpoint_url, is_enabled, created_at, updated_at
             FROM mcp_servers
             ORDER BY name",
        )?;

        let rows = stmt.query_map([], |row| {
            let transport_str: String = row.get(2)?;
            let transport = transport_str
                .parse::<McpTransport>()
                .unwrap_or(McpTransport::Stdio);

            let args_json: Option<String> = row.get(4)?;
            let args: Vec<String> = args_json
                .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
                .unwrap_or_default();

            let env_json: Option<String> = row.get(5)?;
            let env: BTreeMap<String, String> = env_json
                .and_then(|s| serde_json::from_str::<BTreeMap<String, String>>(&s).ok())
                .unwrap_or_default();

            Ok(McpServer {
                id: row.get(0)?,
                name: row.get(1)?,
                transport,
                command: row.get(3)?,
                args,
                env,
                endpoint_url: row.get(6)?,
                is_enabled: {
                    let val: i64 = row.get(7)?;
                    val != 0
                },
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Upsert an MCP server.
    pub fn upsert_mcp_server(&self, server: &McpServer) -> Result<()> {
        let args_json = if server.args.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&server.args)?)
        };
        let env_json = if server.env.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&server.env)?)
        };

        self.conn.execute(
            "INSERT INTO mcp_servers (id, name, transport, command, args_json, env_json, endpoint_url, is_enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, COALESCE(?, datetime('now')), datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                transport = excluded.transport,
                command = excluded.command,
                args_json = excluded.args_json,
                env_json = excluded.env_json,
                endpoint_url = excluded.endpoint_url,
                is_enabled = excluded.is_enabled,
                updated_at = datetime('now')",
            params![
                server.id,
                server.name,
                server.transport.as_str(),
                server.command,
                args_json,
                env_json,
                server.endpoint_url,
                if server.is_enabled { 1 } else { 0 },
                server.created_at,
            ],
        )?;
        Ok(())
    }

    /// Delete an MCP server and its agent mappings.
    pub fn delete_mcp_server(&self, server_id: &str) -> Result<bool> {
        self.conn.execute(
            "DELETE FROM agent_mcp_servers WHERE server_id = ?",
            params![server_id],
        )?;
        let changes = self
            .conn
            .execute("DELETE FROM mcp_servers WHERE id = ?", params![server_id])?;
        Ok(changes > 0)
    }

    /// Set the enabled status of an MCP server.
    pub fn set_mcp_server_enabled(&self, server_id: &str, enabled: bool) -> Result<bool> {
        let changes = self.conn.execute(
            "UPDATE mcp_servers SET is_enabled = ?, updated_at = datetime('now') WHERE id = ?",
            params![if enabled { 1 } else { 0 }, server_id],
        )?;
        Ok(changes > 0)
    }

    /// Get the MCP server IDs assigned to an agent.
    pub fn get_agent_mcp_server_ids(&self, agent_type: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT server_id
             FROM agent_mcp_servers
             WHERE agent_type = ?
             ORDER BY server_id",
        )?;

        let rows = stmt.query_map(params![agent_type], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Set the MCP server IDs assigned to an agent.
    pub fn set_agent_mcp_server_ids(&self, agent_type: &str, server_ids: &[String]) -> Result<()> {
        self.conn.execute(
            "DELETE FROM agent_mcp_servers WHERE agent_type = ?",
            params![agent_type],
        )?;
        for server_id in server_ids {
            self.conn.execute(
                "INSERT OR IGNORE INTO agent_mcp_servers (agent_type, server_id) VALUES (?, ?)",
                params![agent_type, server_id],
            )?;
        }
        Ok(())
    }
}
