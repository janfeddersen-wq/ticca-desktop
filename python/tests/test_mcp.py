"""Tests for the MCP server integration module.

These tests cover:
- Configuration models and validation
- Circuit breaker pattern
- Health monitoring
- Server lifecycle management
- Tool invocation
"""

from __future__ import annotations

import asyncio
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from pydantic import ValidationError

from ticca_agent.mcp import (
    CircuitBreaker,
    CircuitBreakerOpenError,
    CircuitState,
    HealthMonitor,
    HealthStatus,
    HTTPServerConfig,
    MCPError,
    MCPServer,
    MCPServerManager,
    SSEServerConfig,
    ServerEntry,
    ServerEvent,
    ServerState,
    ServerStatus,
    ServerType,
    StdioServerConfig,
    ToolDefinition,
    create_server,
)


# =============================================================================
# Configuration Model Tests
# =============================================================================


class TestSSEServerConfig:
    """Tests for SSE server configuration."""

    def test_valid_https_url(self) -> None:
        """Test valid HTTPS URL is accepted."""
        config = SSEServerConfig(url="https://example.com/mcp")
        assert config.url == "https://example.com/mcp"
        assert config.timeout == 30.0
        assert config.read_timeout == 10.0
        assert config.headers == {}

    def test_valid_http_url(self) -> None:
        """Test valid HTTP URL is accepted."""
        config = SSEServerConfig(url="http://localhost:8080")
        assert config.url == "http://localhost:8080"

    def test_trailing_slash_removed(self) -> None:
        """Test trailing slash is removed from URL."""
        config = SSEServerConfig(url="https://example.com/mcp/")
        assert config.url == "https://example.com/mcp"

    def test_invalid_url_rejected(self) -> None:
        """Test invalid URL is rejected."""
        with pytest.raises(ValidationError) as exc_info:
            SSEServerConfig(url="ftp://example.com")
        assert "URL must start with http://" in str(exc_info.value)

    def test_custom_timeout(self) -> None:
        """Test custom timeout values."""
        config = SSEServerConfig(
            url="https://example.com",
            timeout=60.0,
            read_timeout=30.0,
        )
        assert config.timeout == 60.0
        assert config.read_timeout == 30.0

    def test_custom_headers(self) -> None:
        """Test custom headers."""
        headers = {"Authorization": "Bearer token", "X-Custom": "value"}
        config = SSEServerConfig(
            url="https://example.com",
            headers=headers,
        )
        assert config.headers == headers

    def test_timeout_bounds(self) -> None:
        """Test timeout must be positive and bounded."""
        with pytest.raises(ValidationError):
            SSEServerConfig(url="https://example.com", timeout=0)

        with pytest.raises(ValidationError):
            SSEServerConfig(url="https://example.com", timeout=500)


class TestStdioServerConfig:
    """Tests for Stdio server configuration."""

    def test_minimal_config(self) -> None:
        """Test minimal valid configuration."""
        config = StdioServerConfig(command="npx")
        assert config.command == "npx"
        assert config.args == []
        assert config.env == {}
        assert config.cwd is None
        assert config.timeout == 30.0

    def test_full_config(self) -> None:
        """Test full configuration."""
        config = StdioServerConfig(
            command="python",
            args=["-m", "my_server"],
            env={"DEBUG": "1"},
            cwd="/home/user/project",
            timeout=60.0,
        )
        assert config.command == "python"
        assert config.args == ["-m", "my_server"]
        assert config.env == {"DEBUG": "1"}
        assert config.cwd == "/home/user/project"
        assert config.timeout == 60.0

    def test_empty_command_rejected(self) -> None:
        """Test empty command is rejected."""
        with pytest.raises(ValidationError):
            StdioServerConfig(command="")

    def test_whitespace_command_rejected(self) -> None:
        """Test whitespace-only command is rejected."""
        with pytest.raises(ValidationError):
            StdioServerConfig(command="   ")


class TestHTTPServerConfig:
    """Tests for HTTP server configuration."""

    def test_valid_config(self) -> None:
        """Test valid configuration."""
        config = HTTPServerConfig(url="https://api.example.com/mcp")
        assert config.url == "https://api.example.com/mcp"
        assert config.timeout == 30.0
        assert config.headers == {}

    def test_invalid_url_rejected(self) -> None:
        """Test invalid URL is rejected."""
        with pytest.raises(ValidationError):
            HTTPServerConfig(url="ws://example.com")


