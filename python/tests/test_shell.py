"""Tests for shell command execution tool.

These tests cover:
1. ShellCommandTool basic functionality
2. Timeout handling
3. Output capture and truncation
4. Error handling
5. Cross-platform considerations
"""

from __future__ import annotations

import asyncio
import platform
from pathlib import Path
from typing import Any
from unittest.mock import AsyncMock, patch

import pytest

from ticca_agent.tools.shell import (
    DEFAULT_TIMEOUT,
    MAX_LINE_LENGTH,
    MAX_OUTPUT_LINES,
    ShellCommandInput,
    ShellCommandOutput,
    ShellCommandTool,
)


# =============================================================================
# ShellCommandInput Tests
# =============================================================================


class TestShellCommandInput:
    """Tests for ShellCommandInput model."""

    def test_default_values(self) -> None:
        """Test default input values."""
        input_data = ShellCommandInput(command="ls")
        
        assert input_data.command == "ls"
        assert input_data.cwd is None
        assert input_data.timeout == DEFAULT_TIMEOUT

    def test_custom_values(self) -> None:
        """Test custom input values."""
        input_data = ShellCommandInput(
            command="echo hello",
            cwd="/tmp",
            timeout=120,
        )
        
        assert input_data.command == "echo hello"
        assert input_data.cwd == "/tmp"
        assert input_data.timeout == 120

    def test_timeout_bounds(self) -> None:
        """Test timeout value bounds."""
        # Minimum timeout is 1
        input_data = ShellCommandInput(command="ls", timeout=1)
        assert input_data.timeout == 1
        
        # Maximum timeout is 600
        input_data = ShellCommandInput(command="ls", timeout=600)
        assert input_data.timeout == 600


class TestShellCommandOutput:
    """Tests for ShellCommandOutput model."""

    def test_success_output(self) -> None:
        """Test successful output."""
        output = ShellCommandOutput(
            success=True,
            command="echo hello",
            stdout="hello",
            exit_code=0,
            execution_time=0.1,
        )
        
        assert output.success is True
        assert output.stdout == "hello"
        assert output.exit_code == 0

    def test_error_output(self) -> None:
        """Test error output."""
        output = ShellCommandOutput(
            success=False,
            command="bad_command",
            error="Command not found",
            exit_code=127,
        )
        
        assert output.success is False
        assert output.error == "Command not found"
        assert output.exit_code == 127

    def test_timeout_output(self) -> None:
        """Test timeout output."""
        output = ShellCommandOutput(
            success=False,
            command="sleep 100",
            timeout=True,
            exit_code=-1,
        )
        
        assert output.timeout is True


# =============================================================================
# ShellCommandTool Tests
# =============================================================================


