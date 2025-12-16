"""Core types and utilities for Ticca Agent.

This module contains the fundamental data structures and types used
throughout the Ticca Agent system, including:

- Agent: The main agent class for AI interactions
- RunContext: Execution context for agent runs
- Invocation: Sub-agent invocation utilities
- Prompts: System prompt composition and Claude-Code handling
- Types: Pydantic models for requests, responses, and messages

"""

from __future__ import annotations

# Agent classes
from ticca_agent.core.agent import (
    Agent,
    AgentConfig,
    AgentError,
    AgentProtocol,
    ModelSettings,
    StreamCallback,
)

# Context management
from ticca_agent.core.context import ContextConfig, ContextError, RunContext

# Sub-agent invocation
from ticca_agent.core.invocation import (
    AgentInfo,
    InvocationError,
    InvocationManager,
    InvocationRequest,
    InvocationResult,
    InvocationSession,
    get_agent,
    get_invocation_manager,
    invoke_agent,
    list_agents,
    list_agents_sync,
    register_agent,
    unregister_agent,
)

# Prompt composition
from ticca_agent.core.prompts import (
    CLAUDE_CODE_INSTRUCTION,
    ComposedPrompt,
    PromptComposer,
    PromptConfig,
    PromptInjectionCallback,
    RULE_FILE_NAMES,
    RulesContent,
    compose_prompt,
)

# Core types
from ticca_agent.core.types import (
    AgentRequest,
    AgentResponse,
    Message,
    MessageRole,
    ModelInfo,
    StreamChunk,
    TokenUsage,
)

__all__ = [
    # Agent
    "Agent",
    "AgentConfig",
    "AgentError",
    "AgentProtocol",
    "ModelSettings",
    "StreamCallback",
    # Context
    "ContextConfig",
    "ContextError",
    "RunContext",
    # Invocation
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
    # Prompts
    "CLAUDE_CODE_INSTRUCTION",
    "ComposedPrompt",
    "PromptComposer",
    "PromptConfig",
    "PromptInjectionCallback",
    "RULE_FILE_NAMES",
    "RulesContent",
    "compose_prompt",
    # Types
    "AgentRequest",
    "AgentResponse",
    "Message",
    "MessageRole",
    "ModelInfo",
    "StreamChunk",
    "TokenUsage",
]