class TestServerEntry:
    """Tests for server entry configuration."""

    def test_valid_stdio_entry(self) -> None:
        """Test valid stdio server entry."""
        entry = ServerEntry(
            name="my-server",
            server_type=ServerType.STDIO,
            enabled=True,
            config={"command": "npx"},
        )
        assert entry.name == "my-server"
        assert entry.server_type == ServerType.STDIO
        assert entry.enabled is True

    def test_valid_sse_entry(self) -> None:
        """Test valid SSE server entry."""
        entry = ServerEntry(
            name="web_server",
            server_type=ServerType.SSE,
            config={"url": "https://example.com"},
        )
        assert entry.server_type == ServerType.SSE

    def test_invalid_name_rejected(self) -> None:
        """Test invalid server name is rejected."""
        # Name starting with number
        with pytest.raises(ValidationError):
            ServerEntry(
                name="123-server",
                server_type=ServerType.STDIO,
                config={"command": "test"},
            )

        # Name with special characters
        with pytest.raises(ValidationError):
            ServerEntry(
                name="my server!",
                server_type=ServerType.STDIO,
                config={"command": "test"},
            )

    def test_valid_names(self) -> None:
        """Test valid server names."""
        valid_names = [
            "myserver",
            "my-server",
            "my_server",
            "MyServer123",
            "server-1-test",
        ]
        for name in valid_names:
            entry = ServerEntry(
                name=name,
                server_type=ServerType.STDIO,
                config={"command": "test"},
            )
            assert entry.name == name

    def test_config_validation(self) -> None:
        """Test config is validated for server type."""
        # Missing required command for stdio
        with pytest.raises(ValidationError):
            ServerEntry(
                name="server",
                server_type=ServerType.STDIO,
                config={},  # Missing command
            )

        # Missing required url for SSE
        with pytest.raises(ValidationError):
            ServerEntry(
                name="server",
                server_type=ServerType.SSE,
                config={},  # Missing url
            )

    def test_get_typed_config(self) -> None:
        """Test getting typed configuration."""
        entry = ServerEntry(
            name="server",
            server_type=ServerType.STDIO,
            config={"command": "test", "args": ["--flag"]},
        )
        config = entry.get_typed_config()
        assert isinstance(config, StdioServerConfig)
        assert config.command == "test"
        assert config.args == ["--flag"]


class TestToolDefinition:
    """Tests for tool definition model."""

    def test_basic_tool(self) -> None:
        """Test basic tool definition."""
        tool = ToolDefinition(
            name="read_file",
            description="Read a file",
            server_name="filesystem",
        )
        assert tool.name == "read_file"
        assert tool.description == "Read a file"
        assert tool.server_name == "filesystem"
        assert tool.input_schema == {}

    def test_tool_with_schema(self) -> None:
        """Test tool with input schema."""
        schema = {
            "type": "object",
            "properties": {"path": {"type": "string"}},
            "required": ["path"],
        }
        tool = ToolDefinition(
            name="read_file",
            description="Read a file",
            input_schema=schema,
            server_name="filesystem",
        )
        assert tool.input_schema == schema


# =============================================================================
# Circuit Breaker Tests
# =============================================================================


