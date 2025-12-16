"""Health monitoring for MCP servers.

This module provides health monitoring capabilities for MCP servers,
including periodic health checks, failure tracking, quarantine management,
and automatic recovery attempts.

Key Features:
- Background health monitoring at configurable intervals
- Failure tracking with consecutive failure counting
- Quarantine system for repeatedly failing servers
- Automatic recovery attempts for quarantined servers
- Event audit trail for monitoring and debugging

Example:
    >>> from ticca_agent.mcp.health import HealthMonitor
    >>> monitor = HealthMonitor(check_interval=30.0)
    >>> await monitor.start_monitoring(servers)
    >>> status = await monitor.check_health(server)

"""

from __future__ import annotations

import asyncio
import contextlib
import time
from collections import defaultdict
from collections.abc import Callable, Coroutine
from datetime import UTC, datetime
from typing import TYPE_CHECKING, Any

import structlog
from pydantic import BaseModel, ConfigDict, Field

from ticca_agent.mcp.models import HealthStatus, ServerEvent

if TYPE_CHECKING:
    from ticca_agent.mcp.servers.base import BaseMCPServer

logger = structlog.get_logger(__name__)


class HealthMonitorConfig(BaseModel):
    """Configuration for the health monitor.

    Attributes:
        check_interval: Seconds between health checks.
        failure_threshold: Failures before quarantine.
        recovery_interval: Seconds between recovery attempts.
        quarantine_duration: Seconds to keep server quarantined.
        health_check_timeout: Timeout for health check calls.
    """

    check_interval: float = Field(
        default=30.0,
        gt=0.0,
        le=3600.0,
        description="Seconds between health checks",
    )
    failure_threshold: int = Field(
        default=3,
        ge=1,
        le=100,
        description="Consecutive failures before quarantine",
    )
    recovery_interval: float = Field(
        default=60.0,
        gt=0.0,
        le=3600.0,
        description="Seconds between recovery attempts",
    )
    quarantine_duration: float = Field(
        default=300.0,
        gt=0.0,
        le=86400.0,
        description="Seconds to keep server quarantined",
    )
    health_check_timeout: float = Field(
        default=10.0,
        gt=0.0,
        le=60.0,
        description="Timeout for health check calls",
    )

    model_config = ConfigDict(frozen=True)


class ServerHealthState:
    """Tracks health state for a single server.

    Attributes:
        consecutive_failures: Number of consecutive failures.
        last_success: Timestamp of last successful check.
        last_failure: Timestamp of last failed check.
        last_check: Timestamp of last check (success or failure).
        is_quarantined: Whether server is in quarantine.
        quarantined_at: Timestamp when quarantine started.
        last_recovery_attempt: Timestamp of last recovery attempt.
    """

    def __init__(self) -> None:
        """Initialize server health state."""
        self.consecutive_failures: int = 0
        self.last_success: datetime | None = None
        self.last_failure: datetime | None = None
        self.last_check: datetime | None = None
        self.last_error: str | None = None
        self.is_quarantined: bool = False
        self.quarantined_at: datetime | None = None
        self.last_recovery_attempt: datetime | None = None
        self.total_failures: int = 0
        self.total_successes: int = 0

    def record_success(self) -> None:
        """Record a successful health check."""
        now = datetime.now(UTC)
        self.consecutive_failures = 0
        self.last_success = now
        self.last_check = now
        self.last_error = None
        self.total_successes += 1

    def record_failure(self, error: str) -> None:
        """Record a failed health check."""
        now = datetime.now(UTC)
        self.consecutive_failures += 1
        self.last_failure = now
        self.last_check = now
        self.last_error = error
        self.total_failures += 1

    def enter_quarantine(self) -> None:
        """Enter quarantine state."""
        self.is_quarantined = True
        self.quarantined_at = datetime.now(UTC)

    def exit_quarantine(self) -> None:
        """Exit quarantine state."""
        self.is_quarantined = False
        self.quarantined_at = None
        self.consecutive_failures = 0

    def record_recovery_attempt(self) -> None:
        """Record a recovery attempt."""
        self.last_recovery_attempt = datetime.now(UTC)

    def to_dict(self) -> dict[str, Any]:
        """Convert to dictionary for serialization."""
        return {
            "consecutive_failures": self.consecutive_failures,
            "last_success": self.last_success.isoformat() if self.last_success else None,
            "last_failure": self.last_failure.isoformat() if self.last_failure else None,
            "last_check": self.last_check.isoformat() if self.last_check else None,
            "last_error": self.last_error,
            "is_quarantined": self.is_quarantined,
            "quarantined_at": (
                self.quarantined_at.isoformat() if self.quarantined_at else None
            ),
            "total_failures": self.total_failures,
            "total_successes": self.total_successes,
        }


