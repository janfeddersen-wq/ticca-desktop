"""Core agent implementation for Ticca Agent.

This module provides the main Agent class that orchestrates AI interactions,
manages conversation flow, and coordinates with providers and tools.

The Agent is the central abstraction in Ticca Agent, responsible for:
- Processing user messages and generating responses
- Managing conversation context and history
- Coordinating tool execution and result handling (ReAct pattern)
- Supporting streaming responses for real-time UI updates
- Handling multi-turn conversations with context management

The agent implements the ReAct pattern (Reason + Act):
1. Receive user input
2. Reason about what to do (generate thoughts)
3. Act by calling tools or generating response
4. Observe results and iterate if needed

Example:
    >>> from ticca_agent.core import Agent
    >>> agent = Agent(name="assistant", provider=my_provider)
    >>> response = await agent.run("Hello, how are you?")
    >>> print(response.content)
    "Hello! I'm doing well, thank you for asking!"

"""

from __future__ import annotations

import asyncio
from collections.abc import AsyncIterator, Callable
from typing import TYPE_CHECKING, Any, Protocol, runtime_checkable

from pydantic import BaseModel, Field

if TYPE_CHECKING:
    from ticca_agent.core.context import RunContext
    from ticca_agent.core.prompts import ComposedPrompt, PromptComposer
    from ticca_agent.core.types import (
        AgentRequest,
        AgentResponse,
        Message,
        StreamChunk,
    )
    from ticca_agent.providers.base import BaseProvider
    from ticca_agent.tools.base import BaseTool


# Type alias for stream callbacks
StreamCallback = Callable[["StreamChunk"], None]


@runtime_checkable
class AgentProtocol(Protocol):
    """Protocol defining the interface for all agents.

    This protocol ensures type-safe agent implementations and enables
    dependency injection for testing.
    """

    @property
    def name(self) -> str:
        """Unique identifier for this agent."""
        ...

    @property
    def display_name(self) -> str:
        """Human-readable name for this agent."""
        ...

    async def run(
        self,
        prompt: str,
        *,
        context: RunContext | None = None,
    ) -> AgentResponse:
        """Process a prompt and generate a response."""
        ...

    async def stream(
        self,
        prompt: str,
        *,
        context: RunContext | None = None,
    ) -> AsyncIterator[StreamChunk]:
        """Stream response chunks for a prompt."""
        ...


class AgentConfig(BaseModel):
    """Configuration for an Agent instance.

    Attributes:
        name: Unique identifier for this agent.
        display_name: Human-readable name for the agent.
        system_prompt: Default system prompt for the agent.
        model: Default model to use (provider-specific).
        temperature: Default sampling temperature.
        max_tokens: Default max tokens for responses.
        tools_enabled: Whether tool use is enabled.
        max_tool_iterations: Maximum tool call iterations per request.
        streaming_enabled: Whether streaming is enabled by default.
    """

    name: str = Field(description="Unique agent identifier")
    display_name: str | None = Field(
        default=None,
        description="Human-readable agent name",
    )
    system_prompt: str | None = Field(
        default=None,
        description="Default system prompt for the agent",
    )
    model: str | None = Field(
        default=None,
        description="Default model identifier",
    )
    temperature: float = Field(
        default=0.7,
        ge=0.0,
        le=2.0,
        description="Default sampling temperature",
    )
    max_tokens: int | None = Field(
        default=None,
        ge=1,
        description="Default max tokens for responses",
    )
    tools_enabled: bool = Field(
        default=True,
        description="Whether tool use is enabled",
    )
    max_tool_iterations: int = Field(
        default=10,
        ge=1,
        le=100,
        description="Maximum tool call iterations per request",
    )
    streaming_enabled: bool = Field(
        default=True,
        description="Whether streaming is enabled by default",
    )

    model_config = {"frozen": True}


