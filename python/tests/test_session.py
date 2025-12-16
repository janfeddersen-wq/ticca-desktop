"""Tests for session management.

These tests verify:
1. Session creation and lifecycle
2. Message management within sessions
3. SQLite persistence with aiosqlite
4. Session listing and cleanup
5. Context snapshots
6. Token estimation integration
"""

import asyncio
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from uuid import UUID

import pytest

from ticca_agent.core.types import Message, MessageRole
from ticca_agent.session import (
    CompactionConfig,
    CompactionResult,
    CompactionStrategy,
    ContextCompactor,
    Session,
    SessionConfig,
    SessionInfo,
    SessionManager,
    SessionMetadata,
    SnapshotInfo,
    create_compactor,
)
from ticca_agent.utils.tokens import (
    calculate_available_context,
    estimate_context_overhead,
    estimate_message_tokens,
    estimate_tokens,
)


# =============================================================================
# Fixtures
# =============================================================================


@pytest.fixture
def temp_storage_path() -> Path:
    """Create a temporary directory for session storage."""
    with tempfile.TemporaryDirectory() as tmpdir:
        yield Path(tmpdir)


@pytest.fixture
def session_config() -> SessionConfig:
    """Create a test session configuration."""
    return SessionConfig(
        auto_save_session=True,
        max_saved_sessions=5,
        max_messages=100,
        compaction_threshold=0.85,
    )


@pytest.fixture
async def session_manager(
    temp_storage_path: Path,
    session_config: SessionConfig,
) -> SessionManager:
    """Create and initialize a session manager."""
    manager = SessionManager(session_config, temp_storage_path)
    await manager.initialize()
    yield manager
    await manager.close()


@pytest.fixture
def sample_messages() -> list[Message]:
    """Create sample messages for testing."""
    return [
        Message(role=MessageRole.SYSTEM, content="You are a helpful assistant."),
        Message(role=MessageRole.USER, content="Hello, how are you?"),
        Message(role=MessageRole.ASSISTANT, content="I'm doing great, thanks for asking!"),
        Message(role=MessageRole.USER, content="Can you help me with Python?"),
        Message(
            role=MessageRole.ASSISTANT,
            content="Of course! I'd be happy to help with Python. What would you like to know?",
        ),
    ]


# =============================================================================
# Session Config Tests
# =============================================================================


class TestSessionConfig:
    """Tests for SessionConfig model."""

    def test_default_config(self) -> None:
        """Test default configuration values."""
        config = SessionConfig()

        assert config.auto_save_session is True
        assert config.max_saved_sessions == 20
        assert config.max_messages == 1000
        assert config.compaction_threshold == 0.85
        assert config.enable_persistence is True

    def test_custom_config(self) -> None:
        """Test custom configuration values."""
        config = SessionConfig(
            auto_save_session=False,
            max_saved_sessions=10,
            max_messages=500,
            compaction_threshold=0.9,
        )

        assert config.auto_save_session is False
        assert config.max_saved_sessions == 10
        assert config.max_messages == 500
        assert config.compaction_threshold == 0.9

    def test_config_validation(self) -> None:
        """Test configuration validation."""
        # max_saved_sessions must be >= 1
        with pytest.raises(ValueError):
            SessionConfig(max_saved_sessions=0)

        # compaction_threshold must be in range
        with pytest.raises(ValueError):
            SessionConfig(compaction_threshold=0.3)  # Too low

        with pytest.raises(ValueError):
            SessionConfig(compaction_threshold=1.0)  # Too high

    def test_config_immutability(self) -> None:
        """Test that config is frozen."""
        config = SessionConfig()
        with pytest.raises(Exception):  # Pydantic raises ValidationError
            config.auto_save_session = False  # type: ignore


