"""Session management for Ticca Agent.

This module handles conversation sessions, including:
- Session creation and lifecycle management
- Conversation history persistence with SQLite
- Context window management and compaction
- Session metadata and configuration
- Context snapshots for saving important states

Key Classes:
- Session: Represents a conversation session
- SessionManager: Manages session lifecycle with async SQLite storage
- ContextCompactor: Handles context window overflow with compaction strategies

Example:
    >>> from ticca_agent.session import SessionManager, SessionConfig
    >>> config = SessionConfig(auto_save_session=True, max_saved_sessions=20)
    >>> manager = SessionManager(config, storage_path=Path("~/.ticca/sessions"))
    >>> async with manager:
    ...     session = await manager.create_session(agent_name="code-agent")
    ...     await session.add_message(role="user", content="Hello!")
    ...     await manager.save_session(session)

"""

from __future__ import annotations

# Compaction strategies
from ticca_agent.session.compaction import (
    BaseCompactor,
    CompactionConfig,
    CompactionResult,
    CompactionStrategy,
    ContextCompactor,
    HybridCompactor,
    SlidingWindowCompactor,
    SummarizationCompactor,
    TruncationCompactor,
    create_compactor,
)

# Session management
from ticca_agent.session.manager import (
    Session,
    SessionConfig,
    SessionInfo,
    SessionManager,
    SessionMetadata,
    SnapshotInfo,
)

__all__ = [
    "BaseCompactor",
    "CompactionConfig",
    "CompactionResult",
    "CompactionStrategy",
    "ContextCompactor",
    "HybridCompactor",
    "Session",
    "SessionConfig",
    "SessionInfo",
    "SessionManager",
    "SessionMetadata",
    "SlidingWindowCompactor",
    "SnapshotInfo",
    "SummarizationCompactor",
    "TruncationCompactor",
    "create_compactor",
]
