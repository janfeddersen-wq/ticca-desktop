"""Session management for Ticca Agent.

This module provides session lifecycle management including:
- Session creation and persistence with SQLite
- Conversation history management
- Session metadata handling
- Autosave functionality
- Context snapshot save/load
- Cross-session context sharing

Sessions are the top-level container for conversations and maintain
state across multiple agent interactions.

Example:
    >>> from ticca_agent.session import SessionManager, SessionConfig
    >>> config = SessionConfig(auto_save_session=True, max_saved_sessions=20)
    >>> manager = SessionManager(config, storage_path=Path("~/.ticca/sessions"))
    >>> async with manager:
    ...     session = await manager.create_session(agent_name="code-agent")
    ...     await session.add_message(role="user", content="Review this code")
    ...     await manager.save_session(session)

"""

from __future__ import annotations

import json
from datetime import UTC, datetime
from pathlib import Path
from typing import TYPE_CHECKING, Any
from uuid import UUID, uuid4

import aiosqlite
import structlog
from pydantic import BaseModel, Field

from ticca_agent.utils.tokens import estimate_message_tokens

if TYPE_CHECKING:

    from ticca_agent.core.types import Message, MessageRole

logger = structlog.get_logger(__name__)


# =============================================================================
# Configuration Models
# =============================================================================


class SessionConfig(BaseModel):
    """Configuration for session management.

    Attributes:
        auto_save_session: Whether to automatically save after agent responses.
        max_saved_sessions: Maximum number of sessions to keep (oldest removed).
        max_messages: Maximum messages to keep in history per session.
        compaction_threshold: Ratio of context used before triggering compaction.
        enable_persistence: Whether to persist sessions to database.
    """

    auto_save_session: bool = Field(
        default=True,
        description="Whether to automatically save after agent responses",
    )
    max_saved_sessions: int = Field(
        default=20,
        ge=1,
        le=1000,
        description="Maximum number of sessions to keep",
    )
    max_messages: int = Field(
        default=1000,
        ge=10,
        le=100000,
        description="Maximum messages to keep in history per session",
    )
    compaction_threshold: float = Field(
        default=0.85,
        ge=0.5,
        le=0.95,
        description="Context usage ratio before triggering compaction",
    )
    enable_persistence: bool = Field(
        default=True,
        description="Whether to persist sessions to database",
    )

    model_config = {"frozen": True}


class SessionMetadata(BaseModel):
    """Metadata for a session.

    Attributes:
        title: Human-readable session title.
        description: Optional session description.
        tags: Tags for categorization.
        agent_name: Name of the agent this session is for.
        pinned: Whether the session is pinned.
        archived: Whether the session is archived.
        custom_data: Additional custom metadata.
    """

    title: str = Field(
        default="New Session",
        min_length=1,
        max_length=200,
        description="Human-readable session title",
    )
    description: str | None = Field(
        default=None,
        max_length=1000,
        description="Optional session description",
    )
    tags: list[str] = Field(
        default_factory=list,
        description="Tags for categorization",
    )
    agent_name: str = Field(
        default="default",
        description="Name of the agent this session is for",
    )
    pinned: bool = Field(
        default=False,
        description="Whether the session is pinned",
    )
    archived: bool = Field(
        default=False,
        description="Whether the session is archived",
    )
    custom_data: dict[str, Any] = Field(
        default_factory=dict,
        description="Additional custom metadata",
    )


class SessionInfo(BaseModel):
    """Summary information for a session (used in listings).

    Attributes:
        session_id: Unique session identifier.
        name: Auto-generated session name.
        agent_name: Name of the associated agent.
        created_at: When the session was created.
        updated_at: When the session was last updated.
        message_count: Number of messages in the session.
        total_tokens: Estimated total tokens used.
        is_autosaved: Whether this was auto-saved.
    """

    session_id: str = Field(description="Unique session identifier")
    name: str = Field(description="Auto-generated session name")
    agent_name: str = Field(description="Name of the associated agent")
    created_at: datetime = Field(description="When the session was created")
    updated_at: datetime = Field(description="When the session was last updated")
    message_count: int = Field(ge=0, description="Number of messages")
    total_tokens: int = Field(ge=0, description="Estimated total tokens")
    is_autosaved: bool = Field(default=True, description="Whether auto-saved")

    model_config = {"frozen": True}


