"""Context compaction for Ticca Agent sessions.

This module provides strategies for managing conversation context within
token limits. When conversations grow too long, compaction strategies
help preserve important context while staying within model limits.

Compaction Strategies:
- **Truncation (default):** Simple discard of oldest messages. Fast, no API cost.
- **Summarization:** AI-powered compression preserving semantic meaning.

Protection:
- Recent messages totaling `protected_token_count` tokens are always preserved.
- System messages are always preserved.
- Protected tokens are capped at 75% of model context length.

Example:
    >>> from ticca_agent.session import ContextCompactor, CompactionStrategy
    >>> compactor = ContextCompactor(
    ...     strategy=CompactionStrategy.TRUNCATION,
    ...     protected_token_count=50000,
    ...     compaction_threshold=0.85,
    ... )
    >>> if compactor.should_compact(context_used=100000, context_length=128000):
    ...     result = await compactor.compact(
    ...         messages=conversation_history,
    ...         context_length=128000,
    ...     )
    ...     print(f"Compacted: {result.original_count} -> {result.final_count}")

"""

from __future__ import annotations

from enum import Enum
from typing import TYPE_CHECKING, Any

import structlog
from pydantic import BaseModel, Field

from ticca_agent.utils.tokens import estimate_message_tokens

if TYPE_CHECKING:
    from ticca_agent.core.types import Message
    from ticca_agent.providers.base import BaseProvider

logger = structlog.get_logger(__name__)


# =============================================================================
# Enums and Configuration
# =============================================================================


class CompactionStrategy(str, Enum):
    """Available compaction strategies."""

    TRUNCATION = "truncation"
    """Simple discard of oldest messages. Fast, no API cost."""

    SUMMARIZATION = "summarization"
    """AI-powered compression preserving semantic meaning. Higher quality but has API cost."""


class CompactionConfig(BaseModel):
    """Configuration for context compaction.

    Attributes:
        strategy: Compaction strategy to use.
        protected_token_count: Tokens to protect from compaction (recent messages).
        compaction_threshold: Context usage ratio that triggers compaction.
        max_protected_ratio: Maximum ratio of context for protected tokens.
        preserve_system_messages: Whether to always keep system messages.
        summarization_model: Model to use for summarization (if using that strategy).
    """

    strategy: CompactionStrategy = Field(
        default=CompactionStrategy.TRUNCATION,
        description="Compaction strategy to use",
    )
    protected_token_count: int = Field(
        default=50000,
        ge=1000,
        le=500000,
        description="Number of recent message tokens to protect from compaction",
    )
    compaction_threshold: float = Field(
        default=0.85,
        ge=0.5,
        le=0.95,
        description="Context usage ratio that triggers compaction (e.g., 0.85 = 85%)",
    )
    max_protected_ratio: float = Field(
        default=0.75,
        ge=0.5,
        le=0.9,
        description="Maximum ratio of context length for protected tokens",
    )
    preserve_system_messages: bool = Field(
        default=True,
        description="Whether to always preserve system messages",
    )
    summarization_model: str | None = Field(
        default=None,
        description="Model to use for summarization (uses provider default if not set)",
    )

    model_config = {"frozen": True}


class CompactionResult(BaseModel):
    """Result of a compaction operation.

    Attributes:
        messages: Compacted message list.
        original_count: Number of messages before compaction.
        final_count: Number of messages after compaction.
        original_tokens: Estimated tokens before compaction.
        final_tokens: Estimated tokens after compaction.
        tokens_saved: Tokens saved by compaction.
        strategy_used: Which strategy was applied.
        summary_added: Whether a summary message was added (for summarization).
        metadata: Additional result metadata.
    """

    messages: list[Any] = Field(  # Actually list[Message], but avoiding import
        description="Compacted message list",
    )
    original_count: int = Field(
        ge=0,
        description="Number of messages before compaction",
    )
    final_count: int = Field(
        ge=0,
        description="Number of messages after compaction",
    )
    original_tokens: int = Field(
        ge=0,
        description="Estimated tokens before compaction",
    )
    final_tokens: int = Field(
        ge=0,
        description="Estimated tokens after compaction",
    )
    tokens_saved: int = Field(
        default=0,
        ge=0,
        description="Tokens saved by compaction",
    )
    strategy_used: CompactionStrategy = Field(
        description="Strategy that was applied",
    )
    summary_added: bool = Field(
        default=False,
        description="Whether a summary message was added",
    )
    metadata: dict[str, Any] = Field(
        default_factory=dict,
        description="Additional result metadata",
    )