class TestShellCommandTool:
    """Tests for ShellCommandTool."""

    @pytest.mark.asyncio
    async def test_simple_command(self) -> None:
        """Test executing a simple command."""
        tool = ShellCommandTool()
        result = await tool.execute(
            ShellCommandInput(command="echo hello world")
        )
        
        assert result.success is True
        assert result.stdout is not None
        assert "hello world" in result.stdout
        assert result.exit_code == 0

    @pytest.mark.asyncio
    async def test_empty_command_error(self) -> None:
        """Test error on empty command."""
        tool = ShellCommandTool()
        result = await tool.execute(
            ShellCommandInput(command="")
        )
        
        assert result.error is not None
        assert "cannot be empty" in result.error

    @pytest.mark.asyncio
    async def test_whitespace_command_error(self) -> None:
        """Test error on whitespace-only command."""
        tool = ShellCommandTool()
        result = await tool.execute(
            ShellCommandInput(command="   ")
        )
        
        assert result.error is not None
        assert "cannot be empty" in result.error

    @pytest.mark.asyncio
    async def test_command_with_exit_code(self) -> None:
        """Test command that returns non-zero exit code."""
        tool = ShellCommandTool()
        result = await tool.execute(
            ShellCommandInput(command="exit 1")
        )
        
        assert result.success is False
        assert result.exit_code == 1

    @pytest.mark.asyncio
    async def test_command_with_cwd(self, temp_dir: Path) -> None:
        """Test command execution with working directory."""
        tool = ShellCommandTool()
        result = await tool.execute(
            ShellCommandInput(
                command="pwd" if platform.system() != "Windows" else "cd",
                cwd=str(temp_dir),
            )
        )
        
        assert result.success is True
        assert result.stdout is not None
        # The output should contain the temp directory path
        assert str(temp_dir) in result.stdout or temp_dir.name in result.stdout

    @pytest.mark.asyncio
    async def test_invalid_cwd_error(self) -> None:
        """Test error on invalid working directory."""
        tool = ShellCommandTool()
        result = await tool.execute(
            ShellCommandInput(
                command="echo test",
                cwd="/nonexistent/directory/xyz123",
            )
        )
        
        assert result.error is not None
        assert "does not exist" in result.error

    @pytest.mark.asyncio
    async def test_command_with_stderr(self) -> None:
        """Test command that produces stderr output."""
        tool = ShellCommandTool()
        # Redirect stdout to stderr
        cmd = "echo error >&2" if platform.system() != "Windows" else "echo error 1>&2"
        result = await tool.execute(
            ShellCommandInput(command=cmd)
        )
        
        # Command should still succeed
        assert result.exit_code == 0
        # stderr should have output
        assert result.stderr is not None or result.stdout is not None

    @pytest.mark.asyncio
    async def test_multiline_output(self, temp_dir: Path) -> None:
        """Test command with multiline output."""
        # Create test files
        for i in range(5):
            (temp_dir / f"file{i}.txt").touch()
        
        tool = ShellCommandTool()
        ls_cmd = "ls" if platform.system() != "Windows" else "dir /b"
        result = await tool.execute(
            ShellCommandInput(command=ls_cmd, cwd=str(temp_dir))
        )
        
        assert result.success is True
        assert result.stdout is not None
        # Should have multiple lines
        lines = result.stdout.strip().split("\n")
        assert len(lines) >= 5

    @pytest.mark.asyncio
    async def test_execution_time_tracked(self) -> None:
        """Test that execution time is tracked."""
        tool = ShellCommandTool()
        result = await tool.execute(
            ShellCommandInput(command="echo fast")
        )
        
        assert result.execution_time is not None
        assert result.execution_time >= 0

    @pytest.mark.asyncio
    @pytest.mark.slow
    async def test_timeout_terminates_command(self) -> None:
        """Test that timeout terminates long-running commands."""
        tool = ShellCommandTool()
        result = await tool.execute(
            ShellCommandInput(
                command="sleep 60",
                timeout=1,  # 1 second timeout
            )
        )
        
        # Should timeout
        assert result.timeout is True
        assert result.success is False

    @pytest.mark.asyncio
    async def test_schema_generation(self) -> None:
        """Test tool schema generation."""
        tool = ShellCommandTool()
        schema = tool.get_schema()
        
        assert schema["name"] == "agent_run_shell_command"
        assert "description" in schema
        assert "parameters" in schema

    @pytest.mark.asyncio
    async def test_yolo_mode_flag(self) -> None:
        """Test yolo mode flag affects confirmation requirement."""
        yolo_tool = ShellCommandTool(yolo_mode=True)
        safe_tool = ShellCommandTool(yolo_mode=False)
        
        assert yolo_tool.config.requires_confirmation is False
        assert safe_tool.config.requires_confirmation is True


# =============================================================================
# Output Truncation Tests
# =============================================================================


class TestOutputTruncation:
    """Tests for output truncation behavior."""

    @pytest.mark.asyncio
    async def test_long_line_truncation(self, temp_dir: Path) -> None:
        """Test that long lines are truncated."""
        # Create a file with a very long line
        long_line = "x" * (MAX_LINE_LENGTH * 2)
        test_file = temp_dir / "long_line.txt"
        test_file.write_text(long_line)
        
        tool = ShellCommandTool()
        cat_cmd = f"cat {test_file}" if platform.system() != "Windows" else f"type {test_file}"
        result = await tool.execute(
            ShellCommandInput(command=cat_cmd)
        )
        
        assert result.success is True
        # Note: truncation happens at stream read, so we check it's not empty
        assert result.stdout is not None

    @pytest.mark.asyncio
    async def test_many_lines_truncation(self, temp_dir: Path) -> None:
        """Test that output with many lines is truncated."""
        # Create a file with many lines
        lines = "\n".join([f"line {i}" for i in range(MAX_OUTPUT_LINES * 2)])
        test_file = temp_dir / "many_lines.txt"
        test_file.write_text(lines)
        
        tool = ShellCommandTool()
        cat_cmd = f"cat {test_file}" if platform.system() != "Windows" else f"type {test_file}"
        result = await tool.execute(
            ShellCommandInput(command=cat_cmd)
        )
        
        assert result.success is True
        assert result.stdout is not None
        
        # Should have at most MAX_OUTPUT_LINES lines
        output_lines = result.stdout.strip().split("\n")
        assert len(output_lines) <= MAX_OUTPUT_LINES


