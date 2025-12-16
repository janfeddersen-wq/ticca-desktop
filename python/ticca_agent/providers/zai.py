"""Z.AI provider implementation.

Z.AI provides coding-specialized and general API endpoints.
"""

from __future__ import annotations

import json
from typing import TYPE_CHECKING, Any, ClassVar, Literal

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

# Z.AI API constants
ZAI_CODING_API_BASE = "https://api.z.ai/v1/coding"
ZAI_GENERAL_API_BASE = "https://api.z.ai/v1"


class ZAIConfig(BaseModel):
    """Z.AI-specific configuration.

    Attributes:
        endpoint_type: Whether to use coding or general API.
    """

    endpoint_type: Literal["coding", "general"] = Field(
        default="coding",
        description="Z.AI endpoint type",
    )

    model_config = {"frozen": True}


class ZAIProvider(BaseProvider):
    """Z.AI provider.

    Supports both coding-specialized and general API endpoints.

    Example:
        >>> config = ProviderConfig.from_env("ZAI_API_KEY")
        >>> zai_config = ZAIConfig(endpoint_type="coding")
        >>> async with ZAIProvider(config, zai_config) as provider:
        ...     response = await provider.generate(request)
    """

    PROVIDER_NAME: ClassVar[str] = "zai"
    PROVIDER_TYPE: ClassVar[ProviderType] = ProviderType.ZAI_CODING
    SUPPORTED_MODELS: ClassVar[list[str]] = [
        "zai-coder",
        "zai-coder-large",
        "zai-general",
    ]

    def __init__(
        self,
        config: ProviderConfig,
        zai_config: ZAIConfig | None = None,
    ) -> None:
        """Initialize the Z.AI provider.

        Args:
            config: Base provider configuration.
            zai_config: Z.AI-specific configuration.
        """
        super().__init__(config)
        self.zai_config = zai_config or ZAIConfig()

        # Set base URL based on endpoint type
        if config.base_url:
            self._base_url = config.base_url
        elif self.zai_config.endpoint_type == "coding":
            self._base_url = ZAI_CODING_API_BASE
        else:
            self._base_url = ZAI_GENERAL_API_BASE

        # Track actual provider type
        self._actual_provider_type = (
            ProviderType.ZAI_CODING
            if self.zai_config.endpoint_type == "coding"
            else ProviderType.ZAI_API
        )

    def _convert_messages(self, messages: list[Message]) -> list[dict[str, Any]]:
        """Convert internal messages to Z.AI format.

        Args:
            messages: List of internal Message objects.

        Returns:
            List of message dictionaries.
        """
        converted: list[dict[str, Any]] = []

        for msg in messages:
            zai_msg: dict[str, Any] = {
                "role": msg.role.value,
                "content": msg.content,
            }

            if msg.name:
                zai_msg["name"] = msg.name

            if msg.role == MessageRole.ASSISTANT and msg.tool_calls:
                zai_msg["tool_calls"] = msg.tool_calls

            if msg.role == MessageRole.TOOL and msg.tool_call_id:
                zai_msg["tool_call_id"] = msg.tool_call_id

            converted.append(zai_msg)

        return converted

    def _build_request_body(
        self,
        request: AgentRequest,
        model: str,
        *,
        stream: bool = False,
    ) -> dict[str, Any]:
        """Build the Z.AI API request body.

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
            raise ProviderError("Z.AI API key not configured")

        self._authenticated = True
        logger.info(
            "zai_authenticated",
            provider=self.PROVIDER_NAME,
            endpoint=self.zai_config.endpoint_type,
        )
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
        if self.zai_config.endpoint_type == "coding":
            model = model or "zai-coder"
        else:
            model = model or "zai-general"

        url = f"{self._base_url}/chat/completions"

        body = self._build_request_body(request, model, stream=False)

        logger.debug(
            "zai_generate",
            model=model,
            endpoint=self.zai_config.endpoint_type,
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
                "zai_id": data.get("id"),
                "endpoint_type": self.zai_config.endpoint_type,
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
        if self.zai_config.endpoint_type == "coding":
            model = model or "zai-coder"
        else:
            model = model or "zai-general"

        url = f"{self._base_url}/chat/completions"

        body = self._build_request_body(request, model, stream=True)

        logger.debug(
            "zai_stream",
            model=model,
            endpoint=self.zai_config.endpoint_type,
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
        """List available Z.AI models.

        Returns:
            List of ModelInfo for supported models.
        """
        models = [
            ModelInfo(
                id="zai-coder",
                name="Z.AI Coder",
                provider=self.PROVIDER_NAME,
                max_tokens=32768,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=False,
                description="Z.AI coding-specialized model",
            ),
            ModelInfo(
                id="zai-coder-large",
                name="Z.AI Coder Large",
                provider=self.PROVIDER_NAME,
                max_tokens=65536,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=False,
                description="Z.AI large coding model",
            ),
        ]

        if self.zai_config.endpoint_type == "general":
            models.append(
                ModelInfo(
                    id="zai-general",
                    name="Z.AI General",
                    provider=self.PROVIDER_NAME,
                    max_tokens=32768,
                    supports_streaming=True,
                    supports_tools=True,
                    supports_vision=False,
                    description="Z.AI general-purpose model",
                )
            )

        return models
