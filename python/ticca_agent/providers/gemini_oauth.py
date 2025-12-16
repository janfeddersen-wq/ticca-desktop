"""Gemini OAuth provider for Code Assist.

This module implements the OAuth-based Gemini Code Assist API which uses
a different endpoint (cloudcode-pa.googleapis.com) than the standard Gemini API.
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

# Gemini Code Assist API constants
GEMINI_CODE_ASSIST_API_BASE = "https://cloudcode-pa.googleapis.com/v1"


class GeminiOAuthConfig(BaseModel):
    """Gemini OAuth-specific configuration.

    Attributes:
        project_id: Google Cloud project ID.
        location: Cloud region (default: us-central1).
    """

    project_id: str | None = Field(
        default=None,
        description="Google Cloud project ID",
    )
    location: str = Field(
        default="us-central1",
        description="Cloud region",
    )

    model_config = {"frozen": True}


class GeminiOAuthProvider(BaseProvider):
    """Gemini OAuth-based Code Assist provider.

    Uses OAuth authentication and the cloudcode-pa.googleapis.com endpoint.

    Example:
        >>> config = ProviderConfig(auth_token=SecretStr(access_token))
        >>> oauth_config = GeminiOAuthConfig(project_id="my-project")
        >>> async with GeminiOAuthProvider(config, oauth_config) as provider:
        ...     response = await provider.generate(request)
    """

    PROVIDER_NAME: ClassVar[str] = "gemini_oauth"
    PROVIDER_TYPE: ClassVar[ProviderType] = ProviderType.GEMINI_OAUTH
    SUPPORTED_MODELS: ClassVar[list[str]] = [
        "gemini-1.5-pro",
        "gemini-1.5-flash",
        "gemini-code-assist",
    ]

    def __init__(
        self,
        config: ProviderConfig,
        oauth_config: GeminiOAuthConfig | None = None,
    ) -> None:
        """Initialize the Gemini OAuth provider.

        Args:
            config: Base provider configuration with auth_token.
            oauth_config: Gemini OAuth-specific configuration.
        """
        super().__init__(config)
        self.oauth_config = oauth_config or GeminiOAuthConfig()
        self._base_url = config.base_url or GEMINI_CODE_ASSIST_API_BASE

    def _get_auth_headers(self) -> dict[str, str]:
        """Get OAuth authentication headers."""
        if self.config.auth_token:
            return {"Authorization": f"Bearer {self.config.auth_token.get_secret_value()}"}
        return {}

    def _get_url(self, model: str, stream: bool = False) -> str:
        """Build Gemini Code Assist API URL.

        Args:
            model: Model identifier.
            stream: Whether to use streaming endpoint.

        Returns:
            Full API URL.
        """
        action = "streamGenerateContent" if stream else "generateContent"
        project_id = self.oauth_config.project_id or "default"
        location = self.oauth_config.location

        return (
            f"{self._base_url}/projects/{project_id}"
            f"/locations/{location}/publishers/google/models/{model}:{action}"
        )

    def _convert_messages(
        self,
        messages: list[Message],
    ) -> tuple[str | None, list[dict[str, Any]]]:
        """Convert internal messages to Gemini format.

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
        """Build the Gemini Code Assist API request body.

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

        if generation_config:
            body["generationConfig"] = generation_config

        return body

    async def authenticate(self) -> bool:
        """Verify OAuth token is valid.

        Returns:
            True if authentication successful.

        Raises:
            ProviderError: If authentication fails.
        """
        if not self.config.auth_token:
            raise ProviderError("Gemini OAuth token not configured")

        self._authenticated = True
        logger.info("gemini_oauth_authenticated", provider=self.PROVIDER_NAME)
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
        model = model or "gemini-1.5-pro"
        url = self._get_url(model, stream=False)

        body = self._build_request_body(request, _stream=False)

        logger.debug(
            "gemini_oauth_generate",
            model=model,
            message_count=len(request.messages),
        )

        response = await self._make_request("POST", url, json=body)
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

        return AgentResponse(
            request_id=request.request_id,
            content=content,
            model=model,
            usage=usage,
            finish_reason="stop",
            metadata={
                "gemini_finish_reason": candidate.get("finishReason"),
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
        url = self._get_url(model, stream=True) + "?alt=sse"

        body = self._build_request_body(request, _stream=True)

        logger.debug(
            "gemini_oauth_stream",
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

                if "error" in data:
                    error_msg = data["error"].get("message", "Unknown error")
                    raise ProviderError(f"Stream error: {error_msg}")

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
            finish_reason="stop",
        )

    async def list_models(self) -> list[ModelInfo]:
        """List available Gemini Code Assist models.

        Returns:
            List of ModelInfo for supported models.
        """
        return [
            ModelInfo(
                id="gemini-code-assist",
                name="Gemini Code Assist",
                provider=self.PROVIDER_NAME,
                max_tokens=1048576,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=False,
                description="Gemini Code Assist via OAuth",
            ),
            ModelInfo(
                id="gemini-1.5-pro",
                name="Gemini 1.5 Pro (OAuth)",
                provider=self.PROVIDER_NAME,
                max_tokens=2097152,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="Gemini 1.5 Pro via OAuth",
            ),
        ]
