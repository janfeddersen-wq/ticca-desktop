"""MCP server wrapper and utilities.

This module provides the high-level MCPServer class that wraps different
server implementations and provides a unified interface for the manager.

The MCPServer class handles:
- Server type detection and instantiation
- Configuration management
- State tracking with circuit breaker integration
- Unified tool invocation interface

Example:
    >>> from ticca_agent.mcp.server import MCPServer, create_server
    >>> server = create_server(
    ...     name="filesystem",
    ...     server_type="stdio",
    ...     config={"command": "npx", "args": ["-y", "@mcp/server-fs"]},
    ... )
    >>> await server.start()

"""

from __future__ import annotations

from datetime import UTC, datetime
from typing import Any

import structlog

from ticca_agent.mcp.circuit_breaker import CircuitBreaker, CircuitBreakerOpenError
from ticca_agent.mcp.models import (
    HTTPServerConfig,
    ServerEntry,
    ServerStatus,
    ServerType,
    SSEServerConfig,
    StdioServerConfig,
    ToolDefinition,
)
from ticca_agent.mcp.servers.base import (
    BaseMCPServer,
    MCPProtocolError,
    ServerState,
)
from ticca_agent.mcp.servers.http import HTTPMCPServer
from ticca_agent.mcp.servers.sse import SSEMCPServer
from ticca_agent.mcp.servers.stdio import StdioMCPServer

logger = structlog.get_logger(__name__)


def create_server_from_entry(entry: ServerEntry) -> BaseMCPServer:
    """Create a server instance from a configuration entry.

    Args:
        entry: Server configuration entry.

    Returns:
        Configured server instance.

    Raises:
        ValueError: If server type is unknown.
    """
    if entry.server_type == ServerType.STDIO:
        stdio_config = StdioServerConfig(**entry.config)
        return StdioMCPServer.from_config(entry.name, stdio_config)

    elif entry.server_type == ServerType.SSE:
        sse_config = SSEServerConfig(**entry.config)
        return SSEMCPServer.from_config(entry.name, sse_config)

    elif entry.server_type == ServerType.HTTP:
        http_config = HTTPServerConfig(**entry.config)
        return HTTPMCPServer.from_config(entry.name, http_config)

    else:
        raise ValueError(f"Unknown server type: {entry.server_type}")


def create_server(
    name: str,
    server_type: str | ServerType,
    config: dict[str, Any],
) -> BaseMCPServer:
    """Create a server instance from raw parameters.

    Args:
        name: Server name.
        server_type: Type of server (stdio, sse, http).
        config: Type-specific configuration.

    Returns:
        Configured server instance.

    Raises:
        ValueError: If configuration is invalid.
    """
    # Normalize server type
    if isinstance(server_type, str):
        server_type = ServerType(server_type.lower())

    entry = ServerEntry(
        name=name,
        server_type=server_type,
        enabled=True,
        config=config,
    )

    return create_server_from_entry(entry)


