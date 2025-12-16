"""Sub-agent invocation for Ticca Agent.

This module provides the infrastructure for invoking sub-agents from
within an agent's execution. This enables sophisticated multi-agent
architectures where specialized agents can be delegated tasks.

Key concepts:
- Invocation: A request from one agent to another
- Session: Persistent conversation state between invocations
- Delegation: Passing control to a specialized agent
- Result aggregation: Combining results from multiple agents

Session ID Management:
- New sessions: Provide base name (e.g., "review-auth") → auto-appends hash suffix
- Continue session: Use full session_id from previous response
- One-off tasks: Leave empty for auto-generated ID

Example:
    >>> from ticca_agent.core import invoke_agent, list_agents
    >>> agents = await list_agents()
    >>> result = await invoke_agent(
    ...     agent_name="code-reviewer",
    ...     prompt="Review this Python function",
    ...     context=ctx,
    ... )

"""

from __future__ import annotations

import asyncio
import hashlib
import time
from datetime import datetime
from typing import TYPE_CHECKING, Any, Callable
from uuid import UUID, uuid4

from pydantic import BaseModel, Field

if TYPE_CHECKING:
    from ticca_agent.core.agent import Agent
    from ticca_agent.core.context import RunContext


class InvocationRequest(BaseModel):
    """Request to invoke a sub-agent.

    Attributes:
        invocation_id: Unique identifier for this invocation.
        agent_name: Name of the agent to invoke.
        prompt: The prompt to send to the sub-agent.
        session_id: Session ID for conversation memory.
        parent_run_id: ID of the parent run initiating this invocation.
        timeout: Maximum time for the invocation in seconds.
        metadata: Additional invocation metadata.
        created_at: When the request was created.
    """

    invocation_id: UUID = Field(
        default_factory=uuid4,
        description="Unique identifier for this invocation",
    )
    agent_name: str = Field(
        description="Name of the agent to invoke",
    )
    prompt: str = Field(
        description="Prompt to send to the sub-agent",
    )
    session_id: str | None = Field(
        default=None,
        description="Session ID for conversation memory",
    )
    parent_run_id: UUID | None = Field(
        default=None,
        description="ID of the parent run initiating this invocation",
    )
    timeout: float = Field(
        default=120.0,
        ge=1.0,
        le=600.0,
        description="Maximum invocation time in seconds",
    )
    metadata: dict[str, Any] = Field(
        default_factory=dict,
        description="Additional invocation metadata",
    )
    created_at: datetime = Field(
        default_factory=datetime.utcnow,
        description="Request creation timestamp",
    )

    model_config = {"frozen": True}


class InvocationResult(BaseModel):
    """Result from a sub-agent invocation.

    Attributes:
        invocation_id: ID of the invocation that produced this result.
        agent_name: Name of the agent that was invoked.
        response: The response content from the agent.
        session_id: Session ID used for this invocation.
        success: Whether the invocation succeeded.
        error: Error message if invocation failed.
        prompt_tokens: Tokens used in the prompt.
        completion_tokens: Tokens in the completion.
        duration_ms: Invocation duration in milliseconds.
        created_at: When the result was created.
    """

    invocation_id: UUID = Field(description="ID of the originating invocation")
    agent_name: str = Field(description="Name of the invoked agent")
    response: str | None = Field(
        default=None,
        description="Response content from the agent",
    )
    session_id: str | None = Field(
        default=None,
        description="Session ID used for this invocation",
    )
    success: bool = Field(
        default=True,
        description="Whether the invocation succeeded",
    )
    error: str | None = Field(
        default=None,
        description="Error message if invocation failed",
    )
    prompt_tokens: int = Field(
        default=0,
        ge=0,
        description="Tokens used in the prompt",
    )
    completion_tokens: int = Field(
        default=0,
        ge=0,
        description="Tokens in the completion",
    )
    duration_ms: int = Field(
        default=0,
        ge=0,
        description="Invocation duration in milliseconds",
    )
    created_at: datetime = Field(
        default_factory=datetime.utcnow,
        description="Result creation timestamp",
    )

    model_config = {"frozen": True}


class AgentInfo(BaseModel):
    """Information about an available agent.

    Attributes:
        name: Unique agent identifier.
        display_name: Human-readable agent name.
        description: Description of the agent's capabilities.
        capabilities: List of capability tags.
        is_available: Whether the agent is currently available.
    """

    name: str = Field(description="Unique agent identifier")
    display_name: str = Field(description="Human-readable agent name")
    description: str | None = Field(
        default=None,
        description="Description of agent capabilities",
    )
    capabilities: list[str] = Field(
        default_factory=list,
        description="List of capability tags",
    )
    is_available: bool = Field(
        default=True,
        description="Whether the agent is currently available",
    )

    model_config = {"frozen": True}


