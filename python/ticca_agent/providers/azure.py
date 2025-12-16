"""Azure OpenAI provider implementation.

This module implements the Azure OpenAI API which uses a different
authentication scheme and endpoint structure than the standard OpenAI API.
"""

from __future__ import annotations

import json
import os
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


class AzureOpenAIConfig(BaseModel):
    """Azure OpenAI-specific configuration.

    Attributes:
        deployment_name: Azure deployment name.
        api_version: Azure API version.
    """

    deployment_name: str | None = Field(
        default=None,
        description="Azure deployment name (required for Azure)",
    )
    api_version: str = Field(
        default="2024-02-15-preview",
        description="Azure OpenAI API version",
    )

    model_config = {"frozen": True}


class AzureOpenAIProvider(BaseProvider):
    """Azure OpenAI API provider.

    Uses AZURE_OPENAI_API_KEY and AZURE_OPENAI_ENDPOINT environment variables.

    Example:
        >>> config = ProviderConfig.from_env("AZURE_OPENAI_API_KEY")
        >>> azure_config = AzureOpenAIConfig(deployment_name="gpt-4")
        >>> async with AzureOpenAIProvider(config, azure_config) as provider:
        ...     response = await provider.generate(request)
    """

    PROVIDER_NAME: ClassVar[str] = "azure_openai"
    PROVIDER_TYPE: ClassVar[ProviderType] = ProviderType.AZURE_OPENAI
    SUPPORTED_MODELS: ClassVar[list[str]] = [
        "gpt-4o",
        "gpt-4o-mini",
        "gpt-4-turbo",
        "gpt-4",
        "gpt-35-turbo",
    ]

    def __init__(
        self,
        config: ProviderConfig,
        azure_config: AzureOpenAIConfig | None = None,
    ) -> None:
        """Initialize the Azure OpenAI provider.

        Args:
            config: Base provider configuration.
            azure_config: Azure-specific configuration.
        """
        super().__init__(config)
        self.azure_config = azure_config or AzureOpenAIConfig()

        # Get endpoint from config or environment
        self._endpoint = config.base_url or os.environ.get("AZURE_OPENAI_ENDPOINT", "")
        if self._endpoint and not self._endpoint.endswith("/"):
            self._endpoint = self._endpoint.rstrip("/")

    def _get_auth_headers(self) -> dict[str, str]:
        """Get Azure authentication headers.

        Azure uses api-key header instead of Bearer token.
        """
        if self.config.api_key:
            return {"api-key": self.config.api_key.get_secret_value()}
        return {}

    def _get_url(self, deployment: str) -> str:
        """Build Azure OpenAI API URL.

        Args:
            deployment: Deployment name.

        Returns:
            Full API URL.
        """
        api_version = self.azure_config.api_version
        return (
            f"{self._endpoint}/openai/deployments/{deployment}"
            f"/chat/completions?api-version={api_version}"
        )

    def _convert_messages(self, messages: list[Message]) -> list[dict[str, Any]]:
        """Convert internal messages to Azure OpenAI format.

        Args:
            messages: List of internal Message objects.

        Returns:
            List of Azure OpenAI-format message dictionaries.
        """
        converted: list[dict[str, Any]] = []

        for msg in messages:
            azure_msg: dict[str, Any] = {
                "role": msg.role.value,
                "content": msg.content,
            }

            if msg.name:
                azure_msg["name"] = msg.name

            if msg.role == MessageRole.ASSISTANT and msg.tool_calls:
                azure_msg["tool_calls"] = msg.tool_calls

            if msg.role == MessageRole.TOOL and msg.tool_call_id:
                azure_msg["tool_call_id"] = msg.tool_call_id

            converted.append(azure_msg)

        return converted

    def _build_request_body(
        self,
        request: AgentRequest,
        *,
        stream: bool = False,
    ) -> dict[str, Any]:
        """Build the Azure OpenAI API request body.

        Args:
            request: Agent request.
            stream: Whether to enable streaming.

        Returns:
            Request body dictionary.
        """
        messages = self._convert_messages(request.messages)

        body: dict[str, Any] = {
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
            raise ProviderError("Azure OpenAI API key not configured")

        if not self._endpoint:
            raise ProviderError("Azure OpenAI endpoint not configured")

        self._authenticated = True
        logger.info("azure_openai_authenticated", provider=self.PROVIDER_NAME)
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
            model: Deployment name (defaults to config or gpt-4o).

        Returns:
            Complete agent response.

        Raises:
            ProviderError: If generation fails.
        """
        deployment = model or self.azure_config.deployment_name or "gpt-4o"
        url = self._get_url(deployment)

        body = self._build_request_body(request, stream=False)

        logger.debug(
            "azure_openai_generate",
            deployment=deployment,
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
            model=deployment,
            usage=usage,
            finish_reason=choice.get("finish_reason", "stop"),
            tool_calls=tool_calls,
            metadata={
                "azure_id": data.get("id"),
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
            model: Deployment name.

        Yields:
            StreamChunk objects.

        Raises:
            ProviderError: If streaming fails.
        """
        deployment = model or self.azure_config.deployment_name or "gpt-4o"
        url = self._get_url(deployment)

        body = self._build_request_body(request, stream=True)

        logger.debug(
            "azure_openai_stream",
            deployment=deployment,
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
        """List available Azure OpenAI deployments.

        Returns:
            List of ModelInfo for common Azure deployments.
        """
        # Azure doesn't have a models endpoint, return common deployments
        return [
            ModelInfo(
                id="gpt-4o",
                name="GPT-4o (Azure)",
                provider=self.PROVIDER_NAME,
                max_tokens=128000,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="Azure-hosted GPT-4o",
            ),
            ModelInfo(
                id="gpt-4o-mini",
                name="GPT-4o Mini (Azure)",
                provider=self.PROVIDER_NAME,
                max_tokens=128000,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="Azure-hosted GPT-4o Mini",
            ),
            ModelInfo(
                id="gpt-4-turbo",
                name="GPT-4 Turbo (Azure)",
                provider=self.PROVIDER_NAME,
                max_tokens=128000,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="Azure-hosted GPT-4 Turbo",
            ),
        ]
