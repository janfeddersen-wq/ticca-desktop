"""MCP server configuration models.

This module defines Pydantic models for configuring different MCP server types:
- SSE (Server-Sent Events) servers for web-based communication
- Stdio servers for CLI/process-based communication
- HTTP servers for REST-style communication

All models include comprehensive validation rules per FEATURES.md §8.3.

Example:
    >>> from ticca_agent.mcp.models import StdioServerConfig, ServerEntry
    >>> config = StdioServerConfig(
    ...     command="npx",
    ...     args=["-y", "@modelcontextprotocol/server-filesystem"],
    ...     timeout=30.0,
    ... )
    >>> entry = ServerEntry(
    ...     name="filesystem",
    ...     server_type="stdio",
    ...     enabled=True,
    ...     config=config.model_dump(),
    ... )

"""

from __future__ import annotations

import re
from datetime import UTC, datetime
from enum import Enum
from pathlib import Path
from typing import Any

from pydantic import (
    BaseModel,
    ConfigDict,
    Field,
    field_validator,
    model_validator,
)

# Regex pattern for valid server names: alphanumeric + hyphens/underscores
SERVER_NAME_PATTERN = re.compile(r"^[a-zA-Z][a-zA-Z0-9_-]*$")

# Allowlist of commands that can be executed by stdio MCP servers
# This prevents arbitrary command execution through MCP configuration
ALLOWED_COMMANDS: frozenset[str] = frozenset({
    "npx",
    "node",
    "python",
    "python3",
    "uvx",
    "pip",
    "cargo",
    "deno",
    "bun",
})


class ServerType(str, Enum):
    """Supported MCP server communication types.

    Defines the transport protocol used to communicate with the server.
    """

    SSE = "sse"
    """HTTP with Server-Sent Events for streaming responses."""

    STDIO = "stdio"
    """Standard input/output for process-based communication."""

    HTTP = "http"
    """Streamable HTTP for REST-style communication."""


class SSEServerConfig(BaseModel):
    """Configuration for an SSE (Server-Sent Events) MCP server.

    SSE servers communicate over HTTP with server-sent events for
    streaming responses. Ideal for web-based MCP servers.

    Attributes:
        url: Base URL of the SSE server (must start with http:// or https://).
        timeout: Connection timeout in seconds.
        read_timeout: Read timeout for SSE events in seconds.
        headers: Additional HTTP headers for requests.

    Example:
        >>> config = SSEServerConfig(
        ...     url="https://mcp.example.com/events",
        ...     timeout=30.0,
        ...     headers={"Authorization": "Bearer token"},
        ... )
    """

    url: str = Field(
        description="Base URL of the SSE server",
    )
    timeout: float = Field(
        default=30.0,
        gt=0.0,
        le=300.0,
        description="Connection timeout in seconds",
    )
    read_timeout: float = Field(
        default=10.0,
        gt=0.0,
        le=300.0,
        description="Read timeout for SSE events in seconds",
    )
    headers: dict[str, str] = Field(
        default_factory=dict,
        description="Additional HTTP headers for requests",
    )

    model_config = ConfigDict(frozen=True, extra="forbid")

    @field_validator("url")
    @classmethod
    def validate_url(cls, v: str) -> str:
        """Validate URL starts with http:// or https://."""
        if not v.startswith(("http://", "https://")):
            raise ValueError("URL must start with http:// or https://")
        return v.rstrip("/")