class TestSessionMetadata:
    """Tests for SessionMetadata model."""

    def test_default_metadata(self) -> None:
        """Test default metadata values."""
        metadata = SessionMetadata()

        assert metadata.title == "New Session"
        assert metadata.description is None
        assert metadata.tags == []
        assert metadata.agent_name == "default"
        assert metadata.pinned is False
        assert metadata.archived is False

    def test_custom_metadata(self) -> None:
        """Test custom metadata values."""
        metadata = SessionMetadata(
            title="Code Review Session",
            description="Reviewing auth module",
            tags=["code-review", "auth"],
            agent_name="code-reviewer",
            pinned=True,
        )

        assert metadata.title == "Code Review Session"
        assert metadata.description == "Reviewing auth module"
        assert metadata.tags == ["code-review", "auth"]
        assert metadata.agent_name == "code-reviewer"
        assert metadata.pinned is True


# =============================================================================
# Session Tests
# =============================================================================


class TestSession:
    """Tests for Session class."""

    def test_session_creation(self) -> None:
        """Test session creation with defaults."""
        session = Session()

        assert isinstance(session.session_id, UUID)
        assert session.name.startswith("auto_session_")
        assert session.message_count == 0
        assert session.total_tokens == 0
        assert session.is_dirty is False

    def test_session_creation_with_metadata(self) -> None:
        """Test session creation with custom metadata."""
        metadata = SessionMetadata(
            title="Test Session",
            agent_name="test-agent",
        )
        session = Session(metadata=metadata)

        assert session.title == "Test Session"
        assert session.agent_name == "test-agent"

    def test_session_name_format(self) -> None:
        """Test auto-generated session name format."""
        session = Session()

        # Format: auto_session_YYYYMMDD_HHMMSS_ffffff
        parts = session.name.split("_")
        assert parts[0] == "auto"
        assert parts[1] == "session"
        assert len(parts[2]) == 8  # YYYYMMDD
        assert len(parts[3]) == 6  # HHMMSS
        assert len(parts[4]) == 6  # ffffff (microseconds)

    @pytest.mark.asyncio
    async def test_add_message(self) -> None:
        """Test adding messages to session."""
        session = Session()

        msg = await session.add_message(
            role=MessageRole.USER,
            content="Hello, world!",
        )

        assert msg.role == MessageRole.USER
        assert msg.content == "Hello, world!"
        assert session.message_count == 1
        assert session.total_tokens > 0
        assert session.is_dirty is True

    @pytest.mark.asyncio
    async def test_add_message_with_tool_calls(self) -> None:
        """Test adding message with tool calls."""
        session = Session()

        tool_calls = [
            {
                "id": "call_123",
                "type": "function",
                "function": {"name": "read_file", "arguments": '{"path": "/test.py"}'},
            }
        ]

        msg = await session.add_message(
            role=MessageRole.ASSISTANT,
            content="Let me read that file for you.",
            tool_calls=tool_calls,
        )

        assert msg.tool_calls == tool_calls

    @pytest.mark.asyncio
    async def test_get_messages(self) -> None:
        """Test retrieving messages from session."""
        session = Session()

        await session.add_message(role=MessageRole.USER, content="First")
        await session.add_message(role=MessageRole.ASSISTANT, content="Second")
        await session.add_message(role=MessageRole.USER, content="Third")

        # Get all messages
        all_msgs = await session.get_messages()
        assert len(all_msgs) == 3

        # Get with limit
        limited = await session.get_messages(limit=2)
        assert len(limited) == 2

        # Get with offset
        offset_msgs = await session.get_messages(offset=1)
        assert len(offset_msgs) == 2
        assert offset_msgs[0].content == "Second"

        # Filter by role
        user_msgs = await session.get_messages(role=MessageRole.USER)
        assert len(user_msgs) == 2

    @pytest.mark.asyncio
    async def test_clear_messages(self) -> None:
        """Test clearing all messages."""
        session = Session()

        await session.add_message(role=MessageRole.USER, content="Test")
        assert session.message_count == 1

        await session.clear_messages()

        assert session.message_count == 0
        assert session.total_tokens == 0
        assert session.is_dirty is True

    @pytest.mark.asyncio
    async def test_update_metadata(self) -> None:
        """Test updating session metadata."""
        session = Session()

        await session.update_metadata(
            title="Updated Title",
            pinned=True,
        )

        assert session.title == "Updated Title"
        assert session.metadata.pinned is True
        assert session.is_dirty is True

    def test_rotate_name(self) -> None:
        """Test rotating session name."""
        session = Session()
        original_id = session.session_id
        original_name = session.name

        # Small delay to ensure different timestamp
        import time

        time.sleep(0.01)

        session.rotate_name()

        assert session.session_id != original_id
        assert session.name != original_name
        assert session.name.startswith("auto_session_")

    def test_to_info(self) -> None:
        """Test converting session to info."""
        session = Session()
        info = session.to_info()

        assert isinstance(info, SessionInfo)
        assert info.session_id == session.id_str
        assert info.name == session.name
        assert info.message_count == 0