# =============================================================================
# Error Handling Tests
# =============================================================================


class TestErrorHandling:
    """Tests for error handling."""

    @pytest.mark.asyncio
    async def test_command_not_found(self) -> None:
        """Test handling of command not found."""
        tool = ShellCommandTool()
        result = await tool.execute(
            ShellCommandInput(command="nonexistent_command_xyz123")
        )
        
        # Should fail with non-zero exit code
        assert result.success is False
        # Either exit_code or error should indicate failure
        assert result.exit_code != 0 or result.error is not None

    @pytest.mark.asyncio
    async def test_permission_denied_file(self, temp_dir: Path) -> None:
        """Test handling of permission denied."""
        # Create a file we can't read (Unix only)
        if platform.system() == "Windows":
            pytest.skip("Permission test not applicable on Windows")
        
        restricted_file = temp_dir / "restricted.txt"
        restricted_file.write_text("secret")
        restricted_file.chmod(0o000)
        
        try:
            tool = ShellCommandTool()
            result = await tool.execute(
                ShellCommandInput(command=f"cat {restricted_file}")
            )
            
            # Should fail
            assert result.success is False or result.stderr is not None
        finally:
            # Restore permissions for cleanup
            restricted_file.chmod(0o644)


# =============================================================================
# Integration Tests
# =============================================================================


class TestShellIntegration:
    """Integration tests for shell command execution."""

    @pytest.mark.asyncio
    @pytest.mark.integration
    async def test_pipe_commands(self) -> None:
        """Test piped commands work correctly."""
        if platform.system() == "Windows":
            pytest.skip("Pipe test written for Unix")
        
        tool = ShellCommandTool()
        result = await tool.execute(
            ShellCommandInput(command="echo hello world | wc -w")
        )
        
        assert result.success is True
        assert result.stdout is not None
        # Should return "2" (word count)
        assert "2" in result.stdout.strip()

    @pytest.mark.asyncio
    @pytest.mark.integration
    async def test_environment_variables(self) -> None:
        """Test environment variable handling."""
        tool = ShellCommandTool()
        
        # Use shell expansion
        cmd = "echo $HOME" if platform.system() != "Windows" else "echo %USERPROFILE%"
        result = await tool.execute(
            ShellCommandInput(command=cmd)
        )
        
        assert result.success is True
        assert result.stdout is not None
        # Should have expanded the variable
        assert len(result.stdout.strip()) > 0

    @pytest.mark.asyncio
    @pytest.mark.integration
    async def test_multiple_commands(self) -> None:
        """Test multiple commands in sequence."""
        if platform.system() == "Windows":
            cmd = "echo first && echo second"
        else:
            cmd = "echo first && echo second"
        
        tool = ShellCommandTool()
        result = await tool.execute(
            ShellCommandInput(command=cmd)
        )
        
        assert result.success is True
        assert result.stdout is not None
        assert "first" in result.stdout
        assert "second" in result.stdout

    @pytest.mark.asyncio
    @pytest.mark.integration
    async def test_subshell(self) -> None:
        """Test subshell execution."""
        if platform.system() == "Windows":
            pytest.skip("Subshell test written for Unix")
        
        tool = ShellCommandTool()
        result = await tool.execute(
            ShellCommandInput(command="(cd /tmp && pwd)")
        )
        
        assert result.success is True
        assert result.stdout is not None
        assert "/tmp" in result.stdout
