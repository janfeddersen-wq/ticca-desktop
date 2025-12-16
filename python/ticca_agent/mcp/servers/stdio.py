"""Stdio-based MCP server implementation.

This module provides an MCP server that communicates via stdin/stdout
with a spawned subprocess. This is the most common transport for local
MCP tools like filesystem access, code execution, etc.

The server spawns a subprocess and communicates using JSON-RPC 2.0
messages over stdin/stdout with Content-Length framing.

Example:
    >>> from ticca_agent.mcp.servers.stdio import StdioMCPServer
    >>> server = StdioMCPServer(
    ...     name="filesystem",
    ...     command="npx",
    ...     args=["-y", "@modelcontextprotocol/server-filesystem"],
    ... )
    >>> async with server:
    ...     tools = await server.list_tools()
    ...     result = await server.call_tool("read_file", {"path": "/etc/hosts"})

"""

from __future__ import annotations

import asyncio
import contextlib
import json
import os
from typing import Any

import structlog

from ticca_agent.mcp.models import StdioServerConfig, ToolDefinition
from ticca_agent.mcp.servers.base import (
    BaseMCPServer,
    MCPNotification,
    MCPProtocolError,
    MCPRequest,
    MCPResponse,
    ServerState,
    format_mcp_message,
    parse_mcp_message,
)

logger = structlog.get_logger(__name__)

# MCP protocol constants
MCP_PROTOCOL_VERSION = "2024-11-05"
MCP_CLIENT_INFO = {
    "name": "ticca-agent",
    "version": "0.1.0",
}