# Type for event callbacks
EventCallback = Callable[[ServerEvent], Coroutine[Any, Any, None]]


class HealthMonitor:
    """Health monitor for MCP servers.

    Provides background health monitoring with failure tracking,
    quarantine management, and automatic recovery.

    The monitor runs periodic health checks on all registered servers
    and tracks their health state. Servers that fail repeatedly are
    placed in quarantine to prevent cascading failures.

    Example:
        >>> monitor = HealthMonitor(
        ...     check_interval=30.0,
        ...     failure_threshold=3,
        ... )
        >>> await monitor.start_monitoring(servers)
        >>> # Later...
        >>> await monitor.stop_monitoring()
    """

    def __init__(
        self,
        check_interval: float = 30.0,
        failure_threshold: int = 3,
        recovery_interval: float = 60.0,
        quarantine_duration: float = 300.0,
        health_check_timeout: float = 10.0,
    ) -> None:
        """Initialize the health monitor.

        Args:
            check_interval: Seconds between health checks.
            failure_threshold: Failures before quarantine.
            recovery_interval: Seconds between recovery attempts.
            quarantine_duration: Seconds to keep server quarantined.
            health_check_timeout: Timeout for health check calls.
        """
        self._config = HealthMonitorConfig(
            check_interval=check_interval,
            failure_threshold=failure_threshold,
            recovery_interval=recovery_interval,
            quarantine_duration=quarantine_duration,
            health_check_timeout=health_check_timeout,
        )

        # Server tracking
        self._servers: dict[str, BaseMCPServer] = {}
        self._health_states: dict[str, ServerHealthState] = defaultdict(
            ServerHealthState
        )

        # Monitoring state
        self._monitoring_task: asyncio.Task[None] | None = None
        self._recovery_task: asyncio.Task[None] | None = None
        self._running = False

        # Event tracking
        self._events: list[ServerEvent] = []
        self._max_events = 1000
        self._event_callbacks: list[EventCallback] = []

    @property
    def is_running(self) -> bool:
        """Check if monitoring is running."""
        return self._running

    @property
    def config(self) -> HealthMonitorConfig:
        """Get the monitor configuration."""
        return self._config

    async def start_monitoring(
        self,
        servers: dict[str, BaseMCPServer],
    ) -> None:
        """Start background health monitoring.

        Args:
            servers: Dictionary of server name to server instance.
        """
        if self._running:
            await logger.awarning("Health monitor already running")
            return

        self._servers = servers
        self._running = True

        # Start monitoring tasks
        self._monitoring_task = asyncio.create_task(
            self._monitoring_loop(),
            name="health-monitor",
        )
        self._recovery_task = asyncio.create_task(
            self._recovery_loop(),
            name="health-recovery",
        )

        await logger.ainfo(
            "Health monitoring started",
            server_count=len(servers),
            check_interval=self._config.check_interval,
        )

    async def stop_monitoring(self) -> None:
        """Stop health monitoring."""
        if not self._running:
            return

        self._running = False

        # Cancel monitoring tasks
        if self._monitoring_task:
            self._monitoring_task.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await self._monitoring_task
            self._monitoring_task = None

        if self._recovery_task:
            self._recovery_task.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await self._recovery_task
            self._recovery_task = None

        await logger.ainfo("Health monitoring stopped")

    async def check_health(self, server: BaseMCPServer) -> HealthStatus:
        """Check health of a single server.

        Args:
            server: The server to check.

        Returns:
            Health status of the server.
        """
        start_time = time.monotonic()
        state = self._health_states[server.name]

        try:
            # Perform health check with timeout
            healthy = await asyncio.wait_for(
                server.health_check(),
                timeout=self._config.health_check_timeout,
            )

            latency_ms = (time.monotonic() - start_time) * 1000

            if healthy:
                state.record_success()
                return HealthStatus(
                    healthy=True,
                    latency_ms=latency_ms,
                )
            else:
                error = "Health check returned unhealthy"
                state.record_failure(error)
                return HealthStatus(
                    healthy=False,
                    latency_ms=latency_ms,
                    error=error,
                )

        except TimeoutError:
            error = f"Health check timed out after {self._config.health_check_timeout}s"
            state.record_failure(error)
            latency_ms = (time.monotonic() - start_time) * 1000
            return HealthStatus(
                healthy=False,
                latency_ms=latency_ms,
                error=error,
            )

        except Exception as e:
            error = f"Health check failed: {e}"
            state.record_failure(error)
            latency_ms = (time.monotonic() - start_time) * 1000
            return HealthStatus(
                healthy=False,
                latency_ms=latency_ms,
                error=error,
            )

    def record_failure(self, server_name: str, error: str | None = None) -> None:
        """Record a failure for a server.

        This can be called externally when operations fail.

        Args:
            server_name: Name of the server.
            error: Optional error message.
        """
        state = self._health_states[server_name]
        state.record_failure(error or "Unknown error")

        # Check if should quarantine
        if (
            state.consecutive_failures >= self._config.failure_threshold
            and not state.is_quarantined
        ):
            self._quarantine_server(server_name)

    def record_success(self, server_name: str) -> None:
        """Record a success for a server.

        This can be called externally when operations succeed.

        Args:
            server_name: Name of the server.
        """
        state = self._health_states[server_name]
        state.record_success()

        # Exit quarantine if currently quarantined and successful
        if state.is_quarantined:
            self._unquarantine_server(server_name)

    def is_quarantined(self, server_name: str) -> bool:
        """Check if a server is in quarantine.

        Args:
            server_name: Name of the server.

        Returns:
            True if server is quarantined.
        """
        return self._health_states[server_name].is_quarantined

    def get_health_state(self, server_name: str) -> ServerHealthState:
        """Get health state for a server.

        Args:
            server_name: Name of the server.

        Returns:
            The server's health state.
        """
        return self._health_states[server_name]

    def get_consecutive_failures(self, server_name: str) -> int:
        """Get consecutive failure count for a server.

        Args:
            server_name: Name of the server.

        Returns:
            Number of consecutive failures.
        """
        return self._health_states[server_name].consecutive_failures

    async def attempt_recovery(self, server_name: str) -> bool:
        """Attempt to recover a quarantined server.

        Args:
            server_name: Name of the server.

        Returns:
            True if recovery was successful.
        """
        state = self._health_states[server_name]
        if not state.is_quarantined:
            return True  # Not quarantined, nothing to do

        server = self._servers.get(server_name)
        if not server:
            await logger.awarning(
                "Cannot recover unknown server",
                server=server_name,
            )
            return False

        state.record_recovery_attempt()

        await logger.ainfo(
            "Attempting server recovery",
            server=server_name,
        )

        # Try to reconnect
        try:
            await server.disconnect()
            await server.connect()

            # Check health after reconnection
            status = await self.check_health(server)

            if status.healthy:
                self._unquarantine_server(server_name)
                await self._emit_event(
                    server_name,
                    "recovered",
                    f"Server recovered successfully after {state.consecutive_failures} failures",
                )
                return True
            else:
                await logger.awarning(
                    "Recovery health check failed",
                    server=server_name,
                    error=status.error,
                )
                return False

        except Exception as e:
            await logger.aerror(
                "Recovery attempt failed",
                server=server_name,
                error=str(e),
            )
            return False

    def add_event_callback(self, callback: EventCallback) -> None:
        """Add a callback for server events.

        Args:
            callback: Async callback function.
        """
        self._event_callbacks.append(callback)

    def remove_event_callback(self, callback: EventCallback) -> None:
        """Remove an event callback.

        Args:
            callback: Callback to remove.
        """
        if callback in self._event_callbacks:
            self._event_callbacks.remove(callback)

    def get_events(
        self,
        server_name: str | None = None,
        limit: int = 100,
    ) -> list[ServerEvent]:
        """Get recent events.

        Args:
            server_name: Filter by server name (all if None).
            limit: Maximum events to return.

        Returns:
            List of events, newest first.
        """
        events = self._events
        if server_name:
            events = [e for e in events if e.server_name == server_name]
        return list(reversed(events[-limit:]))

    def get_all_health_states(self) -> dict[str, dict[str, Any]]:
        """Get health states for all servers.

        Returns:
            Dictionary mapping server names to health state dicts.
        """
        return {
            name: state.to_dict()
            for name, state in self._health_states.items()
        }

    def _quarantine_server(self, server_name: str) -> None:
        """Put a server in quarantine."""
        state = self._health_states[server_name]
        state.enter_quarantine()

        # Fire-and-forget event emission - intentionally not awaited
        asyncio.create_task(  # noqa: RUF006
            self._emit_event(
                server_name,
                "quarantined",
                f"Server quarantined after {state.consecutive_failures} consecutive failures",
                {"consecutive_failures": state.consecutive_failures},
            )
        )

    def _unquarantine_server(self, server_name: str) -> None:
        """Remove a server from quarantine."""
        state = self._health_states[server_name]
        state.exit_quarantine()

        asyncio.create_task(  # noqa: RUF006
            self._emit_event(
                server_name,
                "unquarantined",
                "Server removed from quarantine",
            )
        )

    async def _emit_event(
        self,
        server_name: str,
        event_type: str,
        message: str,
        details: dict[str, Any] | None = None,
    ) -> None:
        """Emit a server event."""
        event = ServerEvent(
            server_name=server_name,
            event_type=event_type,
            message=message,
            details=details or {},
        )

        # Store event
        self._events.append(event)
        if len(self._events) > self._max_events:
            self._events = self._events[-self._max_events:]

        # Log
        await logger.ainfo(
            "Server event",
            server=server_name,
            event_type=event_type,
            message=message,
        )

        # Notify callbacks
        for callback in self._event_callbacks:
            try:
                await callback(event)
            except Exception as e:
                await logger.aerror(
                    "Event callback failed",
                    error=str(e),
                )

    async def _monitoring_loop(self) -> None:
        """Background loop for periodic health checks."""
        while self._running:
            try:
                await self._check_all_servers()
                await asyncio.sleep(self._config.check_interval)
            except asyncio.CancelledError:
                break
            except Exception as e:
                await logger.aerror(
                    "Error in health monitoring loop",
                    error=str(e),
                )
                await asyncio.sleep(self._config.check_interval)

    async def _check_all_servers(self) -> None:
        """Check health of all non-quarantined servers."""
        tasks = []
        for name, server in self._servers.items():
            # Skip quarantined servers
            if self._health_states[name].is_quarantined:
                continue
            tasks.append(self._check_server(name, server))

        if tasks:
            await asyncio.gather(*tasks, return_exceptions=True)

    async def _check_server(self, name: str, server: BaseMCPServer) -> None:
        """Check a single server's health."""
        status = await self.check_health(server)

        if not status.healthy:
            state = self._health_states[name]

            # Check if should quarantine
            if (
                state.consecutive_failures >= self._config.failure_threshold
                and not state.is_quarantined
            ):
                self._quarantine_server(name)

    async def _recovery_loop(self) -> None:
        """Background loop for recovery attempts."""
        while self._running:
            try:
                await self._attempt_all_recoveries()
                await asyncio.sleep(self._config.recovery_interval)
            except asyncio.CancelledError:
                break
            except Exception as e:
                await logger.aerror(
                    "Error in recovery loop",
                    error=str(e),
                )
                await asyncio.sleep(self._config.recovery_interval)

    async def _attempt_all_recoveries(self) -> None:
        """Attempt recovery for all quarantined servers."""
        now = datetime.now(UTC)

        for name, state in self._health_states.items():
            if not state.is_quarantined:
                continue

            # Check if quarantine duration has passed
            if state.quarantined_at:
                quarantine_age = (now - state.quarantined_at).total_seconds()
                if quarantine_age < self._config.quarantine_duration:
                    continue

            # Attempt recovery
            await self.attempt_recovery(name)


__all__ = [
    "EventCallback",
    "HealthMonitor",
    "HealthMonitorConfig",
    "ServerHealthState",
]
