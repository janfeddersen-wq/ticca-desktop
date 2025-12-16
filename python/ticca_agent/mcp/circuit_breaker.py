"""Circuit breaker pattern for MCP server resilience.

This module implements the circuit breaker pattern to prevent cascading
failures when MCP servers become unavailable. The circuit breaker tracks
failures and temporarily stops calls to failing servers.

Circuit States:
- CLOSED: Normal operation, all calls pass through
- OPEN: Server is failing, calls are rejected immediately
- HALF_OPEN: Testing if server has recovered

Example:
    >>> from ticca_agent.mcp.circuit_breaker import CircuitBreaker
    >>> breaker = CircuitBreaker(failure_threshold=5)
    >>> result = await breaker.call(some_async_function, arg1, arg2)

"""

from __future__ import annotations

import asyncio
import time
from enum import Enum
from typing import TYPE_CHECKING, Any, TypeVar

import structlog
from pydantic import BaseModel, ConfigDict, Field

if TYPE_CHECKING:
    from collections.abc import Awaitable, Callable

logger = structlog.get_logger(__name__)

T = TypeVar("T")


class CircuitState(str, Enum):
    """State of the circuit breaker.

    Represents the three possible states in the circuit breaker pattern.
    """

    CLOSED = "closed"
    """Normal operation - calls pass through."""

    OPEN = "open"
    """Circuit is open - calls are rejected."""

    HALF_OPEN = "half_open"
    """Testing recovery - limited calls allowed."""


class CircuitBreakerConfig(BaseModel):
    """Configuration for a circuit breaker.

    Attributes:
        failure_threshold: Number of failures before opening circuit.
        recovery_timeout: Seconds before attempting recovery.
        half_open_max_calls: Max calls allowed in half-open state.
        success_threshold: Successes needed in half-open to close.
    """

    failure_threshold: int = Field(
        default=5,
        ge=1,
        le=100,
        description="Failures before opening circuit",
    )
    recovery_timeout: float = Field(
        default=30.0,
        gt=0.0,
        le=3600.0,
        description="Seconds before attempting recovery",
    )
    half_open_max_calls: int = Field(
        default=3,
        ge=1,
        le=10,
        description="Max calls in half-open state",
    )
    success_threshold: int = Field(
        default=2,
        ge=1,
        le=10,
        description="Successes needed to close circuit",
    )

    model_config = ConfigDict(frozen=True)


class CircuitBreakerOpenError(Exception):
    """Raised when a call is rejected due to an open circuit.

    Attributes:
        message: Human-readable error message.
        circuit_name: Name of the circuit breaker.
        retry_after: Seconds until circuit might close.
    """

    def __init__(
        self,
        message: str,
        *,
        circuit_name: str | None = None,
        retry_after: float | None = None,
    ) -> None:
        """Initialize CircuitBreakerOpenError.

        Args:
            message: Error description.
            circuit_name: Name of the circuit.
            retry_after: Seconds until retry might succeed.
        """
        super().__init__(message)
        self.message = message
        self.circuit_name = circuit_name
        self.retry_after = retry_after

    def __str__(self) -> str:
        """Return string representation."""
        parts = [self.message]
        if self.circuit_name:
            parts.insert(0, f"[{self.circuit_name}]")
        if self.retry_after is not None:
            parts.append(f"(retry after {self.retry_after:.1f}s)")
        return " ".join(parts)