class SnapshotInfo(BaseModel):
    """Information about a saved context snapshot.

    Attributes:
        snapshot_id: Unique snapshot identifier.
        session_id: ID of the session this snapshot came from.
        name: User-provided snapshot name.
        created_at: When the snapshot was created.
        message_count: Number of messages in the snapshot.
        total_tokens: Estimated total tokens.
    """

    snapshot_id: str = Field(description="Unique snapshot identifier")
    session_id: str = Field(description="Session this snapshot came from")
    name: str = Field(description="User-provided snapshot name")
    created_at: datetime = Field(description="When the snapshot was created")
    message_count: int = Field(ge=0, description="Number of messages")
    total_tokens: int = Field(ge=0, description="Estimated total tokens")

    model_config = {"frozen": True}


# =============================================================================
# Session Class
# =============================================================================


class Session:
    """Represents a conversation session.

    A session contains the full conversation history and metadata.
    Sessions can be persisted, resumed, and shared.

    Attributes:
        session_id: Unique session identifier.
        name: Auto-generated session name.
        metadata: Session metadata.
        created_at: When the session was created.
        updated_at: When the session was last updated.
    """

    def __init__(
        self,
        *,
        session_id: UUID | None = None,
        name: str | None = None,
        metadata: SessionMetadata | None = None,
        config: SessionConfig | None = None,
        created_at: datetime | None = None,
    ) -> None:
        """Initialize a session.

        Args:
            session_id: Unique identifier (auto-generated if not provided).
            name: Session name (auto-generated if not provided).
            metadata: Session metadata.
            config: Session configuration.
            created_at: Creation timestamp (auto-generated if not provided).
        """
        self._session_id = session_id or uuid4()
        self._name = name or self._generate_session_name()
        self._metadata = metadata or SessionMetadata()
        self._config = config or SessionConfig()
        self._created_at = created_at or datetime.now(UTC)
        self._updated_at = self._created_at
        self._messages: list[Message] = []
        self._total_tokens: int = 0
        self._dirty: bool = False
        self._is_autosaved: bool = True

    @staticmethod
    def _generate_session_name() -> str:
        """Generate auto session name with timestamp and microseconds.

        Format: auto_session_YYYYMMDD_HHMMSS_ffffff

        Includes microseconds to ensure uniqueness even within the same second.
        """
        now = datetime.now(UTC)
        return f"auto_session_{now.strftime('%Y%m%d_%H%M%S_%f')}"

    @property
    def session_id(self) -> UUID:
        """Get the session's unique identifier."""
        return self._session_id

    @property
    def id_str(self) -> str:
        """Get the session ID as a string."""
        return str(self._session_id)

    @property
    def name(self) -> str:
        """Get the auto-generated session name."""
        return self._name

    @property
    def metadata(self) -> SessionMetadata:
        """Get the session metadata."""
        return self._metadata

    @property
    def title(self) -> str:
        """Get the session title."""
        return self._metadata.title

    @property
    def agent_name(self) -> str:
        """Get the associated agent name."""
        return self._metadata.agent_name

    @property
    def created_at(self) -> datetime:
        """Get the creation timestamp."""
        return self._created_at

    @property
    def updated_at(self) -> datetime:
        """Get the last update timestamp."""
        return self._updated_at

    @property
    def message_count(self) -> int:
        """Get the number of messages in the session."""
        return len(self._messages)

    @property
    def total_tokens(self) -> int:
        """Get the total tokens used in this session."""
        return self._total_tokens

    @property
    def is_dirty(self) -> bool:
        """Check if the session has unsaved changes."""
        return self._dirty

    @property
    def is_autosaved(self) -> bool:
        """Check if this session is being auto-saved."""
        return self._is_autosaved

    @property
    def messages(self) -> list[Message]:
        """Get all messages in the session (read-only copy)."""
        return self._messages.copy()

    def mark_dirty(self) -> None:
        """Mark the session as having unsaved changes."""
        self._dirty = True
        self._updated_at = datetime.now(UTC)

    def mark_clean(self) -> None:
        """Mark the session as saved."""
        self._dirty = False

    async def add_message(
        self,
        role: MessageRole,
        content: str,
        *,
        name: str | None = None,
        tool_call_id: str | None = None,
        tool_calls: list[dict[str, Any]] | None = None,
    ) -> Message:
        """Add a message to the session.

        Args:
            role: Message role (user, assistant, system, tool).
            content: Message content.
            name: Optional sender name.
            tool_call_id: Optional tool call ID for tool results.
            tool_calls: Optional tool calls in the message.

        Returns:
            The created Message object.
        """
        from ticca_agent.core.types import Message as MessageModel

        message = MessageModel(
            role=role,
            content=content,
            name=name,
            tool_call_id=tool_call_id,
            tool_calls=tool_calls,
        )

        self._messages.append(message)
        self._total_tokens += estimate_message_tokens(message)
        self.mark_dirty()

        return message

    def add_message_object(self, message: Message) -> None:
        """Add an existing message object to the session.

        Args:
            message: Message object to add.
        """
        self._messages.append(message)
        self._total_tokens += estimate_message_tokens(message)
        self.mark_dirty()

    def set_messages(self, messages: list[Message]) -> None:
        """Replace all messages in the session.

        Args:
            messages: New message list.
        """
        self._messages = messages.copy()
        self._total_tokens = sum(estimate_message_tokens(m) for m in messages)
        self.mark_dirty()

    async def get_messages(
        self,
        *,
        limit: int | None = None,
        offset: int = 0,
        role: MessageRole | None = None,
    ) -> list[Message]:
        """Get messages from the session.

        Args:
            limit: Maximum number of messages to return.
            offset: Number of messages to skip.
            role: Filter by message role.

        Returns:
            List of messages matching the criteria.
        """
        messages = self._messages

        if role is not None:
            messages = [m for m in messages if m.role == role]

        if offset:
            messages = messages[offset:]

        if limit:
            messages = messages[:limit]

        return messages

    async def get_context_messages(
        self,
        max_tokens: int | None = None,
    ) -> list[Message]:
        """Get messages optimized for context window.

        This method returns messages that fit within the token budget,
        preserving system messages and recent context.

        Args:
            max_tokens: Maximum tokens for the context.

        Returns:
            List of messages optimized for the context window.
        """
        from ticca_agent.core.types import MessageRole

        if max_tokens is None:
            return self._messages.copy()

        # Separate system and other messages
        system_messages: list[Message] = []
        other_messages: list[Message] = []

        for msg in self._messages:
            if msg.role == MessageRole.SYSTEM:
                system_messages.append(msg)
            else:
                other_messages.append(msg)

        # Calculate system message tokens
        system_tokens = sum(estimate_message_tokens(m) for m in system_messages)
        remaining_budget = max_tokens - system_tokens

        if remaining_budget <= 0:
            return system_messages

        # Add messages from most recent, fitting within budget
        kept_messages: list[Message] = []
        current_tokens = 0

        for msg in reversed(other_messages):
            msg_tokens = estimate_message_tokens(msg)
            if current_tokens + msg_tokens <= remaining_budget:
                kept_messages.insert(0, msg)
                current_tokens += msg_tokens
            else:
                break

        return system_messages + kept_messages

    async def clear_messages(self) -> None:
        """Clear all messages from the session."""
        self._messages.clear()
        self._total_tokens = 0
        self.mark_dirty()

    async def update_metadata(
        self,
        *,
        title: str | None = None,
        description: str | None = None,
        tags: list[str] | None = None,
        pinned: bool | None = None,
        archived: bool | None = None,
    ) -> None:
        """Update session metadata.

        Args:
            title: New title.
            description: New description.
            tags: New tags.
            pinned: New pinned status.
            archived: New archived status.
        """
        updates: dict[str, Any] = {}

        if title is not None:
            updates["title"] = title
        if description is not None:
            updates["description"] = description
        if tags is not None:
            updates["tags"] = tags
        if pinned is not None:
            updates["pinned"] = pinned
        if archived is not None:
            updates["archived"] = archived

        if updates:
            self._metadata = SessionMetadata(
                **{**self._metadata.model_dump(), **updates}
            )
            self.mark_dirty()

    def to_info(self) -> SessionInfo:
        """Convert session to summary info."""
        return SessionInfo(
            session_id=self.id_str,
            name=self._name,
            agent_name=self.agent_name,
            created_at=self._created_at,
            updated_at=self._updated_at,
            message_count=self.message_count,
            total_tokens=self._total_tokens,
            is_autosaved=self._is_autosaved,
        )

    def rotate_name(self) -> None:
        """Rotate to a new auto-generated session name.

        Called when loading a context snapshot or switching agents.
        """
        self._name = self._generate_session_name()
        self._session_id = uuid4()
        self.mark_dirty()

    def __repr__(self) -> str:
        """Return string representation of the session."""
        return (
            f"Session("
            f"id={self._session_id!s}, "
            f"name={self._name!r}, "
            f"messages={self.message_count})"
        )