class TestCircuitBreaker:
    """Tests for circuit breaker pattern."""

    @pytest.mark.asyncio
    async def test_initial_state_closed(self) -> None:
        """Test circuit starts in closed state."""
        breaker = CircuitBreaker(name="test")
        assert breaker.state == CircuitState.CLOSED
        assert breaker.is_closed is True
        assert breaker.failure_count == 0

    @pytest.mark.asyncio
    async def test_successful_calls(self) -> None:
        """Test successful calls pass through."""
        breaker = CircuitBreaker(name="test")

        async def success() -> str:
            return "ok"

        result = await breaker.call(success)
        assert result == "ok"
        assert breaker.is_closed is True

    @pytest.mark.asyncio
    async def test_circuit_opens_after_failures(self) -> None:
        """Test circuit opens after threshold failures."""
        breaker = CircuitBreaker(
            name="test",
            failure_threshold=3,
        )

        async def fail() -> None:
            raise RuntimeError("Failed")

        # Fail 3 times
        for _ in range(3):
            with pytest.raises(RuntimeError):
                await breaker.call(fail)

        assert breaker.is_open is True
        assert breaker.failure_count == 3

    @pytest.mark.asyncio
    async def test_open_circuit_rejects_calls(self) -> None:
        """Test open circuit rejects calls."""
        breaker = CircuitBreaker(
            name="test",
            failure_threshold=1,
        )

        async def fail() -> None:
            raise RuntimeError("Failed")

        # Open the circuit
        with pytest.raises(RuntimeError):
            await breaker.call(fail)

        # Next call should be rejected
        with pytest.raises(CircuitBreakerOpenError) as exc_info:
            await breaker.call(fail)

        assert "Circuit is open" in str(exc_info.value)
        assert exc_info.value.circuit_name == "test"

    @pytest.mark.asyncio
    async def test_half_open_after_timeout(self) -> None:
        """Test circuit transitions to half-open after timeout."""
        breaker = CircuitBreaker(
            name="test",
            failure_threshold=1,
            recovery_timeout=0.1,  # 100ms
            success_threshold=1,  # Close after 1 success in half-open
        )

        async def fail() -> None:
            raise RuntimeError("Failed")

        # Open the circuit
        with pytest.raises(RuntimeError):
            await breaker.call(fail)

        assert breaker.is_open is True

        # Wait for recovery timeout
        await asyncio.sleep(0.15)

        # Next call should attempt (half-open)
        async def success() -> str:
            return "ok"

        result = await breaker.call(success)
        assert result == "ok"
        assert breaker.is_closed is True

    @pytest.mark.asyncio
    async def test_half_open_failure_reopens(self) -> None:
        """Test failure in half-open reopens circuit."""
        breaker = CircuitBreaker(
            name="test",
            failure_threshold=1,
            recovery_timeout=0.1,
        )

        async def fail() -> None:
            raise RuntimeError("Failed")

        # Open the circuit
        with pytest.raises(RuntimeError):
            await breaker.call(fail)

        # Wait for recovery timeout
        await asyncio.sleep(0.15)

        # Fail again in half-open
        with pytest.raises(RuntimeError):
            await breaker.call(fail)

        assert breaker.is_open is True

    @pytest.mark.asyncio
    async def test_manual_reset(self) -> None:
        """Test manual circuit reset."""
        breaker = CircuitBreaker(
            name="test",
            failure_threshold=1,
        )

        async def fail() -> None:
            raise RuntimeError("Failed")

        # Open the circuit
        with pytest.raises(RuntimeError):
            await breaker.call(fail)

        assert breaker.is_open is True

        # Manual reset
        await breaker.reset()
        assert breaker.is_closed is True
        assert breaker.failure_count == 0

    @pytest.mark.asyncio
    async def test_statistics(self) -> None:
        """Test circuit breaker statistics."""
        breaker = CircuitBreaker(name="test", failure_threshold=5)

        async def success() -> str:
            return "ok"

        async def fail() -> None:
            raise RuntimeError("Failed")

        # Make some calls
        await breaker.call(success)
        await breaker.call(success)
        with pytest.raises(RuntimeError):
            await breaker.call(fail)

        stats = breaker.statistics
        assert stats["name"] == "test"
        assert stats["total_calls"] == 3
        assert stats["total_failures"] == 1
        assert stats["state"] == "closed"


# =============================================================================
# Health Monitor Tests
# =============================================================================


