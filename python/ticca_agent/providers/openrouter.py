"""OpenRouter provider implementation.

OpenRouter is an aggregator providing access to 65+ AI models through
a unified OpenAI-compatible API.
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

# OpenRouter API constants
OPENROUTER_API_BASE = "https://openrouter.ai/api/v1"


class OpenRouterConfig(BaseModel):
    """OpenRouter-specific configuration.

    Attributes:
        site_url: Your site URL for rankings.
        app_name: Your app name for rankings.
        allow_fallbacks: Allow model fallbacks.
    """

    site_url: str | None = Field(
        default=None,
        description="Your site URL for OpenRouter rankings",
    )
    app_name: str | None = Field(
        default="Ticca Desktop",
        description="Your app name for OpenRouter rankings",
    )
    allow_fallbacks: bool = Field(
        default=True,
        description="Allow OpenRouter to fall back to other providers",
    )

    model_config = {"frozen": True}


class OpenRouterProvider(BaseProvider):
    """OpenRouter API provider.

    Provides access to 65+ AI models through a unified API.
    Uses OpenAI-compatible request format.

    Example:
        >>> config = ProviderConfig.from_env("OPENROUTER_API_KEY")
        >>> async with OpenRouterProvider(config) as provider:
        ...     response = await provider.generate(
        ...         request, model="anthropic/claude-3-sonnet"
        ...     )
    """

    PROVIDER_NAME: ClassVar[str] = "openrouter"
    PROVIDER_TYPE: ClassVar[ProviderType] = ProviderType.OPENROUTER
    SUPPORTED_MODELS: ClassVar[list[str]] = [
        "anthropic/claude-3-opus",
        "anthropic/claude-3-sonnet",
        "anthropic/claude-3-haiku",
        "anthropic/claude-3.5-sonnet",
        "openai/gpt-4o",
        "openai/gpt-4-turbo",
        "google/gemini-pro-1.5",
        "meta-llama/llama-3.1-405b-instruct",
        "mistralai/mistral-large",
    ]

    def __init__(
        self,
        config: ProviderConfig,
        openrouter_config: OpenRouterConfig | None = None,
    ) -> None:
        """Initialize the OpenRouter provider.

        Args:
            config: Base provider configuration.
            openrouter_config: OpenRouter-specific configuration.
        """
        super().__init__(config)
        self.openrouter_config = openrouter_config or OpenRouterConfig()
        self._base_url = config.base_url or OPENROUTER_API_BASE

    def _get_default_headers(self) -> dict[str, str]:
        """Get OpenRouter-specific headers."""
        headers = super()._get_default_headers()

        if self.openrouter_config.site_url:
            headers["HTTP-Referer"] = self.openrouter_config.site_url

        if self.openrouter_config.app_name:
            headers["X-Title"] = self.openrouter_config.app_name

        return headers

    def _convert_messages(self, messages: list[Message]) -> list[dict[str, Any]]:
        """Convert internal messages to OpenAI format (used by OpenRouter).

        Args:
            messages: List of internal Message objects.

        Returns:
            List of OpenAI-format message dictionaries.
        """
        converted: list[dict[str, Any]] = []

        for msg in messages:
            openrouter_msg: dict[str, Any] = {
                "role": msg.role.value,
                "content": msg.content,
            }

            if msg.name:
                openrouter_msg["name"] = msg.name

            if msg.role == MessageRole.ASSISTANT and msg.tool_calls:
                openrouter_msg["tool_calls"] = msg.tool_calls

            if msg.role == MessageRole.TOOL and msg.tool_call_id:
                openrouter_msg["tool_call_id"] = msg.tool_call_id

            converted.append(openrouter_msg)

        return converted

    def _build_request_body(
        self,
        request: AgentRequest,
        model: str,
        *,
        stream: bool = False,
    ) -> dict[str, Any]:
        """Build the OpenRouter API request body.

        Args:
            request: Agent request.
            model: Model identifier (e.g., "anthropic/claude-3-sonnet").
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

        # OpenRouter-specific options
        if not self.openrouter_config.allow_fallbacks:
            body["route"] = "fallback"

        # Enable stream options for token counting
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
            raise ProviderError("OpenRouter API key not configured")

        # Verify by getting key info
        try:
            url = f"{self._base_url}/auth/key"
            await self._make_request("GET", url)
            self._authenticated = True
            logger.info("openrouter_authenticated", provider=self.PROVIDER_NAME)
            return True
        except ProviderError:
            raise

    async def generate(
        self,
        request: AgentRequest,
        *,
        model: str | None = None,
    ) -> AgentResponse:
        """Generate a complete response.

        Args:
            request: Agent request.
            model: Model to use (e.g., "anthropic/claude-3-sonnet").

        Returns:
            Complete agent response.

        Raises:
            ProviderError: If generation fails.
        """
        model = model or "anthropic/claude-3.5-sonnet"
        url = f"{self._base_url}/chat/completions"

        body = self._build_request_body(request, model, stream=False)

        logger.debug(
            "openrouter_generate",
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
                "openrouter_id": data.get("id"),
                "openrouter_generation_id": data.get("generation_id"),
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
        model = model or "anthropic/claude-3.5-sonnet"
        url = f"{self._base_url}/chat/completions"

        body = self._build_request_body(request, model, stream=True)

        logger.debug(
            "openrouter_stream",
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

                # Check for usage
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
        """List available OpenRouter models.

        Fetches the current model list from OpenRouter API.

        Returns:
            List of ModelInfo for available models.
        """
        try:
            url = f"{self._base_url}/models"
            response = await self._make_request("GET", url)
            data = response.json()

            models = []
            for model_data in data.get("data", []):
                model_id = model_data.get("id", "")
                name = model_data.get("name", model_id)
                context_length = model_data.get("context_length", 8192)

                models.append(
                    ModelInfo(
                        id=model_id,
                        name=name,
                        provider=self.PROVIDER_NAME,
                        max_tokens=context_length,
                        supports_streaming=True,
                        supports_tools=True,
                        supports_vision="vision" in model_id.lower(),
                        description=model_data.get("description"),
                    )
                )

            return models

        except ProviderError:
            # Fall back to static list
            return [
                ModelInfo(
                    id="anthropic/claude-3.5-sonnet",
                    name="Claude 3.5 Sonnet (OpenRouter)",
                    provider=self.PROVIDER_NAME,
                    max_tokens=200000,
                    supports_streaming=True,
                    supports_tools=True,
                    supports_vision=True,
                    description="Claude 3.5 Sonnet via OpenRouter",
                ),
                ModelInfo(
                    id="openai/gpt-4o",
                    name="GPT-4o (OpenRouter)",
                    provider=self.PROVIDER_NAME,
                    max_tokens=128000,
                    supports_streaming=True,
                    supports_tools=True,
                    supports_vision=True,
                    description="GPT-4o via OpenRouter",
                ),
                ModelInfo(
                    id="google/gemini-pro-1.5",
                    name="Gemini 1.5 Pro (OpenRouter)",
                    provider=self.PROVIDER_NAME,
                    max_tokens=1048576,
                    supports_streaming=True,
                    supports_tools=True,
                    supports_vision=True,
                    description="Gemini 1.5 Pro via OpenRouter",
                ),
            ]
