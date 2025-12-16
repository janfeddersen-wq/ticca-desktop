"""Tests for token estimation utilities.

These tests cover:
1. Basic token estimation
2. Message token estimation
3. Context budget calculations
4. Token budget management
5. Message truncation
"""

from __future__ import annotations

from typing import Any

import pytest

from ticca_agent.core.types import Message, MessageRole
from ticca_agent.utils.tokens import (
    CHARS_PER_TOKEN,
    MODEL_CONTEXT_SIZES,
    TokenBudget,
    TokenCount,
    TokenizerType,
    calculate_available_context,
    count_tokens,
    create_budget,
    estimate_context_overhead,
    estimate_from_words,
    estimate_message_tokens,
    estimate_messages_tokens,
    estimate_tokens,
    estimate_tokens_detailed,
    fits_in_context,
    get_context_size,
    truncate_to_budget,
)


# =============================================================================
# Basic Token Estimation Tests
# =============================================================================


class TestEstimateTokens:
    """Tests for basic token estimation."""

    def test_empty_string(self) -> None:
        """Test estimation for empty string."""
        assert estimate_tokens("") == 0

    def test_single_char(self) -> None:
        """Test estimation for single character."""
        # 1 char / 4 chars per token = 0, but minimum should be reasonable
        result = estimate_tokens("a")
        assert result >= 0

    def test_four_chars(self) -> None:
        """Test estimation for exactly 4 characters."""
        # 4 chars -> 1 token
        assert estimate_tokens("test") == 1

    def test_longer_text(self) -> None:
        """Test estimation for longer text."""
        text = "This is a sample text for testing token estimation."
        result = estimate_tokens(text)
        # ~52 chars / 4 = 13 tokens
        assert 10 <= result <= 20

    def test_very_long_text(self) -> None:
        """Test estimation for very long text."""
        text = "word " * 1000  # 5000 chars
        result = estimate_tokens(text)
        # 5000 / 4 = 1250 tokens
        assert 1200 <= result <= 1300

    def test_unicode_text(self) -> None:
        """Test estimation for Unicode text."""
        text = "Hello 你好 مرحبا 🎉"
        result = estimate_tokens(text)
        # Should return a positive number
        assert result > 0

    def test_code_text(self) -> None:
        """Test estimation for code."""
        code = '''
def hello_world():
    """Print hello world."""
    print("Hello, World!")
    return True
'''
        result = estimate_tokens(code)
        assert result > 0


class TestEstimateTokensDetailed:
    """Tests for detailed token estimation."""

    def test_returns_token_count(self) -> None:
        """Test that it returns TokenCount model."""
        result = estimate_tokens_detailed("Hello world")
        
        assert isinstance(result, TokenCount)
        assert result.count > 0
        assert result.tokenizer == TokenizerType.ESTIMATE
        assert result.is_estimate is True
        assert result.model is None

    def test_empty_string_detailed(self) -> None:
        """Test detailed estimation for empty string."""
        result = estimate_tokens_detailed("")
        
        assert result.count == 0
        assert result.is_estimate is True


# =============================================================================
# Async Token Counting Tests
# =============================================================================


class TestCountTokens:
    """Tests for async token counting."""

    @pytest.mark.asyncio
    async def test_empty_string(self) -> None:
        """Test counting empty string."""
        result = await count_tokens("")
        
        assert result.count == 0
        assert result.is_estimate is False  # Empty is exact

    @pytest.mark.asyncio
    async def test_with_model(self) -> None:
        """Test counting with model specified."""
        result = await count_tokens("Hello world", model="gpt-4")
        
        assert result.count > 0
        assert result.model == "gpt-4"
        # Without tiktoken installed, falls back to estimate
        assert result.is_estimate is True

    @pytest.mark.asyncio
    async def test_without_model(self) -> None:
        """Test counting without model."""
        result = await count_tokens("Hello world")
        
        assert result.count > 0
        assert result.model is None


# =============================================================================
# Message Token Estimation Tests
# =============================================================================


class TestEstimateMessageTokens:
    """Tests for message token estimation."""

    def test_simple_message(self) -> None:
        """Test estimation for simple message."""
        msg = Message(role=MessageRole.USER, content="Hello, how are you?")
        result = estimate_message_tokens(msg)
        
        # Should include content tokens + overhead
        content_tokens = estimate_tokens(msg.content)
        assert result > content_tokens  # Overhead adds tokens

    def test_message_with_name(self) -> None:
        """Test estimation for message with name."""
        msg = Message(
            role=MessageRole.USER,
            content="Hello",
            name="TestUser",
        )
        result = estimate_message_tokens(msg)
        
        # Should include name tokens
        basic_msg = Message(role=MessageRole.USER, content="Hello")
        basic_result = estimate_message_tokens(basic_msg)
        assert result >= basic_result

    def test_message_with_tool_calls(self, message_with_tool_calls: Message) -> None:
        """Test estimation for message with tool calls."""
        result = estimate_message_tokens(message_with_tool_calls)
        
        # Should be larger due to tool call overhead
        basic_msg = Message(
            role=MessageRole.ASSISTANT,
            content=message_with_tool_calls.content,
        )
        basic_result = estimate_message_tokens(basic_msg)
        assert result > basic_result


