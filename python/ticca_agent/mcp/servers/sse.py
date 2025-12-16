"""SSE-based MCP server implementation.

This module provides an MCP server that communicates via Server-Sent Events
(SSE) over HTTP. This transport is ideal for web-based MCP servers that
need to push events to clients.

SSE provides:
- Unidirectional server-to-client event streaming
- Automatic reconnection on connection loss
- Event ID tracking for resumption

Example:
    >>> from ticca_agent.mcp.servers.sse import SSEMCPServer
    >>> server = SSEMCPServer(
    ...     name="web-tools",
    ...     url="https://mcp.example.com/events",
    ...     headers={"Authorization": "Bearer token"},
    ... )
    >>> async with server:
    ...     tools = await server.list_tools()

"""

from __future__ import annotations

import asyncio
import contextlib
import json
from typing import Any

import httpx
import structlog

from ticca_agent.mcp.models import SSEServerConfig, ToolDefinition
from ticca_agent.mcp.servers.base import (
    BaseMCPServer,
    MCPProtocolError,
    MCPRequest,
    MCPResponse,
    ServerState,
)

logger = structlog.get_logger(__name__)

# MCP protocol constants
MCP_PROTOCOL_VERSION = "2024-11-05"
MCP_CLIENT_INFO = {
    "name": "ticca-agent",
    "version": "0.1.0",
}