class CircuitBreaker:
    """Circuit breaker for protecting against cascading failures.

    The circuit breaker monitors failures and transitions between states:

    - CLOSED → OPEN: When failure_threshold consecutive failures occur
    - OPEN → HALF_OPEN: After recovery_timeout seconds
    - HALF_OPEN → CLOSED: When success_threshold successes occur
    - HALF_OPEN → OPEN: When any failure occurs

    Thread-safe via asyncio Lock.

    Example:
        >>> breaker = CircuitBreaker(name="my-server")
        >>> async def risky_operation():
        ...     return await external_call()
        >>> result = await breaker.call(risky_operation)
    """

    def __init__(
        self,
        name: str = "default",
        *,
        failure_threshold: int = 5,
        recovery_timeout: float = 30.0,
        half_open_max_calls: int = 3,
        success_threshold: int = 2,
    ) -> None:
        """Initialize the circuit breaker.

        Args:
            name: Name for logging and identification.
            failure_threshold: Failures before opening circuit.
            recovery_timeout: Seconds before attempting recovery.
            half_open_max_calls: Max calls in half-open state.
            success_threshold: Successes needed to close circuit.
        """
        self._name = name
        self._config = CircuitBreakerConfig(
            failure_threshold=failure_threshold,
            recovery_timeout=recovery_timeout,
            half_open_max_calls=half_open_max_calls,
            success_threshold=success_threshold,
        )

        # State tracking
        self._state = CircuitState.CLOSED
        self._failure_count = 0
        self._success_count = 0
        self._half_open_calls = 0
        self._last_failure_time: float | None = None
        self._opened_at: float | None = None

        # Thread safety
        self._lock = asyncio.Lock()

        # Statistics
        self._total_calls = 0
        self._total_failures = 0
        self._total_rejections = 0

    @property
    def name(self) -> str:
        """Get the circuit breaker name."""
        return self._name

    @property
    def state(self) -> CircuitState:
        """Get the current circuit state."""
        return self._state

    @property
    def failure_count(self) -> int:
        """Get current consecutive failure count."""
        return self._failure_count

    @property
    def is_closed(self) -> bool:
        """Check if circuit is closed (normal operation)."""
        return self._state == CircuitState.CLOSED

    @property
    def is_open(self) -> bool:
        """Check if circuit is open (rejecting calls)."""
        return self._state == CircuitState.OPEN

    @property
    def is_half_open(self) -> bool:
        """Check if circuit is half-open (testing recovery)."""
        return self._state == CircuitState.HALF_OPEN

    @property
    def statistics(self) -> dict[str, Any]:
        """Get circuit breaker statistics."""
        return {
            "name": self._name,
            "state": self._state.value,
            "total_calls": self._total_calls,
            "total_failures": self._total_failures,
            "total_rejections": self._total_rejections,
            "consecutive_failures": self._failure_count,
            "consecutive_successes": self._success_count,
        }

    def _should_attempt_reset(self) -> bool:
        """Check if enough time has passed to attempt reset."""
        if self._opened_at is None:
            return False
        elapsed = time.monotonic() - self._opened_at
        return elapsed >= self._config.recovery_timeout

    def _time_until_reset(self) -> float:
        """Get seconds until reset attempt is allowed."""
        if self._opened_at is None:
            return 0.0
        elapsed = time.monotonic() - self._opened_at
        remaining = self._config.recovery_timeout - elapsed
        return max(0.0, remaining)

    async def _transition_to(self, new_state: CircuitState) -> None:
        """Transition to a new state with logging."""
        old_state = self._state
        self._state = new_state

        await logger.ainfo(
            "Circuit breaker state transition",
            circuit=self._name,
            from_state=old_state.value,
            to_state=new_state.value,
            failure_count=self._failure_count,
        )

        # Reset counters based on new state
        if new_state == CircuitState.CLOSED:
            self._failure_count = 0
            self._success_count = 0
            self._opened_at = None
        elif new_state == CircuitState.OPEN:
            self._opened_at = time.monotonic()
            self._success_count = 0
            self._half_open_calls = 0
        elif new_state == CircuitState.HALF_OPEN:
            self._success_count = 0
            self._half_open_calls = 0

    async def call(
        self,
        func: Callable[..., Awaitable[T]],
        *args: Any,
        **kwargs: Any,
    ) -> T:
        """Execute a function with circuit breaker protection.

        Args:
            func: Async function to execute.
            *args: Positional arguments for the function.
            **kwargs: Keyword arguments for the function.

        Returns:
            The function's return value.

        Raises:
            CircuitBreakerOpenError: If circuit is open.
            Exception: Any exception from the function.
        """
        async with self._lock:
            self._total_calls += 1

            # Check if we should transition from OPEN to HALF_OPEN
            if self._state == CircuitState.OPEN and self._should_attempt_reset():
                await self._transition_to(CircuitState.HALF_OPEN)

            # Reject if circuit is open
            if self._state == CircuitState.OPEN:
                self._total_rejections += 1
                raise CircuitBreakerOpenError(
                    "Circuit is open",
                    circuit_name=self._name,
                    retry_after=self._time_until_reset(),
                )

            # Limit calls in half-open state
            if self._state == CircuitState.HALF_OPEN:
                if self._half_open_calls >= self._config.half_open_max_calls:
                    self._total_rejections += 1
                    raise CircuitBreakerOpenError(
                        "Half-open call limit reached",
                        circuit_name=self._name,
                        retry_after=1.0,
                    )
                self._half_open_calls += 1

        # Execute the function outside the lock
        try:
            result = await func(*args, **kwargs)
            await self.record_success()
            return result
        except Exception:
            await self.record_failure()
            raise

    async def record_success(self) -> None:
        """Record a successful call.

        In HALF_OPEN state, enough successes will close the circuit.
        In CLOSED state, resets the failure counter.
        """
        async with self._lock:
            self._failure_count = 0
            self._success_count += 1

            if (
                self._state == CircuitState.HALF_OPEN
                and self._success_count >= self._config.success_threshold
            ):
                await self._transition_to(CircuitState.CLOSED)

    async def record_failure(self) -> None:
        """Record a failed call.

        In CLOSED state, enough failures will open the circuit.
        In HALF_OPEN state, any failure reopens the circuit.
        """
        async with self._lock:
            self._total_failures += 1
            self._failure_count += 1
            self._success_count = 0
            self._last_failure_time = time.monotonic()

            if self._state == CircuitState.HALF_OPEN:
                # Any failure in half-open reopens the circuit
                await self._transition_to(CircuitState.OPEN)
            elif (
                self._state == CircuitState.CLOSED
                and self._failure_count >= self._config.failure_threshold
            ):
                await self._transition_to(CircuitState.OPEN)

    async def reset(self) -> None:
        """Manually reset the circuit breaker to closed state."""
        async with self._lock:
            await self._transition_to(CircuitState.CLOSED)
            self._failure_count = 0
            self._success_count = 0
            self._half_open_calls = 0
            self._opened_at = None

    async def force_open(self) -> None:
        """Manually open the circuit breaker."""
        async with self._lock:
            await self._transition_to(CircuitState.OPEN)

    def __repr__(self) -> str:
        """Return string representation."""
        return (
            f"CircuitBreaker("
            f"name={self._name!r}, "
            f"state={self._state.value}, "
            f"failures={self._failure_count})"
        )


