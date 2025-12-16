"""Planning agent for task decomposition and coordination.

This module provides a specialized agent for:
- Breaking down complex tasks into subtasks
- Coordinating multiple sub-agents
- Managing task dependencies and execution order
- Aggregating results from delegated tasks

The PlanningAgent excels at orchestration tasks that require
delegating work to specialized agents.

Example:
    >>> from ticca_agent.agents import PlanningAgent
    >>> agent = PlanningAgent(provider=my_provider)
    >>> result = await agent.run(
    ...     "Build a REST API with authentication and tests"
    ... )

"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, ClassVar

from pydantic import Field

from ticca_agent.core.agent import Agent, AgentConfig

if TYPE_CHECKING:
    from ticca_agent.core.context import RunContext
    from ticca_agent.core.types import AgentResponse
    from ticca_agent.providers.base import BaseProvider
    from ticca_agent.tools.base import BaseTool


# Default system prompt for planning tasks
DEFAULT_PLANNING_SYSTEM_PROMPT = """You are an expert project planner and coordinator
specialized in breaking down complex software tasks into manageable subtasks.

Your responsibilities:

1. **Task Analysis:**
   - Understand the full scope of the requested work
   - Identify all major components and deliverables
   - Recognize dependencies between tasks
   - Estimate complexity and potential blockers

2. **Task Decomposition:**
   - Break large tasks into atomic, actionable subtasks
   - Each subtask should be completable by a single agent
   - Define clear acceptance criteria for each subtask
   - Identify which subtasks can be parallelized

3. **Agent Coordination:**
   - Determine which specialized agent is best for each subtask
   - Use `list_agents` to see available agents and their capabilities
   - Delegate tasks using `invoke_agent` with clear prompts
   - Track progress and aggregate results

4. **Quality Assurance:**
   - Verify subtask outputs meet requirements
   - Handle errors and retry failed tasks
   - Ensure coherent integration of all parts
   - Provide clear status updates throughout

When planning:
- Start by understanding the full picture
- Create a clear execution plan before starting
- Use `agent_share_your_reasoning` to explain your plan
- Execute subtasks in optimal order (dependencies first)
- Aggregate and validate results before completing

