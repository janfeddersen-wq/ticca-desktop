"""Agent coordination tools for Ticca Agent.

This module provides tools for multi-agent coordination:
- list_agents: List available sub-agents
- invoke_agent: Invoke a sub-agent with session management
- agent_share_your_reasoning: Share thought process with user

These tools enable sophisticated multi-agent architectures where
agents can delegate tasks to specialized sub-agents.

Example:
    >>> from ticca_agent.tools.agent_tools import ListAgentsTool
    >>> tool = ListAgentsTool(registry)
    >>> result = await tool.run(ListAgentsInput())
    >>> for agent in result.output.agents:
    ...     print(f"{agent.name}: {agent.display_name}")

"""

from __future__ import annotations

import hashlib
import time
from datetime import datetime
from typing import TYPE_CHECKING, Any, Callable
from uuid import uuid4

from pydantic import BaseModel, Field

from ticca_agent.tools.base import BaseTool, ToolConfig

if TYPE_CHECKING:
    from ticca_agent.core.context import RunContext
    from ticca_agent.core.invocation import AgentInfo


# =============================================================================
# List Agents Tool
# =============================================================================


class ListAgentsInput(BaseModel):
    """Input for list_agents tool.

    This tool takes no input parameters.
    """

    pass


class AgentListEntry(BaseModel):
    """Information about an available agent.

    Attributes:
        name: Unique agent identifier.
        display_name: Human-readable agent name.
        description: Description of capabilities.
        capabilities: List of capability tags.
        is_available: Whether the agent is available.
    """

    name: str = Field(description="Agent identifier")
    display_name: str = Field(description="Human-readable name")
    description: str | None = Field(default=None, description="Capabilities")
    capabilities: list[str] = Field(
        default_factory=list,
        description="Capability tags",
    )
    is_available: bool = Field(default=True, description="Availability")


class ListAgentsOutput(BaseModel):
    """Output from list_agents tool.

    Attributes:
        agents: List of available agents.
        error: Error message if listing failed.
    """

    agents: list[AgentListEntry] = Field(
        default_factory=list,
        description="Available agents",
    )
    error: str | None = Field(default=None, description="Error message")


class ListAgentsTool(BaseTool[ListAgentsInput, ListAgentsOutput]):
    """Tool for listing available sub-agents.

    This tool queries the agent registry and returns information
    about all agents available for invocation.
    """

    def __init__(
        self,
        get_agents_callback: Callable[[], list[AgentInfo]] | None = None,
    ) -> None:
        """Initialize the tool.

        Args:
            get_agents_callback: Callback to get available agents.
        """
        super().__init__(
            ToolConfig(
                name="list_agents",
                description=(
                    "List all available sub-agents that can be invoked. "
                    "Returns agent names and their capabilities."
                ),
            )
        )
        self._get_agents = get_agents_callback

    async def execute(
        self,
        input_data: ListAgentsInput,
        context: RunContext | None = None,
    ) -> ListAgentsOutput:
        """Execute the list_agents operation.

        Args:
            input_data: Input parameters (empty).
            context: Optional run context.

        Returns:
            ListAgentsOutput with available agents.
        """
        try:
            if self._get_agents is None:
                # Return empty list if no callback provided
                return ListAgentsOutput(agents=[])

            agent_infos = self._get_agents()

            agents = [
                AgentListEntry(
                    name=info.name,
                    display_name=info.display_name,
                    description=info.description,
                    capabilities=info.capabilities,
                    is_available=info.is_available,
                )
                for info in agent_infos
            ]

            return ListAgentsOutput(agents=agents)

        except Exception as e:
            return ListAgentsOutput(error=f"Failed to list agents: {e}")

    def get_schema(self) -> dict[str, Any]:
        """Get JSON schema for this tool."""
        return {
            "name": self.name,
            "description": self.description,
            "parameters": {
                "type": "object",
                "properties": {},
                "required": [],
            },
        }


# =============================================================================
# Invoke Agent Tool
# =============================================================================