class TestEstimateMessagesTokens:
    """Tests for estimating token count of message lists."""

    def test_empty_list(self) -> None:
        """Test estimation for empty message list."""
        result = estimate_messages_tokens([])
        
        # Should have base overhead
        assert result >= 3  # Array overhead

    def test_single_message(self) -> None:
        """Test estimation for single message."""
        messages = [Message(role=MessageRole.USER, content="Hello")]
        result = estimate_messages_tokens(messages)
        
        single_msg_tokens = estimate_message_tokens(messages[0])
        assert result >= single_msg_tokens

    def test_multiple_messages(self, sample_messages: list[Message]) -> None:
        """Test estimation for multiple messages."""
        result = estimate_messages_tokens(sample_messages)
        
        # Should be sum of individual messages plus overhead
        individual_sum = sum(estimate_message_tokens(m) for m in sample_messages)
        assert result >= individual_sum


# =============================================================================
# Context Size Tests
# =============================================================================


class TestGetContextSize:
    """Tests for getting model context sizes."""

    def test_known_model(self) -> None:
        """Test getting context size for known model."""
        assert get_context_size("gpt-4") == 8192
        assert get_context_size("gpt-4-turbo") == 128000
        assert get_context_size("claude-3-opus") == 200000

    def test_unknown_model_returns_default(self) -> None:
        """Test that unknown model returns default."""
        result = get_context_size("unknown-model-xyz")
        assert result == MODEL_CONTEXT_SIZES["default"]

    def test_prefix_match(self) -> None:
        """Test prefix matching for model variants."""
        # Should match "gpt-4" prefix
        result = get_context_size("gpt-4-0125-preview")
        assert result > 0


# =============================================================================
# Token Budget Tests
# =============================================================================


class TestTokenBudget:
    """Tests for TokenBudget model."""

    def test_available_calculation(self) -> None:
        """Test available tokens calculation."""
        budget = TokenBudget(
            max_tokens=100000,
            reserved_output=4096,
            reserved_system=1000,
        )
        
        expected = 100000 - 4096 - 1000
        assert budget.available == expected

    def test_available_never_negative(self) -> None:
        """Test that available is never negative."""
        budget = TokenBudget(
            max_tokens=1000,
            reserved_output=2000,  # More than max
            reserved_system=1000,
        )
        
        assert budget.available == 0

    def test_frozen_model(self) -> None:
        """Test that budget is immutable."""
        budget = TokenBudget(max_tokens=10000)
        
        with pytest.raises(Exception):
            budget.max_tokens = 20000  # type: ignore


class TestCreateBudget:
    """Tests for create_budget factory function."""

    def test_create_for_known_model(self) -> None:
        """Test creating budget for known model."""
        budget = create_budget("gpt-4-turbo")
        
        assert budget.max_tokens == 128000
        assert budget.reserved_output == 4096
        assert budget.reserved_system == 1000

    def test_create_with_custom_reserves(self) -> None:
        """Test creating budget with custom reservations."""
        budget = create_budget(
            "gpt-4",
            reserved_output=8000,
            reserved_system=2000,
        )
        
        assert budget.max_tokens == 8192
        assert budget.reserved_output == 8000
        assert budget.reserved_system == 2000


# =============================================================================
# Context Fitting Tests
# =============================================================================


class TestFitsInContext:
    """Tests for checking if messages fit in context."""

    def test_small_messages_fit(self, sample_messages: list[Message]) -> None:
        """Test that small messages fit in large context."""
        budget = TokenBudget(
            max_tokens=100000,
            reserved_output=4096,
            reserved_system=1000,
        )
        
        assert fits_in_context(sample_messages, budget) is True

    def test_large_messages_dont_fit(self) -> None:
        """Test that large messages don't fit in small context."""
        # Create messages that exceed budget
        large_content = "word " * 10000  # ~50000 chars = ~12500 tokens
        messages = [
            Message(role=MessageRole.USER, content=large_content),
            Message(role=MessageRole.ASSISTANT, content=large_content),
        ]
        
        budget = TokenBudget(
            max_tokens=1000,
            reserved_output=100,
            reserved_system=100,
        )
        
        assert fits_in_context(messages, budget) is False