# =============================================================================
# Session Manager Tests
# =============================================================================


class TestSessionManager:
    """Tests for SessionManager class."""

    @pytest.mark.asyncio
    async def test_manager_initialization(self, temp_storage_path: Path) -> None:
        """Test manager initialization creates database."""
        config = SessionConfig()
        manager = SessionManager(config, temp_storage_path)

        await manager.initialize()

        # Database file should exist
        db_path = temp_storage_path / "sessions.db"
        assert db_path.exists()

        await manager.close()

    @pytest.mark.asyncio
    async def test_manager_context_manager(self, temp_storage_path: Path) -> None:
        """Test manager as async context manager."""
        config = SessionConfig()
        manager = SessionManager(config, temp_storage_path)

        async with manager:
            assert manager._initialized is True

        assert manager._initialized is False

    @pytest.mark.asyncio
    async def test_create_session(self, session_manager: SessionManager) -> None:
        """Test creating a new session."""
        session = await session_manager.create_session(agent_name="test-agent")

        assert isinstance(session, Session)
        assert session.agent_name == "test-agent"
        assert session.name.startswith("auto_session_")

    @pytest.mark.asyncio
    async def test_save_and_load_session(
        self,
        session_manager: SessionManager,
        sample_messages: list[Message],
    ) -> None:
        """Test saving and loading a session."""
        # Create session with messages
        session = await session_manager.create_session(agent_name="test")
        for msg in sample_messages:
            session.add_message_object(msg)

        await session_manager.save_session(session)

        # Load session
        loaded = await session_manager.load_session(session.id_str)

        assert loaded.id_str == session.id_str
        assert loaded.name == session.name
        assert loaded.message_count == len(sample_messages)

        # Verify messages
        loaded_msgs = loaded.messages
        assert len(loaded_msgs) == len(sample_messages)
        assert loaded_msgs[0].content == sample_messages[0].content

    @pytest.mark.asyncio
    async def test_list_sessions(self, session_manager: SessionManager) -> None:
        """Test listing sessions."""
        # Create multiple sessions
        await session_manager.create_session(agent_name="agent1")
        await session_manager.create_session(agent_name="agent2")
        await session_manager.create_session(agent_name="agent3")

        sessions = await session_manager.list_sessions()

        assert len(sessions) == 3
        # Should be sorted by updated_at descending
        assert sessions[0].updated_at >= sessions[1].updated_at

    @pytest.mark.asyncio
    async def test_delete_session(self, session_manager: SessionManager) -> None:
        """Test deleting a session."""
        session = await session_manager.create_session(agent_name="test")
        session_id = session.id_str

        await session_manager.delete_session(session_id)

        # Session should not be loadable
        with pytest.raises(KeyError):
            await session_manager.load_session(session_id)

    @pytest.mark.asyncio
    async def test_cleanup_old_sessions(
        self,
        temp_storage_path: Path,
    ) -> None:
        """Test cleanup of old sessions beyond limit."""
        # Create manager with low limit
        config = SessionConfig(max_saved_sessions=3)
        manager = SessionManager(config, temp_storage_path)

        async with manager:
            # Create more sessions than limit
            # Note: save_session already calls cleanup_old_sessions
            for i in range(5):
                session = await manager.create_session(agent_name=f"agent{i}")
                await manager.save_session(session)

            # After auto-cleanup, should only have 3 sessions
            sessions = await manager.list_sessions()
            assert len(sessions) == 3
            
            # Additional cleanup call should remove 0 (already cleaned)
            removed = await manager.cleanup_old_sessions()
            assert removed == 0

    @pytest.mark.asyncio
    async def test_cleanup_preserves_pinned(
        self,
        temp_storage_path: Path,
    ) -> None:
        """Test that cleanup preserves pinned sessions."""
        config = SessionConfig(max_saved_sessions=2)
        manager = SessionManager(config, temp_storage_path)

        async with manager:
            # Create sessions, pin first one
            session1 = await manager.create_session(agent_name="pinned")
            await session1.update_metadata(pinned=True)
            await manager.save_session(session1)

            session2 = await manager.create_session(agent_name="regular1")
            await manager.save_session(session2)

            session3 = await manager.create_session(agent_name="regular2")
            await manager.save_session(session3)

            session4 = await manager.create_session(agent_name="regular3")
            await manager.save_session(session4)

            # Cleanup - pinned should be preserved
            sessions = await manager.list_sessions()
            pinned_sessions = [s for s in sessions if s.session_id == session1.id_str]
            assert len(pinned_sessions) == 1

    @pytest.mark.asyncio
    async def test_get_session_preview(
        self,
        session_manager: SessionManager,
        sample_messages: list[Message],
    ) -> None:
        """Test getting session preview."""
        session = await session_manager.create_session(agent_name="test")
        for msg in sample_messages:
            session.add_message_object(msg)
        await session_manager.save_session(session)

        # Get preview of last 3 messages
        preview = await session_manager.get_session_preview(session.id_str, num_messages=3)

        assert len(preview) == 3
        # Should be most recent messages
        assert preview[-1].content == sample_messages[-1].content


