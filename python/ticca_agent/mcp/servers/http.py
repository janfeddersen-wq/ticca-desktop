"""HTTP-based MCP server implementation.

This module provides an MCP server that communicates via streamable HTTP.
This transport is suitable for REST-style MCP servers where each request
gets a direct response.

HTTP transport provides:
- Simple request/response model
- Standard HTTP semantics
- Easy integration with existing infrastructure

Example:
    >>> from ticca_agent.mcp.servers.http import HTTPMCPServer
    >>> server = HTTPMCPServer(
    ...     name="rest-tools",
    ...     url="https://api.example.com/mcp",
    ...     headers={"Authorization": "Bearer token"},
    ... )
    >>> async with server:
    ...     tools = await server.list_tools()

"""

from __future__ import annotations

import asyncio
from typing import Any

import httpx
import structlog

from ticca_agent.mcp.models import HTTPServerConfig, ToolDefinition
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


class HTTPMCPServer(BaseMCPServer):
    """MCP server using HTTP communication.

    Communicates with MCP servers via standard HTTP requests.
    Each MCP request is sent as an HTTP POST, and the response
    is returned directly in the HTTP response body.

    This is the simplest transport, suitable for stateless
    MCP servers or those behind load balancers.

    Attributes:
        url: Base URL of the HTTP server.
        timeout: Request timeout in seconds.
        headers: Additional HTTP headers.

    Example:
        >>> server = HTTPMCPServer(
        ...     name="my-server",
        ...     url="https://api.example.com/mcp",
        ...     timeout=60.0,
        ... )
        >>> await server.connect()
    """

    def __init__(
        self,
        name: str,
        url: str,
        *,
        timeout: float = 30.0,
        headers: dict[str, str] | None = None,
    ) -> None:
        """Initialize the HTTP server.

        Args:
            name: Unique server identifier.
            url: Base URL of the HTTP server.
            timeout: Request timeout in seconds.
            headers: Additional HTTP headers.
        """
        super().__init__(name)
        self._url = url.rstrip("/")
        self._timeout = timeout
        self._headers = headers or {}

        # HTTP client
        self._client: httpx.AsyncClient | None = None
        self._session_id: str | None = None

    @classmethod
    def from_config(
        cls,
        name: str,
        config: HTTPServerConfig,
    ) -> HTTPMCPServer:
        """Create server from configuration.

        Args:
            name: Server name.
            config: HTTP server configuration.

        Returns:
            Configured server instance.
        """
        return cls(
            name=name,
            url=config.url,
            timeout=config.timeout,
            headers=dict(config.headers),
        )

    async def connect(self) -> None:
        """Establish connection to the HTTP server.

        Creates HTTP client and performs MCP initialization.

        Raises:
            MCPProtocolError: If connection fails.
        """
        if self._state == ServerState.RUNNING:
            return

        self._set_state(ServerState.STARTING)

        try:
            await logger.ainfo(
                "Connecting to HTTP MCP server",
                server=self._name,
                url=self._url,
            )

            # Create HTTP client
            self._client = httpx.AsyncClient(
                timeout=httpx.Timeout(self._timeout),
                headers={
                    "Content-Type": "application/json",
                    "Accept": "application/json",
                    **self._headers,
                },
            )

            # Perform MCP initialization
            await self._initialize()

            # Discover tools
            self._tools = await self.list_tools()

            self._set_state(ServerState.RUNNING)

            await logger.ainfo(
                "HTTP MCP server connected",
                server=self._name,
                tool_count=len(self._tools),
            )

        except Exception as e:
            self._set_state(ServerState.ERROR)
            await self._cleanup()
            raise MCPProtocolError(
                f"Failed to connect to HTTP server '{self._name}': {e}"
            ) from e

    async def disconnect(self) -> None:
        """Disconnect from the HTTP server."""
        if self._state in (ServerState.STOPPED, ServerState.STOPPING):
            return

        self._set_state(ServerState.STOPPING)

        try:
            await self._cleanup()
        finally:
            self._set_state(ServerState.STOPPED)

        await logger.ainfo(
            "HTTP MCP server disconnected",
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

        if response.result:
            self._server_info = response.result.get("serverInfo", {})
            self._capabilities = response.result.get("capabilities", {})
            self._session_id = response.result.get("sessionId")

    async def _send_request(self, request: MCPRequest) -> MCPResponse:
        """Send a request via HTTP POST.

        Args:
            request: Request to send.

        Returns:
            Response from server.

        Raises:
            MCPProtocolError: If request fails.
        """
        if not self._client:
            raise MCPProtocolError("Not connected")

        try:
            # Build request body
            body = request.model_dump(exclude_none=True)

            # Add session ID if we have one
            if self._session_id:
                body.setdefault("params", {})
                body["params"]["_sessionId"] = self._session_id

            # Send request
            http_response = await self._client.post(
                self._url,
                json=body,
            )

            # Check HTTP status
            if http_response.status_code >= 400:
                raise MCPProtocolError(
                    f"HTTP error: {http_response.status_code} {http_response.text}"
                )

            # Parse response
            response_data = http_response.json()
            return MCPResponse.model_validate(response_data)

        except httpx.TimeoutException as e:
            raise MCPProtocolError(
                f"Request timed out after {self._timeout}s"
            ) from e

        except httpx.HTTPError as e:
            raise MCPProtocolError(f"HTTP error: {e}") from e

    async def _cleanup(self) -> None:
        """Clean up HTTP client."""
        if self._client:
            await self._client.aclose()
            self._client = None
        self._session_id = None


__all__ = [
    "HTTPMCPServer",
]
