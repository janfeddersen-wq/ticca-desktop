"""Base classes for AI provider implementations.

This module defines the abstract interface that all AI providers must implement,
along with configuration and retry logic for resilient API communication.
"""

from __future__ import annotations

import asyncio
import os
import random
from abc import ABC, abstractmethod
from enum import Enum
from typing import TYPE_CHECKING, Any, ClassVar

import httpx
import structlog
from pydantic import BaseModel, Field, SecretStr

if TYPE_CHECKING:
    from collections.abc import AsyncIterator

    from ticca_agent.core.types import AgentRequest, AgentResponse, ModelInfo, StreamChunk

logger = structlog.get_logger(__name__)


class ProviderType(str, Enum):
    """Supported AI provider types.

    Each provider type represents a different API backend or
    authentication method for AI model access.
    """

    ANTHROPIC = "anthropic"
    """Direct Anthropic API with cache injection."""

    CUSTOM_ANTHROPIC = "custom_anthropic"
    """Custom Anthropic-compatible endpoints."""

    CLAUDE_CODE = "claude_code"
    """Claude Code endpoint (uses auth_token instead of api_key)."""

    OPENAI = "openai"
    """OpenAI models."""

    CUSTOM_OPENAI = "custom_openai"
    """Custom OpenAI-compatible endpoints."""

    AZURE_OPENAI = "azure_openai"
    """Azure-hosted OpenAI models."""

    GEMINI = "gemini"
    """Google Gemini models."""

    CUSTOM_GEMINI = "custom_gemini"
    """Custom Gemini-compatible endpoints."""

    GEMINI_OAUTH = "gemini_oauth"
    """OAuth-based Gemini Code Assist."""

    OPENROUTER = "openrouter"
    """OpenRouter aggregation (65+ models)."""

    CEREBRAS = "cerebras"
    """Cerebras high-speed inference."""

    ZAI_CODING = "zai_coding"
    """Z.AI coding endpoint."""

    ZAI_API = "zai_api"
    """Z.AI general API endpoint."""

    ROUND_ROBIN = "round_robin"
    """Load balancing across multiple models."""


class ProviderError(Exception):
    """Base exception for provider-related errors.

    Attributes:
        message: Human-readable error description.
        status_code: HTTP status code if applicable.
        retryable: Whether this error is safe to retry.
        retry_after: Seconds to wait before retrying (from Retry-After header).
    """

    def __init__(
        self,
        message: str,
        *,
        status_code: int | None = None,
        retryable: bool = False,
        retry_after: float | None = None,
    ) -> None:
        """Initialize ProviderError.

        Args:
            message: Human-readable error description.
            status_code: HTTP status code if applicable.
            retryable: Whether this error is safe to retry.
            retry_after: Seconds to wait before retrying.
        """
        super().__init__(message)
        self.message = message
        self.status_code = status_code
        self.retryable = retryable
        self.retry_after = retry_after

    def __str__(self) -> str:
        """Return string representation of the error."""
        if self.status_code:
            return f"[{self.status_code}] {self.message}"
        return self.message


class AuthenticationError(ProviderError):
    """Raised when authentication fails."""

    def __init__(self, message: str) -> None:
        """Initialize AuthenticationError."""
        super().__init__(message, status_code=401, retryable=False)


class RateLimitError(ProviderError):
    """Raised when rate limited by the provider."""

    def __init__(
        self,
        message: str = "Rate limit exceeded",
        *,
        retry_after: float | None = None,
    ) -> None:
        """Initialize RateLimitError."""
        super().__init__(
            message,
            status_code=429,
            retryable=True,
            retry_after=retry_after,
        )