class InvocationSession(BaseModel):
    """Persistent session state for sub-agent invocations.

    Tracks conversation history and metadata across multiple
    invocations to the same sub-agent.

    Attributes:
        session_id: Full session ID (with hash suffix).
        agent_name: Name of the invoked agent.
        initial_prompt: The first prompt in this session.
        created_at: When the session was created.
        message_count: Number of messages in the session.
        last_updated: When the session was last updated.
        history: Conversation history (role, content pairs).
        metadata: Additional session metadata.
    """

    session_id: str = Field(description="Full session ID with hash")
    agent_name: str = Field(description="Agent name")
    initial_prompt: str = Field(description="First prompt in session")
    created_at: datetime = Field(
        default_factory=datetime.utcnow,
        description="Creation time",
    )
    message_count: int = Field(default=0, description="Message count")
    last_updated: datetime = Field(
        default_factory=datetime.utcnow,
        description="Last update time",
    )
    history: list[dict[str, str]] = Field(
        default_factory=list,
        description="Conversation history",
    )
    metadata: dict[str, Any] = Field(
        default_factory=dict,
        description="Additional session metadata",
    )


class InvocationManager:
    """Manager for sub-agent invocations.

    Handles:
    - Session ID generation and management
    - Session state persistence
    - Agent lookup and invocation
    - Result tracking and aggregation

    Example:
        >>> manager = InvocationManager(agent_getter)
        >>> result = await manager.invoke(
        ...     agent_name="code-reviewer",
        ...     prompt="Review this code",
        ... )
    """

    def __init__(
        self,
        agent_getter: Callable[[str], Agent | None] | None = None,
    ) -> None:
        """Initialize the invocation manager.

        Args:
            agent_getter: Function to get agent by name.
        """
        self._agent_getter = agent_getter
        self._sessions: dict[str, InvocationSession] = {}
        self._invocation_count: int = 0

    def generate_session_id(self, base_name: str | None = None) -> str:
        """Generate a full session ID with hash suffix.

        For new sessions, auto-appends a SHA1 hash suffix for uniqueness.
        For existing sessions (identified by having the hash format),
        returns the ID unchanged.

        Args:
            base_name: Optional base name for the session.

        Returns:
            Full session ID with hash suffix (e.g., "review-auth-a3f2b1").
        """
        if base_name is None:
            base_name = f"session-{uuid4().hex[:8]}"

        # Check if this already looks like a full session ID
        if self._is_full_session_id(base_name):
            return base_name

        # Generate unique hash suffix
        unique_input = f"{base_name}-{time.time()}-{uuid4().hex[:8]}"
        hash_suffix = hashlib.sha1(unique_input.encode()).hexdigest()[:6]

        return f"{base_name}-{hash_suffix}"

    def _is_full_session_id(self, session_id: str) -> bool:
        """Check if a session ID already has a hash suffix.

        Args:
            session_id: Session ID to check.

        Returns:
            True if the ID has a valid hash suffix format.
        """
        # Check if it exists in our sessions
        if session_id in self._sessions:
            return True

        # Check format: should end with -[6 hex chars]
        if "-" not in session_id:
            return False

        parts = session_id.rsplit("-", 1)
        if len(parts) == 2 and len(parts[1]) == 6:
            try:
                int(parts[1], 16)  # Validate hex
                return True
            except ValueError:
                pass

        return False

    def get_or_create_session(
        self,
        session_id: str | None,
        agent_name: str,
        prompt: str,
    ) -> InvocationSession:
        """Get existing session or create a new one.

        Args:
            session_id: Session ID (may be base name or full ID).
            agent_name: Name of the agent being invoked.
            prompt: The prompt being sent.

        Returns:
            Session object (existing or newly created).
        """
        full_id = self.generate_session_id(session_id)

        if full_id in self._sessions:
            session = self._sessions[full_id]
            session.message_count += 1
            session.last_updated = datetime.utcnow()
            return session

        # Create new session
        session = InvocationSession(
            session_id=full_id,
            agent_name=agent_name,
            initial_prompt=prompt,
            message_count=1,
        )
        self._sessions[full_id] = session
        return session

    def get_session(self, session_id: str) -> InvocationSession | None:
        """Get a session by ID.

        Args:
            session_id: Session ID.

        Returns:
            Session if found, None otherwise.
        """
        return self._sessions.get(session_id)

    def clear_session(self, session_id: str) -> bool:
        """Clear a session.

        Args:
            session_id: Session ID.

        Returns:
            True if session was found and cleared.
        """
        if session_id in self._sessions:
            del self._sessions[session_id]
            return True
        return False

    def list_sessions(self) -> list[InvocationSession]:
        """List all active sessions.

        Returns:
            List of all sessions.
        """
        return list(self._sessions.values())

    async def invoke(
        self,
        agent_name: str,
        prompt: str,
        *,
        session_id: str | None = None,
        context: RunContext | None = None,
        timeout: float = 120.0,
        metadata: dict[str, Any] | None = None,
    ) -> InvocationResult:
        """Invoke a sub-agent.

        Args:
            agent_name: Name of the agent to invoke.
            prompt: Prompt to send to the agent.
            session_id: Optional session ID for conversation memory.
            context: Optional parent run context.
            timeout: Maximum invocation time in seconds.
            metadata: Additional invocation metadata.

        Returns:
            InvocationResult containing the agent's response.
        """
        start_time = time.monotonic()
        self._invocation_count += 1

        # Get or create session
        session = self.get_or_create_session(session_id, agent_name, prompt)

        # Create invocation request
        request = InvocationRequest(
            agent_name=agent_name,
            prompt=prompt,
            session_id=session.session_id,
            parent_run_id=context.run_id if context else None,
            timeout=timeout,
            metadata=metadata or {},
        )

        # Add prompt to session history
        session.history.append({"role": "user", "content": prompt})

        # Look up the agent
        if self._agent_getter is None:
            duration_ms = int((time.monotonic() - start_time) * 1000)
            return InvocationResult(
                invocation_id=request.invocation_id,
                agent_name=agent_name,
                session_id=session.session_id,
                success=False,
                error="No agent getter configured",
                duration_ms=duration_ms,
            )

        agent = self._agent_getter(agent_name)
        if agent is None:
            duration_ms = int((time.monotonic() - start_time) * 1000)
            return InvocationResult(
                invocation_id=request.invocation_id,
                agent_name=agent_name,
                session_id=session.session_id,
                success=False,
                error=f"Agent '{agent_name}' not found",
                duration_ms=duration_ms,
            )

        try:
            # Set up history for the agent if continuing session
            if len(session.history) > 1:
                from ticca_agent.core.types import Message, MessageRole

                history_messages = [
                    Message(
                        role=(
                            MessageRole.USER
                            if msg["role"] == "user"
                            else MessageRole.ASSISTANT
                        ),
                        content=msg["content"],
                    )
                    for msg in session.history[:-1]  # Exclude current prompt
                ]
                agent.set_history(history_messages)

            # Create a child context if we have a parent
            from ticca_agent.core.context import RunContext

            child_context: RunContext | None = None
            if context:
                child_context = RunContext(
                    session_id=session.session_id,
                )

            # Execute with timeout
            response = await asyncio.wait_for(
                agent.run(prompt, context=child_context),
                timeout=timeout,
            )

            duration_ms = int((time.monotonic() - start_time) * 1000)

            # Record response in session history
            session.history.append(
                {"role": "assistant", "content": response.content}
            )

            # Calculate token usage
            prompt_tokens = response.usage.prompt_tokens if response.usage else 0
            completion_tokens = (
                response.usage.completion_tokens if response.usage else 0
            )

            return InvocationResult(
                invocation_id=request.invocation_id,
                agent_name=agent_name,
                response=response.content,
                session_id=session.session_id,
                success=True,
                prompt_tokens=prompt_tokens,
                completion_tokens=completion_tokens,
                duration_ms=duration_ms,
            )

        except asyncio.TimeoutError:
            duration_ms = int((time.monotonic() - start_time) * 1000)
            return InvocationResult(
                invocation_id=request.invocation_id,
                agent_name=agent_name,
                session_id=session.session_id,
                success=False,
                error=f"Invocation timed out after {timeout}s",
                duration_ms=duration_ms,
            )

        except Exception as e:
            duration_ms = int((time.monotonic() - start_time) * 1000)
            return InvocationResult(
                invocation_id=request.invocation_id,
                agent_name=agent_name,
                session_id=session.session_id,
                success=False,
                error=f"Invocation failed: {e}",
                duration_ms=duration_ms,
            )


