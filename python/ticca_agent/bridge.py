"""Rust-Python bridge interface for Ticca Agent.

This module provides the bridge interface for communication between
the Rust UI layer and the Python agent system via PyO3. It handles:

- Request/response serialization to/from JSON
- Agent execution (both sync and streaming)
- Error handling (never raises to Rust - returns error responses)
- Agent registry and discovery

The BridgeInterface class is the primary entry point called from Rust.
All methods accept JSON strings and return JSON strings to ensure
clean serialization across the FFI boundary.

Example (from Rust via PyO3):
    >>> from ticca_agent.bridge import BridgeInterface
    >>> bridge = BridgeInterface()
    >>> response_json = await bridge.execute('{"session_id": "...", ...}')
    >>> # Parse response_json as AgentResponse

"""

from __future__ import annotations

import asyncio
import json
import traceback
from collections.abc import Callable
from typing import TYPE_CHECKING, Any
from uuid import uuid4

import structlog

from ticca_agent.bridge_types import (
    AgentInfo,
    AgentRequest,
    AgentResponse,
    AgentState,
    DoneChunk,
    ErrorChunk,
    FinishReason,
    FinishReasonError,
    FinishReasonMaxTokens,
    FinishReasonStop,
    FinishReasonToolUse,
    StateChangeChunk,
    StreamChunk,
    TextDeltaChunk,
    ThinkingDeltaChunk,
    TokenUsage,
    ToolCall,
    ToolResultChunk,
    ToolStartChunk,
    UsageChunk,
)

if TYPE_CHECKING:
    from collections.abc import AsyncIterator

    from ticca_agent.core.agent import Agent
    from ticca_agent.providers.base import BaseProvider

logger = structlog.get_logger(__name__)

__all__ = [
    "BridgeInterface",
    "BridgeError",
    # Re-export bridge types for convenience
    "AgentInfo",
    "AgentRequest",
    "AgentResponse",
    "AgentState",
    "StreamChunk",
    "TextDeltaChunk",
    "ThinkingDeltaChunk",
    "ToolStartChunk",
    "ToolResultChunk",
    "StateChangeChunk",
    "UsageChunk",
    "DoneChunk",
    "ErrorChunk",
    "TokenUsage",
    "ToolCall",
]


# =============================================================================
# Agent Registry
# =============================================================================

# Global agent registry - maps agent names to AgentInfo
_agent_registry: dict[str, AgentInfo] = {}

# Global provider instances - maps agent names to provider instances
_provider_registry: dict[str, BaseProvider] = {}


def register_agent(
    agent_info: AgentInfo,
    provider: BaseProvider | None = None,
) -> None:
    """Register an agent in the global registry.

    Args:
        agent_info: Information about the agent to register.
        provider: Optional provider instance for the agent.

    Raises:
        ValueError: If an agent with the same ID is already registered.
    """
    if agent_info.id in _agent_registry:
        raise ValueError(f"Agent '{agent_info.id}' is already registered")

    _agent_registry[agent_info.id] = agent_info
    if provider:
        _provider_registry[agent_info.id] = provider

    logger.info(
        "agent_registered",
        agent_id=agent_info.id,
        agent_name=agent_info.name,
    )


def unregister_agent(agent_id: str) -> bool:
    """Unregister an agent from the global registry.

    Args:
        agent_id: ID of the agent to unregister.

    Returns:
        True if the agent was found and removed, False otherwise.
    """
    if agent_id in _agent_registry:
        del _agent_registry[agent_id]
        _provider_registry.pop(agent_id, None)
        logger.info("agent_unregistered", agent_id=agent_id)
        return True
    return False


def get_agent_info(agent_id: str) -> AgentInfo | None:
    """Get agent info from the registry.

    Args:
        agent_id: ID of the agent to retrieve.

    Returns:
        AgentInfo if found, None otherwise.
    """
    return _agent_registry.get(agent_id)


def get_agent_provider(agent_id: str) -> BaseProvider | None:
    """Get provider for an agent from the registry.

    Args:
        agent_id: ID of the agent.

    Returns:
        BaseProvider if found, None otherwise.
    """
    return _provider_registry.get(agent_id)


