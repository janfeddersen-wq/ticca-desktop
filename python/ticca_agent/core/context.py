"""Run context management for Ticca Agent.

This module provides the RunContext class that manages execution state,
dependencies, and configuration for agent runs. The context carries
all necessary information through the execution pipeline.

The RunContext pattern enables:
- Dependency injection for providers, tools, and services
- Request-scoped state management
- Cancellation and timeout handling
- Observability with tracing and logging
- Thread-safe access to shared resources

Example:
    >>> from ticca_agent.core import RunContext
    >>> async with RunContext.create(session_id="abc123") as ctx:
    ...     result = await agent.run("Hello!", context=ctx)
    ...     print(f"Tokens used: {ctx.total_tokens}")

"""

from __future__ import annotations

import asyncio
from contextlib import asynccontextmanager
from datetime import datetime
from typing import TYPE_CHECKING, Any, Self
from uuid import UUID, uuid4

from pydantic import BaseModel, Field

if TYPE_CHECKING:
    from collections.abc import AsyncIterator

    from ticca_agent.session.manager import Session


class ContextConfig(BaseModel):
    """Configuration for a RunContext.

    Attributes:
        timeout: Maximum execution time in seconds.
        max_tokens: Token budget for the run.
        enable_tracing: Whether to enable distributed tracing.
        metadata: Additional context metadata.
    """

    timeout: float = Field(
        default=300.0,
        ge=1.0,
        le=3600.0,
        description="Maximum execution time in seconds",
    )
    max_tokens: int | None = Field(
        default=None,
        ge=1,
        description="Token budget for the entire run",
    )
    enable_tracing: bool = Field(
        default=True,
        description="Whether to enable distributed tracing",
    )
    metadata: dict[str, Any] = Field(
        default_factory=dict,
        description="Additional context metadata",
    )

    model_config = {"frozen": True}


