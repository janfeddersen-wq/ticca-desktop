"""Token estimation utilities for Ticca Agent.

This module provides token counting and estimation utilities for
managing context windows across different AI models. Accurate token
counting is crucial for:

- Staying within model context limits
- Optimizing prompt construction
- Cost estimation and tracking
- Context compaction decisions

The module supports multiple tokenization strategies:
- Fast character-based estimation (4 chars ≈ 1 token)
- tiktoken for OpenAI models
- Model-specific tokenizers when available

Example:
    >>> from ticca_agent.utils import estimate_tokens, count_tokens
    >>> estimate = estimate_tokens("Hello, how are you?")
    >>> print(f"Estimated: ~{estimate} tokens")
    >>> exact = await count_tokens("Hello, how are you?", model="gpt-4")
    >>> print(f"Exact: {exact} tokens")

"""

from __future__ import annotations

import re
from enum import Enum
from functools import lru_cache
from typing import TYPE_CHECKING, Any

from pydantic import BaseModel, Field

if TYPE_CHECKING:
    from ticca_agent.core.types import Message


class TokenizerType(str, Enum):
    """Available tokenizer implementations."""

    ESTIMATE = "estimate"
    """Fast character-based estimation."""

    TIKTOKEN = "tiktoken"
    """OpenAI's tiktoken library."""

    TRANSFORMERS = "transformers"
    """HuggingFace transformers tokenizer."""

    CUSTOM = "custom"
    """Custom tokenizer implementation."""


class TokenCount(BaseModel):
    """Token count result with metadata.

    Attributes:
        count: Number of tokens.
        tokenizer: Tokenizer used for counting.
        is_estimate: Whether this is an estimate or exact count.
        model: Model the count is for (if applicable).
    """

    count: int = Field(ge=0, description="Number of tokens")
    tokenizer: TokenizerType = Field(description="Tokenizer used")
    is_estimate: bool = Field(
        default=False,
        description="Whether this is an estimate",
    )
    model: str | None = Field(
        default=None,
        description="Model the count is for",
    )

    model_config = {"frozen": True}


class TokenBudget(BaseModel):
    """Token budget for a context window.

    Attributes:
        max_tokens: Maximum tokens allowed.
        reserved_output: Tokens reserved for output.
        reserved_system: Tokens reserved for system prompt.
        available: Tokens available for conversation.
    """

    max_tokens: int = Field(ge=1, description="Maximum tokens allowed")
    reserved_output: int = Field(
        default=4096,
        ge=0,
        description="Tokens reserved for output",
    )
    reserved_system: int = Field(
        default=1000,
        ge=0,
        description="Tokens reserved for system prompt",
    )

    @property
    def available(self) -> int:
        """Calculate available tokens for conversation."""
        return max(0, self.max_tokens - self.reserved_output - self.reserved_system)

    model_config = {"frozen": True}


# Character-to-token ratio for estimation
# This is a rough average across models
CHARS_PER_TOKEN = 4

# Model-specific context window sizes
MODEL_CONTEXT_SIZES: dict[str, int] = {
    # OpenAI models
    "gpt-4": 8192,
    "gpt-4-32k": 32768,
    "gpt-4-turbo": 128000,
    "gpt-4o": 128000,
    "gpt-3.5-turbo": 16385,
    # Anthropic models
    "claude-3-opus": 200000,
    "claude-3-sonnet": 200000,
    "claude-3-haiku": 200000,
    "claude-3.5-sonnet": 200000,
    "claude-2.1": 200000,
    "claude-2": 100000,
    # Default
    "default": 8192,
}