# =============================================================================
# Truncation Tests
# =============================================================================


class TestTruncateToBudget:
    """Tests for truncating messages to fit budget."""

    def test_no_truncation_needed(self, sample_messages: list[Message]) -> None:
        """Test when no truncation is needed."""
        budget = TokenBudget(
            max_tokens=100000,
            reserved_output=4096,
            reserved_system=1000,
        )
        
        result = truncate_to_budget(sample_messages, budget)
        
        assert len(result) == len(sample_messages)

    def test_preserves_system_message(self) -> None:
        """Test that system messages are preserved."""
        messages = [
            Message(role=MessageRole.SYSTEM, content="System prompt"),
            Message(role=MessageRole.USER, content="word " * 1000),
            Message(role=MessageRole.ASSISTANT, content="word " * 1000),
        ]
        
        budget = TokenBudget(
            max_tokens=1000,
            reserved_output=100,
            reserved_system=100,
        )
        
        result = truncate_to_budget(messages, budget, preserve_system=True)
        
        # System message should be preserved
        system_msgs = [m for m in result if m.role == MessageRole.SYSTEM]
        assert len(system_msgs) >= 1

    def test_preserves_recent_messages(self) -> None:
        """Test that recent messages are preserved."""
        messages = [
            Message(role=MessageRole.USER, content=f"Message {i}" + " word" * 100)
            for i in range(10)
        ]
        
        budget = TokenBudget(
            max_tokens=2000,
            reserved_output=100,
            reserved_system=100,
        )
        
        result = truncate_to_budget(
            messages, budget, preserve_recent=2, preserve_system=False
        )
        
        # Last 2 messages should be preserved
        if len(result) >= 2:
            assert result[-1].content.startswith("Message 9")
            assert result[-2].content.startswith("Message 8")


# =============================================================================
# Context Overhead Tests
# =============================================================================


class TestEstimateContextOverhead:
    """Tests for context overhead estimation."""

    def test_system_prompt_only(self) -> None:
        """Test overhead with just system prompt."""
        overhead = estimate_context_overhead("You are a helpful assistant.")
        
        # Should include system prompt tokens + structure
        assert overhead > 0
        assert overhead >= estimate_tokens("You are a helpful assistant.")

    def test_with_tool_definitions(self) -> None:
        """Test overhead with tool definitions."""
        tools = [
            {
                "name": "search",
                "description": "Search the web",
                "parameters": {
                    "type": "object",
                    "properties": {"query": {"type": "string"}},
                },
            }
        ]
        
        overhead_with_tools = estimate_context_overhead(
            "System prompt",
            tool_definitions=tools,
        )
        overhead_without = estimate_context_overhead("System prompt")
        
        assert overhead_with_tools > overhead_without

    def test_with_mcp_tools(self) -> None:
        """Test overhead with MCP tool definitions."""
        mcp_tools = [
            {
                "name": "mcp_tool",
                "description": "An MCP tool",
                "inputSchema": {"type": "object"},
            }
        ]
        
        overhead = estimate_context_overhead(
            "System prompt",
            mcp_tool_definitions=mcp_tools,
        )
        
        assert overhead > estimate_context_overhead("System prompt")


class TestCalculateAvailableContext:
    """Tests for calculating available context."""

    def test_basic_calculation(self) -> None:
        """Test basic available context calculation."""
        available = calculate_available_context(
            model_context_length=128000,
            system_prompt="You are a helpful assistant.",
            reserved_output=4096,
        )
        
        # Should be less than total minus reserved
        assert available < 128000 - 4096
        assert available > 0

    def test_with_tools_reduces_available(self) -> None:
        """Test that tools reduce available context."""
        tools = [
            {
                "name": "tool1",
                "description": "A tool",
                "parameters": {"type": "object"},
            }
        ]
        
        without_tools = calculate_available_context(
            model_context_length=128000,
            system_prompt="Prompt",
            reserved_output=4096,
        )
        
        with_tools = calculate_available_context(
            model_context_length=128000,
            system_prompt="Prompt",
            tool_definitions=tools,
            reserved_output=4096,
        )
        
        assert with_tools < without_tools


# =============================================================================
# Word-based Estimation Tests
# =============================================================================


class TestEstimateFromWords:
    """Tests for word-based token estimation."""

    def test_single_word(self) -> None:
        """Test estimation for single word."""
        # 1 word * 1.3 = 1 token (rounded)
        assert estimate_from_words(1) == 1

    def test_multiple_words(self) -> None:
        """Test estimation for multiple words."""
        # 100 words * 1.3 = 130 tokens
        assert estimate_from_words(100) == 130

    def test_zero_words(self) -> None:
        """Test estimation for zero words."""
        assert estimate_from_words(0) == 0