class SSEMCPServer(BaseMCPServer):
    """MCP server using Server-Sent Events communication.

    Communicates with remote MCP servers over HTTP with SSE for
    server-to-client streaming. Requests are sent via POST,
    responses and notifications come via SSE stream.

    The server handles:
    - HTTP client lifecycle
    - SSE event stream processing
    - MCP initialization handshake
    - Tool discovery and invocation
    - Automatic reconnection

    Attributes:
        url: Base URL of the SSE server.
        timeout: Connection timeout in seconds.
        read_timeout: SSE read timeout in seconds.
        headers: Additional HTTP headers.

    Example:
        >>> server = SSEMCPServer(
        ...     name="my-server",
        ...     url="https://api.example.com/mcp",
        ...     headers={"X-API-Key": "secret"},
        ... )
        >>> await server.connect()
    """

    def __init__(
        self,
        name: str,
        url: str,
        *,
        timeout: float = 30.0,
        read_timeout: float = 10.0,
        headers: dict[str, str] | None = None,
    ) -> None:
        """Initialize the SSE server.

        Args:
            name: Unique server identifier.
            url: Base URL of the SSE server.
            timeout: Connection timeout in seconds.
            read_timeout: SSE read timeout in seconds.
            headers: Additional HTTP headers.
        """
        super().__init__(name)
        self._url = url.rstrip("/")
        self._timeout = timeout
        self._read_timeout = read_timeout
        self._headers = headers or {}

        # HTTP client
        self._client: httpx.AsyncClient | None = None
        self._sse_task: asyncio.Task[None] | None = None
        self._pending_requests: dict[int | str, asyncio.Future[MCPResponse]] = {}

        # SSE endpoint (discovered during initialization)
        self._sse_endpoint: str | None = None
        self._messages_endpoint: str | None = None

    @classmethod
    def from_config(
        cls,
        name: str,
        config: SSEServerConfig,
    ) -> SSEMCPServer:
        """Create server from configuration.

        Args:
            name: Server name.
            config: SSE server configuration.

        Returns:
            Configured server instance.
        """
        return cls(
            name=name,
            url=config.url,
            timeout=config.timeout,
            read_timeout=config.read_timeout,
            headers=dict(config.headers),
        )

    async def connect(self) -> None:
        """Establish connection to the SSE server.

        Creates HTTP client, connects to SSE stream,
        and performs MCP initialization.

        Raises:
            MCPProtocolError: If connection fails.
        """
        if self._state == ServerState.RUNNING:
            return

        self._set_state(ServerState.STARTING)

        try:
            await logger.ainfo(
                "Connecting to SSE MCP server",
                server=self._name,
                url=self._url,
            )

            # Create HTTP client
            self._client = httpx.AsyncClient(
                timeout=httpx.Timeout(
                    connect=self._timeout,
                    read=self._read_timeout,
                    write=self._timeout,
                    pool=self._timeout,
                ),
                headers={
                    "Accept": "text/event-stream",
                    "Content-Type": "application/json",
                    **self._headers,
                },
            )

            # Discover endpoints
            await self._discover_endpoints()

            # Start SSE reader
            if self._sse_endpoint:
                self._sse_task = asyncio.create_task(
                    self._read_sse_events(),
                    name=f"sse-reader-{self._name}",
                )

            # Perform MCP initialization
            await self._initialize()

            # Discover tools
            self._tools = await self.list_tools()

            self._set_state(ServerState.RUNNING)

            await logger.ainfo(
                "SSE MCP server connected",
                server=self._name,
                tool_count=len(self._tools),
            )

        except Exception as e:
            self._set_state(ServerState.ERROR)
            await self._cleanup()
            raise MCPProtocolError(
                f"Failed to connect to SSE server '{self._name}': {e}"
            ) from e

    async def disconnect(self) -> None:
        """Disconnect from the SSE server."""
        if self._state in (ServerState.STOPPED, ServerState.STOPPING):
            return

        self._set_state(ServerState.STOPPING)

        try:
            await self._cleanup()
        finally:
            self._set_state(ServerState.STOPPED)

        await logger.ainfo(
            "SSE MCP server disconnected",
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
        """
        if not self._client:
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
        """Check server health.

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

    async def _discover_endpoints(self) -> None:
        """Discover MCP endpoints from server."""
        if not self._client:
            return

        # Try to get endpoint info from well-known location
        try:
            response = await self._client.get(f"{self._url}/.well-known/mcp")
            if response.status_code == 200:
                data = response.json()
                self._sse_endpoint = data.get("sse_endpoint", f"{self._url}/sse")
                self._messages_endpoint = data.get(
                    "messages_endpoint", f"{self._url}/messages"
                )
                return
        except Exception:
            pass

        # Default endpoints
        self._sse_endpoint = f"{self._url}/sse"
        self._messages_endpoint = f"{self._url}/messages"

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

        if response.result:
            self._server_info = response.result.get("serverInfo", {})
            self._capabilities = response.result.get("capabilities", {})

    async def _send_request(self, request: MCPRequest) -> MCPResponse:
        """Send a request and wait for response.

        Args:
            request: Request to send.

        Returns:
            Response from server.
        """
        if not self._client or not self._messages_endpoint:
            raise MCPProtocolError("Not connected")

        # Create future for response
        future: asyncio.Future[MCPResponse] = asyncio.get_event_loop().create_future()
        self._pending_requests[request.id] = future

        try:
            # Send request via POST
            http_response = await self._client.post(
                self._messages_endpoint,
                json=request.model_dump(exclude_none=True),
            )

            if http_response.status_code != 200:
                raise MCPProtocolError(
                    f"HTTP error: {http_response.status_code}"
                )

            # For synchronous endpoints, response is in HTTP body
            # For SSE, we wait for the future
            response_data = http_response.json()

            if "id" in response_data:
                # Synchronous response
                return MCPResponse.model_validate(response_data)

            # Wait for SSE response
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

    async def _read_sse_events(self) -> None:
        """Background task to read SSE events."""
        if not self._client or not self._sse_endpoint:
            return

        try:
            async with self._client.stream(
                "GET",
                self._sse_endpoint,
            ) as response:
                event_data = ""
                event_type = "message"

                async for line in response.aiter_lines():
                    if line.startswith("event:"):
                        event_type = line[6:].strip()
                    elif line.startswith("data:"):
                        event_data = line[5:].strip()
                    elif not line and event_data:  # Empty line marks end of event
                        await self._handle_sse_event(event_type, event_data)
                        event_data = ""
                        event_type = "message"

        except asyncio.CancelledError:
            pass
        except Exception as e:
            await logger.aerror(
                "SSE stream error",
                server=self._name,
                error=str(e),
            )

    async def _handle_sse_event(self, event_type: str, data: str) -> None:  # noqa: ARG002
        """Handle an SSE event.

        Args:
            event_type: Type of event.
            data: Event data (JSON string).
        """
        try:
            message_data = json.loads(data)

            # Check if response
            if "id" in message_data and message_data["id"] is not None:
                response = MCPResponse.model_validate(message_data)
                if response.id is not None:
                    future = self._pending_requests.get(response.id)
                    if future and not future.done():
                        future.set_result(response)

        except json.JSONDecodeError as e:
            await logger.awarning(
                "Failed to parse SSE event",
                server=self._name,
                error=str(e),
            )

    async def _cleanup(self) -> None:
        """Clean up HTTP client and tasks."""
        # Cancel SSE task
        if self._sse_task:
            self._sse_task.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await self._sse_task
            self._sse_task = None

        # Cancel pending requests
        for future in self._pending_requests.values():
            if not future.done():
                future.set_exception(
                    MCPProtocolError("Server disconnected")
                )
        self._pending_requests.clear()

        # Close HTTP client
        if self._client:
            await self._client.aclose()
            self._client = None


__all__ = [
    "SSEMCPServer",
]