def list_registered_agents() -> list[AgentInfo]:
    """List all registered agents.

    Returns:
        List of AgentInfo for all registered agents.
    """
    return list(_agent_registry.values())


def clear_agent_registry() -> None:
    """Clear all registered agents.

    Primarily used for testing.
    """
    _agent_registry.clear()
    _provider_registry.clear()


# =============================================================================
# Bridge Interface
# =============================================================================


class BridgeInterface:
    """Interface for Rust-Python communication via PyO3.

    This class provides the methods called from Rust to interact with
    the Python agent system. All methods:

    1. Accept JSON strings as input
    2. Return JSON strings as output
    3. Never raise exceptions - errors are returned as error responses

    The bridge handles:
    - Parsing AgentRequest from JSON
    - Loading the appropriate agent by name
    - Executing the agent with the provider system
    - Streaming via callback for real-time updates
    - Returning AgentResponse as JSON

    Example:
        >>> bridge = BridgeInterface()
        >>> result = await bridge.execute('{"session_id": "s1", ...}')
        >>> # result is JSON string of AgentResponse
    """

    def __init__(self) -> None:
        """Initialize the bridge interface."""
        self._current_state: AgentState = AgentState.IDLE
        logger.info("bridge_interface_initialized")

    async def execute(self, request_json: str) -> str:
        """Execute an agent request and return the response.

        This is the main entry point for non-streaming agent execution.
        Parses the request JSON, executes the agent, and returns
        the response as JSON.

        Args:
            request_json: JSON string containing AgentRequest.

        Returns:
            JSON string containing AgentResponse.
            Never raises - errors are returned as error responses.
        """
        try:
            # Parse the request
            request = self._parse_request(request_json)
            if isinstance(request, AgentResponse):
                # Parsing failed, request is actually an error response
                return request.model_dump_json(by_alias=True)

            logger.info(
                "execute_request",
                session_id=request.session_id,
                conversation_id=request.conversation_id,
                agent_name=request.agent_name,
            )

            # Execute the agent
            response = await self._execute_agent(request)
            return response.model_dump_json(by_alias=True)

        except Exception as e:
            # Catch-all for any unexpected errors
            error_response = self._create_error_response(
                f"Unexpected error: {e}\n{traceback.format_exc()}"
            )
            return error_response.model_dump_json(by_alias=True)

    async def execute_streaming(
        self,
        request_json: str,
        callback: Callable[[str], None],
    ) -> str:
        """Execute an agent request with streaming.

        This method streams response chunks to the Rust side via the
        provided callback. Each chunk is serialized as JSON and passed
        to the callback. The final response is also returned.

        Args:
            request_json: JSON string containing AgentRequest.
            callback: Function that accepts JSON chunk strings.
                     Called for each StreamChunk during execution.

        Returns:
            JSON string containing final AgentResponse.
            Never raises - errors are returned as error responses.
        """
        try:
            # Parse the request
            request = self._parse_request(request_json)
            if isinstance(request, AgentResponse):
                # Parsing failed, send error chunk and return error response
                error_chunk = ErrorChunk(message=str(request.finish_reason))
                self._safe_callback(callback, error_chunk)
                return request.model_dump_json(by_alias=True)

            logger.info(
                "execute_streaming_request",
                session_id=request.session_id,
                conversation_id=request.conversation_id,
                agent_name=request.agent_name,
            )

            # Send initial state change
            self._transition_state(AgentState.THINKING, callback)

            # Execute the agent with streaming
            response = await self._execute_agent_streaming(request, callback)

            # Send final state change
            self._transition_state(AgentState.IDLE, callback)

            # Send done chunk
            done_chunk = DoneChunk(response=response)
            self._safe_callback(callback, done_chunk)

            return response.model_dump_json(by_alias=True)

        except Exception as e:
            # Catch-all for any unexpected errors
            error_msg = f"Unexpected error: {e}\n{traceback.format_exc()}"
            error_chunk = ErrorChunk(message=error_msg)
            self._safe_callback(callback, error_chunk)

            error_response = self._create_error_response(error_msg)
            return error_response.model_dump_json(by_alias=True)

    def list_agents(self) -> str:
        """List all available agents.

        Returns:
            JSON array string containing AgentInfo objects.
            Never raises - errors are returned as empty array.
        """
        try:
            agents = list_registered_agents()

            # If no agents registered, return some defaults
            if not agents:
                agents = self._get_default_agents()

            # Serialize to JSON array
            return json.dumps(
                [a.model_dump(by_alias=True) for a in agents],
                ensure_ascii=False,
            )

        except Exception as e:
            logger.error("list_agents_error", error=str(e))
            return "[]"

    def get_agent_info(self, name: str) -> str:
        """Get information about a specific agent.

        Args:
            name: Name/ID of the agent to retrieve.

        Returns:
            JSON string containing AgentInfo, or "null" if not found.
            Never raises.
        """
        try:
            info = get_agent_info(name)
            if info is None:
                # Check defaults
                defaults = {a.id: a for a in self._get_default_agents()}
                info = defaults.get(name)

            if info is None:
                return "null"

            return info.model_dump_json(by_alias=True)

        except Exception as e:
            logger.error("get_agent_info_error", agent_name=name, error=str(e))
            return "null"

    # =========================================================================
    # Private Methods
    # =========================================================================

    def _parse_request(self, request_json: str) -> AgentRequest | AgentResponse:
        """Parse JSON into AgentRequest.

        Args:
            request_json: JSON string to parse.

        Returns:
            AgentRequest on success, AgentResponse (error) on failure.
        """
        try:
            data = json.loads(request_json)
            return AgentRequest.model_validate(data)
        except json.JSONDecodeError as e:
            logger.error("request_json_parse_error", error=str(e))
            return self._create_error_response(f"Invalid JSON: {e}")
        except Exception as e:
            logger.error("request_validation_error", error=str(e))
            return self._create_error_response(f"Invalid request: {e}")

    async def _execute_agent(self, request: AgentRequest) -> AgentResponse:
        """Execute an agent request (non-streaming).

        Args:
            request: Validated agent request.

        Returns:
            AgentResponse with the result.
        """
        # Get the provider for this agent
        provider = get_agent_provider(request.agent_name)

        if provider is None:
            # Try to get a default provider
            provider = await self._get_default_provider(request.agent_name)

        if provider is None:
            return self._create_error_response(
                f"No provider found for agent '{request.agent_name}'"
            )

        try:
            # Build messages for the provider
            from ticca_agent.core.types import AgentRequest as CoreAgentRequest
            from ticca_agent.core.types import Message, MessageRole

            # Create a simple message list with the user's message
            messages = [
                Message(
                    role=MessageRole.USER,
                    content=request.message,
                )
            ]

            # Create the core request
            core_request = CoreAgentRequest(
                messages=messages,
                model=request.model,
                temperature=request.temperature or 0.7,
                max_tokens=request.max_tokens,
            )

            # Generate response
            core_response = await provider.generate(core_request, model=request.model)

            # Convert to bridge response
            return AgentResponse(
                message_id=str(core_response.response_id),
                content=core_response.content,
                role="assistant",
                tool_calls=self._convert_tool_calls(core_response.tool_calls),
                usage=self._convert_usage(core_response.usage),
                finish_reason=self._convert_finish_reason(core_response.finish_reason),
            )

        except Exception as e:
            logger.error(
                "agent_execution_error",
                agent_name=request.agent_name,
                error=str(e),
                traceback=traceback.format_exc(),
            )
            return self._create_error_response(f"Agent execution failed: {e}")

    async def _execute_agent_streaming(
        self,
        request: AgentRequest,
        callback: Callable[[str], None],
    ) -> AgentResponse:
        """Execute an agent request with streaming.

        Args:
            request: Validated agent request.
            callback: Callback to send stream chunks to Rust.

        Returns:
            Final AgentResponse.
        """
        # Get the provider for this agent
        provider = get_agent_provider(request.agent_name)

        if provider is None:
            provider = await self._get_default_provider(request.agent_name)

        if provider is None:
            return self._create_error_response(
                f"No provider found for agent '{request.agent_name}'"
            )

        try:
            # Build messages for the provider
            from ticca_agent.core.types import AgentRequest as CoreAgentRequest
            from ticca_agent.core.types import Message, MessageRole

            messages = [
                Message(
                    role=MessageRole.USER,
                    content=request.message,
                )
            ]

            core_request = CoreAgentRequest(
                messages=messages,
                model=request.model,
                temperature=request.temperature or 0.7,
                max_tokens=request.max_tokens,
            )

            # Transition to responding state
            self._transition_state(AgentState.RESPONDING, callback)

            # Stream the response
            full_content = ""
            final_usage: TokenUsage | None = None
            finish_reason = "stop"

            async for chunk in provider.stream(core_request, model=request.model):
                # Convert core StreamChunk to bridge StreamChunk
                if chunk.delta:
                    full_content += chunk.delta
                    text_chunk = TextDeltaChunk(content=chunk.delta)
                    self._safe_callback(callback, text_chunk)

                if chunk.usage:
                    final_usage = TokenUsage(
                        prompt_tokens=chunk.usage.prompt_tokens,
                        completion_tokens=chunk.usage.completion_tokens,
                        total_tokens=chunk.usage.total_tokens,
                    )
                    usage_chunk = UsageChunk(usage=final_usage)
                    self._safe_callback(callback, usage_chunk)

                if chunk.finish_reason:
                    finish_reason = chunk.finish_reason

            # Build final response
            return AgentResponse(
                message_id=str(uuid4()),
                content=full_content,
                role="assistant",
                usage=final_usage or TokenUsage(),
                finish_reason=self._convert_finish_reason(finish_reason),
            )

        except Exception as e:
            logger.error(
                "streaming_execution_error",
                agent_name=request.agent_name,
                error=str(e),
                traceback=traceback.format_exc(),
            )
            error_chunk = ErrorChunk(message=str(e))
            self._safe_callback(callback, error_chunk)
            return self._create_error_response(f"Streaming execution failed: {e}")

    def _transition_state(
        self,
        new_state: AgentState,
        callback: Callable[[str], None],
    ) -> None:
        """Transition to a new agent state and notify via callback.

        Args:
            new_state: The new state to transition to.
            callback: Callback to send state change chunk.
        """
        if new_state != self._current_state:
            chunk = StateChangeChunk(
                from_state=self._current_state,
                to_state=new_state,
            )
            self._safe_callback(callback, chunk)
            self._current_state = new_state

    def _safe_callback(
        self,
        callback: Callable[[str], None],
        chunk: (
            TextDeltaChunk
            | ThinkingDeltaChunk
            | ToolStartChunk
            | ToolResultChunk
            | StateChangeChunk
            | UsageChunk
            | DoneChunk
            | ErrorChunk
        ),
    ) -> None:
        """Safely call the callback with a serialized chunk.

        Args:
            callback: The callback function.
            chunk: The chunk to serialize and send.
        """
        try:
            json_str = chunk.model_dump_json(by_alias=True)
            callback(json_str)
        except Exception as e:
            logger.error(
                "callback_error",
                chunk_type=type(chunk).__name__,
                error=str(e),
            )

    def _create_error_response(self, message: str) -> AgentResponse:
        """Create an error response.

        Args:
            message: Error message.

        Returns:
            AgentResponse with error finish reason.
        """
        return AgentResponse(
            message_id=str(uuid4()),
            content="",
            role="assistant",
            finish_reason=FinishReasonError(value=message),
        )

    def _convert_tool_calls(
        self,
        tool_calls: list[dict[str, Any]] | None,
    ) -> list[ToolCall] | None:
        """Convert core tool calls to bridge format.

        Args:
            tool_calls: List of tool call dicts from core.

        Returns:
            List of ToolCall objects or None.
        """
        if not tool_calls:
            return None

        result = []
        for tc in tool_calls:
            try:
                result.append(
                    ToolCall(
                        id=tc.get("id", str(uuid4())),
                        name=tc.get("function", {}).get("name", tc.get("name", "")),
                        arguments=tc.get("function", {}).get(
                            "arguments", tc.get("arguments", {})
                        ),
                    )
                )
            except Exception as e:
                logger.warning("tool_call_conversion_error", error=str(e))

        return result if result else None

    def _convert_usage(
        self,
        usage: Any | None,
    ) -> TokenUsage:
        """Convert core usage to bridge format.

        Args:
            usage: Usage object from core.

        Returns:
            TokenUsage object.
        """
        if usage is None:
            return TokenUsage()

        return TokenUsage(
            prompt_tokens=getattr(usage, "prompt_tokens", 0),
            completion_tokens=getattr(usage, "completion_tokens", 0),
            total_tokens=getattr(usage, "total_tokens", 0),
        )

    def _convert_finish_reason(
        self,
        reason: str | None,
    ) -> FinishReasonStop | FinishReasonToolUse | FinishReasonMaxTokens | FinishReasonError:
        """Convert core finish reason to bridge format.

        Args:
            reason: Finish reason string from core.

        Returns:
            FinishReason variant.
        """
        if reason is None or reason == "stop":
            return FinishReasonStop()
        elif reason in ("tool_calls", "tool_use"):
            return FinishReasonToolUse()
        elif reason in ("length", "max_tokens"):
            return FinishReasonMaxTokens()
        else:
            return FinishReasonError(value=f"Unknown finish reason: {reason}")

    async def _get_default_provider(
        self,
        agent_name: str,
    ) -> BaseProvider | None:
        """Get a default provider for an agent.

        This is a fallback when no provider is explicitly registered.
        It attempts to create a provider based on available configuration.

        Args:
            agent_name: Name of the agent.

        Returns:
            BaseProvider instance or None.
        """
        # TODO: Implement provider auto-discovery based on config
        # For now, return None and let the caller handle it
        logger.warning(
            "no_provider_registered",
            agent_name=agent_name,
            hint="Use register_agent() to register an agent with a provider",
        )
        return None

    def _get_default_agents(self) -> list[AgentInfo]:
        """Get default agent definitions.

        Returns:
            List of default AgentInfo objects.
        """
        return [
            AgentInfo(
                id="default",
                name="Default Agent",
                description="General-purpose AI assistant",
                default_model="claude-3-5-sonnet-20241022",
                available_tools=[],
                capabilities=["chat", "reasoning"],
            ),
            AgentInfo(
                id="code",
                name="Code Agent",
                description="Specialized agent for coding tasks",
                default_model="claude-3-5-sonnet-20241022",
                available_tools=["read_file", "write_file", "run_command"],
                capabilities=["coding", "debugging", "refactoring"],
            ),
        ]