# =============================================================================
# Context Snapshot Tests
# =============================================================================


class TestContextSnapshots:
    """Tests for context snapshot functionality."""

    @pytest.mark.asyncio
    async def test_save_snapshot(
        self,
        session_manager: SessionManager,
        sample_messages: list[Message],
    ) -> None:
        """Test saving a context snapshot."""
        session = await session_manager.create_session(agent_name="test")
        for msg in sample_messages:
            session.add_message_object(msg)

        snapshot_id = await session_manager.save_context_snapshot(
            session,
            name="Important checkpoint",
        )

        assert snapshot_id is not None
        assert len(snapshot_id) > 0

    @pytest.mark.asyncio
    async def test_load_snapshot(
        self,
        session_manager: SessionManager,
        sample_messages: list[Message],
    ) -> None:
        """Test loading a context snapshot."""
        # Create session and save snapshot
        session = await session_manager.create_session(agent_name="test")
        for msg in sample_messages:
            session.add_message_object(msg)
        await session_manager.save_session(session)

        snapshot_id = await session_manager.save_context_snapshot(
            session,
            name="Checkpoint",
        )

        # Load snapshot into new session
        new_session = await session_manager.load_context_snapshot(snapshot_id)

        # Should have rotated ID/name
        assert new_session.session_id != session.session_id
        assert new_session.name != session.name

        # But should have same messages
        assert new_session.message_count == session.message_count

    @pytest.mark.asyncio
    async def test_list_snapshots(
        self,
        session_manager: SessionManager,
    ) -> None:
        """Test listing snapshots."""
        session = await session_manager.create_session(agent_name="test")
        await session.add_message(role=MessageRole.USER, content="Test")

        await session_manager.save_context_snapshot(session, name="Snapshot 1")
        await session_manager.save_context_snapshot(session, name="Snapshot 2")

        snapshots = await session_manager.list_snapshots()

        assert len(snapshots) == 2
        assert all(isinstance(s, SnapshotInfo) for s in snapshots)

    @pytest.mark.asyncio
    async def test_delete_snapshot(
        self,
        session_manager: SessionManager,
    ) -> None:
        """Test deleting a snapshot."""
        session = await session_manager.create_session(agent_name="test")
        await session.add_message(role=MessageRole.USER, content="Test")

        snapshot_id = await session_manager.save_context_snapshot(
            session,
            name="To Delete",
        )

        await session_manager.delete_snapshot(snapshot_id)

        with pytest.raises(KeyError):
            await session_manager.load_context_snapshot(snapshot_id)


