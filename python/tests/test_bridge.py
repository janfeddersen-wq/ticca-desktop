"""Tests for the Rust-Python bridge interface.

These tests verify that:
1. Bridge types serialize correctly to match Rust's serde format
2. BridgeInterface methods handle all error cases gracefully
3. Streaming works correctly with callbacks
4. Agent registry functions work properly
"""

import asyncio
import json
import pytest
from typing import Any
from uuid import uuid4

from ticca_agent.bridge_types import (
    AgentInfo,
    AgentRequest,
    AgentResponse,
    AgentState,
    DoneChunk,
    ErrorChunk,
    FinishReasonError,
    FinishReasonMaxTokens,
    FinishReasonStop,
    FinishReasonToolUse,
    StateChangeChunk,
    TextDeltaChunk,
    ThinkingDeltaChunk,
    TokenUsage,
    ToolCall,
    ToolResultChunk,
    ToolStartChunk,
    UsageChunk,
)
from ticca_agent.bridge import (
    BridgeInterface,
    BridgeError,
    clear_agent_registry,
    create_bridge,
    get_agent_info,
    list_registered_agents,
    register_agent,
    unregister_agent,
)


# =============================================================================
# Fixtures
# =============================================================================


@pytest.fixture
def bridge() -> BridgeInterface:
    """Create a fresh bridge instance."""
    clear_agent_registry()
    return BridgeInterface()


@pytest.fixture
def sample_request() -> AgentRequest:
    """Create a sample agent request."""
    return AgentRequest(
        session_id="test-session",
        conversation_id="test-conversation",
        message="Hello, how are you?",
        agent_name="default",
        temperature=0.7,
    )


@pytest.fixture
def sample_agent_info() -> AgentInfo:
    """Create a sample agent info."""
    return AgentInfo(
        id="test-agent",
        name="Test Agent",
        description="A test agent for unit tests",
        default_model="test-model",
        available_tools=["tool1", "tool2"],
        capabilities=["testing"],
    )


# =============================================================================
# Bridge Types Serialization Tests
# =============================================================================


