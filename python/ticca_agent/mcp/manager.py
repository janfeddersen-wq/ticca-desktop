"""MCP server manager for Ticca Agent.

This module provides the MCPServerManager class that handles the complete
lifecycle of MCP server connections. It provides:

- Server registration and configuration
- Lifecycle management (start/stop all servers)
- Health monitoring with automatic recovery
- Tool aggregation across all servers
- Event audit trail for debugging

Example:
    >>> from pathlib import Path
    >>> from ticca_agent.mcp import MCPServerManager
    >>> manager = MCPServerManager(config_path=Path("mcp_servers.json"))
    >>> await manager.start()
    >>> tools = manager.get_all_tools()
    >>> result = await manager.call_tool("filesystem", "read_file", {"path": "/etc/hosts"})
    >>> await manager.stop()

"""

from __future__ import annotations

import asyncio
import json
from contextlib import asynccontextmanager
from typing import TYPE_CHECKING, Any, Self

import structlog
from pydantic import BaseModel, ConfigDict, Field

from ticca_agent.mcp.circuit_breaker import CircuitBreakerRegistry
from ticca_agent.mcp.health import HealthMonitor
from ticca_agent.mcp.models import (
    ServerEntry,
    ServerEvent,
    ServerStatus,
    ServerType,
    ToolDefinition,
)
from ticca_agent.mcp.server import MCPServer, ServerState

if TYPE_CHECKING:
    from collections.abc import AsyncIterator
    from pathlib import Path

    from ticca_agent.mcp.servers.base import BaseMCPServer

logger = structlog.get_logger(__name__)


class MCPManagerConfig(BaseModel):
    """Configuration for the MCP manager.

    Attributes:
        health_check_interval: Seconds between health checks.
        failure_threshold: Failures before quarantine.
        recovery_interval: Seconds between recovery attempts.
        auto_start_enabled: Auto-start enabled servers.
    """

    health_check_interval: float = Field(
        default=30.0,
        gt=0.0,
        description="Seconds between health checks",
    )
    failure_threshold: int = Field(
        default=3,
        ge=1,
        description="Failures before quarantine",
    )
    recovery_interval: float = Field(
        default=60.0,
        gt=0.0,
        description="Seconds between recovery attempts",
    )
    auto_start_enabled: bool = Field(
        default=True,
        description="Auto-start enabled servers on manager start",
    )

    model_config = ConfigDict(frozen=True)