# =============================================================================
# Exceptions
# =============================================================================


class BridgeError(Exception):
    """Exception for bridge-related errors.

    Note: These errors should be caught and converted to error responses
    before crossing the Rust-Python boundary. Never raise to Rust.

    Attributes:
        message: Human-readable error description.
        code: Optional error code for categorization.
    """

    def __init__(
        self,
        message: str,
        *,
        code: int | None = None,
    ) -> None:
        """Initialize BridgeError.

        Args:
            message: Human-readable error description.
            code: Optional error code.
        """
        super().__init__(message)
        self.message = message
        self.code = code


# =============================================================================
# Convenience Functions
# =============================================================================


def create_bridge() -> BridgeInterface:
    """Create a configured bridge interface.

    This is the primary factory function for creating a bridge instance
    that will be used by the Rust side via PyO3.

    Returns:
        Configured BridgeInterface instance.
    """
    return BridgeInterface()


async def initialize_bridge() -> BridgeInterface:
    """Initialize the bridge and register default agents.

    This async function initializes the bridge and sets up any
    default agents and providers based on available configuration.

    Returns:
        Initialized BridgeInterface instance.
    """
    bridge = create_bridge()

    # TODO: Load configuration and register agents/providers
    # This will be expanded when we integrate with the config system

    logger.info("bridge_initialized")
    return bridge