class StdioServerConfig(BaseModel):
    """Configuration for a Stdio MCP server.

    Stdio servers communicate via stdin/stdout with a spawned process.
    This is the most common transport for local MCP tools.

    Attributes:
        command: Command to execute (required).
        args: Additional command arguments.
        env: Environment variables for the process.
        cwd: Working directory for the process.
        timeout: Operation timeout in seconds.

    Example:
        >>> config = StdioServerConfig(
        ...     command="npx",
        ...     args=["-y", "@modelcontextprotocol/server-filesystem"],
        ...     env={"DEBUG": "true"},
        ...     cwd="/home/user/project",
        ... )
    """

    command: str = Field(
        min_length=1,
        description="Command to execute",
    )
    args: list[str] = Field(
        default_factory=list,
        description="Additional command arguments",
    )
    env: dict[str, str] = Field(
        default_factory=dict,
        description="Environment variables for the process",
    )
    cwd: str | None = Field(
        default=None,
        description="Working directory for the process",
    )
    timeout: float = Field(
        default=30.0,
        gt=0.0,
        le=300.0,
        description="Operation timeout in seconds",
    )

    model_config = ConfigDict(frozen=True, extra="forbid")

    @field_validator("command")
    @classmethod
    def validate_command(cls, v: str) -> str:
        """Validate command is not empty and is in the allowlist.

        Security: Only allows known-safe commands to prevent arbitrary
        command execution through MCP server configuration.
        """
        if not v.strip():
            raise ValueError("Command cannot be empty or whitespace")

        # Extract base command name (handle full paths)
        base_cmd = Path(v).name

        if base_cmd not in ALLOWED_COMMANDS:
            raise ValueError(
                f"Command '{base_cmd}' is not in the allowed list. "
                f"Allowed commands: {', '.join(sorted(ALLOWED_COMMANDS))}"
            )
        return v


class HTTPServerConfig(BaseModel):
    """Configuration for an HTTP MCP server.

    HTTP servers communicate via streamable HTTP requests.
    Suitable for REST-style MCP servers.

    Attributes:
        url: Base URL of the HTTP server (must start with http:// or https://).
        timeout: Request timeout in seconds.
        headers: Additional HTTP headers for requests.

    Example:
        >>> config = HTTPServerConfig(
        ...     url="https://api.example.com/mcp",
        ...     timeout=60.0,
        ...     headers={"X-API-Key": "secret"},
        ... )
    """

    url: str = Field(
        description="Base URL of the HTTP server",
    )
    timeout: float = Field(
        default=30.0,
        gt=0.0,
        le=300.0,
        description="Request timeout in seconds",
    )
    headers: dict[str, str] = Field(
        default_factory=dict,
        description="Additional HTTP headers for requests",
    )

    model_config = ConfigDict(frozen=True, extra="forbid")

    @field_validator("url")
    @classmethod
    def validate_url(cls, v: str) -> str:
        """Validate URL starts with http:// or https://."""
        if not v.startswith(("http://", "https://")):
            raise ValueError("URL must start with http:// or https://")
        return v.rstrip("/")


# Type alias for server configuration union
ServerConfigUnion = SSEServerConfig | StdioServerConfig | HTTPServerConfig


class ServerEntry(BaseModel):
    """A server entry in the configuration file.

    Represents a complete server configuration including identity,
    type, enabled status, and type-specific configuration.

    Attributes:
        name: Unique server identifier (alphanumeric + hyphens/underscores).
        server_type: Type of server (sse, stdio, or http).
        enabled: Whether the server is enabled.
        config: Type-specific configuration dictionary.

    Example:
        >>> entry = ServerEntry(
        ...     name="my-filesystem-server",
        ...     server_type="stdio",
        ...     enabled=True,
        ...     config={
        ...         "command": "npx",
        ...         "args": ["-y", "@modelcontextprotocol/server-filesystem"],
        ...     },
        ... )
    """

    name: str = Field(
        min_length=1,
        max_length=64,
        description="Unique server identifier",
    )
    server_type: ServerType = Field(
        description="Type of server communication",
    )
    enabled: bool = Field(
        default=True,
        description="Whether the server is enabled",
    )
    config: dict[str, Any] = Field(
        default_factory=dict,
        description="Type-specific configuration",
    )

    model_config = ConfigDict(frozen=True, extra="forbid")

    @field_validator("name")
    @classmethod
    def validate_name(cls, v: str) -> str:
        """Validate server name matches required pattern."""
        if not SERVER_NAME_PATTERN.match(v):
            raise ValueError(
                "Server name must start with a letter and contain only "
                "alphanumeric characters, hyphens, or underscores"
            )
        return v

    @model_validator(mode="after")
    def validate_config_for_type(self) -> ServerEntry:
        """Validate configuration matches server type."""
        # Validate config structure based on server type
        try:
            if self.server_type == ServerType.SSE:
                SSEServerConfig(**self.config)
            elif self.server_type == ServerType.STDIO:
                StdioServerConfig(**self.config)
            elif self.server_type == ServerType.HTTP:
                HTTPServerConfig(**self.config)
        except Exception as e:
            raise ValueError(
                f"Invalid config for server type '{self.server_type}': {e}"
            ) from e
        return self

    def get_typed_config(self) -> ServerConfigUnion:
        """Get the configuration as the appropriate typed model.

        Returns:
            The configuration as the correct typed Pydantic model.

        Raises:
            ValueError: If server type is unknown.
        """
        if self.server_type == ServerType.SSE:
            return SSEServerConfig(**self.config)
        elif self.server_type == ServerType.STDIO:
            return StdioServerConfig(**self.config)
        elif self.server_type == ServerType.HTTP:
            return HTTPServerConfig(**self.config)
        else:
            raise ValueError(f"Unknown server type: {self.server_type}")