class MCPServerManager:
    """Manager for MCP server connections.

    The MCPServerManager handles the complete lifecycle of MCP servers:

    - **Configuration**: Load server configs from JSON file
    - **Lifecycle**: Start/stop servers individually or all at once
    - **Health**: Background health monitoring with quarantine
    - **Tools**: Aggregate tools from all running servers
    - **Invocation**: Route tool calls to appropriate servers
    - **Events**: Audit trail of server events

    The manager uses circuit breakers to prevent cascading failures
    and automatically attempts recovery of failed servers.

    Example:
        >>> manager = MCPServerManager(config_path=Path("servers.json"))
        >>> async with manager.use():
        ...     tools = manager.get_all_tools()
        ...     result = await manager.call_tool("fs", "read_file", {"path": "/etc/hosts"})
    """

    def __init__(
        self,
        config_path: Path | None = None,
        *,
        config: MCPManagerConfig | None = None,
    ) -> None:
        """Initialize the MCP manager.

        Args:
            config_path: Path to server configuration JSON file.
            config: Manager configuration.
        """
        self._config_path = config_path
        self._config = config or MCPManagerConfig()

        # Server management
        self._servers: dict[str, MCPServer] = {}
        self._entries: dict[str, ServerEntry] = {}

        # Circuit breakers
        self._circuit_breakers = CircuitBreakerRegistry(
            default_failure_threshold=self._config.failure_threshold,
        )

        # Health monitoring
        self._health_monitor = HealthMonitor(
            check_interval=self._config.health_check_interval,
            failure_threshold=self._config.failure_threshold,
            recovery_interval=self._config.recovery_interval,
        )

        # State
        self._initialized = False
        self._running = False

        # Event tracking
        self._events: list[ServerEvent] = []
        self._max_events = 1000

    @property
    def is_initialized(self) -> bool:
        """Check if manager is initialized."""
        return self._initialized

    @property
    def is_running(self) -> bool:
        """Check if manager is running."""
        return self._running

    @property
    def server_count(self) -> int:
        """Get number of registered servers."""
        return len(self._entries)

    @property
    def running_server_count(self) -> int:
        """Get number of running servers."""
        return sum(1 for s in self._servers.values() if s.is_running)

    async def start(self) -> None:
        """Start the manager and all enabled servers.

        Loads configuration, starts enabled servers, and begins
        health monitoring.
        """
        if self._running:
            await logger.awarning("MCP manager already running")
            return

        await logger.ainfo("Starting MCP server manager")

        # Load configuration
        if self._config_path and self._config_path.exists():
            await self._load_config()

        self._initialized = True

        # Start enabled servers
        if self._config.auto_start_enabled:
            await self._start_enabled_servers()

        # Start health monitoring
        server_instances = self._get_base_servers()
        if server_instances:
            await self._health_monitor.start_monitoring(server_instances)

        self._running = True

        await self._emit_event(
            "manager",
            "started",
            f"Manager started with {self.server_count} servers",
        )

        await logger.ainfo(
            "MCP server manager started",
            total_servers=self.server_count,
            running_servers=self.running_server_count,
        )

    async def stop(self) -> None:
        """Stop all servers and the manager.

        Gracefully stops all running servers and health monitoring.
        """
        if not self._running:
            return

        await logger.ainfo("Stopping MCP server manager")

        # Stop health monitoring
        await self._health_monitor.stop_monitoring()

        # Stop all servers
        stop_tasks = []
        for server in self._servers.values():
            if server.is_running:
                stop_tasks.append(server.stop())

        if stop_tasks:
            await asyncio.gather(*stop_tasks, return_exceptions=True)

        self._running = False

        await self._emit_event(
            "manager",
            "stopped",
            "Manager stopped",
        )

        await logger.ainfo("MCP server manager stopped")

    async def add_server(
        self,
        name: str,
        server_type: str,
        config: dict[str, Any],
        *,
        enabled: bool = True,
        auto_start: bool = True,
    ) -> None:
        """Add and optionally start a new server.

        Args:
            name: Unique server identifier.
            server_type: Type of server (sse, stdio, http).
            config: Type-specific configuration.
            enabled: Whether server is enabled.
            auto_start: Start server immediately if enabled.

        Raises:
            ValueError: If server name already exists.
        """
        if name in self._entries:
            raise ValueError(f"Server '{name}' already exists")

        # Create entry
        entry = ServerEntry(
            name=name,
            server_type=ServerType(server_type.lower()),
            enabled=enabled,
            config=config,
        )

        # Get or create circuit breaker
        circuit_breaker = await self._circuit_breakers.get_or_create(name)

        # Create server wrapper
        server = MCPServer(entry, circuit_breaker=circuit_breaker)

        # Register
        self._entries[name] = entry
        self._servers[name] = server

        await self._emit_event(
            name,
            "added",
            f"Server added (type={server_type}, enabled={enabled})",
        )

        # Auto-start if enabled and requested
        if enabled and auto_start and self._running:
            try:
                await server.start()
            except Exception as e:
                await logger.awarning(
                    "Failed to auto-start server",
                    server=name,
                    error=str(e),
                )

        # Save configuration
        if self._config_path:
            await self._save_config()

    async def remove_server(self, name: str) -> None:
        """Stop and remove a server.

        Args:
            name: Server name.

        Raises:
            KeyError: If server not found.
        """
        if name not in self._entries:
            raise KeyError(f"Server '{name}' not found")

        # Stop if running
        server = self._servers.get(name)
        if server and server.is_running:
            await server.stop()

        # Remove
        del self._entries[name]
        if name in self._servers:
            del self._servers[name]

        # Remove circuit breaker
        await self._circuit_breakers.remove(name)

        await self._emit_event(
            name,
            "removed",
            "Server removed",
        )

        # Save configuration
        if self._config_path:
            await self._save_config()

    async def enable_server(self, name: str) -> None:
        """Enable and start a server.

        Args:
            name: Server name.

        Raises:
            KeyError: If server not found.
        """
        if name not in self._entries:
            raise KeyError(f"Server '{name}' not found")

        entry = self._entries[name]
        if entry.enabled:
            return

        # Create new entry with enabled=True
        new_entry = ServerEntry(
            name=entry.name,
            server_type=entry.server_type,
            enabled=True,
            config=entry.config,
        )
        self._entries[name] = new_entry

        # Update server
        circuit_breaker = await self._circuit_breakers.get_or_create(name)
        self._servers[name] = MCPServer(new_entry, circuit_breaker=circuit_breaker)

        # Start if manager is running
        if self._running:
            try:
                await self._servers[name].start()
            except Exception as e:
                await logger.awarning(
                    "Failed to start enabled server",
                    server=name,
                    error=str(e),
                )

        await self._emit_event(
            name,
            "enabled",
            "Server enabled",
        )

        # Save configuration
        if self._config_path:
            await self._save_config()

    async def disable_server(self, name: str) -> None:
        """Disable and stop a server.

        Args:
            name: Server name.

        Raises:
            KeyError: If server not found.
        """
        if name not in self._entries:
            raise KeyError(f"Server '{name}' not found")

        # Stop if running
        server = self._servers.get(name)
        if server and server.is_running:
            await server.stop()

        # Create new entry with enabled=False
        entry = self._entries[name]
        new_entry = ServerEntry(
            name=entry.name,
            server_type=entry.server_type,
            enabled=False,
            config=entry.config,
        )
        self._entries[name] = new_entry

        await self._emit_event(
            name,
            "disabled",
            "Server disabled",
        )

        # Save configuration
        if self._config_path:
            await self._save_config()

    def get_server_status(self, name: str) -> ServerStatus:
        """Get current status of a server.

        Args:
            name: Server name.

        Returns:
            Server status.

        Raises:
            KeyError: If server not found.
        """
        if name not in self._entries:
            raise KeyError(f"Server '{name}' not found")

        server = self._servers.get(name)
        if server:
            return server.get_status()

        # Return status for uninitialized server
        entry = self._entries[name]
        return ServerStatus(
            name=name,
            server_type=entry.server_type,
            state=ServerState.STOPPED.value,
            enabled=entry.enabled,
        )

    def list_servers(self) -> list[ServerStatus]:
        """List all configured servers with status.

        Returns:
            List of server status objects.
        """
        statuses = []
        for name in self._entries:
            statuses.append(self.get_server_status(name))
        return statuses

    async def call_tool(
        self,
        server_name: str,
        tool_name: str,
        arguments: dict[str, Any],
    ) -> Any:
        """Call a tool on a specific server.

        Args:
            server_name: Name of the server.
            tool_name: Name of the tool.
            arguments: Tool arguments.

        Returns:
            Tool execution result.

        Raises:
            KeyError: If server not found.
            ValueError: If server not running.
            MCPProtocolError: If tool call fails.
        """
        if server_name not in self._servers:
            raise KeyError(f"Server '{server_name}' not found")

        server = self._servers[server_name]
        if not server.is_running:
            raise ValueError(f"Server '{server_name}' is not running")

        return await server.call_tool(tool_name, arguments)

    def get_all_tools(self) -> list[ToolDefinition]:
        """Get tool definitions from all running servers.

        Returns:
            Aggregated list of tools from all running servers.
        """
        tools = []
        for server in self._servers.values():
            if server.is_running:
                tools.extend(server.tools)
        return tools

    def get_server_tools(self, name: str) -> list[ToolDefinition]:
        """Get tools from a specific server.

        Args:
            name: Server name.

        Returns:
            List of tools from the server.

        Raises:
            KeyError: If server not found.
        """
        if name not in self._servers:
            raise KeyError(f"Server '{name}' not found")

        return self._servers[name].tools

    def find_tool_server(self, tool_name: str) -> str | None:
        """Find which server provides a tool.

        Args:
            tool_name: Name of the tool.

        Returns:
            Server name or None if not found.
        """
        for server in self._servers.values():
            if server.is_running:
                for tool in server.tools:
                    if tool.name == tool_name:
                        return server.name
        return None

    def get_events(
        self,
        server_name: str | None = None,
        limit: int = 100,
    ) -> list[ServerEvent]:
        """Get recent events.

        Args:
            server_name: Filter by server (all if None).
            limit: Maximum events to return.

        Returns:
            List of events, newest first.
        """
        events = self._events
        if server_name:
            events = [e for e in events if e.server_name == server_name]
        return list(reversed(events[-limit:]))

    async def restart_server(self, name: str) -> None:
        """Restart a server.

        Args:
            name: Server name.

        Raises:
            KeyError: If server not found.
        """
        if name not in self._servers:
            raise KeyError(f"Server '{name}' not found")

        await self._servers[name].restart()

        await self._emit_event(
            name,
            "restarted",
            "Server restarted",
        )

    async def _load_config(self) -> None:
        """Load server configuration from file."""
        if not self._config_path or not self._config_path.exists():
            return

        try:
            content = self._config_path.read_text()
            data = json.loads(content)

            servers = data.get("servers", [])
            for server_data in servers:
                try:
                    entry = ServerEntry.model_validate(server_data)
                    circuit_breaker = await self._circuit_breakers.get_or_create(
                        entry.name
                    )
                    server = MCPServer(entry, circuit_breaker=circuit_breaker)

                    self._entries[entry.name] = entry
                    self._servers[entry.name] = server

                except Exception as e:
                    await logger.awarning(
                        "Failed to load server config",
                        server=server_data.get("name", "unknown"),
                        error=str(e),
                    )

            await logger.ainfo(
                "Loaded MCP server configuration",
                server_count=len(self._entries),
            )

        except Exception as e:
            await logger.aerror(
                "Failed to load MCP configuration",
                path=str(self._config_path),
                error=str(e),
            )

    async def _save_config(self) -> None:
        """Save server configuration to file."""
        if not self._config_path:
            return

        try:
            data = {
                "servers": [
                    entry.model_dump()
                    for entry in self._entries.values()
                ]
            }

            content = json.dumps(data, indent=2)
            self._config_path.write_text(content)

        except Exception as e:
            await logger.aerror(
                "Failed to save MCP configuration",
                path=str(self._config_path),
                error=str(e),
            )

    async def _start_enabled_servers(self) -> None:
        """Start all enabled servers."""
        start_tasks = []
        for name, entry in self._entries.items():
            if entry.enabled:
                server = self._servers.get(name)
                if server:
                    start_tasks.append(self._start_server_safe(server))

        if start_tasks:
            await asyncio.gather(*start_tasks, return_exceptions=True)

    async def _start_server_safe(self, server: MCPServer) -> None:
        """Start a server with error handling."""
        try:
            await server.start()
        except Exception as e:
            await logger.awarning(
                "Failed to start server",
                server=server.name,
                error=str(e),
            )

    def _get_base_servers(self) -> dict[str, BaseMCPServer]:
        """Get base server instances for health monitoring."""
        servers = {}
        for name, server in self._servers.items():
            if server._server is not None:
                servers[name] = server._server
        return servers

    async def _emit_event(
        self,
        server_name: str,
        event_type: str,
        message: str,
        details: dict[str, Any] | None = None,
    ) -> None:
        """Emit a server event."""
        event = ServerEvent(
            server_name=server_name,
            event_type=event_type,
            message=message,
            details=details or {},
        )

        self._events.append(event)
        if len(self._events) > self._max_events:
            self._events = self._events[-self._max_events:]

        await logger.ainfo(
            "MCP event",
            server=server_name,
            event_type=event_type,
            message=message,
        )

    @asynccontextmanager
    async def use(self) -> AsyncIterator[Self]:
        """Use the manager as an async context manager.

        Yields:
            The started manager.
        """
        await self.start()
        try:
            yield self
        finally:
            await self.stop()


class MCPError(Exception):
    """Exception for MCP-related errors.

    Attributes:
        message: Human-readable error description.
        server_name: Name of the server that caused the error.
        code: Optional error code.
    """

    def __init__(
        self,
        message: str,
        *,
        server_name: str | None = None,
        code: int | None = None,
    ) -> None:
        """Initialize MCPError.

        Args:
            message: Human-readable error description.
            server_name: Name of the server that caused the error.
            code: Optional error code.
        """
        super().__init__(message)
        self.message = message
        self.server_name = server_name
        self.code = code

    def __str__(self) -> str:
        """Return string representation of the error."""
        parts = []
        if self.server_name:
            parts.append(f"server={self.server_name}")
        if self.code:
            parts.append(f"code={self.code}")
        prefix = f"[{', '.join(parts)}] " if parts else ""
        return f"{prefix}{self.message}"


# Legacy exports for backward compatibility
ServerConfig = ServerEntry
ServerInfo = ServerStatus
MCPTool = ToolDefinition
MCPResource = ToolDefinition  # Resources use same structure


__all__ = [
    "MCPError",
    "MCPManagerConfig",
    "MCPResource",
    "MCPServerManager",
    "MCPTool",
    "ServerConfig",
    "ServerInfo",
]
