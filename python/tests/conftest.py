"""Shared test fixtures and configuration for Ticca Agent tests.

This module provides:
- Pytest configuration
- Shared fixtures for tests
- Mock providers and factories
- Test utilities

"""

import asyncio
import json
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, AsyncIterator, Iterator
from unittest.mock import AsyncMock, MagicMock, patch
from uuid import uuid4

import pytest

from ticca_agent.core.types import Message, MessageRole


# =============================================================================
# Pytest Configuration
# =============================================================================


@pytest.fixture(scope="session")
def event_loop() -> Iterator[asyncio.AbstractEventLoop]:
    """Create an event loop for async tests."""
    loop = asyncio.new_event_loop()
    yield loop
    loop.close()


# =============================================================================
# Temporary Directory Fixtures
# =============================================================================


@pytest.fixture
def temp_dir() -> Iterator[Path]:
    """Create a temporary directory for test files."""
    with tempfile.TemporaryDirectory(prefix="ticca_test_") as tmpdir:
        yield Path(tmpdir)


@pytest.fixture
def temp_file(temp_dir: Path) -> Path:
    """Create a temporary test file."""
    file_path = temp_dir / "test_file.txt"
    file_path.write_text("Hello, World!\nThis is a test file.\nLine 3.\n")
    return file_path


@pytest.fixture
def nested_dir_structure(temp_dir: Path) -> Path:
    """Create a nested directory structure for testing."""
    # Create structure:
    # temp_dir/
    #   src/
    #     main.py
    #     utils/
    #       helper.py
    #   tests/
    #     test_main.py
    #   README.md
    
    (temp_dir / "src").mkdir()
    (temp_dir / "src" / "utils").mkdir()
    (temp_dir / "tests").mkdir()
    
    (temp_dir / "src" / "main.py").write_text("def main():\n    pass\n")
    (temp_dir / "src" / "utils" / "helper.py").write_text("def helper():\n    pass\n")
    (temp_dir / "tests" / "test_main.py").write_text("def test_main():\n    pass\n")
    (temp_dir / "README.md").write_text("# Test Project\n\nThis is a test.\n")
    
    return temp_dir


# =============================================================================
# Message Fixtures
# =============================================================================


@pytest.fixture
def sample_messages() -> list[Message]:
    """Create sample messages for testing."""
    return [
        Message(role=MessageRole.SYSTEM, content="You are a helpful assistant."),
        Message(role=MessageRole.USER, content="Hello, how are you?"),
        Message(role=MessageRole.ASSISTANT, content="I'm doing great, thanks!"),
        Message(role=MessageRole.USER, content="Can you help me with Python?"),
        Message(
            role=MessageRole.ASSISTANT,
            content="Of course! What would you like to know?",
        ),
    ]


@pytest.fixture
def large_message_content() -> str:
    """Create large content for token testing."""
    return "This is a test sentence. " * 1000


@pytest.fixture
def message_with_tool_calls() -> Message:
    """Create a message with tool calls."""
    return Message(
        role=MessageRole.ASSISTANT,
        content="Let me read that file for you.",
        tool_calls=[
            {
                "id": "call_123",
                "type": "function",
                "function": {
                    "name": "read_file",
                    "arguments": '{"path": "/test/file.py"}',
                },
            }
        ],
    )


# =============================================================================
# Mock Provider Fixtures
# =============================================================================


class MockProvider:
    """Mock AI provider for testing.
    
    This provider simulates AI responses without making actual API calls.
    """
    
    def __init__(
        self,
        *,
        response_content: str = "Mock response",
        should_error: bool = False,
        error_message: str = "Mock error",
        tokens_used: int = 100,
    ) -> None:
        self.response_content = response_content
        self.should_error = should_error
        self.error_message = error_message
        self.tokens_used = tokens_used
        self.call_count = 0
        self.last_messages: list[dict[str, Any]] = []
    
    async def generate(
        self,
        messages: list[dict[str, Any]],
        **kwargs: Any,
    ) -> dict[str, Any]:
        """Generate a mock response."""
        self.call_count += 1
        self.last_messages = messages
        
        if self.should_error:
            raise RuntimeError(self.error_message)
        
        return {
            "content": self.response_content,
            "usage": {
                "prompt_tokens": self.tokens_used // 2,
                "completion_tokens": self.tokens_used // 2,
                "total_tokens": self.tokens_used,
            },
            "finish_reason": "stop",
        }
    
    async def stream(
        self,
        messages: list[dict[str, Any]],
        **kwargs: Any,
    ) -> AsyncIterator[dict[str, Any]]:
        """Stream mock response chunks."""
        self.call_count += 1
        self.last_messages = messages
        
        if self.should_error:
            raise RuntimeError(self.error_message)
        
        # Yield chunks
        words = self.response_content.split()
        for i, word in enumerate(words):
            yield {
                "type": "text_delta",
                "content": word + " " if i < len(words) - 1 else word,
            }
        
        yield {
            "type": "done",
            "usage": {
                "prompt_tokens": self.tokens_used // 2,
                "completion_tokens": self.tokens_used // 2,
                "total_tokens": self.tokens_used,
            },
            "finish_reason": "stop",
        }