class ModelSettings(BaseModel):
    """Per-model configuration settings.

    These settings can be configured per-model to customize behavior.

    Attributes:
        temperature: Sampling temperature (0.0-1.0).
        seed: Random seed for reproducibility.
        top_p: Nucleus sampling parameter.
        reasoning_effort: For reasoning models (low/medium/high).
        verbosity: Response verbosity (low/medium/high).
        extended_thinking: Enable extended thinking (Claude).
        budget_tokens: Token budget for extended thinking.
    """

    temperature: float | None = Field(
        default=None,
        ge=0.0,
        le=1.0,
        description="Sampling temperature",
    )
    seed: int | None = Field(
        default=None,
        ge=0,
        le=999999,
        description="Random seed",
    )
    top_p: float | None = Field(
        default=None,
        ge=0.0,
        le=1.0,
        description="Nucleus sampling",
    )
    reasoning_effort: str | None = Field(
        default=None,
        description="Reasoning effort level",
    )
    verbosity: str | None = Field(
        default=None,
        description="Response verbosity",
    )
    extended_thinking: bool | None = Field(
        default=None,
        description="Enable extended thinking",
    )
    budget_tokens: int | None = Field(
        default=None,
        ge=1024,
        le=131072,
        description="Extended thinking budget",
    )

    model_config = {"frozen": True}


