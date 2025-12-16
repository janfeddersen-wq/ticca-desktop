"""Custom provider implementation for compatible endpoints.

Supports OpenAI-compatible, Anthropic-compatible, and Gemini-compatible
custom endpoints with environment variable expansion for API keys.
"""

from __future__ import annotations

import json
from enum import Enum
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
    expand_env_var,
)

if TYPE_CHECKING:
    from collections.abc import AsyncIterator

logger = structlog.get_logger(__name__)


class CustomAPIFormat(str, Enum):
    """Supported API formats for custom endpoints."""

    OPENAI = "openai"
    ANTHROPIC = "anthropic"
    GEMINI = "gemini"


class CustomConfig(BaseModel):
    """Custom endpoint configuration.

    Attributes:
        api_format: The API format (openai/anthropic/gemini).
        api_key_env: Environment variable name for API key (with $ prefix).
        model_id: Model identifier to use.
        custom_headers: Additional headers (supports env var expansion).
    """

    api_format: CustomAPIFormat = Field(
        default=CustomAPIFormat.OPENAI,
        description="API format for the custom endpoint",
    )
    api_key_env: str | None = Field(
        default=None,
        description="Environment variable for API key (e.g., $CUSTOM_API_KEY)",
    )
    model_id: str = Field(
        default="default",
        description="Model identifier",
    )
    custom_headers: dict[str, str] = Field(
        default_factory=dict,
        description="Additional headers (supports $ENV_VAR expansion)",
    )

    model_config = {"frozen": True}