class InvokeAgentInput(BaseModel):
    """Input for invoke_agent tool.

    Attributes:
        agent_name: Name of the agent to invoke.
        prompt: Prompt to send to the agent.
        session_id: Optional session ID for conversation memory.
    """

    agent_name: str = Field(description="Name of the agent to invoke")
    prompt: str = Field(description="Prompt to send to the agent")
    session_id: str | None = Field(
        default=None,
        description=(
            "Session ID for conversation memory. "
            "For new sessions, provide a base name (e.g., 'review-auth'). "
            "To continue a session, use the full session_id from previous response."
        ),
    )


class AgentInvokeOutput(BaseModel):
    """Output from invoke_agent tool.

    Attributes:
        response: The agent's response.
        agent_name: Name of the invoked agent.
        session_id: Full session ID (with hash suffix).
        error: Error message if invocation failed.
    """

    response: str | None = Field(default=None, description="Agent response")
    agent_name: str = Field(description="Invoked agent name")
    session_id: str | None = Field(default=None, description="Session ID")
    error: str | None = Field(default=None, description="Error message")


class InvocationSession(BaseModel):
    """Session state for sub-agent invocations.

    Attributes:
        session_id: Full session ID (with hash).
        agent_name: Name of the invoked agent.
        initial_prompt: The first prompt in this session.
        created_at: When the session was created.
        message_count: Number of messages in the session.
        last_updated: When the session was last updated.
        history: Conversation history.
    """

    session_id: str = Field(description="Full session ID")
    agent_name: str = Field(description="Agent name")
    initial_prompt: str = Field(description="First prompt")
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


class InvokeAgentTool(BaseTool[InvokeAgentInput, AgentInvokeOutput]):
    """Tool for invoking sub-agents.

    This tool enables agents to delegate tasks to specialized sub-agents.
    It manages session state and conversation history.

    Session ID Patterns:
    - New session: Provide base name → auto-appends hash suffix
    - Continue session: Use full session_id from previous response
    - One-off: Leave empty for auto-generated ID
    """

    def __init__(
        self,
        invoke_callback: Callable[
            [str, str, str | None], AgentInvokeOutput
        ]
        | None = None,
    ) -> None:
        """Initialize the tool.

        Args:
            invoke_callback: Callback to invoke agents.
                Takes (agent_name, prompt, session_id) and returns output.
        """
        super().__init__(
            ToolConfig(
                name="invoke_agent",
                description=(
                    "Invoke a sub-agent with a prompt. Supports session management "
                    "for multi-turn conversations. Returns the agent's response."
                ),
                timeout=120.0,
            )
        )
        self._invoke_callback = invoke_callback
        self._sessions: dict[str, InvocationSession] = {}

    def _generate_session_id(self, base_name: str | None) -> str:
        """Generate a full session ID with hash suffix.

        Args:
            base_name: Optional base name for the session.

        Returns:
            Full session ID with hash suffix.
        """
        if base_name is None:
            base_name = f"session-{uuid4().hex[:8]}"

        # Check if this already looks like a full session ID (has hash suffix)
        if self._is_full_session_id(base_name):
            return base_name

        # Generate hash suffix
        unique_input = f"{base_name}-{time.time()}-{uuid4().hex[:8]}"
        hash_suffix = hashlib.sha1(unique_input.encode()).hexdigest()[:6]

        return f"{base_name}-{hash_suffix}"

    def _is_full_session_id(self, session_id: str) -> bool:
        """Check if a session ID already has a hash suffix.

        Args:
            session_id: Session ID to check.

        Returns:
            True if the ID has a hash suffix.
        """
        # Check if it exists in our sessions
        if session_id in self._sessions:
            return True

        # Check format: should end with -[6 hex chars]
        parts = session_id.rsplit("-", 1)
        if len(parts) == 2 and len(parts[1]) == 6:
            try:
                int(parts[1], 16)
                return True
            except ValueError:
                pass

        return False

    def _get_or_create_session(
        self,
        session_id: str | None,
        agent_name: str,
        prompt: str,
    ) -> InvocationSession:
        """Get existing session or create new one.

        Args:
            session_id: Session ID (may be base name or full ID).
            agent_name: Name of the agent.
            prompt: The prompt being sent.

        Returns:
            Session object.
        """
        full_id = self._generate_session_id(session_id)

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

    async def execute(
        self,
        input_data: InvokeAgentInput,
        context: RunContext | None = None,
    ) -> AgentInvokeOutput:
        """Execute the invoke_agent operation.

        Args:
            input_data: Input parameters.
            context: Optional run context.

        Returns:
            AgentInvokeOutput with result.
        """
        try:
            # Get or create session
            session = self._get_or_create_session(
                input_data.session_id,
                input_data.agent_name,
                input_data.prompt,
            )

            # Add prompt to history
            session.history.append(
                {"role": "user", "content": input_data.prompt}
            )

            if self._invoke_callback is None:
                return AgentInvokeOutput(
                    agent_name=input_data.agent_name,
                    session_id=session.session_id,
                    error="No invoke callback configured",
                )

            # Invoke the agent
            result = self._invoke_callback(
                input_data.agent_name,
                input_data.prompt,
                session.session_id,
            )

            # Add response to history
            if result.response:
                session.history.append(
                    {"role": "assistant", "content": result.response}
                )

            # Ensure session_id is set in result
            if result.session_id is None:
                result = AgentInvokeOutput(
                    response=result.response,
                    agent_name=result.agent_name,
                    session_id=session.session_id,
                    error=result.error,
                )

            return result

        except Exception as e:
            return AgentInvokeOutput(
                agent_name=input_data.agent_name,
                error=f"Invocation failed: {e}",
            )

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

    def get_schema(self) -> dict[str, Any]:
        """Get JSON schema for this tool."""
        return {
            "name": self.name,
            "description": self.description,
            "parameters": InvokeAgentInput.model_json_schema(),
        }


