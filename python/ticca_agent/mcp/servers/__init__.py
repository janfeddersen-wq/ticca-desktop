"""MCP server implementations.

This package provides concrete implementations for different MCP server types:
- SSE (Server-Sent Events) for web-based servers
- Stdio for process-based servers
- HTTP for REST-style servers

All implementations inherit from BaseMCPServer and provide a consistent
interface for server lifecycle management and tool invocation.

Example:
    >>> from ticca_agent.mcp.servers import StdioMCPServer, SSEMCPServer
    >>> server = StdioMCPServer(
    ...     name="filesystem",
    ...     command="npx",
    ...     args=["-y", "@modelcontextprotocol/server-filesystem"],
    ... )
    >>> await server.connect()
    >>> tools = await server.list_tools()

"""

from ticca_agent.mcp.servers.base import BaseMCPServer
from ticca_agent.mcp.servers.http import HTTPMCPServer
from ticca_agent.mcp.servers.sse import SSEMCPServer
from ticca_agent.mcp.servers.stdio import StdioMCPServer

__all__ = [
    "BaseMCPServer",
    "HTTPMCPServer",
    "SSEMCPServer",
    "StdioMCPServer",
]