Available tools for coordination:
- `list_agents`: Discover available sub-agents
- `invoke_agent`: Delegate tasks to specialized agents
- `agent_share_your_reasoning`: Share your planning process
- `list_files`: Explore project structure
- `read_file`: Review existing code and documentation
"""


class PlanningAgentConfig(AgentConfig):
    """Configuration specific to the PlanningAgent.

    Extends AgentConfig with planning-specific settings.

    Attributes:
        max_subtasks: Maximum subtasks to create per request.
        max_delegation_depth: Maximum nesting depth for agent delegation.
        parallel_execution: Whether to allow parallel subtask execution.
        require_confirmation: Whether to confirm plan before execution.
    """

    max_subtasks: int = Field(
        default=20,
        ge=1,
        le=100,
        description="Maximum subtasks per request",
    )
    max_delegation_depth: int = Field(
        default=3,
        ge=1,
        le=10,
        description="Maximum nesting depth for delegation",
    )
    parallel_execution: bool = Field(
        default=False,
        description="Allow parallel subtask execution",
    )
    require_confirmation: bool = Field(
        default=False,
        description="Require user confirmation before executing plan",
    )

    model_config = {"frozen": True}


class PlanningAgent(Agent):
    """Specialized agent for task planning and coordination.

    The PlanningAgent is optimized for orchestration tasks:
    - Breaking down complex requests into subtasks
    - Delegating work to specialized agents
    - Coordinating execution and aggregating results

    Class Attributes:
        DEFAULT_MODEL: Default model for planning (prefers reasoning models).

    Example:
        >>> config = PlanningAgentConfig(name="planner")
        >>> agent = PlanningAgent(config=config, provider=my_provider)
        >>> result = await agent.plan_and_execute(
        ...     "Implement user authentication with OAuth2"
        ... )
    """

    DEFAULT_MODEL: ClassVar[str | None] = None  # Use provider default

    # Tools that planning agents should have access to
    PLANNING_TOOLS: ClassVar[list[str]] = [
        "list_agents",
        "invoke_agent",
        "agent_share_your_reasoning",
        "list_files",
        "read_file",
    ]

    def __init__(
        self,
        config: PlanningAgentConfig | None = None,
        provider: BaseProvider | None = None,
        tools: list[BaseTool[Any, Any]] | None = None,
    ) -> None:
        """Initialize the PlanningAgent.

        Args:
            config: Agent configuration (uses defaults if not provided).
            provider: AI provider for generation.
            tools: Optional list of additional tools.
        """
        if config is None:
            config = PlanningAgentConfig(
                name="planning-agent",
                system_prompt=DEFAULT_PLANNING_SYSTEM_PROMPT,
            )

        if provider is None:
            raise ValueError("Provider is required for PlanningAgent")

        super().__init__(
            config=config,
            provider=provider,
            tools=tools,
        )

        # TODO: Register planning-specific tools
        # TODO: Set up delegation tracking

    @property
    def planning_config(self) -> PlanningAgentConfig:
        """Get the planning-specific configuration."""
        return self._config  # type: ignore[return-value]

    async def plan_and_execute(
        self,
        task_description: str,
        *,
        context: RunContext | None = None,
        confirm_plan: bool | None = None,
    ) -> AgentResponse:
        """Plan and execute a complex task.

        This method:
        1. Analyzes the task and creates a plan
        2. Optionally confirms the plan with the user
        3. Executes subtasks in order
        4. Aggregates and returns results

        Args:
            task_description: Description of the task to accomplish.
            context: Optional run context.
            confirm_plan: Override plan confirmation setting.

        Returns:
            Response containing the execution results.
        """
        # TODO: Implement planning phase
        # TODO: Create execution plan with subtasks
        # TODO: Handle confirmation if required
        # TODO: Execute subtasks with delegation
        # TODO: Aggregate and return results
        raise NotImplementedError(
            "PlanningAgent.plan_and_execute() not yet implemented"
        )

    async def analyze_task(
        self,
        task_description: str,
        *,
        context: RunContext | None = None,
    ) -> AgentResponse:
        """Analyze a task and return a proposed plan without executing.

        Args:
            task_description: Description of the task.
            context: Optional run context.

        Returns:
            Response containing the proposed plan.
        """
        # TODO: Build analysis prompt
        # TODO: Call agent to analyze task
        # TODO: Return structured plan
        raise NotImplementedError(
            "PlanningAgent.analyze_task() not yet implemented"
        )

    async def delegate_to_agent(
        self,
        agent_name: str,
        task_prompt: str,
        *,
        session_id: str | None = None,
        context: RunContext | None = None,
    ) -> AgentResponse:
        """Delegate a task to a specialized agent.

        Args:
            agent_name: Name of the agent to invoke.
            task_prompt: Task description for the agent.
            session_id: Optional session ID for conversation memory.
            context: Optional run context.

        Returns:
            Response from the delegated agent.
        """
        # TODO: Validate agent exists
        # TODO: Check delegation depth
        # TODO: Invoke agent with proper context
        # TODO: Track delegation in context
        raise NotImplementedError(
            "PlanningAgent.delegate_to_agent() not yet implemented"
        )


# Factory function for easy agent creation
def create_planning_agent(
    provider: BaseProvider,
    *,
    name: str = "planning-agent",
    system_prompt: str | None = None,
    tools: list[BaseTool[Any, Any]] | None = None,
    max_subtasks: int = 20,
) -> PlanningAgent:
    """Create a PlanningAgent with common configuration.

    Args:
        provider: AI provider for generation.
        name: Agent name.
        system_prompt: Custom system prompt.
        tools: Additional tools to register.
        max_subtasks: Maximum subtasks per request.

    Returns:
        Configured PlanningAgent instance.
    """
    config = PlanningAgentConfig(
        name=name,
        system_prompt=system_prompt or DEFAULT_PLANNING_SYSTEM_PROMPT,
        max_subtasks=max_subtasks,
    )

    return PlanningAgent(
        config=config,
        provider=provider,
        tools=tools,
    )


__all__ = [
    "DEFAULT_PLANNING_SYSTEM_PROMPT",
    "PlanningAgent",
    "PlanningAgentConfig",
    "create_planning_agent",
]