class TestBridgeTypesSerialization:
    """Tests for bridge types JSON serialization."""

    def test_agent_request_serialization(self, sample_request: AgentRequest) -> None:
        """Test AgentRequest serializes correctly."""
        json_str = sample_request.model_dump_json()
        data = json.loads(json_str)

        assert data["session_id"] == "test-session"
        assert data["conversation_id"] == "test-conversation"
        assert data["message"] == "Hello, how are you?"
        assert data["agent_name"] == "default"
        assert data["temperature"] == 0.7
        assert data["model"] is None
        assert data["max_tokens"] is None
        assert data["tools_enabled"] == []
        assert data["context"] == {}

    def test_agent_request_deserialization(self) -> None:
        """Test AgentRequest deserializes correctly."""
        data = {
            "session_id": "s1",
            "conversation_id": "c1",
            "message": "Hi",
            "agent_name": "code",
            "model": "gpt-4",
            "temperature": 0.5,
            "max_tokens": 1000,
            "tools_enabled": ["read_file"],
            "context": {"key": "value"},
        }
        request = AgentRequest.model_validate(data)

        assert request.session_id == "s1"
        assert request.model == "gpt-4"
        assert request.max_tokens == 1000
        assert request.tools_enabled == ["read_file"]
        assert request.context == {"key": "value"}

    def test_finish_reason_stop_serialization(self) -> None:
        """Test FinishReasonStop matches Rust's tagged enum format."""
        reason = FinishReasonStop()
        data = json.loads(reason.model_dump_json())

        # Rust: #[serde(tag = "type", content = "value")]
        # Stop has no content, so just {"type": "Stop"}
        assert data == {"type": "Stop"}

    def test_finish_reason_tool_use_serialization(self) -> None:
        """Test FinishReasonToolUse serialization."""
        reason = FinishReasonToolUse()
        data = json.loads(reason.model_dump_json())

        assert data == {"type": "ToolUse"}

    def test_finish_reason_max_tokens_serialization(self) -> None:
        """Test FinishReasonMaxTokens serialization."""
        reason = FinishReasonMaxTokens()
        data = json.loads(reason.model_dump_json())

        assert data == {"type": "MaxTokens"}

    def test_finish_reason_error_serialization(self) -> None:
        """Test FinishReasonError matches Rust's tagged enum with value."""
        reason = FinishReasonError(value="Something went wrong")
        data = json.loads(reason.model_dump_json())

        # Rust: Error(String) -> {"type": "Error", "value": "..."}
        assert data == {"type": "Error", "value": "Something went wrong"}

    def test_agent_state_serialization(self) -> None:
        """Test AgentState serializes to lowercase."""
        # Rust: #[serde(rename_all = "lowercase")]
        assert AgentState.IDLE.value == "idle"
        assert AgentState.THINKING.value == "thinking"
        assert AgentState.ACTING.value == "acting"
        assert AgentState.OBSERVING.value == "observing"
        assert AgentState.RESPONDING.value == "responding"

    def test_token_usage_serialization(self) -> None:
        """Test TokenUsage serialization."""
        usage = TokenUsage.create(100, 50)
        data = json.loads(usage.model_dump_json())

        assert data["prompt_tokens"] == 100
        assert data["completion_tokens"] == 50
        assert data["total_tokens"] == 150

    def test_tool_call_serialization(self) -> None:
        """Test ToolCall serialization."""
        tool_call = ToolCall(
            id="call-123",
            name="read_file",
            arguments={"path": "/tmp/test.txt"},
        )
        data = json.loads(tool_call.model_dump_json())

        assert data["id"] == "call-123"
        assert data["name"] == "read_file"
        assert data["arguments"] == {"path": "/tmp/test.txt"}

    def test_agent_response_serialization(self) -> None:
        """Test AgentResponse serialization."""
        response = AgentResponse.text(
            message_id="msg-1",
            content="Hello!",
            usage=TokenUsage.create(10, 5),
        )
        data = json.loads(response.model_dump_json(by_alias=True))

        assert data["message_id"] == "msg-1"
        assert data["content"] == "Hello!"
        assert data["role"] == "assistant"
        assert data["tool_calls"] is None
        assert data["usage"]["total_tokens"] == 15
        assert data["finish_reason"]["type"] == "Stop"

    def test_agent_response_error_serialization(self) -> None:
        """Test AgentResponse error creation."""
        response = AgentResponse.error("msg-2", "Something went wrong")
        data = json.loads(response.model_dump_json(by_alias=True))

        assert data["message_id"] == "msg-2"
        assert data["content"] == ""
        assert data["finish_reason"]["type"] == "Error"
        assert data["finish_reason"]["value"] == "Something went wrong"


# =============================================================================
# Stream Chunk Serialization Tests
# =============================================================================