class Agent:
    """Core agent implementation for AI interactions.

    The Agent class is the primary interface for interacting with AI providers.
    It manages conversation context, tool execution, and response generation.

    This implementation uses the ReAct pattern:
    1. Reason: The model thinks about what to do
    2. Act: Execute tools based on the reasoning
    3. Observe: Process tool results
    4. Repeat: Continue until task is complete

    Attributes:
        config: Agent configuration.
        provider: The AI provider to use for generation.
        tools: List of tools available to this agent.

    Example:
        >>> config = AgentConfig(name="assistant")
        >>> agent = Agent(config=config, provider=my_provider)
        >>> async with agent.session() as session:
        ...     response = await session.run("Hello!")
    """

    def __init__(
        self,
        config: AgentConfig,
        provider: BaseProvider,
        tools: list[BaseTool[Any, Any]] | None = None,
        *,
        prompt_composer: PromptComposer | None = None,
        model_settings: ModelSettings | None = None,
    ) -> None:
        """Initialize the Agent.

        Args:
            config: Agent configuration.
            provider: AI provider for generation.
            tools: Optional list of tools available to the agent.
            prompt_composer: Optional prompt composer for system prompt handling.
            model_settings: Optional per-model settings.
        """
        self._config = config
        self._provider = provider
        self._tools: list[BaseTool[Any, Any]] = tools or []
        self._message_history: list[Message] = []
        self._prompt_composer = prompt_composer
        self._model_settings = model_settings
        self._composed_prompt: ComposedPrompt | None = None

    @property
    def name(self) -> str:
        """Get the agent's unique identifier."""
        return self._config.name

    @property
    def display_name(self) -> str:
        """Get the agent's human-readable name."""
        return self._config.display_name or self._config.name

    @property
    def config(self) -> AgentConfig:
        """Get the agent's configuration."""
        return self._config

    @property
    def provider(self) -> BaseProvider:
        """Get the agent's AI provider."""
        return self._provider

    @property
    def tools(self) -> list[BaseTool[Any, Any]]:
        """Get the list of available tools."""
        return self._tools.copy()

    @property
    def system_prompt(self) -> str | None:
        """Get the agent's system prompt."""
        return self._config.system_prompt

    def register_tool(self, tool: BaseTool[Any, Any]) -> None:
        """Register a new tool for use by this agent.

        Args:
            tool: The tool to register.

        Raises:
            ValueError: If a tool with the same name is already registered.
        """
        existing_names = {t.name for t in self._tools}
        if tool.name in existing_names:
            raise ValueError(f"Tool '{tool.name}' is already registered")

        self._tools.append(tool)

    def unregister_tool(self, tool_name: str) -> bool:
        """Unregister a tool by name.

        Args:
            tool_name: Name of the tool to unregister.

        Returns:
            True if the tool was found and removed, False otherwise.
        """
        for i, tool in enumerate(self._tools):
            if tool.name == tool_name:
                self._tools.pop(i)
                return True
        return False

    def get_tool(self, name: str) -> BaseTool[Any, Any] | None:
        """Get a tool by name.

        Args:
            name: Tool name.

        Returns:
            Tool if found, None otherwise.
        """
        for tool in self._tools:
            if tool.name == name:
                return tool
        return None

    def get_tool_schemas(self) -> list[dict[str, Any]]:
        """Get JSON schemas for all enabled tools.

        Returns:
            List of tool schemas for AI model consumption.
        """
        return [t.get_schema() for t in self._tools if t.is_enabled]

    async def run(
        self,
        prompt: str,
        *,
        context: RunContext | None = None,
        system_prompt: str | None = None,
        model: str | None = None,
        temperature: float | None = None,
        max_tokens: int | None = None,
        stream_callback: StreamCallback | None = None,
    ) -> AgentResponse:
        """Process a prompt and generate a response.

        This is the main entry point for non-streaming interactions.
        The method handles the complete request-response cycle including
        any necessary tool calls using the ReAct pattern.

        Args:
            prompt: The user's message to process.
            context: Optional run context with additional state.
            system_prompt: Override the default system prompt.
            model: Override the default model.
            temperature: Override the default temperature.
            max_tokens: Override the default max tokens.
            stream_callback: Optional callback for streaming chunks.

        Returns:
            Complete response from the AI model.

        Raises:
            AgentError: If processing fails.
        """
        from ticca_agent.core.types import (
            AgentRequest,
            AgentResponse,
            Message,
            MessageRole,
        )

        # Build the system prompt
        effective_prompt = system_prompt or self._config.system_prompt or ""

        # Handle Claude-Code prompt rewriting if needed
        effective_model = model or self._config.model
        user_message = prompt

        if self._prompt_composer and effective_prompt:
            import os

            composed = await self._prompt_composer.compose(
                base_prompt=effective_prompt,
                agent_name=self.name,
                model_name=effective_model,
                cwd=os.getcwd(),
            )
            self._composed_prompt = composed

            if composed.requires_rewrite:
                # Prepend system prompt to user message for Claude-Code
                effective_prompt, user_message = (
                    self._prompt_composer.rewrite_for_claude_code(
                        composed, prompt
                    )
                )
            else:
                effective_prompt = composed.system_prompt

        # Build message list
        messages: list[Message] = []

        # Add system message if we have a prompt
        if effective_prompt:
            messages.append(
                Message(role=MessageRole.SYSTEM, content=effective_prompt)
            )

        # Add history
        messages.extend(self._message_history)

        # Add user message
        messages.append(Message(role=MessageRole.USER, content=user_message))

        # Create request
        request = AgentRequest(
            messages=messages,
            model=effective_model,
            temperature=temperature or self._config.temperature,
            max_tokens=max_tokens or self._config.max_tokens,
        )

        # Execute with ReAct loop
        response = await self._execute_react_loop(
            request=request,
            context=context,
            stream_callback=stream_callback,
        )

        # Update history
        self._message_history.append(
            Message(role=MessageRole.USER, content=prompt)
        )
        self._message_history.append(
            Message(role=MessageRole.ASSISTANT, content=response.content)
        )

        return response

    async def _execute_react_loop(
        self,
        request: AgentRequest,
        context: RunContext | None = None,
        stream_callback: StreamCallback | None = None,
    ) -> AgentResponse:
        """Execute the ReAct loop for tool use.

        The ReAct pattern iterates between:
        1. Generating a response (which may include tool calls)
        2. Executing any tool calls
        3. Appending results to messages
        4. Repeating until no more tool calls or max iterations

        Args:
            request: The initial request.
            context: Optional run context.
            stream_callback: Optional streaming callback.

        Returns:
            Final response after all tool iterations.
        """
        from ticca_agent.core.types import Message, MessageRole

        messages = list(request.messages)
        iterations = 0
        max_iterations = self._config.max_tool_iterations

        while iterations < max_iterations:
            # Check for cancellation
            if context and context.is_cancelled:
                raise asyncio.CancelledError("Agent run was cancelled")

            # Create request with current messages
            current_request = request.model_copy(
                update={"messages": messages}
            )

            # Generate response
            if stream_callback:
                response = await self._stream_with_callback(
                    current_request, stream_callback
                )
            else:
                response = await self._provider.generate(current_request)

            # Record usage if we have context
            if context and response.usage:
                context.record_usage(
                    prompt_tokens=response.usage.prompt_tokens,
                    completion_tokens=response.usage.completion_tokens,
                )

            # Check if there are tool calls
            if not response.tool_calls or not self._config.tools_enabled:
                return response

            # Add assistant message with tool calls
            messages.append(
                Message(
                    role=MessageRole.ASSISTANT,
                    content=response.content,
                    tool_calls=response.tool_calls,
                )
            )

            # Execute tool calls
            tool_results = await self._execute_tools(
                response.tool_calls, context
            )

            # Add tool results to messages
            messages.extend(tool_results)

            iterations += 1

        # Max iterations reached
        return response

    async def _stream_with_callback(
        self,
        request: AgentRequest,
        callback: StreamCallback,
    ) -> AgentResponse:
        """Stream response while calling callback for each chunk.

        Args:
            request: Request to stream.
            callback: Callback for each chunk.

        Returns:
            Complete aggregated response.
        """
        from ticca_agent.core.types import AgentResponse

        content_parts: list[str] = []
        final_chunk = None

        async for chunk in self._provider.stream(request):
            callback(chunk)
            content_parts.append(chunk.delta)
            if chunk.is_final:
                final_chunk = chunk

        # Build response from accumulated content
        return AgentResponse(
            request_id=request.request_id,
            content="".join(content_parts),
            model=request.model or "unknown",
            usage=final_chunk.usage if final_chunk else None,
            finish_reason=final_chunk.finish_reason or "stop" if final_chunk else "stop",
        )

    async def stream(
        self,
        prompt: str,
        *,
        context: RunContext | None = None,
        system_prompt: str | None = None,
        model: str | None = None,
        temperature: float | None = None,
        max_tokens: int | None = None,
    ) -> AsyncIterator[StreamChunk]:
        """Stream response chunks for a prompt.

        This method enables real-time response display by yielding
        chunks as they are generated by the AI model.

        Args:
            prompt: The user's message to process.
            context: Optional run context with additional state.
            system_prompt: Override the default system prompt.
            model: Override the default model.
            temperature: Override the default temperature.
            max_tokens: Override the default max tokens.

        Yields:
            StreamChunk objects containing partial response content.

        Raises:
            AgentError: If streaming fails.
        """
        from ticca_agent.core.types import (
            AgentRequest,
            Message,
            MessageRole,
        )

        # Build the system prompt
        effective_prompt = system_prompt or self._config.system_prompt or ""

        # Build message list
        messages: list[Message] = []

        if effective_prompt:
            messages.append(
                Message(role=MessageRole.SYSTEM, content=effective_prompt)
            )

        messages.extend(self._message_history)
        messages.append(Message(role=MessageRole.USER, content=prompt))

        # Create request
        request = AgentRequest(
            messages=messages,
            model=model or self._config.model,
            temperature=temperature or self._config.temperature,
            max_tokens=max_tokens or self._config.max_tokens,
        )

        # Stream from provider
        content_parts: list[str] = []

        async for chunk in self._provider.stream(request):
            content_parts.append(chunk.delta)
            yield chunk

            if context and chunk.usage:
                context.record_usage(
                    prompt_tokens=chunk.usage.prompt_tokens,
                    completion_tokens=chunk.usage.completion_tokens,
                )

        # Update history
        self._message_history.append(
            Message(role=MessageRole.USER, content=prompt)
        )
        self._message_history.append(
            Message(
                role=MessageRole.ASSISTANT,
                content="".join(content_parts),
            )
        )

    async def run_streaming(
        self,
        prompt: str,
        context: RunContext | None = None,
    ) -> AsyncIterator[StreamChunk]:
        """Execute with streaming, yielding chunks.

        This is an alias for stream() for API compatibility.

        Args:
            prompt: User message.
            context: Optional run context.

        Yields:
            StreamChunk objects.
        """
        async for chunk in self.stream(prompt, context=context):
            yield chunk

    async def clear_history(self) -> None:
        """Clear the conversation history.

        This resets the agent's message history, starting a fresh
        conversation. System prompt is preserved.
        """
        self._message_history.clear()

    def get_history(self) -> list[Message]:
        """Get a copy of the current message history.

        Returns:
            List of messages in the conversation history.
        """
        return self._message_history.copy()

    def set_history(self, messages: list[Message]) -> None:
        """Set the message history.

        Args:
            messages: Messages to set as history.
        """
        self._message_history = list(messages)

    async def _execute_tools(
        self,
        tool_calls: list[dict[str, Any]],
        context: RunContext | None = None,
    ) -> list[Message]:
        """Execute tool calls and return result messages.

        Args:
            tool_calls: List of tool call requests from the model.
            context: Optional run context.

        Returns:
            List of tool result messages.
        """
        from ticca_agent.core.types import Message, MessageRole

        results: list[Message] = []

        for call in tool_calls:
            tool_name = call.get("name") or call.get("function", {}).get("name")
            tool_id = call.get("id", "unknown")
            arguments = call.get("arguments") or call.get("function", {}).get(
                "arguments", {}
            )

            # Parse arguments if they're a string
            if isinstance(arguments, str):
                import json

                try:
                    arguments = json.loads(arguments)
                except json.JSONDecodeError:
                    arguments = {}

            # Find the tool
            tool = self.get_tool(tool_name)
            if tool is None:
                result_content = f"Error: Tool '{tool_name}' not found"
            else:
                try:
                    # Record tool call in context
                    if context:
                        context.record_tool_call()

                    # Execute the tool
                    result = await tool.run(arguments, context)

                    if result.success:
                        result_content = str(result.output)
                    else:
                        result_content = f"Error: {result.error}"

                except Exception as e:
                    result_content = f"Error executing tool: {e}"

            # Create tool result message
            results.append(
                Message(
                    role=MessageRole.TOOL,
                    content=result_content,
                    tool_call_id=tool_id,
                )
            )

        return results

    def __repr__(self) -> str:
        """Return string representation of the agent."""
        return (
            f"Agent(name={self.name!r}, "
            f"provider={self._provider.PROVIDER_NAME!r}, "
            f"tools={len(self._tools)})"
        )


class AgentError(Exception):
    """Base exception for agent-related errors.

    Attributes:
        message: Human-readable error description.
        agent_name: Name of the agent that raised the error.
        recoverable: Whether the error is potentially recoverable.
    """

    def __init__(
        self,
        message: str,
        *,
        agent_name: str | None = None,
        recoverable: bool = False,
    ) -> None:
        """Initialize AgentError.

        Args:
            message: Human-readable error description.
            agent_name: Name of the agent that raised the error.
            recoverable: Whether the error is potentially recoverable.
        """
        super().__init__(message)
        self.message = message
        self.agent_name = agent_name
        self.recoverable = recoverable

    def __str__(self) -> str:
        """Return string representation of the error."""
        if self.agent_name:
            return f"[{self.agent_name}] {self.message}"
        return self.message


__all__ = [
    "Agent",
    "AgentConfig",
    "AgentError",
    "AgentProtocol",
    "ModelSettings",
    "StreamCallback",
]