def estimate_tokens(text: str) -> int:
    """Fast token estimation using character count.

    This provides a quick estimate suitable for most use cases.
    For exact counts, use count_tokens() with a specific model.

    Args:
        text: Text to estimate tokens for.

    Returns:
        Estimated token count.

    Example:
        >>> estimate_tokens("Hello, world!")
        3
    """
    if not text:
        return 0

    # Basic estimation: ~4 characters per token
    return max(1, len(text) // CHARS_PER_TOKEN)


def estimate_tokens_detailed(text: str) -> TokenCount:
    """Detailed token estimation with metadata.

    Args:
        text: Text to estimate tokens for.

    Returns:
        TokenCount with estimation details.
    """
    return TokenCount(
        count=estimate_tokens(text),
        tokenizer=TokenizerType.ESTIMATE,
        is_estimate=True,
    )


async def count_tokens(
    text: str,
    *,
    model: str | None = None,
) -> TokenCount:
    """Count tokens for text using appropriate tokenizer.

    This function attempts to use the most accurate tokenizer
    available for the specified model.

    Args:
        text: Text to count tokens for.
        model: Target model (affects tokenizer choice).

    Returns:
        TokenCount with exact or estimated count.
    """
    if not text:
        return TokenCount(
            count=0,
            tokenizer=TokenizerType.ESTIMATE,
            is_estimate=False,
            model=model,
        )

    # Try tiktoken for OpenAI models
    if model and (model.startswith("gpt") or model.startswith("text-")):
        try:
            count = await _count_with_tiktoken(text, model)
            return TokenCount(
                count=count,
                tokenizer=TokenizerType.TIKTOKEN,
                is_estimate=False,
                model=model,
            )
        except ImportError:
            pass  # Fall back to estimation

    # Default to estimation
    return TokenCount(
        count=estimate_tokens(text),
        tokenizer=TokenizerType.ESTIMATE,
        is_estimate=True,
        model=model,
    )


async def _count_with_tiktoken(
    text: str,  # noqa: ARG001 - will be used when tiktoken is installed
    model: str,  # noqa: ARG001 - will be used when tiktoken is installed
) -> int:
    """Count tokens using tiktoken.

    Args:
        text: Text to count.
        model: Model name for encoding selection.

    Returns:
        Exact token count.

    Raises:
        ImportError: If tiktoken is not installed.
    """
    # TODO: Install and use tiktoken
    # import tiktoken
    # encoding = tiktoken.encoding_for_model(model)
    # return len(encoding.encode(text))
    raise ImportError("tiktoken is not installed")


def estimate_message_tokens(message: Message) -> int:
    """Estimate tokens for a message.

    Accounts for message overhead (role, formatting) in addition
    to content tokens.

    Args:
        message: Message to estimate.

    Returns:
        Estimated token count including overhead.
    """
    # Base overhead for message structure (~4 tokens)
    overhead = 4

    # Content tokens
    content_tokens = estimate_tokens(message.content)

    # Role tokens (~1 token)
    role_tokens = 1

    # Name tokens if present
    name_tokens = estimate_tokens(message.name) if message.name else 0

    # Tool call overhead if present
    tool_tokens = 0
    if message.tool_calls:
        # Rough estimate for tool call structure
        tool_tokens = len(message.tool_calls) * 20

    return overhead + content_tokens + role_tokens + name_tokens + tool_tokens


def estimate_messages_tokens(messages: list[Message]) -> int:
    """Estimate total tokens for a list of messages.

    Args:
        messages: Messages to estimate.

    Returns:
        Total estimated token count.
    """
    # Base overhead for message array (~3 tokens)
    total = 3

    for message in messages:
        total += estimate_message_tokens(message)

    return total


def get_context_size(model: str) -> int:
    """Get the context window size for a model.

    Args:
        model: Model identifier.

    Returns:
        Context window size in tokens.
    """
    # Check for exact match
    if model in MODEL_CONTEXT_SIZES:
        return MODEL_CONTEXT_SIZES[model]

    # Check for prefix match
    for prefix, size in MODEL_CONTEXT_SIZES.items():
        if model.startswith(prefix):
            return size

    # Return default
    return MODEL_CONTEXT_SIZES["default"]


def create_budget(
    model: str,
    *,
    reserved_output: int = 4096,
    reserved_system: int = 1000,
) -> TokenBudget:
    """Create a token budget for a model.

    Args:
        model: Model identifier.
        reserved_output: Tokens to reserve for output.
        reserved_system: Tokens to reserve for system prompt.

    Returns:
        TokenBudget for the model.
    """
    return TokenBudget(
        max_tokens=get_context_size(model),
        reserved_output=reserved_output,
        reserved_system=reserved_system,
    )


def fits_in_context(
    messages: list[Message],
    budget: TokenBudget,
) -> bool:
    """Check if messages fit within a token budget.

    Args:
        messages: Messages to check.
        budget: Token budget.

    Returns:
        True if messages fit within available tokens.
    """
    estimated = estimate_messages_tokens(messages)
    return estimated <= budget.available


def truncate_to_budget(
    messages: list[Message],
    budget: TokenBudget,
    *,
    preserve_system: bool = True,
    preserve_recent: int = 2,
) -> list[Message]:
    """Truncate messages to fit within budget.

    This is a simple truncation strategy that removes oldest messages
    first while preserving system messages and recent context.

    Args:
        messages: Messages to truncate.
        budget: Token budget.
        preserve_system: Whether to preserve system messages.
        preserve_recent: Number of recent messages to always keep.

    Returns:
        Truncated message list.
    """
    from ticca_agent.core.types import MessageRole

    if fits_in_context(messages, budget):
        return messages

    # Separate system and other messages
    system_messages: list[Message] = []
    other_messages: list[Message] = []

    for msg in messages:
        if preserve_system and msg.role == MessageRole.SYSTEM:
            system_messages.append(msg)
        else:
            other_messages.append(msg)

    # Preserve recent messages
    recent = other_messages[-preserve_recent:] if preserve_recent else []
    middle = other_messages[:-preserve_recent] if preserve_recent else other_messages

    # Calculate available budget for middle messages
    system_tokens = estimate_messages_tokens(system_messages)
    recent_tokens = estimate_messages_tokens(recent)
    available_for_middle = budget.available - system_tokens - recent_tokens

    # Add middle messages from most recent until budget exceeded
    kept_middle: list[Message] = []
    middle_tokens = 0

    for msg in reversed(middle):
        msg_tokens = estimate_message_tokens(msg)
        if middle_tokens + msg_tokens <= available_for_middle:
            kept_middle.insert(0, msg)
            middle_tokens += msg_tokens
        else:
            break

    return system_messages + kept_middle + recent


@lru_cache(maxsize=128)
def _word_count(text: str) -> int:
    """Count words in text (cached).

    Args:
        text: Text to count words in.

    Returns:
        Word count.
    """
    return len(re.findall(r"\b\w+\b", text))


def estimate_from_words(word_count: int) -> int:
    """Estimate tokens from word count.

    On average, 1 word ≈ 1.3 tokens for English text.

    Args:
        word_count: Number of words.

    Returns:
        Estimated token count.
    """
    return int(word_count * 1.3)


def estimate_context_overhead(
    system_prompt: str,
    tool_definitions: list[dict[str, Any]] | None = None,
    mcp_tool_definitions: list[dict[str, Any]] | None = None,
) -> int:
    """Estimate tokens for system prompt and tool definitions.

    This estimates the "overhead" tokens consumed before any conversation
    messages are added. Useful for calculating available context budget.

    Args:
        system_prompt: The system prompt text.
        tool_definitions: List of native tool definitions (JSON schema format).
        mcp_tool_definitions: List of MCP tool definitions.

    Returns:
        Estimated token count for the context overhead.

    Example:
        >>> overhead = estimate_context_overhead(
        ...     "You are a helpful assistant.",
        ...     tool_definitions=[{"name": "search", "description": "..."}],
        ... )
        >>> print(f"Context overhead: {overhead} tokens")
    """
    total = 0

    # System prompt tokens
    if system_prompt:
        total += estimate_tokens(system_prompt)
        # Add overhead for system message structure (~4 tokens)
        total += 4

    # Tool definitions overhead
    # Each tool contributes: name + description + parameter schema
    if tool_definitions:
        for tool in tool_definitions:
            # Tool structure overhead (~10 tokens per tool)
            total += 10

            # Name (usually 1-3 tokens)
            if name := tool.get("name"):
                total += estimate_tokens(str(name))

            # Description
            if desc := tool.get("description"):
                total += estimate_tokens(str(desc))

            # Parameters schema (can be significant)
            if params := tool.get("parameters"):
                # Estimate based on JSON serialization length
                import json
                params_json = json.dumps(params)
                total += estimate_tokens(params_json)

    # MCP tool definitions (similar structure)
    if mcp_tool_definitions:
        for tool in mcp_tool_definitions:
            # MCP tools have similar overhead
            total += 10

            if name := tool.get("name"):
                total += estimate_tokens(str(name))

            if desc := tool.get("description"):
                total += estimate_tokens(str(desc))

            if input_schema := tool.get("inputSchema"):
                import json
                schema_json = json.dumps(input_schema)
                total += estimate_tokens(schema_json)

    return total


def calculate_available_context(
    model_context_length: int,
    system_prompt: str,
    tool_definitions: list[dict[str, Any]] | None = None,
    mcp_tool_definitions: list[dict[str, Any]] | None = None,
    reserved_output: int = 4096,
) -> int:
    """Calculate available context tokens for conversation.

    This computes how many tokens are available for conversation messages
    after accounting for system prompt, tools, and output reservation.

    Args:
        model_context_length: Total context window size.
        system_prompt: The system prompt text.
        tool_definitions: List of native tool definitions.
        mcp_tool_definitions: List of MCP tool definitions.
        reserved_output: Tokens to reserve for model output.

    Returns:
        Available tokens for conversation messages.

    Example:
        >>> available = calculate_available_context(
        ...     model_context_length=128000,
        ...     system_prompt="You are a helpful assistant.",
        ...     reserved_output=4096,
        ... )
        >>> print(f"Available for messages: {available} tokens")
    """
    overhead = estimate_context_overhead(
        system_prompt,
        tool_definitions,
        mcp_tool_definitions,
    )
    return max(0, model_context_length - overhead - reserved_output)


__all__ = [
    "CHARS_PER_TOKEN",
    "MODEL_CONTEXT_SIZES",
    "TokenBudget",
    "TokenCount",
    "TokenizerType",
    "calculate_available_context",
    "count_tokens",
    "create_budget",
    "estimate_context_overhead",
    "estimate_from_words",
    "estimate_message_tokens",
    "estimate_messages_tokens",
    "estimate_tokens",
    "estimate_tokens_detailed",
    "fits_in_context",
    "get_context_size",
    "truncate_to_budget",
]