class RetryConfig(BaseModel):
    """Configuration for exponential backoff retry logic.

    This implements truncated exponential backoff with jitter and
    Retry-After header awareness to handle rate limiting gracefully.

    Attributes:
        max_retries: Maximum number of retry attempts (default: 10).
        initial_delay: Initial delay in seconds before first retry (default: 1.0).
        max_delay: Maximum delay cap per retry in seconds (default: 60.0).
        max_total_wait: Maximum total wait time across all retries (default: 300.0).
        exponential_base: Base for exponential calculation (default: 2.0).
        jitter: Whether to add randomized jitter (default: True).
        retryable_status_codes: HTTP status codes that trigger retry.
    """

    max_retries: int = Field(default=10, ge=0, le=100)
    initial_delay: float = Field(default=1.0, ge=0.1, le=60.0)
    max_delay: float = Field(default=60.0, ge=1.0, le=300.0)
    max_total_wait: float = Field(default=300.0, ge=1.0, le=600.0)
    exponential_base: float = Field(default=2.0, ge=1.1, le=10.0)
    jitter: bool = Field(default=True)
    retryable_status_codes: frozenset[int] = Field(
        default=frozenset({429, 502, 503, 504}),
        description="HTTP status codes that should trigger a retry",
    )

    model_config = {"frozen": True}

    def calculate_delay(
        self,
        attempt: int,
        retry_after: float | None = None,
    ) -> float:
        """Calculate delay for a given retry attempt.

        Uses exponential backoff with optional jitter. Respects
        Retry-After header values when provided.

        Args:
            attempt: The current retry attempt number (0-indexed).
            retry_after: Optional Retry-After header value in seconds.

        Returns:
            Delay in seconds before the next retry.
        """
        # Use Retry-After if provided and reasonable
        if retry_after is not None and retry_after > 0:
            delay = min(retry_after, self.max_delay)
        else:
            # Exponential backoff
            delay = min(
                self.initial_delay * (self.exponential_base**attempt),
                self.max_delay,
            )

        if self.jitter:
            # Add up to 25% jitter in either direction
            jitter_range = delay * 0.25
            delay += random.uniform(-jitter_range, jitter_range)

        return max(0.1, delay)  # Ensure minimum delay

    def is_retryable(self, status_code: int) -> bool:
        """Check if a status code should trigger a retry.

        Args:
            status_code: HTTP status code to check.

        Returns:
            True if the status code is in the retryable set.
        """
        return status_code in self.retryable_status_codes


class ProviderConfig(BaseModel):
    """Configuration for an AI provider.

    This model holds all necessary configuration for connecting to and
    authenticating with an AI provider API.

    Attributes:
        api_key: Secret API key for authentication.
        auth_token: OAuth/bearer token for authentication (used by claude_code).
        base_url: Base URL for the provider's API.
        timeout: Request timeout in seconds (default: 180).
        retry_config: Configuration for retry behavior.
        http2: Enable HTTP/2 support.
        extra: Additional provider-specific configuration.
    """

    api_key: SecretStr | None = Field(
        default=None,
        description="API key for provider authentication",
    )
    auth_token: SecretStr | None = Field(
        default=None,
        description="OAuth/bearer token for authentication",
    )
    base_url: str | None = Field(
        default=None,
        description="Override base URL for the provider API",
    )
    timeout: float = Field(
        default=180.0,
        ge=1.0,
        le=600.0,
        description="Request timeout in seconds",
    )
    retry_config: RetryConfig = Field(
        default_factory=RetryConfig,
        description="Retry configuration for failed requests",
    )
    http2: bool = Field(
        default=False,
        description="Enable HTTP/2 support",
    )
    extra: dict[str, Any] = Field(
        default_factory=dict,
        description="Additional provider-specific configuration",
    )

    model_config = {"frozen": True}

    @classmethod
    def from_env(
        cls,
        api_key_env: str,
        base_url: str | None = None,
        **kwargs: Any,
    ) -> ProviderConfig:
        """Create config from environment variables.

        Args:
            api_key_env: Environment variable name for API key.
            base_url: Override base URL.
            **kwargs: Additional config parameters.

        Returns:
            ProviderConfig instance.
        """
        api_key = os.environ.get(api_key_env)
        return cls(
            api_key=SecretStr(api_key) if api_key else None,
            base_url=base_url,
            **kwargs,
        )


def expand_env_var(value: str) -> str:
    """Expand environment variable references in a string.

    Supports $VAR_NAME and ${VAR_NAME} syntax.

    Args:
        value: String potentially containing environment variable references.

    Returns:
        String with environment variables expanded.
    """
    if not value.startswith("$"):
        return value

    # Handle ${VAR_NAME} or $VAR_NAME syntax
    var_name = value[2:-1] if value.startswith("${") and value.endswith("}") else value[1:]
    return os.environ.get(var_name, "")