class ToolDefinition(BaseModel):
    """Definition of a tool provided by an MCP server.

    Represents the metadata about a tool that can be called via MCP.

    Attributes:
        name: Tool name.
        description: Human-readable description.
        input_schema: JSON Schema for tool input parameters.
        server_name: Name of the server providing this tool.
    """

    name: str = Field(
        min_length=1,
        description="Tool name",
    )
    description: str = Field(
        default="",
        description="Human-readable description",
    )
    input_schema: dict[str, Any] = Field(
        default_factory=dict,
        description="JSON Schema for tool input parameters",
    )
    server_name: str = Field(
        description="Name of the server providing this tool",
    )

    model_config = ConfigDict(frozen=True)


class ServerStatus(BaseModel):
    """Current status of an MCP server.

    Provides a snapshot of a server's current state including
    health information and tool availability.

    Attributes:
        name: Server identifier.
        server_type: Type of server.
        state: Current server state.
        enabled: Whether the server is enabled in config.
        tool_count: Number of tools available.
        last_health_check: Timestamp of last health check.
        last_error: Last error message if any.
        consecutive_failures: Number of consecutive failures.
        uptime_seconds: Time since successful connection.
    """

    name: str = Field(description="Server identifier")
    server_type: ServerType = Field(description="Type of server")
    state: str = Field(description="Current server state")
    enabled: bool = Field(description="Whether the server is enabled")
    tool_count: int = Field(
        default=0,
        ge=0,
        description="Number of tools available",
    )
    last_health_check: datetime | None = Field(
        default=None,
        description="Timestamp of last health check",
    )
    last_error: str | None = Field(
        default=None,
        description="Last error message if any",
    )
    consecutive_failures: int = Field(
        default=0,
        ge=0,
        description="Number of consecutive failures",
    )
    uptime_seconds: float = Field(
        default=0.0,
        ge=0.0,
        description="Time since successful connection",
    )

    model_config = ConfigDict(frozen=True)


class HealthStatus(BaseModel):
    """Result of a health check on an MCP server.

    Attributes:
        healthy: Whether the server is healthy.
        latency_ms: Response latency in milliseconds.
        error: Error message if unhealthy.
        checked_at: Timestamp of the check.
    """

    healthy: bool = Field(description="Whether the server is healthy")
    latency_ms: float = Field(
        default=0.0,
        ge=0.0,
        description="Response latency in milliseconds",
    )
    error: str | None = Field(
        default=None,
        description="Error message if unhealthy",
    )
    checked_at: datetime = Field(
        default_factory=lambda: datetime.now(UTC),
        description="Timestamp of the check",
    )

    model_config = ConfigDict(frozen=True)


class ServerEvent(BaseModel):
    """An event in the server lifecycle audit trail.

    Attributes:
        server_name: Name of the server.
        event_type: Type of event (started, stopped, error, etc.).
        message: Human-readable event message.
        timestamp: When the event occurred.
        details: Additional event details.
    """

    server_name: str = Field(description="Name of the server")
    event_type: str = Field(description="Type of event")
    message: str = Field(description="Human-readable event message")
    timestamp: datetime = Field(
        default_factory=lambda: datetime.now(UTC),
        description="When the event occurred",
    )
    details: dict[str, Any] = Field(
        default_factory=dict,
        description="Additional event details",
    )

    model_config = ConfigDict(frozen=True)


__all__ = [
    "ALLOWED_COMMANDS",
    "SERVER_NAME_PATTERN",
    "HTTPServerConfig",
    "HealthStatus",
    "SSEServerConfig",
    "ServerConfigUnion",
    "ServerEntry",
    "ServerEvent",
    "ServerStatus",
    "ServerType",
    "StdioServerConfig",
    "ToolDefinition",
]