# =============================================================================
# Compaction Tests
# =============================================================================


class TestCompactionConfig:
    """Tests for CompactionConfig."""

    def test_default_config(self) -> None:
        """Test default compaction configuration."""
        config = CompactionConfig()

        assert config.strategy == CompactionStrategy.TRUNCATION
        assert config.protected_token_count == 50000
        assert config.compaction_threshold == 0.85
        assert config.max_protected_ratio == 0.75
        assert config.preserve_system_messages is True

    def test_custom_config(self) -> None:
        """Test custom compaction configuration."""
        config = CompactionConfig(
            strategy=CompactionStrategy.SUMMARIZATION,
            protected_token_count=30000,
            compaction_threshold=0.9,
        )

        assert config.strategy == CompactionStrategy.SUMMARIZATION
        assert config.protected_token_count == 30000
        assert config.compaction_threshold == 0.9


class TestContextCompactor:
    """Tests for ContextCompactor."""

    def test_compactor_creation(self) -> None:
        """Test compactor creation."""
        compactor = ContextCompactor(
            strategy=CompactionStrategy.TRUNCATION,
            protected_token_count=50000,
            compaction_threshold=0.85,
        )

        assert compactor.strategy == CompactionStrategy.TRUNCATION
        assert compactor.protected_token_count == 50000
        assert compactor.compaction_threshold == 0.85

    def test_should_compact_below_threshold(self) -> None:
        """Test should_compact returns False below threshold."""
        compactor = ContextCompactor(compaction_threshold=0.85)

        # 80% usage - should not compact
        assert compactor.should_compact(
            context_used=80000,
            context_length=100000,
        ) is False

    def test_should_compact_above_threshold(self) -> None:
        """Test should_compact returns True above threshold."""
        compactor = ContextCompactor(compaction_threshold=0.85)

        # 90% usage - should compact
        assert compactor.should_compact(
            context_used=90000,
            context_length=100000,
        ) is True

    def test_should_compact_at_threshold(self) -> None:
        """Test should_compact at exactly threshold."""
        compactor = ContextCompactor(compaction_threshold=0.85)

        # Exactly 85% - should not compact (threshold is >)
        assert compactor.should_compact(
            context_used=85000,
            context_length=100000,
        ) is False

    @pytest.mark.asyncio
    async def test_truncation_compaction(self, sample_messages: list[Message]) -> None:
        """Test truncation compaction strategy."""
        # Create compactor with small protected budget
        compactor = ContextCompactor(
            strategy=CompactionStrategy.TRUNCATION,
            protected_token_count=1000,  # Minimum allowed
        )

        result = await compactor.compact(
            messages=sample_messages,
            context_length=50000,  # Realistic context length
        )

        assert isinstance(result, CompactionResult)
        assert result.original_count == len(sample_messages)
        assert result.strategy_used == CompactionStrategy.TRUNCATION
        assert result.summary_added is False
        # Should have kept system message
        assert any(m.role == MessageRole.SYSTEM for m in result.messages)

    @pytest.mark.asyncio
    async def test_truncation_preserves_system_messages(
        self,
        sample_messages: list[Message],
    ) -> None:
        """Test that truncation always preserves system messages."""
        compactor = ContextCompactor(
            strategy=CompactionStrategy.TRUNCATION,
            protected_token_count=1000,  # Minimum allowed
        )

        result = await compactor.compact(
            messages=sample_messages,
            context_length=50000,
        )

        # System message should always be present
        system_msgs = [m for m in result.messages if m.role == MessageRole.SYSTEM]
        assert len(system_msgs) >= 1

    @pytest.mark.asyncio
    async def test_compaction_result_tokens(self, sample_messages: list[Message]) -> None:
        """Test that compaction result includes token counts."""
        compactor = ContextCompactor(protected_token_count=1000)

        result = await compactor.compact(
            messages=sample_messages,
            context_length=50000,
        )

        assert result.original_tokens > 0
        assert result.final_tokens > 0
        assert result.tokens_saved >= 0
        assert result.original_tokens == result.final_tokens + result.tokens_saved

    @pytest.mark.asyncio
    async def test_summarization_requires_provider(self) -> None:
        """Test that summarization strategy requires a provider."""
        compactor = ContextCompactor(strategy=CompactionStrategy.SUMMARIZATION)

        messages = [Message(role=MessageRole.USER, content="Test")]

        with pytest.raises(ValueError, match="requires a provider"):
            await compactor.compact(
                messages=messages,
                context_length=1000,
                summarization_provider=None,
            )