class BaseProvider(ABC):
    """Abstract base class for AI provider implementations.

    All AI providers (OpenAI, Anthropic, etc.) must inherit from this class
    and implement the required abstract methods. The base class provides
    common functionality like HTTP client management and retry logic.

    Class Attributes:
        PROVIDER_NAME: Unique identifier for this provider.
        PROVIDER_TYPE: The ProviderType enum value.
        SUPPORTED_MODELS: List of model identifiers this provider supports.

    Attributes:
        config: Provider configuration instance.
        _authenticated: Internal flag tracking authentication state.
        _client: HTTP client for API requests.
    """

    PROVIDER_NAME: ClassVar[str] = "base"
    PROVIDER_TYPE: ClassVar[ProviderType | None] = None
    SUPPORTED_MODELS: ClassVar[list[str]] = []

    def __init__(self, config: ProviderConfig) -> None:
        """Initialize the provider with configuration.

        Args:
            config: Provider configuration including API credentials.
        """
        self.config = config
        self._authenticated: bool = False
        self._client: httpx.AsyncClient | None = None
        self._total_wait_time: float = 0.0

    @property
    def is_authenticated(self) -> bool:
        """Check if the provider has been authenticated.

        Returns:
            True if authenticate() has been called successfully.
        """
        return self._authenticated

    async def _get_client(self) -> httpx.AsyncClient:
        """Get or create the HTTP client.

        Returns:
            Configured httpx.AsyncClient instance.
        """
        if self._client is None:
            self._client = httpx.AsyncClient(
                timeout=httpx.Timeout(self.config.timeout),
                http2=self.config.http2,
                limits=httpx.Limits(
                    max_keepalive_connections=5,
                    max_connections=10,
                ),
            )
        return self._client

    async def close(self) -> None:
        """Close the HTTP client and release resources."""
        if self._client is not None:
            await self._client.aclose()
            self._client = None

    def _get_default_headers(self) -> dict[str, str]:
        """Get default headers for API requests.

        Override in subclasses to add provider-specific headers.

        Returns:
            Dictionary of HTTP headers.
        """
        return {
            "Content-Type": "application/json",
            "Accept": "application/json",
        }

    def _get_auth_headers(self) -> dict[str, str]:
        """Get authentication headers.

        Override in subclasses for different auth methods.

        Returns:
            Dictionary of authentication headers.
        """
        headers: dict[str, str] = {}

        if self.config.api_key:
            headers["Authorization"] = f"Bearer {self.config.api_key.get_secret_value()}"

        if self.config.auth_token:
            headers["Authorization"] = f"Bearer {self.config.auth_token.get_secret_value()}"

        return headers

    @staticmethod
    def _parse_retry_after(response: httpx.Response) -> float | None:
        """Parse Retry-After header from response.

        Args:
            response: HTTP response to parse.

        Returns:
            Retry delay in seconds, or None if not present.
        """
        retry_after = response.headers.get("Retry-After")
        if retry_after is None:
            return None

        try:
            return float(retry_after)
        except ValueError:
            # Could be HTTP date format, but we'll just ignore it
            return None

    async def _make_request(
        self,
        method: str,
        url: str,
        *,
        json: dict[str, Any] | None = None,
        headers: dict[str, str] | None = None,
        stream: bool = False,
    ) -> httpx.Response:
        """Make an HTTP request with retry logic.

        Args:
            method: HTTP method (GET, POST, etc.).
            url: Full URL for the request.
            json: JSON body for the request.
            headers: Additional headers.
            stream: Whether to return a streaming response.

        Returns:
            HTTP response.

        Raises:
            ProviderError: If request fails after retries.
        """
        client = await self._get_client()
        retry_config = self.config.retry_config

        # Combine headers
        request_headers = {
            **self._get_default_headers(),
            **self._get_auth_headers(),
            **(headers or {}),
        }

        self._total_wait_time = 0.0
        last_error: ProviderError | None = None

        for attempt in range(retry_config.max_retries + 1):
            try:
                if stream:
                    # For streaming, we return the response directly
                    # The caller is responsible for handling the stream
                    response = await client.stream(
                        method,
                        url,
                        json=json,
                        headers=request_headers,
                    ).__aenter__()

                    # Check for errors
                    if response.status_code >= 400:
                        await response.aread()
                        self._handle_error_response(response)

                    return response
                else:
                    response = await client.request(
                        method,
                        url,
                        json=json,
                        headers=request_headers,
                    )

                    # Check for retryable errors
                    if retry_config.is_retryable(response.status_code):
                        retry_after = self._parse_retry_after(response)
                        raise ProviderError(
                            f"Request failed with status {response.status_code}",
                            status_code=response.status_code,
                            retryable=True,
                            retry_after=retry_after,
                        )

                    # Check for non-retryable errors
                    if response.status_code >= 400:
                        self._handle_error_response(response)

                    return response

            except httpx.TimeoutException as e:
                last_error = ProviderError(
                    f"Request timed out: {e}",
                    retryable=True,
                )
            except httpx.NetworkError as e:
                last_error = ProviderError(
                    f"Network error: {e}",
                    retryable=True,
                )
            except ProviderError as e:
                last_error = e
                if not e.retryable:
                    raise

            # Calculate and apply delay if we should retry
            if attempt < retry_config.max_retries:
                delay = retry_config.calculate_delay(
                    attempt,
                    last_error.retry_after if last_error else None,
                )

                # Check total wait time limit
                if self._total_wait_time + delay > retry_config.max_total_wait:
                    raise ProviderError(
                        f"Max total wait time ({retry_config.max_total_wait}s) exceeded",
                        status_code=last_error.status_code if last_error else None,
                        retryable=False,
                    )

                logger.warning(
                    "request_retry",
                    attempt=attempt + 1,
                    max_retries=retry_config.max_retries,
                    delay=delay,
                    error=str(last_error),
                )

                self._total_wait_time += delay
                await asyncio.sleep(delay)

        # All retries exhausted
        if last_error:
            raise ProviderError(
                f"Request failed after {retry_config.max_retries} retries: {last_error.message}",
                status_code=last_error.status_code,
                retryable=False,
            )

        raise ProviderError("Request failed unexpectedly")  # pragma: no cover

    def _handle_error_response(self, response: httpx.Response) -> None:
        """Handle error response and raise appropriate exception.

        Args:
            response: Error response to handle.

        Raises:
            ProviderError: With appropriate error details.
        """
        status_code = response.status_code

        try:
            error_data = response.json()
            message = error_data.get("error", {}).get("message", response.text)
        except Exception:
            message = response.text or f"HTTP {status_code}"

        if status_code == 401:
            raise AuthenticationError(message)
        elif status_code == 429:
            retry_after = self._parse_retry_after(response)
            raise RateLimitError(message, retry_after=retry_after)
        else:
            raise ProviderError(
                message,
                status_code=status_code,
                retryable=False,
            )

    @abstractmethod
    async def authenticate(self) -> bool:
        """Authenticate with the provider API.

        This method should validate credentials and establish any
        necessary session state for subsequent API calls.

        Returns:
            True if authentication was successful.

        Raises:
            ProviderError: If authentication fails.
        """
        ...

    @abstractmethod
    async def generate(
        self,
        request: AgentRequest,
        *,
        model: str | None = None,
    ) -> AgentResponse:
        """Generate a complete response for the given request.

        Args:
            request: The agent request containing messages and parameters.
            model: Optional model override (uses default if not specified).

        Returns:
            Complete response from the AI model.

        Raises:
            ProviderError: If generation fails after retries.
        """
        ...

    @abstractmethod
    async def stream(
        self,
        request: AgentRequest,
        *,
        model: str | None = None,
    ) -> AsyncIterator[StreamChunk]:
        """Stream response chunks for the given request.

        This method yields response tokens as they are generated,
        enabling real-time UI updates and lower perceived latency.

        Args:
            request: The agent request containing messages and parameters.
            model: Optional model override (uses default if not specified).

        Yields:
            StreamChunk objects containing partial response content.

        Raises:
            ProviderError: If streaming fails.
        """
        ...
        # This yield is needed to make this an async generator
        # Concrete implementations will yield actual chunks
        if False:  # pragma: no cover
            yield

    @abstractmethod
    async def list_models(self) -> list[ModelInfo]:
        """List available models from this provider.

        Returns:
            List of ModelInfo objects describing available models.

        Raises:
            ProviderError: If listing fails.
        """
        ...

    async def __aenter__(self) -> BaseProvider:
        """Async context manager entry."""
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: Any,
    ) -> None:
        """Async context manager exit."""
        await self.close()

    def __repr__(self) -> str:
        """Return string representation of the provider."""
        return (
            f"{self.__class__.__name__}("
            f"provider={self.PROVIDER_NAME!r}, "
            f"authenticated={self._authenticated})"
        )