class TestHealthMonitor:
    """Tests for health monitoring."""

    def test_initial_state(self) -> None:
        """Test monitor initial state."""
        monitor = HealthMonitor()
        assert monitor.is_running is False

    def test_custom_config(self) -> None:
        """Test custom configuration."""
        monitor = HealthMonitor(
            check_interval=60.0,
            failure_threshold=5,
            recovery_interval=120.0,
        )
        assert monitor.config.check_interval == 60.0
        assert monitor.config.failure_threshold == 5
        assert monitor.config.recovery_interval == 120.0

    def test_record_success(self) -> None:
        """Test recording success."""
        monitor = HealthMonitor(failure_threshold=3)
        
        # Record some failures
        monitor.record_failure("server1", "error")
        monitor.record_failure("server1", "error")
        assert monitor.get_consecutive_failures("server1") == 2

        # Record success
        monitor.record_success("server1")
        assert monitor.get_consecutive_failures("server1") == 0

    @pytest.mark.asyncio
    async def test_quarantine_after_failures(self) -> None:
        """Test server is quarantined after failures."""
        monitor = HealthMonitor(failure_threshold=3)

        # Record failures
        for i in range(3):
            monitor.record_failure("server1", f"error {i}")

        # Give asyncio.create_task a chance to run
        await asyncio.sleep(0.01)

        assert monitor.is_quarantined("server1") is True

    def test_not_quarantined_below_threshold(self) -> None:
        """Test server is not quarantined below threshold."""
        monitor = HealthMonitor(failure_threshold=3)

        monitor.record_failure("server1", "error")
        monitor.record_failure("server1", "error")

        assert monitor.is_quarantined("server1") is False

    def test_health_state_tracking(self) -> None:
        """Test health state tracking."""
        monitor = HealthMonitor()

        monitor.record_failure("server1", "test error")
        state = monitor.get_health_state("server1")

        assert state.consecutive_failures == 1
        assert state.last_error == "test error"
        assert state.last_failure is not None
        assert state.total_failures == 1


# =============================================================================
# Server Status Tests
# =============================================================================


class TestServerStatus:
    """Tests for server status model."""

    def test_basic_status(self) -> None:
        """Test basic server status."""
        status = ServerStatus(
            name="test-server",
            server_type=ServerType.STDIO,
            state="running",
            enabled=True,
        )
        assert status.name == "test-server"
        assert status.server_type == ServerType.STDIO
        assert status.state == "running"
        assert status.enabled is True
        assert status.tool_count == 0

    def test_full_status(self) -> None:
        """Test full server status."""
        now = datetime.now(timezone.utc)
        status = ServerStatus(
            name="test-server",
            server_type=ServerType.SSE,
            state="quarantined",
            enabled=True,
            tool_count=5,
            last_health_check=now,
            last_error="Connection timeout",
            consecutive_failures=3,
            uptime_seconds=3600.0,
        )
        assert status.tool_count == 5
        assert status.last_error == "Connection timeout"
        assert status.consecutive_failures == 3
        assert status.uptime_seconds == 3600.0


# =============================================================================
# Server Event Tests
# =============================================================================


class TestServerEvent:
    """Tests for server event model."""

    def test_basic_event(self) -> None:
        """Test basic event creation."""
        event = ServerEvent(
            server_name="test-server",
            event_type="started",
            message="Server started successfully",
        )
        assert event.server_name == "test-server"
        assert event.event_type == "started"
        assert event.message == "Server started successfully"
        assert event.timestamp is not None
        assert event.details == {}

    def test_event_with_details(self) -> None:
        """Test event with details."""
        event = ServerEvent(
            server_name="test-server",
            event_type="error",
            message="Connection failed",
            details={"error_code": 500, "retry_count": 3},
        )
        assert event.details["error_code"] == 500
        assert event.details["retry_count"] == 3


# =============================================================================
# MCPError Tests
# =============================================================================


class TestMCPError:
    """Tests for MCP error class."""

    def test_basic_error(self) -> None:
        """Test basic error."""
        error = MCPError("Something went wrong")
        assert error.message == "Something went wrong"
        assert str(error) == "Something went wrong"

    def test_error_with_server(self) -> None:
        """Test error with server name."""
        error = MCPError("Connection failed", server_name="filesystem")
        assert error.server_name == "filesystem"
        assert "server=filesystem" in str(error)

    def test_error_with_code(self) -> None:
        """Test error with code."""
        error = MCPError("Internal error", code=-32603)
        assert error.code == -32603
        assert "code=-32603" in str(error)

    def test_full_error(self) -> None:
        """Test error with all attributes."""
        error = MCPError(
            "Tool not found",
            server_name="tools",
            code=-32601,
        )
        error_str = str(error)
        assert "server=tools" in error_str
        assert "code=-32601" in error_str
        assert "Tool not found" in error_str


# =============================================================================
# Server Creation Tests
# =============================================================================