# =============================================================================
# Context Compactor
# =============================================================================


class ContextCompactor:
    """Compactor for managing context window overflow.

    The ContextCompactor monitors context usage and applies compaction
    strategies when the context window approaches its limit.

    Features:
        - Threshold-based triggering (default: 85% of context)
        - Protected recent messages (default: 50,000 tokens)
        - Multiple strategies: truncation (fast) and summarization (quality)
        - System message preservation

    Example:
        >>> compactor = ContextCompactor(
        ...     strategy=CompactionStrategy.TRUNCATION,
        ...     protected_token_count=50000,
        ...     compaction_threshold=0.85,
        ... )
        >>> if compactor.should_compact(context_used, context_length):
        ...     result = await compactor.compact(messages, context_length)
    """

    def __init__(
        self,
        strategy: CompactionStrategy = CompactionStrategy.TRUNCATION,
        protected_token_count: int = 50000,
        compaction_threshold: float = 0.85,
        *,
        config: CompactionConfig | None = None,
    ) -> None:
        """Initialize the compactor.

        Args:
            strategy: Compaction strategy to use.
            protected_token_count: Tokens to protect from compaction.
            compaction_threshold: Context usage ratio that triggers compaction.
            config: Full configuration (overrides other parameters if provided).
        """
        if config is not None:
            self._config = config
        else:
            self._config = CompactionConfig(
                strategy=strategy,
                protected_token_count=protected_token_count,
                compaction_threshold=compaction_threshold,
            )

    @property
    def config(self) -> CompactionConfig:
        """Get the compaction configuration."""
        return self._config

    @property
    def strategy(self) -> CompactionStrategy:
        """Get the current compaction strategy."""
        return self._config.strategy

    @property
    def protected_token_count(self) -> int:
        """Get the protected token count."""
        return self._config.protected_token_count

    @property
    def compaction_threshold(self) -> float:
        """Get the compaction threshold."""
        return self._config.compaction_threshold

    def should_compact(self, context_used: int, context_length: int) -> bool:
        """Check if compaction is needed based on context usage.

        Compaction is triggered when:
            (context_used / context_length) > compaction_threshold

        Args:
            context_used: Current tokens used in context.
            context_length: Maximum context length for the model.

        Returns:
            True if compaction should be performed.
        """
        if context_length <= 0:
            return False

        usage_ratio = context_used / context_length
        should = usage_ratio > self._config.compaction_threshold

        if should:
            logger.debug(
                "compaction_needed",
                context_used=context_used,
                context_length=context_length,
                usage_ratio=round(usage_ratio, 3),
                threshold=self._config.compaction_threshold,
            )

        return should

    async def compact(
        self,
        messages: list[Message],
        context_length: int,
        summarization_provider: BaseProvider | None = None,
    ) -> CompactionResult:
        """Compact messages using the configured strategy.

        Args:
            messages: Messages to compact.
            context_length: Model's context length for calculating budgets.
            summarization_provider: Provider for summarization (required for
                SUMMARIZATION strategy).

        Returns:
            CompactionResult with compacted messages.

        Raises:
            ValueError: If summarization strategy is used without a provider.
        """
        original_count = len(messages)
        original_tokens = self._estimate_total_tokens(messages)

        # Calculate protected token budget (capped at 75% of context)
        max_protected = int(context_length * self._config.max_protected_ratio)
        effective_protected = min(self._config.protected_token_count, max_protected)

        logger.info(
            "compaction_started",
            strategy=self._config.strategy.value,
            original_count=original_count,
            original_tokens=original_tokens,
            protected_tokens=effective_protected,
            context_length=context_length,
        )

        # Calculate target tokens (leave room for new messages)
        # We want to compact down to threshold - some buffer
        target_tokens = int(context_length * (self._config.compaction_threshold - 0.1))

        if self._config.strategy == CompactionStrategy.TRUNCATION:
            compacted_messages = self._truncate(
                messages,
                target_tokens=target_tokens,
                protected_tokens=effective_protected,
            )
            summary_added = False
        elif self._config.strategy == CompactionStrategy.SUMMARIZATION:
            if summarization_provider is None:
                raise ValueError(
                    "Summarization strategy requires a provider. "
                    "Pass summarization_provider or use TRUNCATION strategy."
                )
            compacted_messages, summary_added = await self._summarize(
                messages,
                target_tokens=target_tokens,
                protected_tokens=effective_protected,
                provider=summarization_provider,
            )
        else:
            # Fallback to truncation
            compacted_messages = self._truncate(
                messages,
                target_tokens=target_tokens,
                protected_tokens=effective_protected,
            )
            summary_added = False

        final_count = len(compacted_messages)
        final_tokens = self._estimate_total_tokens(compacted_messages)
        tokens_saved = original_tokens - final_tokens

        logger.info(
            "compaction_completed",
            strategy=self._config.strategy.value,
            original_count=original_count,
            final_count=final_count,
            original_tokens=original_tokens,
            final_tokens=final_tokens,
            tokens_saved=tokens_saved,
            summary_added=summary_added,
        )

        return CompactionResult(
            messages=compacted_messages,
            original_count=original_count,
            final_count=final_count,
            original_tokens=original_tokens,
            final_tokens=final_tokens,
            tokens_saved=tokens_saved,
            strategy_used=self._config.strategy,
            summary_added=summary_added,
        )

    def _truncate(
        self,
        messages: list[Message],
        target_tokens: int,
        protected_tokens: int,
    ) -> list[Message]:
        """Truncation strategy implementation.

        Simple discard of oldest messages while preserving:
        - All system messages
        - Recent messages within protected token budget

        Args:
            messages: Messages to truncate.
            target_tokens: Target total tokens after truncation.
            protected_tokens: Tokens of recent messages to always keep.

        Returns:
            Truncated message list.
        """
        from ticca_agent.core.types import MessageRole

        # Partition messages
        system_messages: list[Message] = []
        other_messages: list[Message] = []

        for msg in messages:
            if self._config.preserve_system_messages and msg.role == MessageRole.SYSTEM:
                system_messages.append(msg)
            else:
                other_messages.append(msg)

        # Calculate system message tokens
        system_tokens = self._estimate_total_tokens(system_messages)

        # Calculate how many recent messages to protect
        protected_messages: list[Message] = []
        protected_msg_tokens = 0

        # Work backwards from most recent
        for msg in reversed(other_messages):
            msg_tokens = estimate_message_tokens(msg)
            if protected_msg_tokens + msg_tokens <= protected_tokens:
                protected_messages.insert(0, msg)
                protected_msg_tokens += msg_tokens
            else:
                # Once we exceed protected budget, stop
                break

        # Calculate remaining budget for older messages
        remaining_budget = target_tokens - system_tokens - protected_msg_tokens

        # Find where protected messages start in the original list
        protected_start = len(other_messages) - len(protected_messages)
        compactable_messages = other_messages[:protected_start]

        # Add compactable messages from most recent that fit in budget
        kept_middle: list[Message] = []
        middle_tokens = 0

        for msg in reversed(compactable_messages):
            msg_tokens = estimate_message_tokens(msg)
            if middle_tokens + msg_tokens <= remaining_budget:
                kept_middle.insert(0, msg)
                middle_tokens += msg_tokens
            else:
                break

        # Combine: system + kept_middle + protected
        return system_messages + kept_middle + protected_messages

    async def _summarize(
        self,
        messages: list[Message],
        target_tokens: int,  # noqa: ARG002 - reserved for future use
        protected_tokens: int,
        provider: BaseProvider,
    ) -> tuple[list[Message], bool]:
        """Summarization strategy implementation.

        Uses AI to summarize older messages, preserving semantic meaning.

        Args:
            messages: Messages to summarize.
            target_tokens: Target total tokens after summarization.
            protected_tokens: Tokens of recent messages to always keep.
            provider: AI provider for generating summaries.

        Returns:
            Tuple of (compacted messages, whether summary was added).
        """
        from ticca_agent.core.types import Message as MessageModel
        from ticca_agent.core.types import MessageRole

        # First, partition like truncation
        system_messages: list[Message] = []
        other_messages: list[Message] = []

        for msg in messages:
            if self._config.preserve_system_messages and msg.role == MessageRole.SYSTEM:
                system_messages.append(msg)
            else:
                other_messages.append(msg)

        # Calculate protected messages (same as truncation)
        protected_messages: list[Message] = []
        protected_msg_tokens = 0

        for msg in reversed(other_messages):
            msg_tokens = estimate_message_tokens(msg)
            if protected_msg_tokens + msg_tokens <= protected_tokens:
                protected_messages.insert(0, msg)
                protected_msg_tokens += msg_tokens
            else:
                break

        # Find messages to summarize
        protected_start = len(other_messages) - len(protected_messages)
        to_summarize = other_messages[:protected_start]

        if not to_summarize:
            # Nothing to summarize, just return original
            return system_messages + protected_messages, False

        # Generate summary using the provider
        summary_text = await self._generate_summary(to_summarize, provider)

        # Create summary message
        summary_message = MessageModel(
            role=MessageRole.SYSTEM,
            content=f"[CONVERSATION SUMMARY]\n\n{summary_text}\n\n[END SUMMARY]",
        )

        # Combine: system + summary + protected
        # Put summary after system messages
        return [*system_messages, summary_message, *protected_messages], True

    async def _generate_summary(
        self,
        messages: list[Message],
        provider: BaseProvider,
    ) -> str:
        """Generate a summary of messages using AI.

        Args:
            messages: Messages to summarize.
            provider: AI provider for generation.

        Returns:
            Summary text.
        """
        from ticca_agent.core.types import AgentRequest, MessageRole
        from ticca_agent.core.types import Message as MessageModel

        # Build conversation representation for summarization
        conversation_text = self._format_messages_for_summary(messages)

        # Create summarization prompt
        system_prompt = """You are a conversation summarizer. Your task is to create a concise but comprehensive summary of the conversation provided.

Guidelines:
- Preserve key decisions, conclusions, and important context
- Note any code changes, file modifications, or technical decisions made
- Keep track of the user's goals and progress toward them
- Maintain relevant technical details (file paths, function names, error messages)
- Be concise but don't lose critical information
- Format the summary clearly with sections if helpful

Provide only the summary, no additional commentary."""

        request = AgentRequest(
            messages=[
                MessageModel(role=MessageRole.SYSTEM, content=system_prompt),
                MessageModel(
                    role=MessageRole.USER,
                    content=f"Please summarize this conversation:\n\n{conversation_text}",
                ),
            ],
            temperature=0.3,  # Lower temperature for consistent summaries
            max_tokens=2000,  # Limit summary length
        )

        try:
            response = await provider.generate(
                request,
                model=self._config.summarization_model,
            )
            return response.content
        except Exception as e:
            logger.error(
                "summarization_failed",
                error=str(e),
                message_count=len(messages),
            )
            # Fall back to a simple text summary
            return self._generate_fallback_summary(messages)

    def _format_messages_for_summary(self, messages: list[Message]) -> str:
        """Format messages as text for the summarization prompt."""
        lines: list[str] = []

        for msg in messages:
            role = msg.role.value.upper()
            content = msg.content

            # Truncate very long messages
            if len(content) > 2000:
                content = content[:2000] + "... [truncated]"

            lines.append(f"[{role}]: {content}")

            # Include tool calls if present
            if msg.tool_calls:
                for tc in msg.tool_calls:
                    tool_name = tc.get("function", {}).get("name", "unknown")
                    lines.append(f"  -> Tool call: {tool_name}")

        return "\n\n".join(lines)

    def _generate_fallback_summary(self, messages: list[Message]) -> str:
        """Generate a simple fallback summary without AI."""
        lines = [
            f"Previous conversation with {len(messages)} messages.",
            "",
            "Key points:",
        ]

        # Extract first user message as context
        for msg in messages:
            if msg.role.value == "user":
                first_user = msg.content[:200]
                if len(msg.content) > 200:
                    first_user += "..."
                lines.append(f"- Started with: {first_user}")
                break

        # Count message types
        from collections import Counter
        role_counts = Counter(msg.role.value for msg in messages)
        lines.append(f"- Message counts: {dict(role_counts)}")

        return "\n".join(lines)

    def _estimate_total_tokens(self, messages: list[Message]) -> int:
        """Estimate total tokens for a list of messages."""
        return sum(estimate_message_tokens(msg) for msg in messages)