class StdioMCPServer(BaseMCPServer):
    """MCP server using stdin/stdout communication.

    Spawns a subprocess and communicates via JSON-RPC over stdio.
    This is the standard transport for local MCP servers.

    The server handles:
    - Process lifecycle (spawn, monitor, terminate)
    - JSON-RPC message framing with Content-Length
    - MCP initialization handshake
    - Tool discovery and invocation
    - Health monitoring via ping

    Attributes:
        command: Command to execute.
        args: Additional command arguments.
        env: Environment variables for the process.
        cwd: Working directory.
        timeout: Operation timeout in seconds.

    Example:
        >>> server = StdioMCPServer(
        ...     name="my-server",
        ...     command="python",
        ...     args=["-m", "my_mcp_server"],
        ...     env={"DEBUG": "1"},
        ... )
        >>> await server.connect()
        >>> tools = await server.list_tools()
    """

    def __init__(
        self,
        name: str,
        command: str,
        *,
        args: list[str] | None = None,
        env: dict[str, str] | None = None,
        cwd: str | None = None,
        timeout: float = 30.0,
    ) -> None:
        """Initialize the stdio server.

        Args:
            name: Unique server identifier.
            command: Command to execute.
            args: Additional command arguments.
            env: Environment variables for the process.
            cwd: Working directory.
            timeout: Operation timeout in seconds.
        """
        super().__init__(name)
        self._command = command
        self._args = args or []
        self._env = env or {}
        self._cwd = cwd
        self._timeout = timeout

        # Process management
        self._process: asyncio.subprocess.Process | None = None
        self._reader_task: asyncio.Task[None] | None = None
        self._pending_requests: dict[int | str, asyncio.Future[MCPResponse]] = {}

    @classmethod
    def from_config(
        cls,
        name: str,
        config: StdioServerConfig,
    ) -> StdioMCPServer:
        """Create server from configuration.

        Args:
            name: Server name.
            config: Stdio server configuration.

        Returns:
            Configured server instance.
        """
        return cls(
            name=name,
            command=config.command,
            args=list(config.args),
            env=dict(config.env),
            cwd=config.cwd,
            timeout=config.timeout,
        )

    async def connect(self) -> None:
        """Start the server process and establish connection.

        Spawns the subprocess, performs MCP initialization handshake,
        and discovers available tools.

        Raises:
            MCPProtocolError: If connection or initialization fails.
        """
        if self._state == ServerState.RUNNING:
            return

        self._set_state(ServerState.STARTING)

        try:
            # Build environment
            process_env = os.environ.copy()
            process_env.update(self._env)

            # Build command
            cmd = [self._command, *self._args]

            await logger.ainfo(
                "Starting stdio MCP server",
                server=self._name,
                command=cmd,
            )

            # Start subprocess
            self._process = await asyncio.create_subprocess_exec(
                *cmd,
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
                env=process_env,
                cwd=self._cwd,
            )

            # Start message reader task
            self._reader_task = asyncio.create_task(
                self._read_messages(),
                name=f"mcp-reader-{self._name}",
            )

            # Perform MCP initialization
            await self._initialize()

            # Discover tools
            self._tools = await self.list_tools()

            self._set_state(ServerState.RUNNING)

            await logger.ainfo(
                "Stdio MCP server connected",
                server=self._name,
                tool_count=len(self._tools),
            )

        except Exception as e:
            self._set_state(ServerState.ERROR)
            await self._cleanup()
            raise MCPProtocolError(
                f"Failed to connect to server '{self._name}': {e}"
            ) from e

    async def disconnect(self) -> None:
        """Stop the server process.

        Sends shutdown notification, terminates the process,
        and cleans up resources.
        """
        if self._state in (ServerState.STOPPED, ServerState.STOPPING):
            return

        self._set_state(ServerState.STOPPING)

        try:
            # Send shutdown notification if connected
            if self._process and self._process.stdin:
                try:
                    notification = MCPNotification(
                        method="notifications/shutdown",
                    )
                    await self._send_notification(notification)
                except Exception:
                    pass  # Best effort

        finally:
            await self._cleanup()
            self._set_state(ServerState.STOPPED)

        await logger.ainfo(
            "Stdio MCP server disconnected",
            server=self._name,
        )

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
            ValueError: If not connected.
        """
        if not self.is_connected:
            raise ValueError(f"Server '{self._name}' is not connected")

        self._update_activity()
        self._total_requests += 1

        request = MCPRequest(
            id=self._next_request_id(),
            method="tools/call",
            params={
                "name": name,
                "arguments": arguments,
            },
        )

        try:
            response = await self._send_request(request)
            response.raise_for_error()

            # Extract content from result
            result = response.result
            if isinstance(result, dict) and "content" in result:
                content = result["content"]
                if isinstance(content, list) and len(content) > 0:
                    # Return first content item's text
                    item = content[0]
                    if isinstance(item, dict) and "text" in item:
                        return item["text"]
                return content
            return result

        except Exception:
            self._total_errors += 1
            raise

    async def list_tools(self) -> list[ToolDefinition]:
        """List tools provided by the server.

        Returns:
            List of available tools.

        Raises:
            MCPProtocolError: If listing fails.
            ValueError: If not connected.
        """
        if not self._process:
            raise ValueError(f"Server '{self._name}' is not connected")

        request = MCPRequest(
            id=self._next_request_id(),
            method="tools/list",
        )

        response = await self._send_request(request)
        response.raise_for_error()

        tools = []
        if response.result and "tools" in response.result:
            for tool_data in response.result["tools"]:
                tools.append(
                    ToolDefinition(
                        name=tool_data.get("name", ""),
                        description=tool_data.get("description", ""),
                        input_schema=tool_data.get("inputSchema", {}),
                        server_name=self._name,
                    )
                )

        return tools

    async def health_check(self) -> bool:
        """Check server health via ping.

        Returns:
            True if server responds to ping.
        """
        if not self.is_connected:
            return False

        try:
            request = MCPRequest(
                id=self._next_request_id(),
                method="ping",
            )

            response = await asyncio.wait_for(
                self._send_request(request),
                timeout=5.0,
            )

            return not response.is_error

        except Exception:
            return False

    async def _initialize(self) -> None:
        """Perform MCP initialization handshake."""
        request = MCPRequest(
            id=self._next_request_id(),
            method="initialize",
            params={
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {
                    "tools": {},
                },
                "clientInfo": MCP_CLIENT_INFO,
            },
        )

        response = await self._send_request(request)
        response.raise_for_error()

        # Store server info and capabilities
        if response.result:
            self._server_info = response.result.get("serverInfo", {})
            self._capabilities = response.result.get("capabilities", {})

        # Send initialized notification
        notification = MCPNotification(
            method="notifications/initialized",
        )
        await self._send_notification(notification)

    async def _send_request(self, request: MCPRequest) -> MCPResponse:
        """Send a request and wait for response.

        Args:
            request: Request to send.

        Returns:
            Response from server.

        Raises:
            MCPProtocolError: If send fails or times out.
        """
        if not self._process or not self._process.stdin:
            raise MCPProtocolError("Process not running")

        # Create future for response
        future: asyncio.Future[MCPResponse] = asyncio.get_event_loop().create_future()
        self._pending_requests[request.id] = future

        try:
            # Send request
            message = format_mcp_message(request.to_json())
            self._process.stdin.write(message)
            await self._process.stdin.drain()

            # Wait for response with timeout
            response = await asyncio.wait_for(
                future,
                timeout=self._timeout,
            )

            return response

        except TimeoutError as e:
            raise MCPProtocolError(
                f"Request timed out after {self._timeout}s"
            ) from e

        finally:
            self._pending_requests.pop(request.id, None)

    async def _send_notification(self, notification: MCPNotification) -> None:
        """Send a notification (no response expected).

        Args:
            notification: Notification to send.
        """
        if not self._process or not self._process.stdin:
            return

        message = format_mcp_message(notification.to_json())
        self._process.stdin.write(message)
        await self._process.stdin.drain()

    async def _read_messages(self) -> None:
        """Background task to read messages from server."""
        if not self._process or not self._process.stdout:
            return

        reader = self._process.stdout

        try:
            while True:
                try:
                    message_str = await parse_mcp_message(reader)
                    message_data = json.loads(message_str)

                    # Check if this is a response (has id)
                    if "id" in message_data and message_data["id"] is not None:
                        response = MCPResponse.model_validate(message_data)
                        if response.id is not None:
                            future = self._pending_requests.get(response.id)
                            if future and not future.done():
                                future.set_result(response)
                    else:
                        # This is a notification from server
                        await self._handle_notification(message_data)

                except json.JSONDecodeError as e:
                    await logger.awarning(
                        "Failed to parse MCP message",
                        server=self._name,
                        error=str(e),
                    )

        except asyncio.CancelledError:
            pass
        except Exception as e:
            await logger.aerror(
                "Error reading MCP messages",
                server=self._name,
                error=str(e),
            )
            # Cancel any pending requests
            for future in self._pending_requests.values():
                if not future.done():
                    future.set_exception(MCPProtocolError(f"Reader stopped: {e}"))

    async def _handle_notification(self, data: dict[str, Any]) -> None:
        """Handle a notification from the server.

        Args:
            data: Notification data.
        """
        method = data.get("method", "")

        await logger.adebug(
            "Received MCP notification",
            server=self._name,
            method=method,
        )

        # Handle specific notifications if needed
        if method == "notifications/tools/list_changed":
            # Tools have changed, refresh cache
            try:
                self._tools = await self.list_tools()
            except Exception as e:
                await logger.awarning(
                    "Failed to refresh tools after change notification",
                    server=self._name,
                    error=str(e),
                )

    async def _cleanup(self) -> None:
        """Clean up process and resources."""
        # Cancel reader task
        if self._reader_task:
            self._reader_task.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await self._reader_task
            self._reader_task = None

        # Cancel pending requests
        for future in self._pending_requests.values():
            if not future.done():
                future.set_exception(
                    MCPProtocolError("Server disconnected")
                )
        self._pending_requests.clear()

        # Terminate process
        if self._process:
            try:
                self._process.terminate()
                try:
                    await asyncio.wait_for(
                        self._process.wait(),
                        timeout=5.0,
                    )
                except TimeoutError:
                    self._process.kill()
                    await self._process.wait()
            except ProcessLookupError:
                pass  # Process already dead
            self._process = None


__all__ = [
    "MCP_CLIENT_INFO",
    "MCP_PROTOCOL_VERSION",
    "StdioMCPServer",
]
