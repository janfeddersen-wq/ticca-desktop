"""OpenAI provider implementation.

This module implements the OpenAI Chat Completions API with support for
GPT-4, GPT-4o, and GPT-5 models including reasoning_effort and verbosity
settings for GPT-5.
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

# OpenAI API constants
OPENAI_API_BASE = "https://api.openai.com/v1"


class OpenAIConfig(BaseModel):
    """OpenAI-specific configuration.

    Attributes:
        organization: Optional organization ID.
        reasoning_effort: GPT-5 reasoning effort (low/medium/high).
        verbosity: GPT-5 verbosity level (low/medium/high).
    """

    organization: str | None = Field(
        default=None,
        description="OpenAI organization ID",
    )
    reasoning_effort: Literal["low", "medium", "high"] | None = Field(
        default=None,
        description="GPT-5 reasoning effort level",
    )
    verbosity: Literal["low", "medium", "high"] | None = Field(
        default=None,
        description="GPT-5 verbosity level",
    )

    model_config = {"frozen": True}


class OpenAIProvider(BaseProvider):
    """OpenAI API provider.

    Supports GPT-4, GPT-4o, and GPT-5 models with tool calling,
    streaming, and GPT-5 specific parameters.

    Example:
        >>> config = ProviderConfig.from_env("OPENAI_API_KEY")
        >>> async with OpenAIProvider(config) as provider:
        ...     response = await provider.generate(request, model="gpt-4o")
    """

    PROVIDER_NAME: ClassVar[str] = "openai"
    PROVIDER_TYPE: ClassVar[ProviderType] = ProviderType.OPENAI
    SUPPORTED_MODELS: ClassVar[list[str]] = [
        "gpt-4o",
        "gpt-4o-mini",
        "gpt-4-turbo",
        "gpt-4",
        "gpt-3.5-turbo",
        "gpt-5",
        "gpt-5-turbo",
        "o1",
        "o1-mini",
        "o1-preview",
        "o3-mini",
    ]

    def __init__(
        self,
        config: ProviderConfig,
        openai_config: OpenAIConfig | None = None,
    ) -> None:
        """Initialize the OpenAI provider.

        Args:
            config: Base provider configuration.
            openai_config: OpenAI-specific configuration.
        """
        super().__init__(config)
        self.openai_config = openai_config or OpenAIConfig()
        self._base_url = config.base_url or OPENAI_API_BASE

    def _get_default_headers(self) -> dict[str, str]:
        """Get OpenAI-specific headers."""
        headers = super()._get_default_headers()
        if self.openai_config.organization:
            headers["OpenAI-Organization"] = self.openai_config.organization
        return headers

    def _convert_messages(self, messages: list[Message]) -> list[dict[str, Any]]:
        """Convert internal messages to OpenAI format.

        Args:
            messages: List of internal Message objects.

        Returns:
            List of OpenAI-format message dictionaries.
        """
        converted: list[dict[str, Any]] = []

        for msg in messages:
            openai_msg: dict[str, Any] = {
                "role": msg.role.value,
                "content": msg.content,
            }

            if msg.name:
                openai_msg["name"] = msg.name

            if msg.role == MessageRole.ASSISTANT and msg.tool_calls:
                openai_msg["tool_calls"] = msg.tool_calls

            if msg.role == MessageRole.TOOL and msg.tool_call_id:
                openai_msg["tool_call_id"] = msg.tool_call_id

            converted.append(openai_msg)

        return converted

    def _build_request_body(
        self,
        request: AgentRequest,
        model: str,
        *,
        stream: bool = False,
    ) -> dict[str, Any]:
        """Build the OpenAI API request body.

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

        # GPT-5 specific parameters
        if model.startswith("gpt-5") or model.startswith("o1") or model.startswith("o3"):
            if self.openai_config.reasoning_effort:
                body["reasoning_effort"] = self.openai_config.reasoning_effort

            # Verbosity only for non-codex models
            if self.openai_config.verbosity and "codex" not in model.lower():
                body["verbosity"] = self.openai_config.verbosity

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
            raise ProviderError("OpenAI API key not configured")

        # Verify by listing models
        try:
            url = f"{self._base_url}/models"
            await self._make_request("GET", url)
            self._authenticated = True
            logger.info("openai_authenticated", provider=self.PROVIDER_NAME)
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
            model: Model to use (defaults to gpt-4o).

        Returns:
            Complete agent response.

        Raises:
            ProviderError: If generation fails.
        """
        model = model or "gpt-4o"
        url = f"{self._base_url}/chat/completions"

        body = self._build_request_body(request, model, stream=False)

        logger.debug(
            "openai_generate",
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
                "openai_id": data.get("id"),
                "system_fingerprint": data.get("system_fingerprint"),
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
        model = model or "gpt-4o"
        url = f"{self._base_url}/chat/completions"

        body = self._build_request_body(request, model, stream=True)

        logger.debug(
            "openai_stream",
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

                # Check for usage in final chunk
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
        """List available OpenAI models.

        Returns:
            List of ModelInfo for supported models.
        """
        return [
            ModelInfo(
                id="gpt-4o",
                name="GPT-4o",
                provider=self.PROVIDER_NAME,
                max_tokens=128000,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="Most capable GPT-4 model with vision",
            ),
            ModelInfo(
                id="gpt-4o-mini",
                name="GPT-4o Mini",
                provider=self.PROVIDER_NAME,
                max_tokens=128000,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="Smaller, faster GPT-4o variant",
            ),
            ModelInfo(
                id="gpt-4-turbo",
                name="GPT-4 Turbo",
                provider=self.PROVIDER_NAME,
                max_tokens=128000,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="GPT-4 with large context window",
            ),
            ModelInfo(
                id="gpt-5",
                name="GPT-5",
                provider=self.PROVIDER_NAME,
                max_tokens=256000,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="Latest GPT model with reasoning capabilities",
            ),
            ModelInfo(
                id="o1",
                name="o1",
                provider=self.PROVIDER_NAME,
                max_tokens=200000,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="OpenAI's reasoning model",
            ),
            ModelInfo(
                id="o1-mini",
                name="o1 Mini",
                provider=self.PROVIDER_NAME,
                max_tokens=128000,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=False,
                description="Smaller o1 variant",
            ),
            ModelInfo(
                id="o3-mini",
                name="o3 Mini",
                provider=self.PROVIDER_NAME,
                max_tokens=200000,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="Fast reasoning model",
            ),
        ]