@pytest.fixture
def mock_provider() -> MockProvider:
    """Create a mock AI provider."""
    return MockProvider()


@pytest.fixture
def error_provider() -> MockProvider:
    """Create a mock provider that errors."""
    return MockProvider(should_error=True, error_message="API Error")


# =============================================================================
# Bridge Test Fixtures
# =============================================================================


@pytest.fixture
def sample_agent_request() -> dict[str, Any]:
    """Create a sample agent request dictionary."""
    return {
        "session_id": str(uuid4()),
        "conversation_id": str(uuid4()),
        "message": "Hello, world!",
        "agent_name": "default",
        "temperature": 0.7,
        "max_tokens": None,
        "tools_enabled": [],
        "context": {},
    }


# =============================================================================
# MCP Test Fixtures
# =============================================================================


@pytest.fixture
def mcp_server_config() -> dict[str, Any]:
    """Create a sample MCP server configuration."""
    return {
        "name": "test-server",
        "server_type": "stdio",
        "enabled": True,
        "config": {
            "command": "python",
            "args": ["-m", "test_server"],
        },
    }


@pytest.fixture
def mcp_tool_definition() -> dict[str, Any]:
    """Create a sample MCP tool definition."""
    return {
        "name": "test_tool",
        "description": "A test tool for testing",
        "inputSchema": {
            "type": "object",
            "properties": {
                "param1": {"type": "string", "description": "First parameter"},
                "param2": {"type": "integer", "description": "Second parameter"},
            },
            "required": ["param1"],
        },
    }


# =============================================================================
# Session Test Fixtures
# =============================================================================


@pytest.fixture
def session_config() -> dict[str, Any]:
    """Create a sample session configuration."""
    return {
        "auto_save_session": True,
        "max_saved_sessions": 10,
        "max_messages": 1000,
        "compaction_threshold": 0.85,
        "enable_persistence": True,
    }


# =============================================================================
# File Operations Test Helpers
# =============================================================================


@pytest.fixture
def sandbox_root(temp_dir: Path) -> Path:
    """Create a sandbox root directory for file operations."""
    sandbox = temp_dir / "sandbox"
    sandbox.mkdir()
    return sandbox


@pytest.fixture
def files_in_sandbox(sandbox_root: Path) -> dict[str, Path]:
    """Create test files within sandbox."""
    files = {}
    
    # Create test files
    files["readme"] = sandbox_root / "README.md"
    files["readme"].write_text("# Test\n\nThis is a test file.\n")
    
    files["source"] = sandbox_root / "src" / "main.py"
    files["source"].parent.mkdir(parents=True)
    files["source"].write_text('print("Hello")\n')
    
    files["config"] = sandbox_root / "config.toml"
    files["config"].write_text("[settings]\nkey = 'value'\n")
    
    return files


# =============================================================================
# Async Test Helpers
# =============================================================================


@pytest.fixture
def mock_asyncio_subprocess() -> MagicMock:
    """Create a mock asyncio subprocess."""
    mock_process = AsyncMock()
    mock_process.returncode = 0
    mock_process.stdout = AsyncMock()
    mock_process.stderr = AsyncMock()
    mock_process.stdout.readline = AsyncMock(return_value=b"")
    mock_process.stderr.readline = AsyncMock(return_value=b"")
    mock_process.wait = AsyncMock()
    return mock_process


# =============================================================================
# Marker Definitions
# =============================================================================


def pytest_configure(config: pytest.Config) -> None:
    """Configure custom pytest markers."""
    config.addinivalue_line(
        "markers", "slow: marks tests as slow (deselect with '-m \"not slow\"')"
    )
    config.addinivalue_line(
        "markers", "integration: marks tests as integration tests"
    )
    config.addinivalue_line(
        "markers", "security: marks tests related to security features"
    )