# =============================================================================
# Session Manager
# =============================================================================


# Database schema version for migrations
DB_SCHEMA_VERSION = 1

# SQL statements
SQL_CREATE_SESSIONS_TABLE = """
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    agent_name TEXT NOT NULL,
    metadata TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    message_count INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    is_autosaved INTEGER NOT NULL DEFAULT 1
)
"""

SQL_CREATE_MESSAGES_TABLE = """
CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    name TEXT,
    tool_call_id TEXT,
    tool_calls TEXT,
    position INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
)
"""

SQL_CREATE_SNAPSHOTS_TABLE = """
CREATE TABLE IF NOT EXISTS snapshots (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    message_count INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    messages TEXT NOT NULL
)
"""

SQL_CREATE_SCHEMA_VERSION = """
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY
)
"""

SQL_CREATE_INDEXES = """
CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_sessions_agent_name ON sessions(agent_name);
CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages(session_id);
CREATE INDEX IF NOT EXISTS idx_messages_position ON messages(session_id, position);
CREATE INDEX IF NOT EXISTS idx_snapshots_session_id ON snapshots(session_id);
"""


class SessionManager:
    """Manager for session lifecycle operations.

    The SessionManager handles session creation, persistence, retrieval,
    and cleanup using aiosqlite for async database operations.

    Features:
        - Auto-save after each agent response
        - Rolling history with configurable limit
        - Context snapshots for saving important states
        - Session preview for browsing

    Example:
        >>> config = SessionConfig(auto_save_session=True, max_saved_sessions=20)
        >>> manager = SessionManager(config, storage_path=Path("~/.ticca/sessions"))
        >>> async with manager:
        ...     session = await manager.create_session(agent_name="code")
        ...     await manager.save_session(session)
    """

    def __init__(
        self,
        config: SessionConfig,
        storage_path: Path,
    ) -> None:
        """Initialize the session manager.

        Args:
            config: Session configuration.
            storage_path: Path to directory for session storage.
        """
        self._config = config
        self._storage_path = Path(storage_path).expanduser()
        self._db_path = self._storage_path / "sessions.db"
        self._conn: aiosqlite.Connection | None = None
        self._initialized: bool = False
        self._sessions: dict[UUID, Session] = {}  # In-memory cache

    @property
    def config(self) -> SessionConfig:
        """Get the session configuration."""
        return self._config

    @property
    def storage_path(self) -> Path:
        """Get the storage path."""
        return self._storage_path

    async def initialize(self) -> None:
        """Initialize the session manager and database.

        Creates the storage directory and database schema if needed.
        Must be called before using the manager.
        """
        if self._initialized:
            return

        # Create storage directory
        self._storage_path.mkdir(parents=True, exist_ok=True)

        # Connect to database
        self._conn = await aiosqlite.connect(self._db_path)
        self._conn.row_factory = aiosqlite.Row

        # Enable foreign keys
        await self._conn.execute("PRAGMA foreign_keys = ON")

        # Create schema
        await self._conn.execute(SQL_CREATE_SCHEMA_VERSION)
        await self._conn.execute(SQL_CREATE_SESSIONS_TABLE)
        await self._conn.execute(SQL_CREATE_MESSAGES_TABLE)
        await self._conn.execute(SQL_CREATE_SNAPSHOTS_TABLE)
        await self._conn.executescript(SQL_CREATE_INDEXES)

        # Check/set schema version
        cursor = await self._conn.execute(
            "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1"
        )
        row = await cursor.fetchone()
        if row is None:
            await self._conn.execute(
                "INSERT INTO schema_version (version) VALUES (?)",
                (DB_SCHEMA_VERSION,),
            )

        await self._conn.commit()
        self._initialized = True

        logger.info(
            "session_manager_initialized",
            storage_path=str(self._storage_path),
            db_path=str(self._db_path),
        )

    async def close(self) -> None:
        """Close the session manager and release resources."""
        if self._conn:
            # Save all dirty sessions
            for session in self._sessions.values():
                if session.is_dirty:
                    await self.save_session(session)

            await self._conn.close()
            self._conn = None

        self._sessions.clear()
        self._initialized = False

    async def __aenter__(self) -> SessionManager:
        """Async context manager entry."""
        await self.initialize()
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: Any,
    ) -> None:
        """Async context manager exit."""
        await self.close()

    def _ensure_initialized(self) -> None:
        """Ensure manager is initialized."""
        if not self._initialized or self._conn is None:
            raise RuntimeError(
                "SessionManager not initialized. Call initialize() first or use as context manager."
            )

    async def create_session(self, agent_name: str) -> Session:
        """Create a new session with auto-generated name.

        Args:
            agent_name: Name of the agent for this session.

        Returns:
            Newly created session.
        """
        self._ensure_initialized()

        metadata = SessionMetadata(agent_name=agent_name)
        session = Session(metadata=metadata, config=self._config)

        # Cache in memory
        self._sessions[session.session_id] = session

        # Persist if enabled
        if self._config.enable_persistence:
            await self.save_session(session)

        logger.info(
            "session_created",
            session_id=session.id_str,
            name=session.name,
            agent_name=agent_name,
        )

        return session

    async def save_session(self, session: Session) -> None:
        """Save session to persistent storage.

        Args:
            session: The session to save.
        """
        self._ensure_initialized()
        assert self._conn is not None

        if not self._config.enable_persistence:
            session.mark_clean()
            return

        # Upsert session record
        await self._conn.execute(
            """
            INSERT INTO sessions (id, name, agent_name, metadata, created_at, updated_at,
                                  message_count, total_tokens, is_autosaved)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                metadata = excluded.metadata,
                updated_at = excluded.updated_at,
                message_count = excluded.message_count,
                total_tokens = excluded.total_tokens,
                is_autosaved = excluded.is_autosaved
            """,
            (
                session.id_str,
                session.name,
                session.agent_name,
                session.metadata.model_dump_json(),
                session.created_at.isoformat(),
                session.updated_at.isoformat(),
                session.message_count,
                session.total_tokens,
                1 if session.is_autosaved else 0,
            ),
        )

        # Delete existing messages for this session
        await self._conn.execute(
            "DELETE FROM messages WHERE session_id = ?",
            (session.id_str,),
        )

        # Insert all messages
        for position, message in enumerate(session.messages):
            tool_calls_json = (
                json.dumps(message.tool_calls) if message.tool_calls else None
            )
            await self._conn.execute(
                """
                INSERT INTO messages (session_id, role, content, name, tool_call_id,
                                      tool_calls, position)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    session.id_str,
                    message.role.value,
                    message.content,
                    message.name,
                    message.tool_call_id,
                    tool_calls_json,
                    position,
                ),
            )

        await self._conn.commit()
        session.mark_clean()

        # Cleanup old sessions if needed
        await self.cleanup_old_sessions()

        logger.debug(
            "session_saved",
            session_id=session.id_str,
            name=session.name,
            message_count=session.message_count,
        )

    async def load_session(self, session_id: str) -> Session:
        """Load a session from storage.

        Args:
            session_id: Session identifier (UUID string).

        Returns:
            Loaded session.

        Raises:
            KeyError: If session not found.
        """
        self._ensure_initialized()
        assert self._conn is not None

        # Check in-memory cache first
        try:
            uuid = UUID(session_id)
            if uuid in self._sessions:
                return self._sessions[uuid]
        except ValueError:
            pass

        # Load from database
        cursor = await self._conn.execute(
            "SELECT * FROM sessions WHERE id = ?",
            (session_id,),
        )
        row = await cursor.fetchone()

        if row is None:
            raise KeyError(f"Session not found: {session_id}")

        # Load messages
        cursor = await self._conn.execute(
            "SELECT * FROM messages WHERE session_id = ? ORDER BY position",
            (session_id,),
        )
        message_rows = list(await cursor.fetchall())

        # Reconstruct session
        session = self._row_to_session(row, message_rows)
        self._sessions[session.session_id] = session

        logger.debug(
            "session_loaded",
            session_id=session_id,
            name=session.name,
            message_count=session.message_count,
        )

        return session

    async def list_sessions(self) -> list[SessionInfo]:
        """List all saved sessions with metadata.

        Returns:
            List of session info sorted by updated_at (most recent first).
        """
        self._ensure_initialized()
        assert self._conn is not None

        cursor = await self._conn.execute(
            """
            SELECT id, name, agent_name, created_at, updated_at,
                   message_count, total_tokens, is_autosaved
            FROM sessions
            ORDER BY updated_at DESC
            """
        )
        rows = await cursor.fetchall()

        return [
            SessionInfo(
                session_id=row["id"],
                name=row["name"],
                agent_name=row["agent_name"],
                created_at=datetime.fromisoformat(row["created_at"]),
                updated_at=datetime.fromisoformat(row["updated_at"]),
                message_count=row["message_count"],
                total_tokens=row["total_tokens"],
                is_autosaved=bool(row["is_autosaved"]),
            )
            for row in rows
        ]

    async def delete_session(self, session_id: str) -> None:
        """Delete a session from storage.

        Args:
            session_id: Session identifier to delete.
        """
        self._ensure_initialized()
        assert self._conn is not None

        # Remove from cache
        try:
            uuid = UUID(session_id)
            self._sessions.pop(uuid, None)
        except ValueError:
            pass

        # Delete from database (CASCADE will handle messages)
        await self._conn.execute(
            "DELETE FROM sessions WHERE id = ?",
            (session_id,),
        )
        await self._conn.commit()

        logger.info("session_deleted", session_id=session_id)

    async def cleanup_old_sessions(self) -> int:
        """Remove sessions beyond max_saved_sessions limit.

        Keeps the most recent sessions, removing oldest first.
        Pinned sessions are not removed.

        Returns:
            Number of sessions removed.
        """
        self._ensure_initialized()
        assert self._conn is not None

        # Get all sessions sorted by updated_at
        cursor = await self._conn.execute(
            """
            SELECT id, metadata FROM sessions
            ORDER BY updated_at DESC
            """
        )
        rows = await cursor.fetchall()

        # Filter out pinned sessions and find ones to delete
        kept_count = 0
        to_delete: list[str] = []

        for row in rows:
            metadata = SessionMetadata.model_validate_json(row["metadata"])
            if metadata.pinned:
                continue  # Never delete pinned sessions

            kept_count += 1
            if kept_count > self._config.max_saved_sessions:
                to_delete.append(row["id"])

        # Delete old sessions
        for session_id in to_delete:
            await self._conn.execute(
                "DELETE FROM sessions WHERE id = ?",
                (session_id,),
            )
            # Also remove from cache
            try:
                uuid = UUID(session_id)
                self._sessions.pop(uuid, None)
            except ValueError:
                pass

        if to_delete:
            await self._conn.commit()
            logger.info(
                "sessions_cleaned_up",
                removed_count=len(to_delete),
                max_sessions=self._config.max_saved_sessions,
            )

        return len(to_delete)

    async def get_session_preview(
        self,
        session_id: str,
        num_messages: int = 5,
    ) -> list[Message]:
        """Get preview of recent messages from a session.

        Args:
            session_id: Session identifier.
            num_messages: Number of recent messages to return.

        Returns:
            List of recent messages.

        Raises:
            KeyError: If session not found.
        """
        self._ensure_initialized()
        assert self._conn is not None

        # Get total message count
        cursor = await self._conn.execute(
            "SELECT COUNT(*) as count FROM messages WHERE session_id = ?",
            (session_id,),
        )
        row = await cursor.fetchone()
        if row is None:
            raise KeyError(f"Session not found: {session_id}")

        total = row["count"]
        offset = max(0, total - num_messages)

        # Get recent messages
        cursor = await self._conn.execute(
            """
            SELECT role, content, name, tool_call_id, tool_calls
            FROM messages
            WHERE session_id = ?
            ORDER BY position
            LIMIT ? OFFSET ?
            """,
            (session_id, num_messages, offset),
        )
        rows = await cursor.fetchall()

        return [self._row_to_message(row) for row in rows]

    # =========================================================================
    # Context Snapshots
    # =========================================================================

    async def save_context_snapshot(
        self,
        session: Session,
        name: str,
    ) -> str:
        """Save current message history as a named snapshot.

        Args:
            session: Session to snapshot.
            name: User-provided name for the snapshot.

        Returns:
            Snapshot ID.
        """
        self._ensure_initialized()
        assert self._conn is not None

        snapshot_id = str(uuid4())
        now = datetime.now(UTC)

        # Serialize messages
        messages_json = json.dumps(
            [msg.model_dump() for msg in session.messages]
        )

        await self._conn.execute(
            """
            INSERT INTO snapshots (id, session_id, name, created_at,
                                   message_count, total_tokens, messages)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            """,
            (
                snapshot_id,
                session.id_str,
                name,
                now.isoformat(),
                session.message_count,
                session.total_tokens,
                messages_json,
            ),
        )
        await self._conn.commit()

        logger.info(
            "snapshot_saved",
            snapshot_id=snapshot_id,
            session_id=session.id_str,
            name=name,
            message_count=session.message_count,
        )

        return snapshot_id

    async def load_context_snapshot(self, snapshot_id: str) -> Session:
        """Load a saved snapshot into a new session.

        Creates a new session with rotated autosave ID containing
        the snapshot's messages.

        Args:
            snapshot_id: Snapshot identifier.

        Returns:
            New session with snapshot's messages.

        Raises:
            KeyError: If snapshot not found.
        """
        self._ensure_initialized()
        assert self._conn is not None

        cursor = await self._conn.execute(
            "SELECT * FROM snapshots WHERE id = ?",
            (snapshot_id,),
        )
        row = await cursor.fetchone()

        if row is None:
            raise KeyError(f"Snapshot not found: {snapshot_id}")

        # Load the original session to get agent info
        try:
            original_session = await self.load_session(row["session_id"])
            agent_name = original_session.agent_name
        except KeyError:
            agent_name = "default"

        # Create new session with rotated name
        session = await self.create_session(agent_name)

        # Load messages from snapshot
        from ticca_agent.core.types import Message as MessageModel

        messages_data = json.loads(row["messages"])
        for msg_data in messages_data:
            message = MessageModel.model_validate(msg_data)
            session.add_message_object(message)

        logger.info(
            "snapshot_loaded",
            snapshot_id=snapshot_id,
            new_session_id=session.id_str,
            message_count=session.message_count,
        )

        return session

    async def list_snapshots(self) -> list[SnapshotInfo]:
        """List all saved snapshots.

        Returns:
            List of snapshot info sorted by created_at (most recent first).
        """
        self._ensure_initialized()
        assert self._conn is not None

        cursor = await self._conn.execute(
            """
            SELECT id, session_id, name, created_at, message_count, total_tokens
            FROM snapshots
            ORDER BY created_at DESC
            """
        )
        rows = await cursor.fetchall()

        return [
            SnapshotInfo(
                snapshot_id=row["id"],
                session_id=row["session_id"],
                name=row["name"],
                created_at=datetime.fromisoformat(row["created_at"]),
                message_count=row["message_count"],
                total_tokens=row["total_tokens"],
            )
            for row in rows
        ]

    async def delete_snapshot(self, snapshot_id: str) -> None:
        """Delete a snapshot.

        Args:
            snapshot_id: Snapshot identifier to delete.
        """
        self._ensure_initialized()
        assert self._conn is not None

        await self._conn.execute(
            "DELETE FROM snapshots WHERE id = ?",
            (snapshot_id,),
        )
        await self._conn.commit()

        logger.info("snapshot_deleted", snapshot_id=snapshot_id)

    # =========================================================================
    # Helper Methods
    # =========================================================================

    def _row_to_session(
        self,
        session_row: aiosqlite.Row,
        message_rows: list[aiosqlite.Row],
    ) -> Session:
        """Convert database rows to a Session object."""

        metadata = SessionMetadata.model_validate_json(session_row["metadata"])

        session = Session(
            session_id=UUID(session_row["id"]),
            name=session_row["name"],
            metadata=metadata,
            config=self._config,
            created_at=datetime.fromisoformat(session_row["created_at"]),
        )
        session._updated_at = datetime.fromisoformat(session_row["updated_at"])
        session._is_autosaved = bool(session_row["is_autosaved"])

        # Load messages
        for row in message_rows:
            message = self._row_to_message(row)
            session._messages.append(message)

        # Recalculate tokens
        session._total_tokens = sum(
            estimate_message_tokens(m) for m in session._messages
        )
        session.mark_clean()  # Just loaded, not dirty

        return session

    def _row_to_message(self, row: aiosqlite.Row) -> Message:
        """Convert a database row to a Message object."""
        from ticca_agent.core.types import Message as MessageModel
        from ticca_agent.core.types import MessageRole

        tool_calls = None
        if row["tool_calls"]:
            tool_calls = json.loads(row["tool_calls"])

        return MessageModel(
            role=MessageRole(row["role"]),
            content=row["content"],
            name=row["name"],
            tool_call_id=row["tool_call_id"],
            tool_calls=tool_calls,
        )


__all__ = [
    "Session",
    "SessionConfig",
    "SessionInfo",
    "SessionManager",
    "SessionMetadata",
    "SnapshotInfo",
]
