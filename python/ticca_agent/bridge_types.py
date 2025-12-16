"""Bridge type definitions matching Rust types.rs.

This module defines Pydantic models that serialize/deserialize to match
the Rust bridge types exactly. These types are used for Rust-Python
communication via PyO3.

IMPORTANT: These types MUST match the Rust types in crates/ticca-bridge/src/types.rs.
Any changes here require corresponding changes in Rust.

# Serialization Notes:

- FinishReason uses tagged enum format: {"type": "Stop"} or {"type": "Error", "value": "msg"}
- StreamChunk uses externally tagged format: {"type": "TextDelta", "content": "..."}
- AgentState is lowercase: "idle", "thinking", "acting", "observing", "responding"
"""

from __future__ import annotations

from enum import Enum
from typing import Annotated, Any, Literal, Union

from pydantic import BaseModel, Field

__all__ = [
    # Request/Response
    "AgentRequest",
    "AgentResponse",
    # Sub-types
    "ToolCall",
    "TokenUsage",
    "AgentState",
    "AgentInfo",
    # FinishReason variants
    "FinishReason",
    "FinishReasonStop",
    "FinishReasonToolUse",
    "FinishReasonMaxTokens",
    "FinishReasonError",
    # Stream chunks
    "StreamChunk",
    "TextDeltaChunk",
    "ThinkingDeltaChunk",
    "ToolStartChunk",
    "ToolResultChunk",
    "StateChangeChunk",
    "UsageChunk",
    "DoneChunk",
    "ErrorChunk",
]


# =============================================================================
# Core Types
# =============================================================================


class AgentRequest(BaseModel):
    """Request from Rust to Python agent.

    Contains all information needed to execute an agent request,
    including conversation context and model configuration.

    Attributes:
        session_id: Unique session identifier.
        conversation_id: Unique conversation identifier within the session.
        message: The user's message.
        agent_name: Name of the agent to invoke.
        model: Optional model override.
        temperature: Temperature for generation (0.0 - 2.0).
        max_tokens: Maximum tokens to generate.
        tools_enabled: List of enabled tool names.
        context: Additional context data.
    """

    session_id: str = Field(description="Unique session identifier")
    conversation_id: str = Field(description="Unique conversation identifier")
    message: str = Field(description="The user's message")
    agent_name: str = Field(description="Name of the agent to invoke")
    model: str | None = Field(default=None, description="Optional model override")
    temperature: float | None = Field(
        default=None,
        ge=0.0,
        le=2.0,
        description="Temperature for generation",
    )
    max_tokens: int | None = Field(
        default=None,
        ge=1,
        description="Maximum tokens to generate",
    )
    tools_enabled: list[str] = Field(
        default_factory=list,
        description="List of enabled tool names",
    )
    context: dict[str, Any] = Field(
        default_factory=dict,
        description="Additional context data",
    )

    model_config = {"frozen": True}


class ToolCall(BaseModel):
    """A tool call made by the agent.

    Attributes:
        id: Unique identifier for this tool call.
        name: Name of the tool being called.
        arguments: Arguments passed to the tool (as dict, not JSON string).
    """

    id: str = Field(description="Unique identifier for this tool call")
    name: str = Field(description="Name of the tool being called")
    arguments: dict[str, Any] = Field(
        default_factory=dict,
        description="Arguments passed to the tool",
    )

    model_config = {"frozen": True}


class TokenUsage(BaseModel):
    """Token usage statistics.

    Attributes:
        prompt_tokens: Tokens used in the prompt.
        completion_tokens: Tokens generated in completion.
        total_tokens: Total tokens used.
    """

    prompt_tokens: int = Field(default=0, ge=0, description="Tokens used in the prompt")
    completion_tokens: int = Field(
        default=0, ge=0, description="Tokens generated in completion"
    )
    total_tokens: int = Field(default=0, ge=0, description="Total tokens used")

    model_config = {"frozen": True}

    @classmethod
    def create(cls, prompt: int, completion: int) -> TokenUsage:
        """Create token usage with calculated total.

        Args:
            prompt: Number of prompt tokens.
            completion: Number of completion tokens.

        Returns:
            TokenUsage instance with total calculated.
        """
        return cls(
            prompt_tokens=prompt,
            completion_tokens=completion,
            total_tokens=prompt + completion,
        )


# =============================================================================
# FinishReason - Tagged Enum matching Rust
# =============================================================================


class FinishReasonBase(BaseModel):
    """Base class for finish reason variants.

    Rust uses `#[serde(tag = "type", content = "value")]` which produces:
    - {"type": "Stop"}
    - {"type": "ToolUse"}
    - {"type": "MaxTokens"}
    - {"type": "Error", "value": "error message"}
    """

    model_config = {"frozen": True}


class FinishReasonStop(FinishReasonBase):
    """Normal completion (stop sequence hit)."""

    type: Literal["Stop"] = "Stop"


class FinishReasonToolUse(FinishReasonBase):
    """Agent wants to use a tool."""

    type: Literal["ToolUse"] = "ToolUse"


class FinishReasonMaxTokens(FinishReasonBase):
    """Hit the maximum token limit."""

    type: Literal["MaxTokens"] = "MaxTokens"


class FinishReasonError(FinishReasonBase):
    """An error occurred."""

    type: Literal["Error"] = "Error"
    value: str = Field(description="Error message")


# Discriminated union for FinishReason
FinishReason = Annotated[
    Union[
        FinishReasonStop,
        FinishReasonToolUse,
        FinishReasonMaxTokens,
        FinishReasonError,
    ],
    Field(discriminator="type"),
]


# =============================================================================
# AgentState Enum
# =============================================================================