class TestCreateCompactor:
    """Tests for create_compactor factory function."""

    def test_create_truncation_compactor(self) -> None:
        """Test creating truncation compactor."""
        compactor = create_compactor("truncation")

        assert isinstance(compactor, ContextCompactor)
        assert compactor.strategy == CompactionStrategy.TRUNCATION

    def test_create_summarization_compactor(self) -> None:
        """Test creating summarization compactor."""
        compactor = create_compactor("summarization")

        assert isinstance(compactor, ContextCompactor)
        assert compactor.strategy == CompactionStrategy.SUMMARIZATION

    def test_create_compactor_with_kwargs(self) -> None:
        """Test creating compactor with custom kwargs."""
        compactor = create_compactor(
            "truncation",
            protected_token_count=30000,
            compaction_threshold=0.9,
        )

        assert compactor.protected_token_count == 30000
        assert compactor.compaction_threshold == 0.9

    def test_create_compactor_invalid_strategy(self) -> None:
        """Test creating compactor with invalid strategy."""
        with pytest.raises(ValueError, match="Unknown compaction strategy"):
            create_compactor("invalid")


# =============================================================================
# Token Estimation Tests
# =============================================================================


class TestTokenEstimation:
    """Tests for token estimation utilities."""

    def test_estimate_tokens_basic(self) -> None:
        """Test basic token estimation."""
        text = "Hello, world!"
        tokens = estimate_tokens(text)

        # ~13 chars / 4 chars per token ≈ 3 tokens
        assert tokens >= 1
        assert tokens <= 10

    def test_estimate_tokens_empty(self) -> None:
        """Test token estimation for empty string."""
        assert estimate_tokens("") == 0

    def test_estimate_message_tokens(self) -> None:
        """Test message token estimation."""
        message = Message(
            role=MessageRole.USER,
            content="Hello, how are you?",
        )

        tokens = estimate_message_tokens(message)

        # Should include content + overhead
        assert tokens > 0
        assert tokens >= estimate_tokens(message.content)

    def test_estimate_message_tokens_with_tool_calls(self) -> None:
        """Test message token estimation with tool calls."""
        message = Message(
            role=MessageRole.ASSISTANT,
            content="Let me help.",
            tool_calls=[
                {"id": "1", "function": {"name": "test", "arguments": "{}"}},
                {"id": "2", "function": {"name": "test2", "arguments": "{}"}},
            ],
        )

        tokens = estimate_message_tokens(message)

        # Should include tool call overhead
        base_message = Message(role=MessageRole.ASSISTANT, content="Let me help.")
        assert tokens > estimate_message_tokens(base_message)

    def test_estimate_context_overhead_basic(self) -> None:
        """Test context overhead estimation."""
        system_prompt = "You are a helpful assistant."
        overhead = estimate_context_overhead(system_prompt)

        assert overhead > 0
        assert overhead >= estimate_tokens(system_prompt)

    def test_estimate_context_overhead_with_tools(self) -> None:
        """Test context overhead with tool definitions."""
        system_prompt = "You are a helpful assistant."
        tools = [
            {
                "name": "read_file",
                "description": "Read contents of a file",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "File path"}
                    },
                },
            },
            {
                "name": "write_file",
                "description": "Write contents to a file",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "content": {"type": "string"},
                    },
                },
            },
        ]

        overhead_with_tools = estimate_context_overhead(
            system_prompt,
            tool_definitions=tools,
        )
        overhead_without = estimate_context_overhead(system_prompt)

        assert overhead_with_tools > overhead_without

    def test_calculate_available_context(self) -> None:
        """Test calculating available context."""
        available = calculate_available_context(
            model_context_length=128000,
            system_prompt="You are a helpful assistant.",
            reserved_output=4096,
        )

        assert available > 0
        assert available < 128000
        # Should account for system prompt and reserved output
        assert available < 128000 - 4096


