"""Model Context Protocol (MCP) integration for Ticca Agent.

This module provides comprehensive MCP client and server implementations for
interoperability with the broader MCP ecosystem.

MCP enables:
- Standardized tool discovery and invocation
- Context sharing between applications
- Resource access and management
- Cross-application agent communication

Server Types:
- **SSE**: HTTP with Server-Sent Events for web-based servers
- **Stdio**: stdin/stdout for CLI tools and local processes
- **HTTP**: Streamable HTTP for REST-style servers

Key Components:
- **MCPServerManager**: Manages MCP server connections with lifecycle control
- **MCPServer**: High-level wrapper for individual server instances
- **HealthMonitor**: Background health monitoring with quarantine
- **CircuitBreaker**: Fault tolerance with circuit breaker pattern

Example:
    >>> from pathlib import Path
    >>> from ticca_agent.mcp import MCPServerManager
    >>>
    >>> manager = MCPServerManager(config_path=Path("mcp_servers.json"))
    >>> async with manager.use():
    ...     # List all available tools
    ...     tools = manager.get_all_tools()
    ...     # Call a tool
    ...     result = await manager.call_tool("filesystem", "read_file", {"path": "/etc/hosts"})

See Also:
    - FEATURES.md §8 for detailed requirements
    - https://modelcontextprotocol.io for MCP specification

"""

from __future__ import annotations

# Circuit breaker
from ticca_agent.mcp.circuit_breaker import (
    CircuitBreaker,
    CircuitBreakerConfig,
    CircuitBreakerOpenError,
    CircuitBreakerRegistry,
    CircuitState,
)

# Health monitoring
from ticca_agent.mcp.health import (
    EventCallback,
    HealthMonitor,
    HealthMonitorConfig,
    ServerHealthState,
)

# Manager
from ticca_agent.mcp.manager import (
    MCPError,
    MCPManagerConfig,
    MCPResource,
    MCPServerManager,
    MCPTool,
    ServerConfig,
    ServerInfo,
)

# Models
from ticca_agent.mcp.models import (
    SERVER_NAME_PATTERN,
    HealthStatus,
    HTTPServerConfig,
    ServerConfigUnion,
    ServerEntry,
    ServerEvent,
    ServerStatus,
    ServerType,
    SSEServerConfig,
    StdioServerConfig,
    ToolDefinition,
)

# Server wrapper
from ticca_agent.mcp.server import (
    MCPProtocolError,
    MCPServer,
    ServerState,
    create_server,
    create_server_from_entry,
)

# Server implementations
from ticca_agent.mcp.servers import (
    BaseMCPServer,
    HTTPMCPServer,
    SSEMCPServer,
    StdioMCPServer,
)
from ticca_agent.mcp.servers.base import (
    MCPNotification,
    MCPRequest,
    MCPResponse,
    format_mcp_message,
    parse_mcp_message,
)

__all__ = [
    "SERVER_NAME_PATTERN",
    "BaseMCPServer",
    "CircuitBreaker",
    "CircuitBreakerConfig",
    "CircuitBreakerOpenError",
    "CircuitBreakerRegistry",
    "CircuitState",
    "EventCallback",
    "HTTPMCPServer",
    "HTTPServerConfig",
    "HealthMonitor",
    "HealthMonitorConfig",
    "HealthStatus",
    "MCPError",
    "MCPManagerConfig",
    "MCPNotification",
    "MCPProtocolError",
    "MCPRequest",
    "MCPResource",
    "MCPResponse",
    "MCPServer",
    "MCPServerManager",
    "MCPTool",
    "SSEMCPServer",
    "SSEServerConfig",
    "ServerConfig",
    "ServerConfigUnion",
    "ServerEntry",
    "ServerEvent",
    "ServerHealthState",
    "ServerInfo",
    "ServerState",
    "ServerStatus",
    "ServerType",
    "StdioMCPServer",
    "StdioServerConfig",
    "ToolDefinition",
    "create_server",
    "create_server_from_entry",
    "format_mcp_message",
    "parse_mcp_message",
]