class RunContext:
    """Execution context for agent runs.

    The RunContext manages all state and dependencies needed during
    an agent execution. It provides:
    - Unique run identification
    - Session association
    - Token usage tracking
    - Cancellation support
    - Scoped dependency access

    Contexts should be created using the async context manager pattern
    to ensure proper cleanup.

    Attributes:
        run_id: Unique identifier for this run.
        session_id: Optional session this run belongs to.
        config: Context configuration.
        created_at: When the context was created.
    """

    def __init__(
        self,
        *,
        run_id: UUID | None = None,
        session_id: str | None = None,
        session: Session | None = None,
        config: ContextConfig | None = None,
    ) -> None:
        """Initialize the RunContext.

        Args:
            run_id: Unique run identifier (auto-generated if not provided).
            session_id: Optional session ID for the run.
            session: Optional session object.
            config: Context configuration.
        """
        self._run_id = run_id or uuid4()
        self._session_id = session_id
        self._session = session
        self._config = config or ContextConfig()
        self._created_at = datetime.utcnow()

        # Mutable state - tracking during execution
        self._prompt_tokens: int = 0
        self._completion_tokens: int = 0
        self._tool_calls: int = 0
        self._cancelled: bool = False
        self._cancel_event: asyncio.Event = asyncio.Event()
        self._dependencies: dict[str, Any] = {}

        # TODO: Initialize tracing span if enabled
        # TODO: Set up logging context

    @property
    def run_id(self) -> UUID:
        """Get the unique run identifier."""
        return self._run_id

    @property
    def session_id(self) -> str | None:
        """Get the associated session ID."""
        return self._session_id

    @property
    def session(self) -> Session | None:
        """Get the associated session object."""
        return self._session

    @property
    def config(self) -> ContextConfig:
        """Get the context configuration."""
        return self._config

    @property
    def created_at(self) -> datetime:
        """Get the context creation timestamp."""
        return self._created_at

    @property
    def prompt_tokens(self) -> int:
        """Get the total prompt tokens used."""
        return self._prompt_tokens

    @property
    def completion_tokens(self) -> int:
        """Get the total completion tokens used."""
        return self._completion_tokens

    @property
    def total_tokens(self) -> int:
        """Get the total tokens used (prompt + completion)."""
        return self._prompt_tokens + self._completion_tokens

    @property
    def tool_calls(self) -> int:
        """Get the number of tool calls made."""
        return self._tool_calls

    @property
    def is_cancelled(self) -> bool:
        """Check if the context has been cancelled."""
        return self._cancelled

    def record_usage(
        self,
        prompt_tokens: int = 0,
        completion_tokens: int = 0,
    ) -> None:
        """Record token usage for this run.

        Args:
            prompt_tokens: Number of prompt tokens to add.
            completion_tokens: Number of completion tokens to add.
        """
        self._prompt_tokens += prompt_tokens
        self._completion_tokens += completion_tokens

        # TODO: Check against max_tokens budget if configured
        # TODO: Emit usage event for monitoring

    def record_tool_call(self) -> None:
        """Record a tool call for this run."""
        self._tool_calls += 1

    def cancel(self) -> None:
        """Cancel this run.

        This sets the cancelled flag and signals any waiting operations
        to terminate gracefully.
        """
        self._cancelled = True
        self._cancel_event.set()

    async def wait_for_cancel(self) -> None:
        """Wait until the context is cancelled.

        This is useful for background tasks that should terminate
        when the run is cancelled.
        """
        await self._cancel_event.wait()

    def check_cancelled(self) -> None:
        """Check if cancelled and raise if so.

        Raises:
            asyncio.CancelledError: If the context has been cancelled.
        """
        if self._cancelled:
            raise asyncio.CancelledError("Run context was cancelled")

    def set_dependency(self, key: str, value: Any) -> None:
        """Set a dependency in the context.

        Args:
            key: Dependency identifier.
            value: Dependency value.
        """
        self._dependencies[key] = value

    def get_dependency(self, key: str, default: Any = None) -> Any:
        """Get a dependency from the context.

        Args:
            key: Dependency identifier.
            default: Default value if not found.

        Returns:
            The dependency value or default.
        """
        return self._dependencies.get(key, default)

    @classmethod
    @asynccontextmanager
    async def create(
        cls,
        *,
        session_id: str | None = None,
        session: Session | None = None,
        config: ContextConfig | None = None,
        **kwargs: Any,
    ) -> AsyncIterator[Self]:
        """Create a RunContext as an async context manager.

        This is the recommended way to create contexts, as it ensures
        proper setup and cleanup.

        Args:
            session_id: Optional session ID.
            session: Optional session object.
            config: Context configuration.
            **kwargs: Additional arguments passed to constructor.

        Yields:
            Configured RunContext instance.
        """
        context = cls(
            session_id=session_id,
            session=session,
            config=config,
            **kwargs,
        )

        try:
            # TODO: Start tracing span
            # TODO: Initialize any async resources
            yield context
        finally:
            # TODO: End tracing span with final metrics
            # TODO: Cleanup any async resources
            # TODO: Record final token usage to session
            pass

    def __repr__(self) -> str:
        """Return string representation of the context."""
        return (
            f"RunContext("
            f"run_id={self._run_id!s}, "
            f"session_id={self._session_id!r}, "
            f"tokens={self.total_tokens})"
        )


class ContextError(Exception):
    """Exception raised for context-related errors.

    Attributes:
        message: Human-readable error description.
        run_id: ID of the run that failed.
    """

    def __init__(
        self,
        message: str,
        *,
        run_id: UUID | None = None,
    ) -> None:
        """Initialize ContextError.

        Args:
            message: Human-readable error description.
            run_id: ID of the run that failed.
        """
        super().__init__(message)
        self.message = message
        self.run_id = run_id


__all__ = [
    "ContextConfig",
    "ContextError",
    "RunContext",
]
