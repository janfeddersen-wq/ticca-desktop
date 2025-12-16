"""Ticca Agent - AI Agent system for Ticca Desktop.

This package provides a flexible, type-safe AI agent framework built on
PydanticAI for seamless integration with multiple AI providers.

Key Features:
- Multiple AI provider support (OpenAI, Anthropic, local models)
- Type-safe agent and tool definitions with Pydantic
- Streaming response support
- Session management with context compaction
- MCP (Model Context Protocol) integration
- Rust-Python bridge for Tauri integration

Example:
    >>> from ticca_agent import Agent, AgentConfig
    >>> from ticca_agent.providers import BaseProvider
    >>>
    >>> config = AgentConfig(name="assistant")
    >>> agent = Agent(config=config, provider=my_provider)
    >>> response = await agent.run("Hello, world!")

"""

from __future__ import annotations

# Version and metadata
__version__ = "0.1.0"
__author__ = "Ticca Team"

# Core agent classes
from ticca_agent.core.agent import Agent, AgentConfig, AgentError, AgentProtocol
from ticca_agent.core.context import ContextConfig, ContextError, RunContext
from ticca_agent.core.invocation import (
    InvocationError,
    InvocationRequest,
    InvocationResult,
    get_agent,
    invoke_agent,
    list_agents,
    register_agent,
    unregister_agent,
)
from ticca_agent.core.types import (
    Message,
    MessageRole,
    ModelInfo,
)

# Bridge for Rust integration - use bridge types as canonical
from ticca_agent.bridge import (
    BridgeError,
    BridgeInterface,
    create_bridge,
    initialize_bridge,
    register_agent as register_bridge_agent,
    unregister_agent as unregister_bridge_agent,
    get_agent_info,
    get_agent_provider,
    list_registered_agents,
    clear_agent_registry,
)

# Bridge types - these are the canonical types for Rust-Python communication
from ticca_agent.bridge_types import (
    AgentInfo,
    AgentRequest,
    AgentResponse,
    AgentState,
    DoneChunk,
    ErrorChunk,
    FinishReason,
    FinishReasonError,
    FinishReasonMaxTokens,
    FinishReasonStop,
    FinishReasonToolUse,
    StateChangeChunk,
    StreamChunk,
    TextDeltaChunk,
    ThinkingDeltaChunk,
    TokenUsage,
    ToolCall,
    ToolResultChunk,
    ToolStartChunk,
    UsageChunk,
)

__all__ = [
    # Metadata
    "__version__",
    "__author__",
    # Core Agent
    "Agent",
    "AgentConfig",
    "AgentError",
    "AgentProtocol",
    # Context
    "RunContext",
    "ContextConfig",
    "ContextError",
    # Invocation
    "InvocationError",
    "InvocationRequest",
    "InvocationResult",
    "get_agent",
    "invoke_agent",
    "list_agents",
    "register_agent",
    "unregister_agent",
    # Core Types
    "Message",
    "MessageRole",
    "ModelInfo",
    # Bridge Interface
    "BridgeError",
    "BridgeInterface",
    "create_bridge",
    "initialize_bridge",
    "register_bridge_agent",
    "unregister_bridge_agent",
    "get_agent_info",
    "get_agent_provider",
    "list_registered_agents",
    "clear_agent_registry",
    # Bridge Types (Rust-compatible)
    "AgentInfo",
    "AgentRequest",
    "AgentResponse",
    "AgentState",
    "TokenUsage",
    "ToolCall",
    "FinishReason",
    "FinishReasonStop",
    "FinishReasonToolUse",
    "FinishReasonMaxTokens",
    "FinishReasonError",
    # Stream Chunks
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