# =============================================================================
# Legacy Compactor Classes (for backwards compatibility)
# =============================================================================


class BaseCompactor:
    """Abstract base class for compaction strategies.

    .. deprecated::
        Use `ContextCompactor` instead. This class is kept for backwards
        compatibility.
    """

    def __init__(self, config: CompactionConfig | None = None) -> None:
        """Initialize the compactor."""
        self._config = config or CompactionConfig()

    @property
    def config(self) -> CompactionConfig:
        """Get the compaction configuration."""
        return self._config

    async def compact(
        self,
        messages: list[Message],
        *,
        max_tokens: int | None = None,
    ) -> CompactionResult:
        """Compact messages to fit within token limits."""
        raise NotImplementedError("Subclasses must implement compact()")


class TruncationCompactor(BaseCompactor):
    """Simple compaction by removing oldest messages.

    .. deprecated::
        Use `ContextCompactor(strategy=CompactionStrategy.TRUNCATION)` instead.
    """

    async def compact(
        self,
        messages: list[Message],
        *,
        max_tokens: int | None = None,
    ) -> CompactionResult:
        """Compact by truncation."""
        compactor = ContextCompactor(strategy=CompactionStrategy.TRUNCATION)
        return await compactor.compact(
            messages,
            context_length=max_tokens or 128000,
        )


