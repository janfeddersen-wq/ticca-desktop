"""Cerebras provider implementation.

Cerebras provides high-speed inference using their custom hardware.
The API is OpenAI-compatible.
"""

from __future__ import annotations

import json
from typing import TYPE_CHECKING, Any, ClassVar

import structlog

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

# Cerebras API constants
CEREBRAS_API_BASE = "https://api.cerebras.ai/v1"


class CerebrasProvider(BaseProvider):
    """Cerebras high-speed inference provider.

    Uses OpenAI-compatible API format.

    Example:
        >>> config = ProviderConfig.from_env("CEREBRAS_API_KEY")
        >>> async with CerebrasProvider(config) as provider:
        ...     response = await provider.generate(request, model="llama3.1-70b")
    """

    PROVIDER_NAME: ClassVar[str] = "cerebras"
    PROVIDER_TYPE: ClassVar[ProviderType] = ProviderType.CEREBRAS
    SUPPORTED_MODELS: ClassVar[list[str]] = [
        "llama3.1-8b",
        "llama3.1-70b",
        "llama-3.3-70b",
    ]

    def __init__(self, config: ProviderConfig) -> None:
        """Initialize the Cerebras provider.

        Args:
            config: Base provider configuration.
        """
        super().__init__(config)
        self._base_url = config.base_url or CEREBRAS_API_BASE

    def _convert_messages(self, messages: list[Message]) -> list[dict[str, Any]]:
        """Convert internal messages to OpenAI format.

        Args:
            messages: List of internal Message objects.

        Returns:
            List of OpenAI-format message dictionaries.
        """
        converted: list[dict[str, Any]] = []

        for msg in messages:
            cerebras_msg: dict[str, Any] = {
                "role": msg.role.value,
                "content": msg.content,
            }

            if msg.name:
                cerebras_msg["name"] = msg.name

            if msg.role == MessageRole.ASSISTANT and msg.tool_calls:
                cerebras_msg["tool_calls"] = msg.tool_calls

            if msg.role == MessageRole.TOOL and msg.tool_call_id:
                cerebras_msg["tool_call_id"] = msg.tool_call_id

            converted.append(cerebras_msg)

        return converted

    def _build_request_body(
        self,
        request: AgentRequest,
        model: str,
        *,
        stream: bool = False,
    ) -> dict[str, Any]:
        """Build the Cerebras API request body.

        Args:
            request: Agent request.
            model: Model identifier.
            stream: Whether to enable streaming.

        Returns:
            Request body dictionary.
        """
        messages = self._convert_messages(request.messages)

        body: dict[str, Any] = {
            "model": model,
            "messages": messages,
            "stream": stream,
        }

        if request.max_tokens:
            body["max_tokens"] = request.max_tokens

        if request.temperature is not None:
            body["temperature"] = request.temperature

        if request.stop_sequences:
            body["stop"] = request.stop_sequences

        if stream:
            body["stream_options"] = {"include_usage": True}

        return body

    async def authenticate(self) -> bool:
        """Verify API key is valid.

        Returns:
            True if authentication successful.

        Raises:
            ProviderError: If authentication fails.
        """
        if not self.config.api_key:
            raise ProviderError("Cerebras API key not configured")

        self._authenticated = True
        logger.info("cerebras_authenticated", provider=self.PROVIDER_NAME)
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
            model: Model to use.

        Returns:
            Complete agent response.

        Raises:
            ProviderError: If generation fails.
        """
        model = model or "llama3.1-70b"
        url = f"{self._base_url}/chat/completions"

        body = self._build_request_body(request, model, stream=False)

        logger.debug(
            "cerebras_generate",
            model=model,
            message_count=len(request.messages),
        )

        response = await self._make_request("POST", url, json=body)
        data = response.json()

        # Extract response
        choice = data.get("choices", [{}])[0]
        message = choice.get("message", {})
        content = message.get("content", "")
        tool_calls = message.get("tool_calls")

        # Parse usage
        usage_data = data.get("usage", {})
        usage = TokenUsage(
            prompt_tokens=usage_data.get("prompt_tokens", 0),
            completion_tokens=usage_data.get("completion_tokens", 0),
            total_tokens=usage_data.get("total_tokens", 0),
        )

        return AgentResponse(
            request_id=request.request_id,
            content=content or "",
            model=data.get("model", model),
            usage=usage,
            finish_reason=choice.get("finish_reason", "stop"),
            tool_calls=tool_calls,
            metadata={
                "cerebras_id": data.get("id"),
                # Cerebras provides timing info
                "time_info": data.get("time_info"),
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
        model = model or "llama3.1-70b"
        url = f"{self._base_url}/chat/completions"

        body = self._build_request_body(request, model, stream=True)

        logger.debug(
            "cerebras_stream",
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
        finish_reason: str | None = None
        usage: TokenUsage | None = None

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

                data_str = line[6:]

                if data_str == "[DONE]":
                    break

                try:
                    data = json.loads(data_str)
                except json.JSONDecodeError:
                    continue

                if "usage" in data:
                    usage_data = data["usage"]
                    usage = TokenUsage(
                        prompt_tokens=usage_data.get("prompt_tokens", 0),
                        completion_tokens=usage_data.get("completion_tokens", 0),
                        total_tokens=usage_data.get("total_tokens", 0),
                    )

                choices = data.get("choices", [])
                if not choices:
                    continue

                choice = choices[0]
                delta = choice.get("delta", {})
                text = delta.get("content", "")

                if choice.get("finish_reason"):
                    finish_reason = choice["finish_reason"]

                if text:
                    yield StreamChunk(
                        request_id=request.request_id,
                        chunk_index=chunk_index,
                        delta=text,
                        is_final=False,
                    )
                    chunk_index += 1

        # Final chunk
        yield StreamChunk(
            request_id=request.request_id,
            chunk_index=chunk_index,
            delta="",
            is_final=True,
            usage=usage,
            finish_reason=finish_reason or "stop",
        )

    async def list_models(self) -> list[ModelInfo]:
        """List available Cerebras models.

        Returns:
            List of ModelInfo for supported models.
        """
        return [
            ModelInfo(
                id="llama3.1-8b",
                name="Llama 3.1 8B (Cerebras)",
                provider=self.PROVIDER_NAME,
                max_tokens=8192,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=False,
                description="Llama 3.1 8B on Cerebras hardware - ultra-fast inference",
            ),
            ModelInfo(
                id="llama3.1-70b",
                name="Llama 3.1 70B (Cerebras)",
                provider=self.PROVIDER_NAME,
                max_tokens=8192,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=False,
                description="Llama 3.1 70B on Cerebras hardware - ultra-fast inference",
            ),
            ModelInfo(
                id="llama-3.3-70b",
                name="Llama 3.3 70B (Cerebras)",
                provider=self.PROVIDER_NAME,
                max_tokens=8192,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=False,
                description="Llama 3.3 70B on Cerebras hardware - ultra-fast inference",
            ),
        ]
