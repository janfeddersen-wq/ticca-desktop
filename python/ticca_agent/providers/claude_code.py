"""Claude Code provider implementation.

This provider implements the Claude Code API endpoint which requires special
handling for system prompts. The endpoint does not support custom system prompts,
so we implement prompt rewriting as specified in FEATURES.md §2.4.

CRITICAL REQUIREMENT:
When model name starts with "claude-code":
1. Original system prompt is PREPENDED to first user message
2. Agent instructions are REPLACED with fixed string:
   "You are Claude Code, Anthropic's official CLI for Claude."
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

# Claude Code API constants
CLAUDE_CODE_API_BASE = "https://api.anthropic.com"
CLAUDE_CODE_API_VERSION = "2023-06-01"
CLAUDE_CODE_FIXED_INSTRUCTION = "You are Claude Code, Anthropic's official CLI for Claude."
CLAUDE_CODE_DEFAULT_CONTEXT = 200000


class ClaudeCodeConfig(BaseModel):
    """Claude Code-specific configuration.

    Attributes:
        anthropic_version: API version string.
        default_context_length: Default context length for claude-code models.
    """

    anthropic_version: str = Field(
        default=CLAUDE_CODE_API_VERSION,
        description="Anthropic API version",
    )
    default_context_length: int = Field(
        default=CLAUDE_CODE_DEFAULT_CONTEXT,
        description="Default context length for claude-code models",
    )

    model_config = {"frozen": True}


def rewrite_request_for_claude_code(request: AgentRequest) -> AgentRequest:
    """Rewrite request for Claude Code endpoint.

    This function implements the critical prompt rewriting requirement:
    1. Extract the original system prompt
    2. Prepend it to the first user message
    3. Replace system prompt with fixed instruction

    Args:
        request: Original agent request.

    Returns:
        Rewritten request suitable for Claude Code endpoint.

    Example:
        BEFORE:
            system: "You are a helpful coding assistant."
            user: "Write a hello world program"

        AFTER:
            system: "You are Claude Code, Anthropic's official CLI for Claude."
            user: "You are a helpful coding assistant.\n\nWrite a hello world program"
    """
    messages = list(request.messages)
    original_system: str | None = None
    new_messages: list[Message] = []
    first_user_found = False

    for msg in messages:
        if msg.role == MessageRole.SYSTEM:
            # Capture original system prompt
            original_system = msg.content
            # Replace with fixed instruction
            new_messages.append(
                Message(
                    role=MessageRole.SYSTEM,
                    content=CLAUDE_CODE_FIXED_INSTRUCTION,
                    name=msg.name,
                )
            )
        elif msg.role == MessageRole.USER and not first_user_found:
            # Prepend original system to first user message
            first_user_found = True
            new_content = f"{original_system}\n\n{msg.content}" if original_system else msg.content
            new_messages.append(
                Message(
                    role=msg.role,
                    content=new_content,
                    name=msg.name,
                    tool_call_id=msg.tool_call_id,
                    tool_calls=msg.tool_calls,
                )
            )
        else:
            new_messages.append(msg)

    # If no system message was found, add the fixed one
    if not any(m.role == MessageRole.SYSTEM for m in new_messages):
        new_messages.insert(
            0,
            Message(
                role=MessageRole.SYSTEM,
                content=CLAUDE_CODE_FIXED_INSTRUCTION,
            ),
        )

    # Create new request with rewritten messages
    return AgentRequest(
        request_id=request.request_id,
        messages=new_messages,
        model=request.model,
        temperature=request.temperature,
        max_tokens=request.max_tokens,
        stop_sequences=request.stop_sequences,
        metadata=request.metadata,
    )


class ClaudeCodeProvider(BaseProvider):
    """Claude Code API provider.

    This provider uses auth_token instead of api_key and implements
    the required system prompt rewriting for the Claude Code endpoint.

    The prompt rewriting ensures that custom agent personalities work
    with the Claude Code API which doesn't support custom system prompts.

    Example:
        >>> config = ProviderConfig(
        ...     auth_token=SecretStr(os.environ["CLAUDE_CODE_ACCESS_TOKEN"]),
        ... )
        >>> async with ClaudeCodeProvider(config) as provider:
        ...     response = await provider.generate(request)
    """

    PROVIDER_NAME: ClassVar[str] = "claude_code"
    PROVIDER_TYPE: ClassVar[ProviderType] = ProviderType.CLAUDE_CODE
    SUPPORTED_MODELS: ClassVar[list[str]] = [
        "claude-code-sonnet-4",
        "claude-code-opus-4",
    ]

    def __init__(
        self,
        config: ProviderConfig,
        claude_code_config: ClaudeCodeConfig | None = None,
    ) -> None:
        """Initialize the Claude Code provider.

        Args:
            config: Base provider configuration with auth_token.
            claude_code_config: Claude Code-specific configuration.
        """
        super().__init__(config)
        self.claude_code_config = claude_code_config or ClaudeCodeConfig()
        self._base_url = config.base_url or CLAUDE_CODE_API_BASE

    def _get_default_headers(self) -> dict[str, str]:
        """Get Claude Code-specific headers."""
        headers = super()._get_default_headers()
        headers["anthropic-version"] = self.claude_code_config.anthropic_version
        return headers

    def _get_auth_headers(self) -> dict[str, str]:
        """Get authentication headers using auth_token.

        Claude Code uses auth_token (OAuth bearer token) instead of api_key.
        """
        if self.config.auth_token:
            return {"Authorization": f"Bearer {self.config.auth_token.get_secret_value()}"}
        # Fall back to api_key if auth_token not set
        if self.config.api_key:
            return {"x-api-key": self.config.api_key.get_secret_value()}
        return {}

    def _convert_messages(
        self,
        messages: list[Message],
    ) -> tuple[str | None, list[dict[str, Any]]]:
        """Convert internal messages to Anthropic format.

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

    def _build_request_body(
        self,
        request: AgentRequest,
        model: str,
        *,
        stream: bool = False,
    ) -> dict[str, Any]:
        """Build the Claude Code API request body.

        Args:
            request: Agent request (already rewritten).
            model: Model identifier.
            stream: Whether to enable streaming.

        Returns:
            Request body dictionary.
        """
        system_prompt, messages = self._convert_messages(request.messages)

        body: dict[str, Any] = {
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

        return body

    def _map_model_name(self, model: str) -> str:
        """Map claude-code model names to actual Anthropic model IDs.

        Args:
            model: Model name (potentially claude-code-prefixed).

        Returns:
            Actual Anthropic model identifier.
        """
        # Map claude-code-* to actual model names
        model_mappings = {
            "claude-code-sonnet-4": "claude-sonnet-4-20250514",
            "claude-code-opus-4": "claude-opus-4-20250514",
            "claude-code-sonnet": "claude-3-5-sonnet-20241022",
            "claude-code-haiku": "claude-3-5-haiku-20241022",
        }
        return model_mappings.get(model, model)

    async def authenticate(self) -> bool:
        """Verify authentication token is valid.

        Returns:
            True if authentication successful.

        Raises:
            ProviderError: If authentication fails.
        """
        if not self.config.auth_token and not self.config.api_key:
            raise ProviderError("Claude Code auth_token or api_key not configured")

        self._authenticated = True
        logger.info("claude_code_authenticated", provider=self.PROVIDER_NAME)
        return True

    async def generate(
        self,
        request: AgentRequest,
        *,
        model: str | None = None,
    ) -> AgentResponse:
        """Generate a complete response with prompt rewriting.

        Args:
            request: Agent request.
            model: Model to use.

        Returns:
            Complete agent response.

        Raises:
            ProviderError: If generation fails.
        """
        model = model or "claude-code-sonnet-4"

        # CRITICAL: Apply prompt rewriting for claude-code models
        if model.startswith("claude-code"):
            request = rewrite_request_for_claude_code(request)
            logger.debug("claude_code_prompt_rewritten", model=model)

        actual_model = self._map_model_name(model)
        url = f"{self._base_url}/v1/messages"

        body = self._build_request_body(request, actual_model, stream=False)

        logger.debug(
            "claude_code_generate",
            model=actual_model,
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
            model=model,  # Return original model name
            usage=usage,
            finish_reason=data.get("stop_reason", "stop"),
            tool_calls=tool_calls if tool_calls else None,
            metadata={
                "anthropic_id": data.get("id"),
                "actual_model": actual_model,
            },
        )

    async def stream(
        self,
        request: AgentRequest,
        *,
        model: str | None = None,
    ) -> AsyncIterator[StreamChunk]:
        """Stream response chunks with prompt rewriting.

        Args:
            request: Agent request.
            model: Model to use.

        Yields:
            StreamChunk objects.

        Raises:
            ProviderError: If streaming fails.
        """
        model = model or "claude-code-sonnet-4"

        # CRITICAL: Apply prompt rewriting for claude-code models
        if model.startswith("claude-code"):
            request = rewrite_request_for_claude_code(request)
            logger.debug("claude_code_prompt_rewritten_stream", model=model)

        actual_model = self._map_model_name(model)
        url = f"{self._base_url}/v1/messages"

        body = self._build_request_body(request, actual_model, stream=True)

        logger.debug(
            "claude_code_stream",
            model=actual_model,
            message_count=len(request.messages),
        )

        client = await self._get_client()
        headers = {
            **self._get_default_headers(),
            **self._get_auth_headers(),
            "Accept": "text/event-stream",
        }

        chunk_index = 0
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

                data_str = line[6:]

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
                        yield StreamChunk(
                            request_id=request.request_id,
                            chunk_index=chunk_index,
                            delta=text,
                            is_final=False,
                        )
                        chunk_index += 1

                elif event_type == "message_delta":
                    usage_data = data.get("usage", {})

                elif event_type == "message_stop":
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
        """List available Claude Code models.

        Returns:
            List of ModelInfo for supported models.
        """
        return [
            ModelInfo(
                id="claude-code-sonnet-4",
                name="Claude Code Sonnet 4",
                provider=self.PROVIDER_NAME,
                max_tokens=CLAUDE_CODE_DEFAULT_CONTEXT,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="Claude Code with Sonnet 4 (OAuth authenticated)",
            ),
            ModelInfo(
                id="claude-code-opus-4",
                name="Claude Code Opus 4",
                provider=self.PROVIDER_NAME,
                max_tokens=CLAUDE_CODE_DEFAULT_CONTEXT,
                supports_streaming=True,
                supports_tools=True,
                supports_vision=True,
                description="Claude Code with Opus 4 (OAuth authenticated)",
            ),
        ]