class SlidingWindowCompactor(BaseCompactor):
    """Compaction using a sliding window of recent messages.

    .. deprecated::
        Use `ContextCompactor` with appropriate protected_token_count instead.
    """

    def __init__(
        self,
        config: CompactionConfig | None = None,
        *,
        window_size: int = 50,
    ) -> None:
        """Initialize with window size."""
        super().__init__(config)
        self._window_size = window_size

    async def compact(
        self,
        messages: list[Message],
        *,
        max_tokens: int | None = None,  # noqa: ARG002 - kept for interface compatibility
    ) -> CompactionResult:
        """Compact using sliding window."""
        from ticca_agent.core.types import MessageRole

        original_count = len(messages)
        original_tokens = sum(estimate_message_tokens(m) for m in messages)

        # Keep system messages + last N messages
        system_msgs = [m for m in messages if m.role == MessageRole.SYSTEM]
        other_msgs = [m for m in messages if m.role != MessageRole.SYSTEM]

        kept = system_msgs + other_msgs[-self._window_size:]
        final_tokens = sum(estimate_message_tokens(m) for m in kept)

        return CompactionResult(
            messages=kept,
            original_count=original_count,
            final_count=len(kept),
            original_tokens=original_tokens,
            final_tokens=final_tokens,
            tokens_saved=original_tokens - final_tokens,
            strategy_used=CompactionStrategy.TRUNCATION,
            summary_added=False,
        )


