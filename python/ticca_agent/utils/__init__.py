"""Utility functions and helpers for Ticca Agent.

This module contains shared utilities used across the package:
- Token counting and estimation
- Logging configuration
- Async helpers
- Configuration management

"""

from __future__ import annotations

# Token utilities
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

__all__ = [
    # Constants
    "CHARS_PER_TOKEN",
    "MODEL_CONTEXT_SIZES",
    # Types
    "TokenBudget",
    "TokenCount",
    "TokenizerType",
    # Functions
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