class TestStreamChunkSerialization:
    """Tests for stream chunk JSON serialization."""

    def test_text_delta_chunk(self) -> None:
        """Test TextDeltaChunk serialization."""
        chunk = TextDeltaChunk(content="Hello")
        data = json.loads(chunk.model_dump_json())

        # Rust: #[serde(tag = "type")]
        assert data == {"type": "TextDelta", "content": "Hello"}

    def test_thinking_delta_chunk(self) -> None:
        """Test ThinkingDeltaChunk serialization."""
        chunk = ThinkingDeltaChunk(content="Let me think...")
        data = json.loads(chunk.model_dump_json())

        assert data == {"type": "ThinkingDelta", "content": "Let me think..."}

    def test_tool_start_chunk(self) -> None:
        """Test ToolStartChunk serialization."""
        tool_call = ToolCall(id="tc-1", name="search", arguments={"query": "test"})
        chunk = ToolStartChunk(tool_call=tool_call)
        data = json.loads(chunk.model_dump_json())

        assert data["type"] == "ToolStart"
        assert data["tool_call"]["id"] == "tc-1"
        assert data["tool_call"]["name"] == "search"

    def test_tool_result_chunk(self) -> None:
        """Test ToolResultChunk serialization."""
        chunk = ToolResultChunk(tool_call_id="tc-1", result="Search results here")
        data = json.loads(chunk.model_dump_json())

        assert data == {
            "type": "ToolResult",
            "tool_call_id": "tc-1",
            "result": "Search results here",
        }

    def test_state_change_chunk(self) -> None:
        """Test StateChangeChunk serialization with aliases."""
        chunk = StateChangeChunk(
            from_state=AgentState.IDLE,
            to_state=AgentState.THINKING,
        )
        # Must use by_alias=True to get 'from'/'to' instead of 'from_state'/'to_state'
        data = json.loads(chunk.model_dump_json(by_alias=True))

        # Rust uses 'from' and 'to'
        assert data == {
            "type": "StateChange",
            "from": "idle",
            "to": "thinking",
        }

    def test_usage_chunk(self) -> None:
        """Test UsageChunk serialization."""
        usage = TokenUsage(prompt_tokens=100, completion_tokens=50, total_tokens=150)
        chunk = UsageChunk(usage=usage)
        data = json.loads(chunk.model_dump_json())

        assert data["type"] == "Usage"
        assert data["usage"]["total_tokens"] == 150

    def test_done_chunk(self) -> None:
        """Test DoneChunk serialization."""
        response = AgentResponse.text("msg-1", "Done!", TokenUsage())
        chunk = DoneChunk(response=response)
        data = json.loads(chunk.model_dump_json(by_alias=True))

        assert data["type"] == "Done"
        assert data["response"]["message_id"] == "msg-1"
        assert data["response"]["content"] == "Done!"

    def test_error_chunk(self) -> None:
        """Test ErrorChunk serialization."""
        chunk = ErrorChunk(message="Something went wrong")
        data = json.loads(chunk.model_dump_json())

        assert data == {"type": "Error", "message": "Something went wrong"}


# =============================================================================
# Agent Registry Tests
# =============================================================================


class TestAgentRegistry:
    """Tests for agent registry functions."""

    def setup_method(self) -> None:
        """Clear registry before each test."""
        clear_agent_registry()

    def test_register_agent(self, sample_agent_info: AgentInfo) -> None:
        """Test registering an agent."""
        register_agent(sample_agent_info)

        info = get_agent_info("test-agent")
        assert info is not None
        assert info.name == "Test Agent"

    def test_register_duplicate_agent_raises(self, sample_agent_info: AgentInfo) -> None:
        """Test that registering duplicate agent raises ValueError."""
        register_agent(sample_agent_info)

        with pytest.raises(ValueError, match="already registered"):
            register_agent(sample_agent_info)

    def test_unregister_agent(self, sample_agent_info: AgentInfo) -> None:
        """Test unregistering an agent."""
        register_agent(sample_agent_info)
        assert unregister_agent("test-agent") is True
        assert get_agent_info("test-agent") is None

    def test_unregister_nonexistent_agent(self) -> None:
        """Test unregistering a non-existent agent returns False."""
        assert unregister_agent("nonexistent") is False

    def test_list_registered_agents(self, sample_agent_info: AgentInfo) -> None:
        """Test listing registered agents."""
        register_agent(sample_agent_info)
        agents = list_registered_agents()

        assert len(agents) == 1
        assert agents[0].id == "test-agent"

    def test_clear_registry(self, sample_agent_info: AgentInfo) -> None:
        """Test clearing the registry."""
        register_agent(sample_agent_info)
        clear_agent_registry()

        assert list_registered_agents() == []


# =============================================================================
# BridgeInterface Tests
# =============================================================================