# Module-level agent registry
_agent_registry: dict[str, Agent] = {}
_invocation_manager: InvocationManager | None = None


def get_invocation_manager() -> InvocationManager:
    """Get the global invocation manager.

    Creates one if it doesn't exist.

    Returns:
        The global InvocationManager instance.
    """
    global _invocation_manager
    if _invocation_manager is None:
        _invocation_manager = InvocationManager(agent_getter=get_agent)
    return _invocation_manager


def register_agent(agent: Agent) -> None:
    """Register an agent in the global registry.

    Args:
        agent: The agent to register.

    Raises:
        ValueError: If an agent with the same name is already registered.
    """
    if agent.name in _agent_registry:
        raise ValueError(f"Agent '{agent.name}' is already registered")

    _agent_registry[agent.name] = agent


def unregister_agent(agent_name: str) -> bool:
    """Unregister an agent from the global registry.

    Args:
        agent_name: Name of the agent to unregister.

    Returns:
        True if the agent was found and removed, False otherwise.
    """
    if agent_name in _agent_registry:
        del _agent_registry[agent_name]
        return True
    return False


def get_agent(agent_name: str) -> Agent | None:
    """Get an agent from the registry by name.

    Args:
        agent_name: Name of the agent to retrieve.

    Returns:
        The agent if found, None otherwise.
    """
    return _agent_registry.get(agent_name)