class AgentState(str, Enum):
    """Agent execution state.

    Represents the current phase of agent execution,
    useful for UI feedback. Serializes to lowercase.
    """

    IDLE = "idle"
    THINKING = "thinking"
    ACTING = "acting"
    OBSERVING = "observing"
    RESPONDING = "responding"


# =============================================================================
# AgentResponse
# =============================================================================


class AgentResponse(BaseModel):
    """Response from Python agent to Rust.

    Represents a complete agent response including any tool calls
    and token usage statistics.

    Attributes:
        message_id: Unique message identifier.
        content: Response content.
        role: Role of the responder (usually "assistant").
        tool_calls: Any tool calls made by the agent.
        usage: Token usage statistics.
        finish_reason: Reason the generation finished.
    """

    message_id: str = Field(description="Unique message identifier")
    content: str = Field(default="", description="Response content")
    role: str = Field(default="assistant", description="Role of the responder")
    tool_calls: list[ToolCall] | None = Field(
        default=None,
        description="Tool calls made by the agent",
    )
    usage: TokenUsage = Field(
        default_factory=TokenUsage,
        description="Token usage statistics",
    )
    finish_reason: FinishReason = Field(
        default_factory=FinishReasonStop,
        description="Reason the generation finished",
    )

    model_config = {"frozen": True}

    @classmethod
    def text(
        cls,
        message_id: str,
        content: str,
        usage: TokenUsage | None = None,
    ) -> AgentResponse:
        """Create a simple text response.

        Args:
            message_id: Unique message identifier.
            content: Response text content.
            usage: Optional token usage statistics.

        Returns:
            AgentResponse instance.
        """
        return cls(
            message_id=message_id,
            content=content,
            usage=usage or TokenUsage(),
            finish_reason=FinishReasonStop(),
        )

    @classmethod
    def error(cls, message_id: str, error_message: str) -> AgentResponse:
        """Create an error response.

        Args:
            message_id: Unique message identifier.
            error_message: The error message.

        Returns:
            AgentResponse instance with error finish reason.
        """
        return cls(
            message_id=message_id,
            content="",
            finish_reason=FinishReasonError(value=error_message),
        )


# =============================================================================
# StreamChunk - Externally Tagged Enum matching Rust
# =============================================================================


class StreamChunkBase(BaseModel):
    """Base class for stream chunk variants.

    Rust uses `#[serde(tag = "type")]` which produces externally tagged variants:
    - {"type": "TextDelta", "content": "..."}
    - {"type": "Done", "response": {...}}
    """

    model_config = {"frozen": True}


class TextDeltaChunk(StreamChunkBase):
    """Text content delta."""

    type: Literal["TextDelta"] = "TextDelta"
    content: str = Field(description="Text content")


class ThinkingDeltaChunk(StreamChunkBase):
    """Thinking/reasoning delta (for models that expose this)."""

    type: Literal["ThinkingDelta"] = "ThinkingDelta"
    content: str = Field(description="Thinking content")


class ToolStartChunk(StreamChunkBase):
    """A tool call is starting."""

    type: Literal["ToolStart"] = "ToolStart"
    tool_call: ToolCall = Field(description="Tool call information")


class ToolResultChunk(StreamChunkBase):
    """Result from a tool call."""

    type: Literal["ToolResult"] = "ToolResult"
    tool_call_id: str = Field(description="ID of the tool call")
    result: str = Field(description="Tool execution result")


class StateChangeChunk(StreamChunkBase):
    """Agent state transition."""

    type: Literal["StateChange"] = "StateChange"
    # Note: Rust uses 'from' and 'to' but 'from' is a Python keyword
    # We use serialization_alias to serialize to 'from'/'to' while
    # accepting 'from_state'/'to_state' for Python construction
    from_state: AgentState = Field(
        serialization_alias="from",
        description="Previous state",
    )
    to_state: AgentState = Field(
        serialization_alias="to",
        description="New state",
    )

    model_config = {"frozen": True}


class UsageChunk(StreamChunkBase):
    """Token usage update."""

    type: Literal["Usage"] = "Usage"
    usage: TokenUsage = Field(description="Token usage statistics")


class DoneChunk(StreamChunkBase):
    """Stream completed successfully."""

    type: Literal["Done"] = "Done"
    response: AgentResponse = Field(description="Final response")


class ErrorChunk(StreamChunkBase):
    """An error occurred."""

    type: Literal["Error"] = "Error"
    message: str = Field(description="Error message")


# Discriminated union for StreamChunk
StreamChunk = Annotated[
    Union[
        TextDeltaChunk,
        ThinkingDeltaChunk,
        ToolStartChunk,
        ToolResultChunk,
        StateChangeChunk,
        UsageChunk,
        DoneChunk,
        ErrorChunk,
    ],
    Field(discriminator="type"),
]


# =============================================================================
# AgentInfo
# =============================================================================


class AgentInfo(BaseModel):
    """Information about an available agent.

    Attributes:
        id: Unique agent identifier.
        name: Human-readable name.
        description: Agent description.
        default_model: Default model for this agent.
        available_tools: Available tools for this agent.
        capabilities: Agent capabilities/tags.
    """

    id: str = Field(description="Unique agent identifier")
    name: str = Field(description="Human-readable name")
    description: str | None = Field(default=None, description="Agent description")
    default_model: str = Field(description="Default model for this agent")
    available_tools: list[str] = Field(
        default_factory=list,
        description="Available tools for this agent",
    )
    capabilities: list[str] = Field(
        default_factory=list,
        description="Agent capabilities/tags",
    )

    model_config = {"frozen": True}
