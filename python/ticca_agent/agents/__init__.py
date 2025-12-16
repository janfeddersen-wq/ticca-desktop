"""Agent implementations for Ticca Agent.

This module contains the core agent implementations that orchestrate
interactions between users, AI providers, and tools.

Agents are the high-level abstraction that:
- Manage conversation state and history
- Route requests to appropriate providers
- Handle tool execution and response formatting
- Implement agent-specific behaviors (chat, coding, planning, review)

Built-in Agents:
- CodeAgent: Full-stack coding assistant
- PlanningAgent: Task decomposition and coordination
- CodeReviewerAgent: Security, performance, and design review

Agent Registry:
- Built-in agents are automatically registered
- Custom agents can be registered via register_agent()
- Agents can be discovered via get_agent_info() and list_agents_sync()

Example:
    >>> from ticca_agent.agents import get_agent_registry
    >>> registry = get_agent_registry()
    >>> agent_info = registry.get_agent_info("code-agent")
    >>> print(agent_info.display_name)

"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Any, Callable

if TYPE_CHECKING:
    from ticca_agent.core.agent import Agent
    from ticca_agent.providers.base import BaseProvider
    from ticca_agent.tools.base import BaseTool

# Import agent classes
from ticca_agent.agents.code_agent import (
    CodeAgent,
    CodeAgentConfig,
    DEFAULT_CODE_SYSTEM_PROMPT,
    create_code_agent,
)
from ticca_agent.agents.code_reviewer import (
    CodeReviewerAgent,
    CodeReviewerConfig,
    DEFAULT_CODE_REVIEW_SYSTEM_PROMPT,
    ReviewFocus,
    create_code_reviewer,
)
from ticca_agent.agents.planning_agent import (
    DEFAULT_PLANNING_SYSTEM_PROMPT,
    PlanningAgent,
    PlanningAgentConfig,
    create_planning_agent,
)


# Type alias for agent factory functions
# Using Callable instead of Protocol for flexibility with different signatures
AgentFactory = Callable[..., "Agent"]


@dataclass
class AgentRegistration:
    """Registration information for an agent.

    Attributes:
        name: Unique agent identifier.
        display_name: Human-readable agent name.
        description: Description of agent capabilities.
        factory: Factory function to create agent instances.
        capabilities: List of capability tags.
        default_tools: Tool names this agent uses by default.
        pinned_model: Optional model to always use for this agent.
    """

    name: str
    display_name: str
    description: str
    factory: AgentFactory
    capabilities: list[str] = field(default_factory=list)
    default_tools: list[str] = field(default_factory=list)
    pinned_model: str | None = None


class AgentRegistry:
    """Registry for managing available agents.

    The registry provides:
    - Agent registration and discovery
    - Factory functions for agent creation
    - Capability-based agent lookup
    - Model pinning support

    Example:
        >>> registry = AgentRegistry()
        >>> registry.register(
        ...     name="my-agent",
        ...     display_name="My Agent",
        ...     description="A custom agent",
        ...     factory=create_my_agent,
        ... )
        >>> agent = registry.create_agent("my-agent", provider)
    """

    def __init__(self) -> None:
        """Initialize an empty registry."""
        self._agents: dict[str, AgentRegistration] = {}
        self._pinned_models: dict[str, str] = {}

    def register(
        self,
        name: str,
        display_name: str,
        description: str,
        factory: AgentFactory,
        *,
        capabilities: list[str] | None = None,
        default_tools: list[str] | None = None,
        pinned_model: str | None = None,
    ) -> None:
        """Register an agent.

        Args:
            name: Unique agent identifier.
            display_name: Human-readable name.
            description: Agent capabilities description.
            factory: Factory function to create instances.
            capabilities: Optional capability tags.
            default_tools: Optional default tool names.
            pinned_model: Optional model to pin for this agent.

        Raises:
            ValueError: If agent with name already exists.
        """
        if name in self._agents:
            raise ValueError(f"Agent '{name}' is already registered")

        self._agents[name] = AgentRegistration(
            name=name,
            display_name=display_name,
            description=description,
            factory=factory,
            capabilities=capabilities or [],
            default_tools=default_tools or [],
            pinned_model=pinned_model,
        )

        if pinned_model:
            self._pinned_models[name] = pinned_model

    def unregister(self, name: str) -> bool:
        """Unregister an agent.

        Args:
            name: Agent identifier.

        Returns:
            True if agent was found and removed.
        """
        if name in self._agents:
            del self._agents[name]
            self._pinned_models.pop(name, None)
            return True
        return False

    def get_registration(self, name: str) -> AgentRegistration | None:
        """Get agent registration by name.

        Args:
            name: Agent identifier.

        Returns:
            Registration if found, None otherwise.
        """
        return self._agents.get(name)

    def get_agent_info(self, name: str) -> AgentRegistration | None:
        """Get agent information (alias for get_registration).

        Args:
            name: Agent identifier.

        Returns:
            Registration if found, None otherwise.
        """
        return self.get_registration(name)

    def list_agents(self) -> list[AgentRegistration]:
        """List all registered agents.

        Returns:
            List of all agent registrations.
        """
        return list(self._agents.values())

    def list_agent_names(self) -> list[str]:
        """List all registered agent names.

        Returns:
            List of agent names.
        """
        return list(self._agents.keys())

    def find_by_capability(self, capability: str) -> list[AgentRegistration]:
        """Find agents with a specific capability.

        Args:
            capability: Capability tag to search for.

        Returns:
            List of agents with the capability.
        """
        return [
            agent
            for agent in self._agents.values()
            if capability in agent.capabilities
        ]

    def create_agent(
        self,
        name: str,
        provider: BaseProvider,
        *,
        tools: list[BaseTool[Any, Any]] | None = None,
        **kwargs: Any,
    ) -> Agent:
        """Create an agent instance.

        Args:
            name: Agent identifier.
            provider: AI provider for the agent.
            tools: Optional tools to provide to the agent.
            **kwargs: Additional configuration passed to factory.

        Returns:
            Configured agent instance.

        Raises:
            ValueError: If agent not found.
        """
        registration = self._agents.get(name)
        if registration is None:
            raise ValueError(f"Agent '{name}' not found in registry")

        return registration.factory(
            provider,
            tools=tools,
            **kwargs,
        )

    def get_pinned_model(self, name: str) -> str | None:
        """Get pinned model for an agent.

        Args:
            name: Agent identifier.

        Returns:
            Pinned model name or None.
        """
        return self._pinned_models.get(name)

    def set_pinned_model(self, name: str, model: str | None) -> None:
        """Set or clear pinned model for an agent.

        Args:
            name: Agent identifier.
            model: Model to pin, or None to clear.
        """
        if name not in self._agents:
            raise ValueError(f"Agent '{name}' not found in registry")

        if model is None:
            self._pinned_models.pop(name, None)
        else:
            self._pinned_models[name] = model

    def __len__(self) -> int:
        """Return number of registered agents."""
        return len(self._agents)

    def __contains__(self, name: str) -> bool:
        """Check if agent is registered."""
        return name in self._agents


# Global default registry
_default_registry: AgentRegistry | None = None


def get_agent_registry() -> AgentRegistry:
    """Get the default agent registry.

    Creates and populates the registry with built-in agents on first call.

    Returns:
        The default AgentRegistry instance.
    """
    global _default_registry

    if _default_registry is None:
        _default_registry = AgentRegistry()
        _register_builtin_agents(_default_registry)

    return _default_registry


def _register_builtin_agents(registry: AgentRegistry) -> None:
    """Register all built-in agents.

    Args:
        registry: Registry to populate.
    """
    # Code Agent - Default full-stack coding assistant
    registry.register(
        name="code-agent",
        display_name="Code Agent",
        description=(
            "Full-stack coding assistant for code generation, analysis, "
            "refactoring, and documentation."
        ),
        factory=create_code_agent,
        capabilities=[
            "code-generation",
            "code-analysis",
            "refactoring",
            "documentation",
            "file-operations",
            "shell-commands",
        ],
        default_tools=[
            "list_files",
            "read_file",
            "edit_file",
            "delete_file",
            "grep",
            "agent_run_shell_command",
            "agent_share_your_reasoning",
        ],
    )

    # Planning Agent - Task decomposition and coordination
    registry.register(
        name="planning-agent",
        display_name="Planning Agent",
        description=(
            "Task decomposition and coordination agent that breaks down "
            "complex tasks and delegates to specialized sub-agents."
        ),
        factory=create_planning_agent,
        capabilities=[
            "task-planning",
            "task-decomposition",
            "agent-coordination",
            "project-management",
        ],
        default_tools=[
            "list_agents",
            "invoke_agent",
            "agent_share_your_reasoning",
            "list_files",
            "read_file",
        ],
    )

    # Code Reviewer - Code review agent
    registry.register(
        name="code-reviewer",
        display_name="Code Reviewer",
        description=(
            "Code review specialist for security, performance, design, "
            "and quality analysis. Read-only by design."
        ),
        factory=create_code_reviewer,
        capabilities=[
            "code-review",
            "security-analysis",
            "performance-analysis",
            "design-review",
            "quality-assessment",
        ],
        default_tools=[
            "read_file",
            "grep",
            "list_files",
        ],
    )


# Convenience functions for accessing the default registry
def list_agents_sync() -> list[AgentRegistration]:
    """List all available agents (synchronous).

    Returns:
        List of agent registrations.
    """
    return get_agent_registry().list_agents()


def get_agent_info(name: str) -> AgentRegistration | None:
    """Get information about an agent.

    Args:
        name: Agent identifier.

    Returns:
        Agent registration or None if not found.
    """
    return get_agent_registry().get_agent_info(name)


def create_agent(
    name: str,
    provider: BaseProvider,
    **kwargs: Any,
) -> Agent:
    """Create an agent by name.

    Args:
        name: Agent identifier.
        provider: AI provider.
        **kwargs: Additional configuration.

    Returns:
        Configured agent instance.
    """
    return get_agent_registry().create_agent(name, provider, **kwargs)


__all__ = [
    # Agent Classes
    "CodeAgent",
    "CodeAgentConfig",
    "CodeReviewerAgent",
    "CodeReviewerConfig",
    "PlanningAgent",
    "PlanningAgentConfig",
    # System Prompts
    "DEFAULT_CODE_REVIEW_SYSTEM_PROMPT",
    "DEFAULT_CODE_SYSTEM_PROMPT",
    "DEFAULT_PLANNING_SYSTEM_PROMPT",
    # Factory Functions
    "create_code_agent",
    "create_code_reviewer",
    "create_planning_agent",
    # Enums
    "ReviewFocus",
    # Registry
    "AgentFactory",
    "AgentRegistration",
    "AgentRegistry",
    "create_agent",
    "get_agent_info",
    "get_agent_registry",
    "list_agents_sync",
]
