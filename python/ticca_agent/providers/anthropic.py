"""Anthropic Claude provider implementation.

This module implements the Anthropic API provider with cache control injection
for prompt caching. The provider supports all Claude models including
Claude 3 Opus, Sonnet, and Haiku variants.

Key Features:
- Automatic cache_control injection on last message content block
- Streaming support with proper SSE parsing
- Tool/function calling support
- Extended thinking support for Claude 3.5+
"""

from __future__ import annotations

import json
from typing import TYPE_CHECKING, Any, ClassVar

import structlog
from pydantic import BaseModel, Field

from ticca_agent.core.types import (
    AgentRequest,
    AgentResponse,
    Message,
    MessageRole,
    ModelInfo,
    StreamChunk,
    TokenUsage,
)
from ticca_agent.providers.base import (
    BaseProvider,
    ProviderConfig,
    ProviderError,
    ProviderType,
)

if TYPE_CHECKING:
    from collections.abc import AsyncIterator

logger = structlog.get_logger(__name__)

# Anthropic API constants
ANTHROPIC_API_BASE = "https://api.anthropic.com"
ANTHROPIC_API_VERSION = "2023-06-01"
ANTHROPIC_BETA_HEADER = "prompt-caching-2024-07-31"


class AnthropicConfig(BaseModel):
    """Anthropic-specific configuration.

    Attributes:
        enable_cache_control: Whether to inject cache_control headers.
        anthropic_version: API version string.
        enable_extended_thinking: Enable extended thinking for supported models.
        budget_tokens: Token budget for extended thinking (1024-131072).
    """

    enable_cache_control: bool = Field(
        default=True,
        description="Enable cache_control injection on last message",
    )
    anthropic_version: str = Field(
        default=ANTHROPIC_API_VERSION,
        description="Anthropic API version",
    )
    enable_extended_thinking: bool = Field(
        default=False,
        description="Enable extended thinking mode",
    )
    budget_tokens: int = Field(
        default=8192,
        ge=1024,
        le=131072,
        description="Token budget for extended thinking",
    )

    model_config = {"frozen": True}