class SummarizationCompactor(BaseCompactor):
    """Compaction by summarizing older messages.

    .. deprecated::
        Use `ContextCompactor(strategy=CompactionStrategy.SUMMARIZATION)` instead.
    """

    def __init__(
        self,
        config: CompactionConfig | None = None,
        *,
        summarizer: Any = None,
    ) -> None:
        """Initialize with summarizer."""
        super().__init__(config)
        self._summarizer = summarizer

    async def compact(
        self,
        messages: list[Message],
        *,
        max_tokens: int | None = None,
    ) -> CompactionResult:
        """Compact by summarization."""
        raise NotImplementedError(
            "Use ContextCompactor with CompactionStrategy.SUMMARIZATION instead."
        )


class HybridCompactor(BaseCompactor):
    """Hybrid compaction combining multiple strategies.

    .. deprecated::
        Use `ContextCompactor` which handles strategy selection automatically.
    """

    async def compact(
        self,
        messages: list[Message],
        *,
        max_tokens: int | None = None,
    ) -> CompactionResult:
        """Compact using hybrid strategy."""
        # Fall back to truncation
        compactor = ContextCompactor(strategy=CompactionStrategy.TRUNCATION)
        return await compactor.compact(
            messages,
            context_length=max_tokens or 128000,
        )


def create_compactor(
    strategy: str = "truncation",
    config: CompactionConfig | None = None,
    **kwargs: Any,
) -> ContextCompactor:
    """Create a compactor with the specified strategy.

    Args:
        strategy: Compaction strategy name ("truncation" or "summarization").
        config: Compaction configuration.
        **kwargs: Additional configuration overrides.

    Returns:
        Configured ContextCompactor instance.

    Raises:
        ValueError: If strategy is not recognized.
    """
    strategy_map = {
        "truncation": CompactionStrategy.TRUNCATION,
        "summarization": CompactionStrategy.SUMMARIZATION,
        # Legacy names
        "sliding_window": CompactionStrategy.TRUNCATION,
        "hybrid": CompactionStrategy.TRUNCATION,
    }

    if strategy not in strategy_map:
        raise ValueError(
            f"Unknown compaction strategy: {strategy}. "
            f"Available: truncation, summarization"
        )

    if config is not None:
        return ContextCompactor(config=config)

    return ContextCompactor(
        strategy=strategy_map[strategy],
        protected_token_count=kwargs.get("protected_token_count", 50000),
        compaction_threshold=kwargs.get("compaction_threshold", 0.85),
    )


__all__ = [
    "BaseCompactor",
    "CompactionConfig",
    "CompactionResult",
    "CompactionStrategy",
    "ContextCompactor",
    "HybridCompactor",
    "SlidingWindowCompactor",
    "SummarizationCompactor",
    "TruncationCompactor",
    "create_compactor",
]