class CircuitBreakerRegistry:
    """Registry for managing multiple circuit breakers.

    Provides centralized management of circuit breakers for different
    servers or services.

    Example:
        >>> registry = CircuitBreakerRegistry()
        >>> breaker = registry.get_or_create("server-1")
        >>> await breaker.call(some_function)
    """

    def __init__(
        self,
        *,
        default_failure_threshold: int = 5,
        default_recovery_timeout: float = 30.0,
    ) -> None:
        """Initialize the registry.

        Args:
            default_failure_threshold: Default failure threshold.
            default_recovery_timeout: Default recovery timeout.
        """
        self._breakers: dict[str, CircuitBreaker] = {}
        self._default_failure_threshold = default_failure_threshold
        self._default_recovery_timeout = default_recovery_timeout
        self._lock = asyncio.Lock()

    async def get_or_create(
        self,
        name: str,
        *,
        failure_threshold: int | None = None,
        recovery_timeout: float | None = None,
    ) -> CircuitBreaker:
        """Get an existing circuit breaker or create a new one.

        Args:
            name: Circuit breaker name.
            failure_threshold: Override default threshold.
            recovery_timeout: Override default timeout.

        Returns:
            The circuit breaker for the given name.
        """
        async with self._lock:
            if name not in self._breakers:
                self._breakers[name] = CircuitBreaker(
                    name=name,
                    failure_threshold=(
                        failure_threshold or self._default_failure_threshold
                    ),
                    recovery_timeout=(
                        recovery_timeout or self._default_recovery_timeout
                    ),
                )
            return self._breakers[name]

    async def get(self, name: str) -> CircuitBreaker | None:
        """Get a circuit breaker by name.

        Args:
            name: Circuit breaker name.

        Returns:
            The circuit breaker or None if not found.
        """
        return self._breakers.get(name)

    async def remove(self, name: str) -> bool:
        """Remove a circuit breaker.

        Args:
            name: Circuit breaker name.

        Returns:
            True if removed, False if not found.
        """
        async with self._lock:
            if name in self._breakers:
                del self._breakers[name]
                return True
            return False

    async def reset_all(self) -> None:
        """Reset all circuit breakers to closed state."""
        for breaker in self._breakers.values():
            await breaker.reset()

    def list_breakers(self) -> list[CircuitBreaker]:
        """List all circuit breakers."""
        return list(self._breakers.values())

    def get_all_statistics(self) -> dict[str, dict[str, Any]]:
        """Get statistics for all circuit breakers."""
        return {
            name: breaker.statistics
            for name, breaker in self._breakers.items()
        }


__all__ = [
    "CircuitBreaker",
    "CircuitBreakerConfig",
    "CircuitBreakerOpenError",
    "CircuitBreakerRegistry",
    "CircuitState",
]
