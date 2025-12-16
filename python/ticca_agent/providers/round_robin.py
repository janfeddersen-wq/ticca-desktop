"""Round-robin load balancing provider.

Distributes requests across multiple models/providers for load balancing
and redundancy.
"""

from __future__ import annotations

import itertools
import random
from typing import TYPE_CHECKING, ClassVar

import structlog
from pydantic import BaseModel, Field

from ticca_agent.core.types import (
    AgentRequest,
    AgentResponse,
    ModelInfo,
    StreamChunk,
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


class ModelTarget(BaseModel):
    """A target model in the round-robin pool.

    Attributes:
        provider: Provider name/type.
        model: Model identifier.
        weight: Relative weight (higher = more requests).
        enabled: Whether this target is active.
    """

    provider: str = Field(description="Provider name")
    model: str = Field(description="Model identifier")
    weight: int = Field(default=1, ge=1, le=100, description="Relative weight")
    enabled: bool = Field(default=True, description="Whether target is active")

    model_config = {"frozen": True}


class RoundRobinConfig(BaseModel):
    """Round-robin configuration.

    Attributes:
        targets: List of model targets to balance across.
        strategy: Load balancing strategy (round_robin, weighted, random).
        failover: Whether to failover to next target on error.
        max_failover_attempts: Maximum failover attempts before giving up.
    """

    targets: list[ModelTarget] = Field(
        default_factory=list,
        description="Model targets for load balancing",
    )
    strategy: str = Field(
        default="round_robin",
        description="Load balancing strategy",
    )
    failover: bool = Field(
        default=True,
        description="Enable failover on errors",
    )
    max_failover_attempts: int = Field(
        default=3,
        ge=1,
        le=10,
        description="Maximum failover attempts",
    )

    model_config = {"frozen": True}


class RoundRobinProvider(BaseProvider):
    """Round-robin load balancing across multiple models.

    Distributes requests across configured targets using various
    strategies: round-robin, weighted, or random selection.

    Supports automatic failover when targets fail.

    Example:
        >>> config = ProviderConfig()
        >>> rr_config = RoundRobinConfig(targets=[
        ...     ModelTarget(provider="anthropic", model="claude-3-sonnet"),
        ...     ModelTarget(provider="openai", model="gpt-4o"),
        ... ])
        >>> provider = RoundRobinProvider(config, rr_config, providers)
    """

    PROVIDER_NAME: ClassVar[str] = "round_robin"
    PROVIDER_TYPE: ClassVar[ProviderType] = ProviderType.ROUND_ROBIN
    SUPPORTED_MODELS: ClassVar[list[str]] = []

    def __init__(
        self,
        config: ProviderConfig,
        round_robin_config: RoundRobinConfig,
        providers: dict[str, BaseProvider],
    ) -> None:
        """Initialize the round-robin provider.

        Args:
            config: Base provider configuration.
            round_robin_config: Round-robin specific configuration.
            providers: Dictionary mapping provider names to provider instances.
        """
        super().__init__(config)
        self.round_robin_config = round_robin_config
        self.providers = providers

        # Filter to enabled targets
        self._active_targets = [t for t in round_robin_config.targets if t.enabled]

        # Initialize round-robin iterator
        self._reset_iterator()

    def _reset_iterator(self) -> None:
        """Reset the round-robin iterator."""
        if self.round_robin_config.strategy == "weighted":
            # Build weighted list
            weighted_targets = []
            for target in self._active_targets:
                weighted_targets.extend([target] * target.weight)
            self._iterator = itertools.cycle(weighted_targets)
        else:
            self._iterator = itertools.cycle(self._active_targets)

    def _get_next_target(self) -> ModelTarget | None:
        """Get the next target based on strategy.

        Returns:
            Next ModelTarget, or None if no targets available.
        """
        if not self._active_targets:
            return None

        if self.round_robin_config.strategy == "random":
            return random.choice(self._active_targets)

        return next(self._iterator)

    def _get_provider_for_target(self, target: ModelTarget) -> BaseProvider | None:
        """Get the provider instance for a target.

        Args:
            target: Model target.

        Returns:
            Provider instance, or None if not found.
        """
        return self.providers.get(target.provider)

    async def authenticate(self) -> bool:
        """Authenticate all underlying providers.

        Returns:
            True if at least one provider is authenticated.

        Raises:
            ProviderError: If no providers can be authenticated.
        """
        if not self._active_targets:
            raise ProviderError("No targets configured for round-robin")

        authenticated_count = 0
        for target in self._active_targets:
            provider = self._get_provider_for_target(target)
            if provider:
                try:
                    await provider.authenticate()
                    authenticated_count += 1
                except ProviderError as e:
                    logger.warning(
                        "round_robin_auth_failed",
                        provider=target.provider,
                        model=target.model,
                        error=str(e),
                    )

        if authenticated_count == 0:
            raise ProviderError("No round-robin targets could be authenticated")

        self._authenticated = True
        logger.info(
            "round_robin_authenticated",
            targets=len(self._active_targets),
            authenticated=authenticated_count,
        )
        return True

    async def generate(
        self,
        request: AgentRequest,
        *,
        model: str | None = None,  # noqa: ARG002
    ) -> AgentResponse:
        """Generate response using load-balanced target.

        Args:
            request: Agent request.
            model: Ignored (uses round-robin selection).

        Returns:
            Agent response from selected target.

        Raises:
            ProviderError: If all targets fail.
        """
        attempts = 0
        max_attempts = (
            self.round_robin_config.max_failover_attempts if self.round_robin_config.failover else 1
        )
        last_error: ProviderError | None = None

        while attempts < max_attempts:
            target = self._get_next_target()
            if not target:
                raise ProviderError("No targets available")

            provider = self._get_provider_for_target(target)
            if not provider:
                logger.warning(
                    "round_robin_provider_missing",
                    provider=target.provider,
                )
                attempts += 1
                continue

            try:
                logger.debug(
                    "round_robin_generate",
                    provider=target.provider,
                    model=target.model,
                    attempt=attempts + 1,
                )

                response = await provider.generate(request, model=target.model)

                # Add round-robin metadata
                response = AgentResponse(
                    request_id=response.request_id,
                    response_id=response.response_id,
                    content=response.content,
                    model=f"{target.provider}/{target.model}",
                    usage=response.usage,
                    finish_reason=response.finish_reason,
                    tool_calls=response.tool_calls,
                    metadata={
                        **response.metadata,
                        "round_robin_provider": target.provider,
                        "round_robin_model": target.model,
                        "round_robin_attempt": attempts + 1,
                    },
                )

                return response

            except ProviderError as e:
                last_error = e
                logger.warning(
                    "round_robin_target_failed",
                    provider=target.provider,
                    model=target.model,
                    error=str(e),
                    attempt=attempts + 1,
                )

                if not self.round_robin_config.failover:
                    raise

                attempts += 1

        raise ProviderError(
            f"All round-robin targets failed after {attempts} attempts",
            status_code=last_error.status_code if last_error else None,
        )

    async def stream(
        self,
        request: AgentRequest,
        *,
        model: str | None = None,  # noqa: ARG002
    ) -> AsyncIterator[StreamChunk]:
        """Stream response using load-balanced target.

        Args:
            request: Agent request.
            model: Ignored (uses round-robin selection).

        Yields:
            Stream chunks from selected target.

        Raises:
            ProviderError: If all targets fail.
        """
        attempts = 0
        max_attempts = (
            self.round_robin_config.max_failover_attempts if self.round_robin_config.failover else 1
        )
        last_error: ProviderError | None = None

        while attempts < max_attempts:
            target = self._get_next_target()
            if not target:
                raise ProviderError("No targets available")

            provider = self._get_provider_for_target(target)
            if not provider:
                logger.warning(
                    "round_robin_provider_missing",
                    provider=target.provider,
                )
                attempts += 1
                continue

            try:
                logger.debug(
                    "round_robin_stream",
                    provider=target.provider,
                    model=target.model,
                    attempt=attempts + 1,
                )

                async for chunk in provider.stream(request, model=target.model):
                    yield chunk

                return  # Stream completed successfully

            except ProviderError as e:
                last_error = e
                logger.warning(
                    "round_robin_stream_failed",
                    provider=target.provider,
                    model=target.model,
                    error=str(e),
                    attempt=attempts + 1,
                )

                if not self.round_robin_config.failover:
                    raise

                attempts += 1

        raise ProviderError(
            f"All round-robin targets failed streaming after {attempts} attempts",
            status_code=last_error.status_code if last_error else None,
        )

    async def list_models(self) -> list[ModelInfo]:
        """List all models available through round-robin.

        Returns:
            Combined list of models from all targets.
        """
        models: list[ModelInfo] = []

        for target in self._active_targets:
            models.append(
                ModelInfo(
                    id=f"{target.provider}/{target.model}",
                    name=f"{target.provider.title()}: {target.model}",
                    provider=self.PROVIDER_NAME,
                    max_tokens=8192,  # Default, actual varies by target
                    supports_streaming=True,
                    supports_tools=True,
                    supports_vision=False,
                    description=f"Round-robin target: {target.provider}/{target.model}",
                )
            )

        return models

    async def close(self) -> None:
        """Close all underlying providers."""
        for provider in self.providers.values():
            await provider.close()
        await super().close()