# =============================================================================
# Integration Tests
# =============================================================================


class TestSessionCompactionIntegration:
    """Integration tests for session with compaction."""

    @pytest.mark.asyncio
    async def test_session_context_messages_with_budget(
        self,
        sample_messages: list[Message],
    ) -> None:
        """Test get_context_messages respects token budget."""
        session = Session()
        for msg in sample_messages:
            session.add_message_object(msg)

        # Get context with small budget
        context_msgs = await session.get_context_messages(max_tokens=100)

        # Should return fewer messages
        assert len(context_msgs) <= len(sample_messages)
        # But should include system message if it fits
        if len(context_msgs) > 0:
            # System message should be first if preserved
            if any(m.role == MessageRole.SYSTEM for m in context_msgs):
                assert context_msgs[0].role == MessageRole.SYSTEM

    @pytest.mark.asyncio
    async def test_full_session_workflow(
        self,
        temp_storage_path: Path,
    ) -> None:
        """Test complete session workflow."""
        config = SessionConfig(
            auto_save_session=True,
            max_saved_sessions=10,
        )

        async with SessionManager(config, temp_storage_path) as manager:
            # 1. Create session
            session = await manager.create_session(agent_name="code-agent")

            # 2. Add messages
            await session.add_message(
                role=MessageRole.SYSTEM,
                content="You are a helpful coding assistant.",
            )
            await session.add_message(
                role=MessageRole.USER,
                content="Help me write a Python function.",
            )
            await session.add_message(
                role=MessageRole.ASSISTANT,
                content="I'd be happy to help! What should the function do?",
            )

            # 3. Save session
            await manager.save_session(session)

            # 4. Create snapshot
            snapshot_id = await manager.save_context_snapshot(
                session,
                name="Before implementation",
            )

            # 5. Add more messages
            await session.add_message(
                role=MessageRole.USER,
                content="A function to calculate fibonacci.",
            )
            await manager.save_session(session)

            # 6. Verify session
            sessions = await manager.list_sessions()
            assert len(sessions) >= 1

            # 7. Load snapshot into new session
            new_session = await manager.load_context_snapshot(snapshot_id)
            assert new_session.message_count == 3  # Before the last message

            # 8. Verify preview
            preview = await manager.get_session_preview(session.id_str, num_messages=2)
            assert len(preview) == 2
            assert preview[-1].content == "A function to calculate fibonacci."