class MCPServer:
    """High-level wrapper for MCP server instances.

    Provides a unified interface for managing MCP servers with:
    - Lifecycle management (start/stop)
    - Circuit breaker integration for fault tolerance
    - Status tracking and reporting
    - Tool invocation with error handling

    This class wraps the underlying server implementations (Stdio, SSE, HTTP)
    and adds resilience features on top.

    Attributes:
        name: Server identifier.
        server_type: Type of server.
        enabled: Whether server is enabled in config.
        state: Current server state.

    Example:
        >>> server = MCPServer(
        ...     entry=ServerEntry(...),
        ...     circuit_breaker=CircuitBreaker(),
        ... )
        >>> await server.start()
        >>> result = await server.call_tool("read_file", {"path": "/etc/hosts"})
        >>> await server.stop()
    """

    def __init__(
        self,
        entry: ServerEntry,
        *,
        circuit_breaker: CircuitBreaker | None = None,
    ) -> None:
        """Initialize the MCP server wrapper.

        Args:
            entry: Server configuration entry.
            circuit_breaker: Optional circuit breaker for fault tolerance.
        """
        self._entry = entry
        self._server: BaseMCPServer | None = None
        self._circuit_breaker = circuit_breaker or CircuitBreaker(
            name=entry.name,
            failure_threshold=5,
            recovery_timeout=30.0,
        )
        self._last_error: str | None = None
        self._consecutive_failures = 0
        self._started_at: datetime | None = None

    @property
    def name(self) -> str:
        """Get server name."""
        return self._entry.name

    @property
    def server_type(self) -> ServerType:
        """Get server type."""
        return self._entry.server_type

    @property
    def enabled(self) -> bool:
        """Check if server is enabled."""
        return self._entry.enabled

    @property
    def state(self) -> ServerState:
        """Get current server state."""
        if self._server is None:
            return ServerState.STOPPED
        return self._server.state

    @property
    def is_running(self) -> bool:
        """Check if server is running."""
        return self._server is not None and self._server.is_connected

    @property
    def is_quarantined(self) -> bool:
        """Check if server is in quarantine (circuit open)."""
        return self._circuit_breaker.is_open

    @property
    def tools(self) -> list[ToolDefinition]:
        """Get cached tool definitions."""
        if self._server is None:
            return []
        return self._server._tools.copy()

    @property
    def tool_count(self) -> int:
        """Get number of available tools."""
        return len(self.tools)

    @property
    def uptime_seconds(self) -> float:
        """Get uptime in seconds."""
        if self._started_at is None:
            return 0.0
        return (datetime.now(UTC) - self._started_at).total_seconds()

    @property
    def consecutive_failures(self) -> int:
        """Get consecutive failure count."""
        return self._consecutive_failures

    @property
    def last_error(self) -> str | None:
        """Get last error message."""
        return self._last_error

    def get_status(self) -> ServerStatus:
        """Get current server status.

        Returns:
            ServerStatus with current state information.
        """
        state = self.state
        if self.is_quarantined:
            state = ServerState.QUARANTINED

        return ServerStatus(
            name=self.name,
            server_type=self.server_type,
            state=state.value,
            enabled=self.enabled,
            tool_count=self.tool_count,
            last_health_check=None,  # Updated by health monitor
            last_error=self._last_error,
            consecutive_failures=self._consecutive_failures,
            uptime_seconds=self.uptime_seconds,
        )

    async def start(self) -> None:
        """Start the server.

        Creates the underlying server instance and connects.

        Raises:
            MCPProtocolError: If connection fails.
        """
        if self.is_running:
            return

        await logger.ainfo(
            "Starting MCP server",
            server=self.name,
            server_type=self.server_type.value,
        )

        try:
            # Create server instance
            self._server = create_server_from_entry(self._entry)

            # Connect
            await self._server.connect()

            self._started_at = datetime.now(UTC)
            self._last_error = None
            self._consecutive_failures = 0

            await logger.ainfo(
                "MCP server started",
                server=self.name,
                tool_count=self.tool_count,
            )

        except Exception as e:
            self._last_error = str(e)
            self._consecutive_failures += 1
            await self._circuit_breaker.record_failure()

            await logger.aerror(
                "Failed to start MCP server",
                server=self.name,
                error=str(e),
            )

            raise MCPProtocolError(
                f"Failed to start server '{self.name}': {e}"
            ) from e

    async def stop(self) -> None:
        """Stop the server.

        Disconnects and cleans up the underlying server.
        """
        if self._server is None:
            return

        await logger.ainfo(
            "Stopping MCP server",
            server=self.name,
        )

        try:
            await self._server.disconnect()
        except Exception as e:
            await logger.awarning(
                "Error stopping MCP server",
                server=self.name,
                error=str(e),
            )
        finally:
            self._server = None
            self._started_at = None

    async def restart(self) -> None:
        """Restart the server.

        Stops and starts the server.
        """
        await self.stop()
        await self.start()

    async def call_tool(
        self,
        tool_name: str,
        arguments: dict[str, Any],
    ) -> Any:
        """Call a tool on the server.

        Uses circuit breaker to protect against cascading failures.

        Args:
            tool_name: Name of the tool to call.
            arguments: Tool arguments.

        Returns:
            Tool execution result.

        Raises:
            CircuitBreakerOpenError: If circuit is open.
            MCPProtocolError: If tool call fails.
            ValueError: If server not running.
        """
        if not self.is_running:
            raise ValueError(f"Server '{self.name}' is not running")

        if self._server is None:
            raise ValueError(f"Server '{self.name}' is not initialized")

        async def _call() -> Any:
            assert self._server is not None
            return await self._server.call_tool(tool_name, arguments)

        try:
            result = await self._circuit_breaker.call(_call)
            self._consecutive_failures = 0
            self._last_error = None
            return result

        except CircuitBreakerOpenError:
            raise

        except Exception as e:
            self._consecutive_failures += 1
            self._last_error = str(e)
            raise

    async def list_tools(self) -> list[ToolDefinition]:
        """List tools from the server.

        Refreshes the tool cache from the server.

        Returns:
            List of available tools.
        """
        if not self.is_running or self._server is None:
            return []

        return await self._server.list_tools()

    async def health_check(self) -> bool:
        """Check server health.

        Returns:
            True if server is healthy.
        """
        if not self.is_running or self._server is None:
            return False

        try:
            healthy = await self._server.health_check()
            if healthy:
                await self._circuit_breaker.record_success()
                self._consecutive_failures = 0
            else:
                await self._circuit_breaker.record_failure()
                self._consecutive_failures += 1
            return healthy

        except Exception as e:
            await self._circuit_breaker.record_failure()
            self._consecutive_failures += 1
            self._last_error = str(e)
            return False

    async def reset_circuit_breaker(self) -> None:
        """Reset the circuit breaker to closed state."""
        await self._circuit_breaker.reset()
        self._consecutive_failures = 0

    async def __aenter__(self) -> MCPServer:
        """Enter async context (start)."""
        await self.start()
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: Any,
    ) -> None:
        """Exit async context (stop)."""
        await self.stop()

    def __repr__(self) -> str:
        """Return string representation."""
        return (
            f"MCPServer("
            f"name={self.name!r}, "
            f"type={self.server_type.value}, "
            f"state={self.state.value})"
        )


# Re-export for convenience
__all__ = [
    "MCPProtocolError",
    "MCPServer",
    "ServerState",
    "create_server",
    "create_server_from_entry",
]