async def list_agents() -> list[AgentInfo]:
    """List all available agents.

    Returns:
        List of AgentInfo objects describing available agents.
    """
    agents = []
    for name, agent in _agent_registry.items():
        agents.append(
            AgentInfo(
                name=name,
                display_name=agent.display_name,
                description=agent.config.system_prompt[:200] + "..."
                if agent.config.system_prompt
                else None,
                capabilities=[t.name for t in agent.tools],
                is_available=True,
            )
        )
    return agents


def list_agents_sync() -> list[AgentInfo]:
    """List all available agents (synchronous version).

    Returns:
        List of AgentInfo objects.
    """
    agents = []
    for name, agent in _agent_registry.items():
        agents.append(
            AgentInfo(
                name=name,
                display_name=agent.display_name,
                description=agent.config.system_prompt[:200] + "..."
                if agent.config.system_prompt
                else None,
                capabilities=[t.name for t in agent.tools],
                is_available=True,
            )
        )
    return agents


async def invoke_agent(
    agent_name: str,
    prompt: str,
    *,
    session_id: str | None = None,
    context: RunContext | None = None,
    timeout: float = 120.0,
    metadata: dict[str, Any] | None = None,
) -> InvocationResult:
    """Invoke a sub-agent with a prompt.

    This is the primary function for delegating work to specialized agents.
    It handles agent lookup, session management, and result formatting.

    Args:
        agent_name: Name of the agent to invoke.
        prompt: The prompt to send to the agent.
        session_id: Optional session ID for conversation memory.
            - For new sessions: provide base name (e.g., "review-auth")
            - To continue: use full session_id from previous response
            - For one-off: leave as None for auto-generated ID
        context: Optional parent run context.
        timeout: Maximum time for the invocation.
        metadata: Additional metadata for the invocation.

    Returns:
        InvocationResult containing the agent's response.

    Raises:
        InvocationError: If the invocation fails.
    """
    manager = get_invocation_manager()
    return await manager.invoke(
        agent_name=agent_name,
        prompt=prompt,
        session_id=session_id,
        context=context,
        timeout=timeout,
        metadata=metadata,
    )


class InvocationError(Exception):
    """Exception raised when agent invocation fails.

    Attributes:
        message: Human-readable error description.
        agent_name: Name of the agent that failed.
        invocation_id: ID of the failed invocation.
        session_id: Session ID of the failed invocation.
    """

    def __init__(
        self,
        message: str,
        *,
        agent_name: str | None = None,
        invocation_id: UUID | None = None,
        session_id: str | None = None,
    ) -> None:
        """Initialize InvocationError.

        Args:
            message: Human-readable error description.
            agent_name: Name of the agent that failed.
            invocation_id: ID of the failed invocation.
            session_id: Session ID of the failed invocation.
        """
        super().__init__(message)
        self.message = message
        self.agent_name = agent_name
        self.invocation_id = invocation_id
        self.session_id = session_id

    def __str__(self) -> str:
        """Return string representation of the error."""
        parts = []
        if self.agent_name:
            parts.append(f"agent={self.agent_name}")
        if self.session_id:
            parts.append(f"session={self.session_id}")
        if self.invocation_id:
            parts.append(f"id={self.invocation_id}")
        prefix = f"[{', '.join(parts)}] " if parts else ""
        return f"{prefix}{self.message}"


__all__ = [
    "AgentInfo",
    "InvocationError",
    "InvocationManager",
    "InvocationRequest",
    "InvocationResult",
    "InvocationSession",
    "get_agent",
    "get_invocation_manager",
    "invoke_agent",
    "list_agents",
    "list_agents_sync",
    "register_agent",
    "unregister_agent",
]