class TestBridgeInterface:
    """Tests for BridgeInterface methods."""

    def setup_method(self) -> None:
        """Clear registry before each test."""
        clear_agent_registry()

    def test_list_agents_returns_defaults(self, bridge: BridgeInterface) -> None:
        """Test list_agents returns defaults when no agents registered."""
        result = bridge.list_agents()
        agents = json.loads(result)

        assert isinstance(agents, list)
        assert len(agents) >= 1
        # Default agents should include 'default'
        agent_ids = [a["id"] for a in agents]
        assert "default" in agent_ids

    def test_list_agents_returns_registered(
        self,
        bridge: BridgeInterface,
        sample_agent_info: AgentInfo,
    ) -> None:
        """Test list_agents returns registered agents."""
        register_agent(sample_agent_info)
        result = bridge.list_agents()
        agents = json.loads(result)

        assert len(agents) == 1
        assert agents[0]["id"] == "test-agent"

    def test_get_agent_info_found(self, bridge: BridgeInterface) -> None:
        """Test get_agent_info for existing agent."""
        result = bridge.get_agent_info("default")
        data = json.loads(result)

        assert data is not None
        assert data["id"] == "default"

    def test_get_agent_info_not_found(self, bridge: BridgeInterface) -> None:
        """Test get_agent_info for non-existent agent."""
        result = bridge.get_agent_info("nonexistent")
        assert result == "null"

    @pytest.mark.asyncio
    async def test_execute_invalid_json(self, bridge: BridgeInterface) -> None:
        """Test execute with invalid JSON returns error response."""
        result = await bridge.execute("not valid json")
        data = json.loads(result)

        assert data["finish_reason"]["type"] == "Error"
        assert "Invalid JSON" in data["finish_reason"]["value"]

    @pytest.mark.asyncio
    async def test_execute_invalid_request(self, bridge: BridgeInterface) -> None:
        """Test execute with invalid request returns error response."""
        # Missing required fields
        result = await bridge.execute('{"session_id": "test"}')
        data = json.loads(result)

        assert data["finish_reason"]["type"] == "Error"
        assert "Invalid request" in data["finish_reason"]["value"]

    @pytest.mark.asyncio
    async def test_execute_no_provider(self, bridge: BridgeInterface) -> None:
        """Test execute with no provider returns error response."""
        request = AgentRequest(
            session_id="s1",
            conversation_id="c1",
            message="Hello",
            agent_name="nonexistent-agent",
        )
        result = await bridge.execute(request.model_dump_json())
        data = json.loads(result)

        assert data["finish_reason"]["type"] == "Error"
        assert "No provider found" in data["finish_reason"]["value"]

    @pytest.mark.asyncio
    async def test_execute_streaming_invalid_json(
        self,
        bridge: BridgeInterface,
    ) -> None:
        """Test execute_streaming with invalid JSON sends error chunk."""
        chunks_received: list[str] = []

        def callback(chunk: str) -> None:
            chunks_received.append(chunk)

        result = await bridge.execute_streaming("invalid json", callback)

        # Should have received at least an error chunk
        assert len(chunks_received) >= 1
        error_chunk = json.loads(chunks_received[-1])
        assert error_chunk["type"] == "Error"

    @pytest.mark.asyncio
    async def test_execute_streaming_state_transitions(
        self,
        bridge: BridgeInterface,
    ) -> None:
        """Test execute_streaming sends state change chunks."""
        chunks_received: list[str] = []

        def callback(chunk: str) -> None:
            chunks_received.append(chunk)

        request = AgentRequest(
            session_id="s1",
            conversation_id="c1",
            message="Hello",
            agent_name="nonexistent",
        )
        await bridge.execute_streaming(request.model_dump_json(), callback)

        # Parse all chunks
        parsed = [json.loads(c) for c in chunks_received]
        types = [c["type"] for c in parsed]

        # Should have state changes even if agent fails
        assert "StateChange" in types


# =============================================================================
# AgentInfo Serialization Tests
# =============================================================================


class TestAgentInfoSerialization:
    """Tests for AgentInfo serialization."""

    def test_agent_info_serialization(self, sample_agent_info: AgentInfo) -> None:
        """Test AgentInfo serializes correctly."""
        data = json.loads(sample_agent_info.model_dump_json())

        assert data["id"] == "test-agent"
        assert data["name"] == "Test Agent"
        assert data["description"] == "A test agent for unit tests"
        assert data["default_model"] == "test-model"
        assert data["available_tools"] == ["tool1", "tool2"]
        assert data["capabilities"] == ["testing"]

    def test_agent_info_optional_description(self) -> None:
        """Test AgentInfo with no description."""
        info = AgentInfo(
            id="minimal",
            name="Minimal Agent",
            default_model="model",
        )
        data = json.loads(info.model_dump_json())

        assert data["description"] is None
        assert data["available_tools"] == []
        assert data["capabilities"] == []
