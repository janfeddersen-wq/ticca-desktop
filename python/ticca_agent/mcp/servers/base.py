"""Base class for MCP server implementations.

This module provides the abstract base class that all MCP server types
must implement. It defines the common interface for server lifecycle
management, tool invocation, and health checking.

The MCP (Model Context Protocol) uses JSON-RPC 2.0 over various transports
to enable communication between AI applications and external tools.

Example:
    >>> class MyServer(BaseMCPServer):
    ...     async def connect(self) -> None:
    ...         # Implementation
    ...         pass

"""

from __future__ import annotations

import asyncio
from abc import ABC, abstractmethod
from datetime import UTC, datetime
from enum import Enum
from typing import Any

import structlog
from pydantic import BaseModel, ConfigDict, Field

from ticca_agent.mcp.models import ToolDefinition  # noqa: TC001

logger = structlog.get_logger(__name__)


class ServerState(str, Enum):
    """State of an MCP server.

    Represents the lifecycle states per FEATURES.md §8.4.
    """

    STOPPED = "stopped"
    """Server is disabled/not running."""

    STARTING = "starting"
    """Server is transitioning to running state."""

    RUNNING = "running"
    """Server is active and available."""

    STOPPING = "stopping"
    """Server is transitioning to stopped state."""

    ERROR = "error"
    """Server failed to initialize."""

    QUARANTINED = "quarantined"
    """Server is temporarily disabled due to errors."""


class MCPRequest(BaseModel):
    """JSON-RPC 2.0 request message for MCP.

    Attributes:
        jsonrpc: JSON-RPC version (always "2.0").
        id: Request ID for response correlation.
        method: RPC method name.
        params: Method parameters.
    """

    jsonrpc: str = Field(default="2.0", description="JSON-RPC version")
    id: int | str = Field(description="Request ID")
    method: str = Field(description="RPC method name")
    params: dict[str, Any] | None = Field(
        default=None,
        description="Method parameters",
    )

    model_config = ConfigDict(frozen=True)

    def to_json(self) -> str:
        """Serialize to JSON string."""
        return self.model_dump_json(exclude_none=True)


class MCPResponse(BaseModel):
    """JSON-RPC 2.0 response message for MCP.

    Attributes:
        jsonrpc: JSON-RPC version (always "2.0").
        id: Request ID this responds to.
        result: Result data if successful.
        error: Error information if failed.
    """

    jsonrpc: str = Field(default="2.0", description="JSON-RPC version")
    id: int | str | None = Field(
        default=None,
        description="Request ID",
    )
    result: Any | None = Field(
        default=None,
        description="Result data if successful",
    )
    error: dict[str, Any] | None = Field(
        default=None,
        description="Error information if failed",
    )

    model_config = ConfigDict(frozen=True)

    @classmethod
    def from_json(cls, data: str) -> MCPResponse:
        """Parse from JSON string."""
        return cls.model_validate_json(data)

    @property
    def is_error(self) -> bool:
        """Check if this is an error response."""
        return self.error is not None

    def raise_for_error(self) -> None:
        """Raise exception if this is an error response."""
        if self.error:
            code = self.error.get("code", -1)
            message = self.error.get("message", "Unknown error")
            raise MCPProtocolError(message, code=code)


class MCPNotification(BaseModel):
    """JSON-RPC 2.0 notification message (no response expected).

    Attributes:
        jsonrpc: JSON-RPC version (always "2.0").
        method: Notification method name.
        params: Notification parameters.
    """

    jsonrpc: str = Field(default="2.0", description="JSON-RPC version")
    method: str = Field(description="Notification method name")
    params: dict[str, Any] | None = Field(
        default=None,
        description="Notification parameters",
    )

    model_config = ConfigDict(frozen=True)

    def to_json(self) -> str:
        """Serialize to JSON string."""
        return self.model_dump_json(exclude_none=True)


class MCPProtocolError(Exception):
    """Exception for MCP protocol errors.

    Attributes:
        message: Human-readable error description.
        code: JSON-RPC error code.
        data: Additional error data.
    """

    def __init__(
        self,
        message: str,
        *,
        code: int = -32000,
        data: Any = None,
    ) -> None:
        """Initialize MCPProtocolError.

        Args:
            message: Error description.
            code: JSON-RPC error code.
            data: Additional error data.
        """
        super().__init__(message)
        self.message = message
        self.code = code
        self.data = data

    def __str__(self) -> str:
        """Return string representation."""
        return f"[{self.code}] {self.message}"


