"""Google Gemini provider implementation.

This module implements the Google Gemini API for generative AI models.
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

# Gemini API constants
GEMINI_API_BASE = "https://generativelanguage.googleapis.com/v1beta"


class GeminiConfig(BaseModel):
    """Gemini-specific configuration.

    Attributes:
        safety_settings: Safety threshold settings.
        generation_config: Generation parameters.
    """

    safety_settings: list[dict[str, str]] | None = Field(
        default=None,
        description="Safety threshold settings",
    )
    generation_config: dict[str, Any] | None = Field(
        default=None,
        description="Default generation config",
    )

    model_config = {"frozen": True}


class GeminiProvider(BaseProvider):
    """Google Gemini API provider.

    Supports Gemini Pro, Gemini Flash, and other Gemini models.

    Example:
        >>> config = ProviderConfig.from_env("GEMINI_API_KEY")
        >>> async with GeminiProvider(config) as provider:
        ...     response = await provider.generate(request, model="gemini-1.5-pro")
    """

    PROVIDER_NAME: ClassVar[str] = "gemini"
    PROVIDER_TYPE: ClassVar[ProviderType] = ProviderType.GEMINI
    SUPPORTED_MODELS: ClassVar[list[str]] = [
        "gemini-1.5-pro",
        "gemini-1.5-flash",
        "gemini-1.5-flash-8b",
        "gemini-2.0-flash-exp",
        "gemini-2.0-pro-exp",
        "gemini-exp-1206",
    ]

    def __init__(
        self,
        config: ProviderConfig,
        gemini_config: GeminiConfig | None = None,
    ) -> None:
        """Initialize the Gemini provider.

        Args:
            config: Base provider configuration.
            gemini_config: Gemini-specific configuration.
        """
        super().__init__(config)
        self.gemini_config = gemini_config or GeminiConfig()
        self._base_url = config.base_url or GEMINI_API_BASE

    def _get_auth_headers(self) -> dict[str, str]:
        """Get authentication headers for Gemini API.

        Returns:
            Headers dict with API key if configured.
        """
        if self.config.api_key:
            return {"x-goog-api-key": self.config.api_key.get_secret_value()}
        return {}

    def _build_url(self, model: str, *, stream: bool = False) -> str:
        """Build Gemini API URL.

        Args:
            model: Model identifier.
            stream: Whether to use streaming endpoint.

        Returns:
            Full API URL (without API key - use _get_auth_headers for auth).
        """
        action = "streamGenerateContent" if stream else "generateContent"
        return f"{self._base_url}/models/{model}:{action}"

    def _convert_messages(
        self,
        messages: list[Message],
    ) -> tuple[str | None, list[dict[str, Any]]]:
        """Convert internal messages to Gemini format.

        Gemini uses 'user' and 'model' roles with 'parts' structure.

        Args:
            messages: List of internal Message objects.

        Returns:
            Tuple of (system_instruction, contents).
        """
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
            elif msg.role == MessageRole.TOOL:
                # Tool results in Gemini format
                contents.append(
                    {
                        "role": "user",
                        "parts": [
                            {
                                "functionResponse": {
                                    "name": msg.name or "tool",
                                    "response": {"content": msg.content},
                                },
                            }
                        ],
                    }
                )

        return system_instruction, contents

    def _build_request_body(
        self,
        request: AgentRequest,
        *,
        _stream: bool = False,
    ) -> dict[str, Any]:
        """Build the Gemini API request body.

        Args:
            request: Agent request.
            stream: Whether to enable streaming.

        Returns:
            Request body dictionary.
        """
        system_instruction, contents = self._convert_messages(request.messages)

        body: dict[str, Any] = {
            "contents": contents,
        }

        if system_instruction:
            body["systemInstruction"] = {
                "parts": [{"text": system_instruction}],
            }

        # Generation config
        generation_config: dict[str, Any] = {}

        if request.max_tokens:
            generation_config["maxOutputTokens"] = request.max_tokens

        if request.temperature is not None:
            generation_config["temperature"] = request.temperature

        if request.stop_sequences:
            generation_config["stopSequences"] = request.stop_sequences

        # Merge with default config
        if self.gemini_config.generation_config:
            generation_config = {
                **self.gemini_config.generation_config,
                **generation_config,
            }

        if generation_config:
            body["generationConfig"] = generation_config

        # Safety settings
        if self.gemini_config.safety_settings:
            body["safetySettings"] = self.gemini_config.safety_settings

        return body

    async def authenticate(self) -> bool:
        """Verify API key is valid.

        Returns:
            True if authentication successful.

        Raises:
            ProviderError: If authentication fails.
        """
        if not self.config.api_key:
            raise ProviderError("Gemini API key not configured")

        self._authenticated = True
        logger.info("gemini_authenticated", provider=self.PROVIDER_NAME)
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
            model: Model to use (defaults to gemini-1.5-pro).

        Returns:
            Complete agent response.

        Raises:
            ProviderError: If generation fails.
        """
        model = model or "gemini-1.5-pro"
        url = self._build_url(model, stream=False)

        body = self._build_request_body(request, _stream=False)

        logger.debug(
            "gemini_generate",
            model=model,
            message_count=len(request.messages),
        )

        # Use auth headers instead of API key in URL (security best practice)
        response = await self._make_request("POST", url, json=body, headers=self._get_auth_headers())
        data = response.json()

        # Check for errors
        if "error" in data:
            error_msg = data["error"].get("message", "Unknown error")
            raise ProviderError(error_msg)

        # Extract response
        candidates = data.get("candidates", [])
        if not candidates:
            raise ProviderError("No response candidates returned")

        candidate = candidates[0]
        content_parts = candidate.get("content", {}).get("parts", [])
        content = "".join(part.get("text", "") for part in content_parts)

        # Parse usage
        usage_data = data.get("usageMetadata", {})
        usage = TokenUsage(
            prompt_tokens=usage_data.get("promptTokenCount", 0),
            completion_tokens=usage_data.get("candidatesTokenCount", 0),
            total_tokens=usage_data.get("totalTokenCount", 0),
        )

        # Map finish reason
        finish_reason_map = {
            "STOP": "stop",
            "MAX_TOKENS": "length",
            "SAFETY": "content_filter",
            "RECITATION": "content_filter",
        }
        gemini_finish_reason = candidate.get("finishReason", "STOP")
        finish_reason = finish_reason_map.get(gemini_finish_reason, "stop")

        return AgentResponse(
            request_id=request.request_id,
            content=content,
            model=model,
            usage=usage,
            finish_reason=finish_reason,
            metadata={
                "gemini_finish_reason": gemini_finish_reason,
                "safety_ratings": candidate.get("safetyRatings"),
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
        model = model or "gemini-1.5-pro"
        url = self._build_url(model, stream=True) + "?alt=sse"

        body = self._build_request_body(request, _stream=True)

        logger.debug(
            "gemini_stream",
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

                try:
                    data = json.loads(data_str)
                except json.JSONDecodeError:
                    continue

                # Check for errors
                if "error" in data:
                    error_msg = data["error"].get("message", "Unknown error")
                    raise ProviderError(f"Stream error: {error_msg}")

                candidates = data.get("candidates", [])
                if not candidates:
                    continue

                candidate = candidates[0]

                # Extract text delta
                content_parts = candidate.get("content", {}).get("parts", [])
                for part in content_parts:
                    text = part.get("text", "")
                    if text:
                        total_content += text
                        yield StreamChunk(
                            request_id=request.request_id,
                            chunk_index=chunk_index,
                            delta=text,
                            is_final=False,
                        )
                        chunk_index += 1

                # Check for finish reason
                if candidate.get("finishReason"):
                    finish_reason_map = {
                        "STOP": "stop",
                        "MAX_TOKENS": "length",
                        "SAFETY": "content_filter",
                    }
                    finish_reason = finish_reason_map.get(candidate["finishReason"], "stop")

                # Extract usage from final chunk
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
        """List available Gemini models.

        Returns:
            List of ModelInfo for supported models.
        """
        return [
            ModelInfo(
                id="gemini-1.5-pro",
                name="Gemini 1.5 Pro",
                provider=self.PROVIDER_NAME,
                max_tokens=2097152,  # 2M context
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="Most capable Gemini model with 2M context",
            ),
            ModelInfo(
                id="gemini-1.5-flash",
                name="Gemini 1.5 Flash",
                provider=self.PROVIDER_NAME,
                max_tokens=1048576,  # 1M context
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="Fast and efficient Gemini model",
            ),
            ModelInfo(
                id="gemini-1.5-flash-8b",
                name="Gemini 1.5 Flash 8B",
                provider=self.PROVIDER_NAME,
                max_tokens=1048576,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="Smallest Gemini model, fastest responses",
            ),
            ModelInfo(
                id="gemini-2.0-flash-exp",
                name="Gemini 2.0 Flash (Experimental)",
                provider=self.PROVIDER_NAME,
                max_tokens=1048576,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="Next-gen Gemini Flash model",
            ),
            ModelInfo(
                id="gemini-2.0-pro-exp",
                name="Gemini 2.0 Pro (Experimental)",
                provider=self.PROVIDER_NAME,
                max_tokens=2097152,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="Next-gen Gemini Pro model",
            ),
        ]