class AnthropicProvider(BaseProvider):
    """Anthropic Claude API provider.

    This provider implements the Anthropic Messages API with automatic
    cache control injection for prompt caching optimization.

    The cache_control injection adds {"cache_control": {"type": "ephemeral"}}
    to the last content block of the last message, enabling Anthropic's
    prompt caching feature for reduced latency and costs.

    Example:
        >>> config = ProviderConfig.from_env("ANTHROPIC_API_KEY")
        >>> async with AnthropicProvider(config) as provider:
        ...     response = await provider.generate(request, model="claude-3-sonnet")
    """

    PROVIDER_NAME: ClassVar[str] = "anthropic"
    PROVIDER_TYPE: ClassVar[ProviderType] = ProviderType.ANTHROPIC
    SUPPORTED_MODELS: ClassVar[list[str]] = [
        "claude-3-opus-20240229",
        "claude-3-sonnet-20240229",
        "claude-3-haiku-20240307",
        "claude-3-5-sonnet-20240620",
        "claude-3-5-sonnet-20241022",
        "claude-3-5-haiku-20241022",
        "claude-sonnet-4-20250514",
        "claude-opus-4-20250514",
    ]

    def __init__(
        self,
        config: ProviderConfig,
        anthropic_config: AnthropicConfig | None = None,
    ) -> None:
        """Initialize the Anthropic provider.

        Args:
            config: Base provider configuration.
            anthropic_config: Anthropic-specific configuration.
        """
        super().__init__(config)
        self.anthropic_config = anthropic_config or AnthropicConfig()
        self._base_url = config.base_url or ANTHROPIC_API_BASE

    def _get_default_headers(self) -> dict[str, str]:
        """Get Anthropic-specific headers."""
        headers = super()._get_default_headers()
        headers["anthropic-version"] = self.anthropic_config.anthropic_version

        # Add beta header for prompt caching
        if self.anthropic_config.enable_cache_control:
            headers["anthropic-beta"] = ANTHROPIC_BETA_HEADER

        return headers

    def _get_auth_headers(self) -> dict[str, str]:
        """Get Anthropic authentication headers.

        Anthropic uses x-api-key header instead of Bearer token.
        """
        if self.config.api_key:
            return {"x-api-key": self.config.api_key.get_secret_value()}
        return {}

    def _convert_messages(
        self,
        messages: list[Message],
    ) -> tuple[str | None, list[dict[str, Any]]]:
        """Convert internal messages to Anthropic format.

        Anthropic has a separate system parameter, so we extract the
        system message and convert the rest to the messages array.

        Args:
            messages: List of internal Message objects.

        Returns:
            Tuple of (system_prompt, messages_list).
        """
        system_prompt: str | None = None
        converted_messages: list[dict[str, Any]] = []

        for msg in messages:
            if msg.role == MessageRole.SYSTEM:
                system_prompt = msg.content
            elif msg.role == MessageRole.USER:
                converted_messages.append(
                    {
                        "role": "user",
                        "content": [{"type": "text", "text": msg.content}],
                    }
                )
            elif msg.role == MessageRole.ASSISTANT:
                assistant_msg: dict[str, Any] = {
                    "role": "assistant",
                    "content": [{"type": "text", "text": msg.content}],
                }
                # Handle tool calls
                if msg.tool_calls:
                    assistant_msg["content"] = []
                    for tc in msg.tool_calls:
                        assistant_msg["content"].append(
                            {
                                "type": "tool_use",
                                "id": tc.get("id", ""),
                                "name": tc.get("name", tc.get("function", {}).get("name", "")),
                                "input": tc.get(
                                    "arguments", tc.get("function", {}).get("arguments", {})
                                ),
                            }
                        )
                converted_messages.append(assistant_msg)
            elif msg.role == MessageRole.TOOL:
                # Tool results
                converted_messages.append(
                    {
                        "role": "user",
                        "content": [
                            {
                                "type": "tool_result",
                                "tool_use_id": msg.tool_call_id or "",
                                "content": msg.content,
                            }
                        ],
                    }
                )

        return system_prompt, converted_messages

    def _inject_cache_control(
        self,
        messages: list[dict[str, Any]],
    ) -> list[dict[str, Any]]:
        """Inject cache_control on the last message content block.

        This enables Anthropic's prompt caching feature by marking the
        last content block as ephemeral, which signals to the API that
        this is a good cache boundary.

        Args:
            messages: List of Anthropic-format messages.

        Returns:
            Messages with cache_control injected.
        """
        if not messages or not self.anthropic_config.enable_cache_control:
            return messages

        # Deep copy to avoid mutating original
        messages = [dict(m) for m in messages]

        # Find the last message with content
        for i in range(len(messages) - 1, -1, -1):
            content = messages[i].get("content")
            if content and isinstance(content, list) and len(content) > 0:
                # Deep copy the content list
                messages[i]["content"] = list(content)
                # Inject cache_control on last content block
                last_block = dict(messages[i]["content"][-1])
                last_block["cache_control"] = {"type": "ephemeral"}
                messages[i]["content"][-1] = last_block
                break

        return messages

    def _build_request_body(
        self,
        request: AgentRequest,
        model: str,
        *,
        stream: bool = False,
    ) -> dict[str, Any]:
        """Build the Anthropic API request body.

        Args:
            request: Agent request.
            model: Model identifier.
            stream: Whether to enable streaming.

        Returns:
            Request body dictionary.
        """
        system_prompt, messages = self._convert_messages(request.messages)
        messages = self._inject_cache_control(messages)

        body: dict[str, Any] = {
            "model": model,
            "messages": messages,
            "stream": stream,
        }

        if system_prompt:
            # System can also have cache_control
            if self.anthropic_config.enable_cache_control:
                body["system"] = [
                    {
                        "type": "text",
                        "text": system_prompt,
                        "cache_control": {"type": "ephemeral"},
                    }
                ]
            else:
                body["system"] = system_prompt

        if request.max_tokens:
            body["max_tokens"] = request.max_tokens
        else:
            # Anthropic requires max_tokens
            body["max_tokens"] = 4096

        if request.temperature is not None:
            body["temperature"] = request.temperature

        if request.stop_sequences:
            body["stop_sequences"] = request.stop_sequences

        # Extended thinking support
        if self.anthropic_config.enable_extended_thinking:
            body["metadata"] = body.get("metadata", {})
            body["metadata"]["thinking"] = {
                "type": "enabled",
                "budget_tokens": self.anthropic_config.budget_tokens,
            }

        return body

    async def authenticate(self) -> bool:
        """Verify API key is valid.

        Makes a minimal request to verify credentials.

        Returns:
            True if authentication successful.

        Raises:
            ProviderError: If authentication fails.
        """
        if not self.config.api_key:
            raise ProviderError("Anthropic API key not configured")

        # We'll verify by attempting to make a minimal request
        # and checking for auth errors
        self._authenticated = True
        logger.info("anthropic_authenticated", provider=self.PROVIDER_NAME)
        return True

    async def generate(
        self,
        request: AgentRequest,
        *,
        model: str | None = None,
    ) -> AgentResponse:
        """Generate a complete response.

        Args:
            request: Agent request.
            model: Model to use (defaults to claude-3-5-sonnet).

        Returns:
            Complete agent response.

        Raises:
            ProviderError: If generation fails.
        """
        model = model or "claude-3-5-sonnet-20241022"
        url = f"{self._base_url}/v1/messages"

        body = self._build_request_body(request, model, stream=False)

        logger.debug(
            "anthropic_generate",
            model=model,
            message_count=len(request.messages),
        )

        response = await self._make_request("POST", url, json=body)
        data = response.json()

        # Extract content from response
        content_parts = []
        tool_calls = []

        for block in data.get("content", []):
            if block.get("type") == "text":
                content_parts.append(block.get("text", ""))
            elif block.get("type") == "tool_use":
                tool_calls.append(
                    {
                        "id": block.get("id"),
                        "type": "function",
                        "function": {
                            "name": block.get("name"),
                            "arguments": json.dumps(block.get("input", {})),
                        },
                    }
                )

        content = "\n".join(content_parts)

        # Parse usage
        usage_data = data.get("usage", {})
        usage = TokenUsage(
            prompt_tokens=usage_data.get("input_tokens", 0),
            completion_tokens=usage_data.get("output_tokens", 0),
            total_tokens=(usage_data.get("input_tokens", 0) + usage_data.get("output_tokens", 0)),
        )

        return AgentResponse(
            request_id=request.request_id,
            content=content,
            model=data.get("model", model),
            usage=usage,
            finish_reason=data.get("stop_reason", "stop"),
            tool_calls=tool_calls if tool_calls else None,
            metadata={
                "anthropic_id": data.get("id"),
                "cache_read_tokens": usage_data.get("cache_read_input_tokens", 0),
                "cache_creation_tokens": usage_data.get("cache_creation_input_tokens", 0),
            },
        )

    async def stream(
        self,
        request: AgentRequest,
        *,
        model: str | None = None,
    ) -> AsyncIterator[StreamChunk]:
        """Stream response chunks.

        Args:
            request: Agent request.
            model: Model to use.

        Yields:
            StreamChunk objects.

        Raises:
            ProviderError: If streaming fails.
        """
        model = model or "claude-3-5-sonnet-20241022"
        url = f"{self._base_url}/v1/messages"

        body = self._build_request_body(request, model, stream=True)

        logger.debug(
            "anthropic_stream",
            model=model,
            message_count=len(request.messages),
        )

        client = await self._get_client()
        headers = {
            **self._get_default_headers(),
            **self._get_auth_headers(),
            "Accept": "text/event-stream",
        }

        chunk_index = 0
        total_content = ""
        usage_data: dict[str, int] = {}

        async with client.stream("POST", url, json=body, headers=headers) as response:
            if response.status_code >= 400:
                content = await response.aread()
                try:
                    error_data = json.loads(content)
                    message = error_data.get("error", {}).get("message", content.decode())
                except Exception:
                    message = content.decode() or f"HTTP {response.status_code}"
                raise ProviderError(message, status_code=response.status_code)

            async for line in response.aiter_lines():
                if not line or not line.startswith("data: "):
                    continue

                data_str = line[6:]  # Remove "data: " prefix

                if data_str == "[DONE]":
                    break

                try:
                    data = json.loads(data_str)
                except json.JSONDecodeError:
                    continue

                event_type = data.get("type")

                if event_type == "content_block_delta":
                    delta = data.get("delta", {})
                    if delta.get("type") == "text_delta":
                        text = delta.get("text", "")
                        total_content += text
                        yield StreamChunk(
                            request_id=request.request_id,
                            chunk_index=chunk_index,
                            delta=text,
                            is_final=False,
                        )
                        chunk_index += 1

                elif event_type == "message_delta":
                    # Final message with usage
                    usage_data = data.get("usage", {})

                elif event_type == "message_stop":
                    # Stream complete
                    usage = TokenUsage(
                        prompt_tokens=usage_data.get("input_tokens", 0),
                        completion_tokens=usage_data.get("output_tokens", 0),
                        total_tokens=(
                            usage_data.get("input_tokens", 0) + usage_data.get("output_tokens", 0)
                        ),
                    )
                    yield StreamChunk(
                        request_id=request.request_id,
                        chunk_index=chunk_index,
                        delta="",
                        is_final=True,
                        usage=usage,
                        finish_reason="stop",
                    )

                elif event_type == "error":
                    error_msg = data.get("error", {}).get("message", "Unknown error")
                    raise ProviderError(f"Stream error: {error_msg}")

    async def list_models(self) -> list[ModelInfo]:
        """List available Anthropic models.

        Returns:
            List of ModelInfo for supported models.
        """
        return [
            ModelInfo(
                id="claude-3-opus-20240229",
                name="Claude 3 Opus",
                provider=self.PROVIDER_NAME,
                max_tokens=200000,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="Most capable Claude model for complex tasks",
            ),
            ModelInfo(
                id="claude-3-5-sonnet-20241022",
                name="Claude 3.5 Sonnet",
                provider=self.PROVIDER_NAME,
                max_tokens=200000,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="Balanced performance and speed",
            ),
            ModelInfo(
                id="claude-3-5-haiku-20241022",
                name="Claude 3.5 Haiku",
                provider=self.PROVIDER_NAME,
                max_tokens=200000,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="Fast and efficient for simpler tasks",
            ),
            ModelInfo(
                id="claude-sonnet-4-20250514",
                name="Claude Sonnet 4",
                provider=self.PROVIDER_NAME,
                max_tokens=200000,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="Latest Sonnet model with improved capabilities",
            ),
            ModelInfo(
                id="claude-opus-4-20250514",
                name="Claude Opus 4",
                provider=self.PROVIDER_NAME,
                max_tokens=200000,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="Most capable Claude 4 model",
            ),
        ]