# =============================================================================
# Share Reasoning Tool
# =============================================================================


class ShareReasoningInput(BaseModel):
    """Input for agent_share_your_reasoning tool.

    Attributes:
        reasoning: Current thought process and analysis.
        next_steps: Planned upcoming actions.
    """

    reasoning: str = Field(
        default="",
        description=(
            "Current thought process, analysis, or reasoning. "
            "Should explain the 'why' behind decisions."
        ),
    )
    next_steps: str | None = Field(
        default=None,
        description="Planned upcoming actions or steps.",
    )


class ReasoningOutput(BaseModel):
    """Output from agent_share_your_reasoning tool.

    Attributes:
        success: Always True (reasoning was shared).
    """

    success: bool = Field(default=True, description="Success status")


class ShareReasoningTool(BaseTool[ShareReasoningInput, ReasoningOutput]):
    """Tool for sharing agent reasoning with the user.

    This tool provides transparency into the agent's decision-making
    process by displaying current reasoning and planned actions.

    The reasoning is typically displayed in a special UI format
    distinct from regular response text.
    """

    def __init__(
        self,
        display_callback: Callable[[str, str | None], None] | None = None,
    ) -> None:
        """Initialize the tool.

        Args:
            display_callback: Callback to display reasoning.
                Takes (reasoning, next_steps).
        """
        super().__init__(
            ToolConfig(
                name="agent_share_your_reasoning",
                description=(
                    "Share your current reasoning and planned next steps with the user. "
                    "Use this to explain complex decision-making processes."
                ),
            )
        )
        self._display_callback = display_callback

    async def execute(
        self,
        input_data: ShareReasoningInput,
        context: RunContext | None = None,
    ) -> ReasoningOutput:
        """Execute the share_reasoning operation.

        Args:
            input_data: Reasoning and next steps.
            context: Optional run context.

        Returns:
            ReasoningOutput (always success).
        """
        if self._display_callback:
            try:
                self._display_callback(
                    input_data.reasoning,
                    input_data.next_steps,
                )
            except Exception:
                # Display errors shouldn't fail the operation
                pass

        return ReasoningOutput(success=True)

    def get_schema(self) -> dict[str, Any]:
        """Get JSON schema for this tool."""
        return {
            "name": self.name,
            "description": self.description,
            "parameters": ShareReasoningInput.model_json_schema(),
        }


__all__ = [
    # List Agents
    "AgentListEntry",
    "ListAgentsInput",
    "ListAgentsOutput",
    "ListAgentsTool",
    # Invoke Agent
    "AgentInvokeOutput",
    "InvocationSession",
    "InvokeAgentInput",
    "InvokeAgentTool",
    # Share Reasoning
    "ReasoningOutput",
    "ShareReasoningInput",
    "ShareReasoningTool",
]