class CustomProvider(BaseProvider):
    """Custom endpoint provider.

    Supports OpenAI-compatible, Anthropic-compatible, and Gemini-compatible
    endpoints with flexible configuration.

    Environment variable expansion is performed at model creation time,
    not at config load time.

    Example:
        >>> config = ProviderConfig(base_url="https://my-llm.example.com/v1")
        >>> custom_config = CustomConfig(
        ...     api_format=CustomAPIFormat.OPENAI,
        ...     api_key_env="$MY_CUSTOM_KEY",
        ... )
        >>> async with CustomProvider(config, custom_config) as provider:
        ...     response = await provider.generate(request)
    """

    PROVIDER_NAME: ClassVar[str] = "custom"
    PROVIDER_TYPE: ClassVar[ProviderType | None] = None
    SUPPORTED_MODELS: ClassVar[list[str]] = []

    def __init__(
        self,
        config: ProviderConfig,
        custom_config: CustomConfig | None = None,
    ) -> None:
        """Initialize the custom provider.

        Args:
            config: Base provider configuration.
            custom_config: Custom endpoint configuration.
        """
        super().__init__(config)
        self.custom_config = custom_config or CustomConfig()

        # Track actual provider type based on API format
        if self.custom_config.api_format == CustomAPIFormat.OPENAI:
            self._actual_provider_type = ProviderType.CUSTOM_OPENAI
        elif self.custom_config.api_format == CustomAPIFormat.ANTHROPIC:
            self._actual_provider_type = ProviderType.CUSTOM_ANTHROPIC
        else:
            self._actual_provider_type = ProviderType.CUSTOM_GEMINI

        # Expand API key from environment variable at creation time
        self._resolved_api_key: str | None = None
        if self.custom_config.api_key_env:
            self._resolved_api_key = expand_env_var(self.custom_config.api_key_env)
        elif config.api_key:
            self._resolved_api_key = config.api_key.get_secret_value()

        # Expand custom headers
        self._resolved_headers: dict[str, str] = {}
        for key, value in self.custom_config.custom_headers.items():
            self._resolved_headers[key] = expand_env_var(value)

    def _get_auth_headers(self) -> dict[str, str]:
        """Get authentication headers based on API format."""
        if not self._resolved_api_key:
            return {}

        if self.custom_config.api_format == CustomAPIFormat.ANTHROPIC:
            return {"x-api-key": self._resolved_api_key}
        else:
            return {"Authorization": f"Bearer {self._resolved_api_key}"}

    def _get_default_headers(self) -> dict[str, str]:
        """Get default headers including custom headers."""
        headers = super()._get_default_headers()
        headers.update(self._resolved_headers)

        # Add format-specific headers
        if self.custom_config.api_format == CustomAPIFormat.ANTHROPIC:
            headers["anthropic-version"] = "2023-06-01"

        return headers

    def _convert_messages_openai(
        self,
        messages: list[Message],
    ) -> list[dict[str, Any]]:
        """Convert messages to OpenAI format."""
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

    def _convert_messages_anthropic(
        self,
        messages: list[Message],
    ) -> tuple[str | None, list[dict[str, Any]]]:
        """Convert messages to Anthropic format."""
        system_prompt: str | None = None
        converted: list[dict[str, Any]] = []

        for msg in messages:
            if msg.role == MessageRole.SYSTEM:
                system_prompt = msg.content
            elif msg.role == MessageRole.USER:
                converted.append(
                    {
                        "role": "user",
                        "content": [{"type": "text", "text": msg.content}],
                    }
                )
            elif msg.role == MessageRole.ASSISTANT:
                converted.append(
                    {
                        "role": "assistant",
                        "content": [{"type": "text", "text": msg.content}],
                    }
                )
            elif msg.role == MessageRole.TOOL:
                converted.append(
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

        return system_prompt, converted

    def _convert_messages_gemini(
        self,
        messages: list[Message],
    ) -> tuple[str | None, list[dict[str, Any]]]:
        """Convert messages to Gemini format."""
        system_instruction: str | None = None
        contents: list[dict[str, Any]] = []

        for msg in messages:
            if msg.role == MessageRole.SYSTEM:
                system_instruction = msg.content
            elif msg.role == MessageRole.USER:
                contents.append(
                    {
                        "role": "user",
                        "parts": [{"text": msg.content}],
                    }
                )
            elif msg.role == MessageRole.ASSISTANT:
                contents.append(
                    {
                        "role": "model",
                        "parts": [{"text": msg.content}],
                    }
                )

        return system_instruction, contents

    def _build_request_body(
        self,
        request: AgentRequest,
        model: str,
        *,
        stream: bool = False,
    ) -> dict[str, Any]:
        """Build request body based on API format."""
        api_format = self.custom_config.api_format

        if api_format == CustomAPIFormat.OPENAI:
            messages = self._convert_messages_openai(request.messages)
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

        elif api_format == CustomAPIFormat.ANTHROPIC:
            system_prompt, messages = self._convert_messages_anthropic(request.messages)
            body = {
                "model": model,
                "messages": messages,
                "stream": stream,
            }
            if system_prompt:
                body["system"] = system_prompt
            if request.max_tokens:
                body["max_tokens"] = request.max_tokens
            else:
                body["max_tokens"] = 4096
            if request.temperature is not None:
                body["temperature"] = request.temperature
            if request.stop_sequences:
                body["stop_sequences"] = request.stop_sequences

        else:  # Gemini
            system_instruction, contents = self._convert_messages_gemini(request.messages)
            body = {"contents": contents}
            if system_instruction:
                body["systemInstruction"] = {
                    "parts": [{"text": system_instruction}],
                }
            generation_config: dict[str, Any] = {}
            if request.max_tokens:
                generation_config["maxOutputTokens"] = request.max_tokens
            if request.temperature is not None:
                generation_config["temperature"] = request.temperature
            if request.stop_sequences:
                generation_config["stopSequences"] = request.stop_sequences
            if generation_config:
                body["generationConfig"] = generation_config

        return body

    def _get_url(self, stream: bool = False) -> str:
        """Get the API URL based on format."""
        base_url = self.config.base_url or ""

        if self.custom_config.api_format == CustomAPIFormat.OPENAI:
            return f"{base_url}/chat/completions"
        elif self.custom_config.api_format == CustomAPIFormat.ANTHROPIC:
            return f"{base_url}/v1/messages"
        else:  # Gemini
            model = self.custom_config.model_id
            action = "streamGenerateContent" if stream else "generateContent"
            return f"{base_url}/models/{model}:{action}"

    async def authenticate(self) -> bool:
        """Verify configuration is valid.

        Returns:
            True if configuration seems valid.

        Raises:
            ProviderError: If configuration is invalid.
        """
        if not self.config.base_url:
            raise ProviderError("Custom endpoint base_url not configured")

        self._authenticated = True
        logger.info(
            "custom_authenticated",
            provider=self.PROVIDER_NAME,
            api_format=self.custom_config.api_format.value,
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
        model = model or self.custom_config.model_id
        url = self._get_url(stream=False)

        body = self._build_request_body(request, model, stream=False)

        logger.debug(
            "custom_generate",
            model=model,
            api_format=self.custom_config.api_format.value,
            message_count=len(request.messages),
        )

        response = await self._make_request("POST", url, json=body)
        data = response.json()

        # Parse response based on format
        api_format = self.custom_config.api_format

        if api_format == CustomAPIFormat.OPENAI:
            choice = data.get("choices", [{}])[0]
            message = choice.get("message", {})
            content = message.get("content", "")
            tool_calls = message.get("tool_calls")
            finish_reason = choice.get("finish_reason", "stop")
            usage_data = data.get("usage", {})
            usage = TokenUsage(
                prompt_tokens=usage_data.get("prompt_tokens", 0),
                completion_tokens=usage_data.get("completion_tokens", 0),
                total_tokens=usage_data.get("total_tokens", 0),
            )

        elif api_format == CustomAPIFormat.ANTHROPIC:
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
            tool_calls = tool_calls if tool_calls else None
            finish_reason = data.get("stop_reason", "stop")
            usage_data = data.get("usage", {})
            usage = TokenUsage(
                prompt_tokens=usage_data.get("input_tokens", 0),
                completion_tokens=usage_data.get("output_tokens", 0),
                total_tokens=(
                    usage_data.get("input_tokens", 0) + usage_data.get("output_tokens", 0)
                ),
            )

        else:  # Gemini
            candidates = data.get("candidates", [])
            if not candidates:
                raise ProviderError("No response candidates")
            candidate = candidates[0]
            content_parts = candidate.get("content", {}).get("parts", [])
            content = "".join(part.get("text", "") for part in content_parts)
            tool_calls = None
            finish_reason = "stop"
            usage_data = data.get("usageMetadata", {})
            usage = TokenUsage(
                prompt_tokens=usage_data.get("promptTokenCount", 0),
                completion_tokens=usage_data.get("candidatesTokenCount", 0),
                total_tokens=usage_data.get("totalTokenCount", 0),
            )

        return AgentResponse(
            request_id=request.request_id,
            content=content or "",
            model=model,
            usage=usage,
            finish_reason=finish_reason,
            tool_calls=tool_calls,
            metadata={
                "api_format": api_format.value,
                "custom_endpoint": self.config.base_url,
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
        model = model or self.custom_config.model_id
        url = self._get_url(stream=True)

        body = self._build_request_body(request, model, stream=True)

        logger.debug(
            "custom_stream",
            model=model,
            api_format=self.custom_config.api_format.value,
            message_count=len(request.messages),
        )

        client = await self._get_client()
        headers = {
            **self._get_default_headers(),
            **self._get_auth_headers(),
            "Accept": "text/event-stream",
        }

        api_format = self.custom_config.api_format
        chunk_index = 0
        usage: TokenUsage | None = None
        finish_reason: str | None = None

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
                if not line:
                    continue

                # Handle SSE format
                if line.startswith("data: "):
                    data_str = line[6:]
                    if data_str == "[DONE]":
                        break
                else:
                    continue

                try:
                    data = json.loads(data_str)
                except json.JSONDecodeError:
                    continue

                # Parse based on format
                if api_format == CustomAPIFormat.OPENAI:
                    if "usage" in data:
                        usage_data = data["usage"]
                        usage = TokenUsage(
                            prompt_tokens=usage_data.get("prompt_tokens", 0),
                            completion_tokens=usage_data.get("completion_tokens", 0),
                            total_tokens=usage_data.get("total_tokens", 0),
                        )
                    choices = data.get("choices", [])
                    if choices:
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

                elif api_format == CustomAPIFormat.ANTHROPIC:
                    event_type = data.get("type")
                    if event_type == "content_block_delta":
                        delta = data.get("delta", {})
                        if delta.get("type") == "text_delta":
                            text = delta.get("text", "")
                            if text:
                                yield StreamChunk(
                                    request_id=request.request_id,
                                    chunk_index=chunk_index,
                                    delta=text,
                                    is_final=False,
                                )
                                chunk_index += 1
                    elif event_type == "message_delta":
                        usage_data = data.get("usage", {})
                        if usage_data:
                            usage = TokenUsage(
                                prompt_tokens=usage_data.get("input_tokens", 0),
                                completion_tokens=usage_data.get("output_tokens", 0),
                                total_tokens=(
                                    usage_data.get("input_tokens", 0)
                                    + usage_data.get("output_tokens", 0)
                                ),
                            )
                        finish_reason = data.get("delta", {}).get("stop_reason", "stop")

                else:  # Gemini
                    candidates = data.get("candidates", [])
                    if candidates:
                        candidate = candidates[0]
                        content_parts = candidate.get("content", {}).get("parts", [])
                        for part in content_parts:
                            text = part.get("text", "")
                            if text:
                                yield StreamChunk(
                                    request_id=request.request_id,
                                    chunk_index=chunk_index,
                                    delta=text,
                                    is_final=False,
                                )
                                chunk_index += 1
                    if "usageMetadata" in data:
                        usage_data = data["usageMetadata"]
                        usage = TokenUsage(
                            prompt_tokens=usage_data.get("promptTokenCount", 0),
                            completion_tokens=usage_data.get("candidatesTokenCount", 0),
                            total_tokens=usage_data.get("totalTokenCount", 0),
                        )

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
        """List available models.

        Returns:
            List with the configured model.
        """
        return [
            ModelInfo(
                id=self.custom_config.model_id,
                name=f"Custom: {self.custom_config.model_id}",
                provider=self.PROVIDER_NAME,
                max_tokens=8192,
                supports_streaming=True,
                supports_tools=self.custom_config.api_format != CustomAPIFormat.GEMINI,
                supports_vision=False,
                description=f"Custom endpoint ({self.custom_config.api_format.value} format)",
            )
        ]
