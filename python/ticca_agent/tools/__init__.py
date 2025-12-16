"""Tool implementations for Ticca Agent.

This module contains tool definitions that extend agent capabilities,
including file operations, code execution, shell commands, and agent coordination.

Tools follow the standard function-calling interface and can be:
- Registered dynamically at runtime
- Scoped to specific agents or conversations
- Validated with Pydantic schemas

Available Tool Categories:

**File Operations:**
- ListFilesTool: Directory exploration with recursive support
- ReadFileTool: Read file contents with token estimation
- GrepTool: Text search across files
- EditFileTool: Create, modify, and delete file content
- DeleteFileTool: Remove files permanently

**Shell Execution:**
- ShellCommandTool: Execute shell commands with output capture

**Agent Coordination:**
- ListAgentsTool: List available sub-agents
- InvokeAgentTool: Invoke sub-agents with session management
- ShareReasoningTool: Share thought process with user

Key Classes:
- BaseTool: Abstract base class for all tools
- ToolConfig: Configuration for tool behavior
- ToolRegistry: Registry for managing available tools

Example:
    >>> from ticca_agent.tools import ReadFileTool
    >>> tool = ReadFileTool()
    >>> result = await tool.run({"file_path": "main.py"})
    >>> print(result.output.content)

"""

from __future__ import annotations

from typing import Any

# Base classes
from ticca_agent.tools.base import (
    BaseTool,
    InputT,
    OutputT,
    ToolConfig,
    ToolExecutionError,
    ToolRegistry,
    ToolResult,
    default_registry,
)

# File operation tools
from ticca_agent.tools.file_ops import (
    ContentPayload,
    DeleteFileInput,
    DeleteFileOutput,
    DeleteFileTool,
    DeleteSnippetPayload,
    EditFileOutput,
    EditFileTool,
    FileEntry,
    GrepInput,
    GrepOutput,
    GrepTool,
    IGNORE_PATTERNS,
    ListFilesInput,
    ListFilesOutput,
    ListFilesTool,
    MatchInfo,
    ReadFileInput,
    ReadFileOutput,
    ReadFileTool,
    Replacement,
    ReplacementsPayload,
    estimate_tokens,
    should_ignore,
)

# Shell execution tool
from ticca_agent.tools.shell import (
    DEFAULT_TIMEOUT,
    MAX_LINE_LENGTH,
    MAX_OUTPUT_LINES,
    ShellCommandInput,
    ShellCommandOutput,
    ShellCommandTool,
)

# Agent coordination tools
from ticca_agent.tools.agent_tools import (
    AgentInvokeOutput,
    AgentListEntry,
    InvocationSession,
    InvokeAgentInput,
    InvokeAgentTool,
    ListAgentsInput,
    ListAgentsOutput,
    ListAgentsTool,
    ReasoningOutput,
    ShareReasoningInput,
    ShareReasoningTool,
)


def create_default_tools() -> list[BaseTool[Any, Any]]:
    """Create a set of default tools for agents.

    Returns:
        List of commonly-used tools.
    """
    return [
        ListFilesTool(),
        ReadFileTool(),
        GrepTool(),
        EditFileTool(),
        DeleteFileTool(),
        ShellCommandTool(),
        ShareReasoningTool(),
    ]


def create_read_only_tools() -> list[BaseTool[Any, Any]]:
    """Create a set of read-only tools (no file modifications).

    Suitable for review agents and analysis tasks.

    Returns:
        List of read-only tools.
    """
    return [
        ListFilesTool(),
        ReadFileTool(),
        GrepTool(),
    ]


def create_coordination_tools() -> list[BaseTool[Any, Any]]:
    """Create tools for agent coordination.

    Suitable for planning and orchestration agents.

    Returns:
        List of coordination tools.
    """
    return [
        ListAgentsTool(),
        InvokeAgentTool(),
        ShareReasoningTool(),
        ListFilesTool(),
        ReadFileTool(),
    ]


__all__ = [
    # Base classes
    "BaseTool",
    "InputT",
    "OutputT",
    "ToolConfig",
    "ToolExecutionError",
    "ToolRegistry",
    "ToolResult",
    "default_registry",
    # File operations
    "ContentPayload",
    "DeleteFileInput",
    "DeleteFileOutput",
    "DeleteFileTool",
    "DeleteSnippetPayload",
    "EditFileOutput",
    "EditFileTool",
    "FileEntry",
    "GrepInput",
    "GrepOutput",
    "GrepTool",
    "IGNORE_PATTERNS",
    "ListFilesInput",
    "ListFilesOutput",
    "ListFilesTool",
    "MatchInfo",
    "ReadFileInput",
    "ReadFileOutput",
    "ReadFileTool",
    "Replacement",
    "ReplacementsPayload",
    "estimate_tokens",
    "should_ignore",
    # Shell execution
    "DEFAULT_TIMEOUT",
    "MAX_LINE_LENGTH",
    "MAX_OUTPUT_LINES",
    "ShellCommandInput",
    "ShellCommandOutput",
    "ShellCommandTool",
    # Agent coordination
    "AgentInvokeOutput",
    "AgentListEntry",
    "InvocationSession",
    "InvokeAgentInput",
    "InvokeAgentTool",
    "ListAgentsInput",
    "ListAgentsOutput",
    "ListAgentsTool",
    "ReasoningOutput",
    "ShareReasoningInput",
    "ShareReasoningTool",
    # Factory functions
    "create_coordination_tools",
    "create_default_tools",
    "create_read_only_tools",
]