class BaseMCPServer(ABC):
    """Abstract base class for MCP server connections.

    All MCP server implementations must inherit from this class and
    implement the abstract methods for server lifecycle and communication.

    The base class provides:
    - State management and tracking
    - Request ID generation
    - Common logging infrastructure
    - Statistics tracking

    Attributes:
        name: Unique server identifier.
        state: Current server state.

    Example:
        >>> class StdioServer(BaseMCPServer):
        ...     async def connect(self) -> None:
        ...         self._set_state(ServerState.STARTING)
        ...         # ... connection logic ...
        ...         self._set_state(ServerState.RUNNING)
    """

    def __init__(self, name: str) -> None:
        """Initialize the base server.

        Args:
            name: Unique server identifier.
        """
        self._name = name
        self._state = ServerState.STOPPED
        self._request_id = 0
        self._connected_at: datetime | None = None
        self._last_activity: datetime | None = None

        # Cached data
        self._tools: list[ToolDefinition] = []
        self._capabilities: dict[str, Any] = {}
        self._server_info: dict[str, Any] = {}

        # Statistics
        self._total_requests = 0
        self._total_errors = 0
        self._lock = asyncio.Lock()

    @property
    def name(self) -> str:
        """Get the server name."""
        return self._name

    @property
    def state(self) -> ServerState:
        """Get the current server state."""
        return self._state

    @property
    def is_connected(self) -> bool:
        """Check if server is connected and running."""
        return self._state == ServerState.RUNNING

    @property
    def connected_at(self) -> datetime | None:
        """Get connection timestamp."""
        return self._connected_at

    @property
    def uptime_seconds(self) -> float:
        """Get uptime in seconds."""
        if self._connected_at is None:
            return 0.0
        return (datetime.now(UTC) - self._connected_at).total_seconds()

    @property
    def capabilities(self) -> dict[str, Any]:
        """Get server capabilities."""
        return self._capabilities.copy()

    @property
    def statistics(self) -> dict[str, Any]:
        """Get server statistics."""
        return {
            "name": self._name,
            "state": self._state.value,
            "uptime_seconds": self.uptime_seconds,
            "total_requests": self._total_requests,
            "total_errors": self._total_errors,
            "tool_count": len(self._tools),
        }

    def _set_state(self, new_state: ServerState) -> None:
        """Set server state with logging."""
        old_state = self._state
        self._state = new_state

        if new_state == ServerState.RUNNING and old_state != ServerState.RUNNING:
            self._connected_at = datetime.now(UTC)
        elif new_state in (ServerState.STOPPED, ServerState.ERROR):
            self._connected_at = None

        # Fire-and-forget log task - intentionally not awaited
        asyncio.create_task(  # noqa: RUF006
            logger.ainfo(
                "Server state changed",
                server=self._name,
                from_state=old_state.value,
                to_state=new_state.value,
            )
        )

    def _next_request_id(self) -> int:
        """Generate next request ID."""
        self._request_id += 1
        return self._request_id

    def _update_activity(self) -> None:
        """Update last activity timestamp."""
        self._last_activity = datetime.now(UTC)

    @abstractmethod
    async def connect(self) -> None:
        """Establish connection to the server.

        Implementations should:
        1. Set state to STARTING
        2. Establish transport connection
        3. Perform MCP initialization handshake
        4. Discover capabilities and tools
        5. Set state to RUNNING

        Raises:
            MCPProtocolError: If connection fails.
        """
        ...

    @abstractmethod
    async def disconnect(self) -> None:
        """Disconnect from the server.

        Implementations should:
        1. Set state to STOPPING
        2. Send shutdown notification if connected
        3. Close transport
        4. Clean up resources
        5. Set state to STOPPED
        """
        ...

    @abstractmethod
    async def call_tool(
        self,
        name: str,
        arguments: dict[str, Any],
    ) -> Any:
        """Call a tool on the server.

        Args:
            name: Tool name.
            arguments: Tool arguments.

        Returns:
            Tool execution result.

        Raises:
            MCPProtocolError: If tool call fails.
            ValueError: If tool not found.
        """
        ...

    @abstractmethod
    async def list_tools(self) -> list[ToolDefinition]:
        """List tools provided by the server.

        Returns:
            List of available tools.

        Raises:
            MCPProtocolError: If listing fails.
        """
        ...

    @abstractmethod
    async def health_check(self) -> bool:
        """Check server health.

        Returns:
            True if server is healthy.
        """
        ...

    async def get_cached_tools(self) -> list[ToolDefinition]:
        """Get cached tool definitions.

        Returns cached tools without making a request to the server.
        Use list_tools() to refresh the cache.

        Returns:
            Cached list of tools.
        """
        return self._tools.copy()

    async def refresh_tools(self) -> list[ToolDefinition]:
        """Refresh and return tool definitions.

        Forces a refresh of the tool cache from the server.

        Returns:
            Updated list of tools.
        """
        self._tools = await self.list_tools()
        return self._tools.copy()

    async def __aenter__(self) -> BaseMCPServer:
        """Enter async context (connect)."""
        await self.connect()
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: Any,
    ) -> None:
        """Exit async context (disconnect)."""
        await self.disconnect()

    def __repr__(self) -> str:
        """Return string representation."""
        return (
            f"{self.__class__.__name__}("
            f"name={self._name!r}, "
            f"state={self._state.value})"
        )


def format_mcp_message(message: str) -> bytes:
    """Format a message for MCP wire protocol.

    MCP uses HTTP-style framing with Content-Length header.

    Args:
        message: JSON message to format.

    Returns:
        Formatted message bytes.
    """
    encoded = message.encode("utf-8")
    header = f"Content-Length: {len(encoded)}\r\n\r\n"
    return header.encode("utf-8") + encoded


async def parse_mcp_message(reader: asyncio.StreamReader) -> str:
    """Parse a message from MCP wire protocol.

    Reads Content-Length header and message body.

    Args:
        reader: Async stream reader.

    Returns:
        Parsed JSON message string.

    Raises:
        MCPProtocolError: If parsing fails.
    """
    # Read headers until empty line
    content_length = None

    while True:
        line = await reader.readline()
        if not line:
            raise MCPProtocolError("Connection closed while reading headers")

        line_str = line.decode("utf-8").strip()

        if not line_str:  # Empty line marks end of headers
            break

        if line_str.lower().startswith("content-length:"):
            try:
                content_length = int(line_str.split(":", 1)[1].strip())
            except ValueError as e:
                raise MCPProtocolError(
                    f"Invalid Content-Length header: {line_str}"
                ) from e

    if content_length is None:
        raise MCPProtocolError("Missing Content-Length header")

    # Read body
    body = await reader.readexactly(content_length)
    return body.decode("utf-8")


__all__ = [
    "BaseMCPServer",
    "MCPNotification",
    "MCPProtocolError",
    "MCPRequest",
    "MCPResponse",
    "ServerState",
    "format_mcp_message",
    "parse_mcp_message",
]