class TestCreateServer:
    """Tests for server factory function."""

    def test_create_stdio_server(self) -> None:
        """Test creating stdio server."""
        from ticca_agent.mcp.servers.stdio import StdioMCPServer

        server = create_server(
            name="test",
            server_type="stdio",
            config={"command": "npx"},
        )
        assert isinstance(server, StdioMCPServer)
        assert server.name == "test"

    def test_create_sse_server(self) -> None:
        """Test creating SSE server."""
        from ticca_agent.mcp.servers.sse import SSEMCPServer

        server = create_server(
            name="test",
            server_type="sse",
            config={"url": "https://example.com"},
        )
        assert isinstance(server, SSEMCPServer)

    def test_create_http_server(self) -> None:
        """Test creating HTTP server."""
        from ticca_agent.mcp.servers.http import HTTPMCPServer

        server = create_server(
            name="test",
            server_type="http",
            config={"url": "https://api.example.com"},
        )
        assert isinstance(server, HTTPMCPServer)

    def test_create_with_enum_type(self) -> None:
        """Test creating server with enum type."""
        from ticca_agent.mcp.servers.stdio import StdioMCPServer

        server = create_server(
            name="test",
            server_type=ServerType.STDIO,
            config={"command": "test"},
        )
        assert isinstance(server, StdioMCPServer)

    def test_invalid_config_rejected(self) -> None:
        """Test invalid config is rejected."""
        with pytest.raises(ValidationError):
            create_server(
                name="test",
                server_type="stdio",
                config={},  # Missing command
            )


# =============================================================================
# MCPServer Wrapper Tests
# =============================================================================


class TestMCPServer:
    """Tests for MCPServer wrapper class."""

    def test_initial_state(self) -> None:
        """Test server initial state."""
        entry = ServerEntry(
            name="test",
            server_type=ServerType.STDIO,
            enabled=True,
            config={"command": "test"},
        )
        server = MCPServer(entry)

        assert server.name == "test"
        assert server.server_type == ServerType.STDIO
        assert server.enabled is True
        assert server.state == ServerState.STOPPED
        assert server.is_running is False
        assert server.tool_count == 0

    def test_disabled_server(self) -> None:
        """Test disabled server."""
        entry = ServerEntry(
            name="test",
            server_type=ServerType.STDIO,
            enabled=False,
            config={"command": "test"},
        )
        server = MCPServer(entry)

        assert server.enabled is False

    def test_get_status(self) -> None:
        """Test getting server status."""
        entry = ServerEntry(
            name="test",
            server_type=ServerType.HTTP,
            enabled=True,
            config={"url": "https://example.com"},
        )
        server = MCPServer(entry)

        status = server.get_status()
        assert status.name == "test"
        assert status.server_type == ServerType.HTTP
        assert status.state == "stopped"
        assert status.enabled is True

    def test_custom_circuit_breaker(self) -> None:
        """Test server with custom circuit breaker."""
        entry = ServerEntry(
            name="test",
            server_type=ServerType.STDIO,
            config={"command": "test"},
        )
        breaker = CircuitBreaker(name="custom", failure_threshold=10)
        server = MCPServer(entry, circuit_breaker=breaker)

        assert server._circuit_breaker is breaker


# =============================================================================
# Integration Tests
# =============================================================================


class TestServerLifecycle:
    """Integration tests for server lifecycle."""

    @pytest.mark.asyncio
    async def test_server_repr(self) -> None:
        """Test server string representation."""
        entry = ServerEntry(
            name="test-server",
            server_type=ServerType.STDIO,
            config={"command": "test"},
        )
        server = MCPServer(entry)

        repr_str = repr(server)
        assert "test-server" in repr_str
        assert "stdio" in repr_str
        assert "stopped" in repr_str


# =============================================================================
# Health Status Tests
# =============================================================================


class TestHealthStatus:
    """Tests for health status model."""

    def test_healthy_status(self) -> None:
        """Test healthy status."""
        status = HealthStatus(
            healthy=True,
            latency_ms=50.0,
        )
        assert status.healthy is True
        assert status.latency_ms == 50.0
        assert status.error is None

    def test_unhealthy_status(self) -> None:
        """Test unhealthy status."""
        status = HealthStatus(
            healthy=False,
            latency_ms=0.0,
            error="Connection refused",
        )
        assert status.healthy is False
        assert status.error == "Connection refused"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
