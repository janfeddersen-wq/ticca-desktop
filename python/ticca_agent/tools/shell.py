"""Shell command execution tool for Ticca Agent.

This module provides safe shell command execution with:
- Output capture (stdout/stderr)
- Timeout handling (default 60s inactivity timeout)
- Output truncation (last 256 lines)
- Line length limits (256 characters)
- Cross-platform support (Windows and POSIX)

Example:
    >>> from ticca_agent.tools.shell import ShellCommandTool
    >>> tool = ShellCommandTool()
    >>> result = await tool.run(ShellCommandInput(command="ls -la"))
    >>> print(result.output.stdout)

"""

from __future__ import annotations

import asyncio
import os
import platform
import signal
import sys
import time
from typing import TYPE_CHECKING, Any, ClassVar

from pydantic import BaseModel, Field

from ticca_agent.tools.base import BaseTool, ToolConfig

if TYPE_CHECKING:
    from ticca_agent.core.context import RunContext


# Configuration constants
DEFAULT_TIMEOUT = 60  # Inactivity timeout in seconds
MAX_OUTPUT_LINES = 256  # Maximum lines to keep from output
MAX_LINE_LENGTH = 256  # Maximum characters per line
DEFAULT_CWD: str | None = None  # Use current directory


class ShellCommandInput(BaseModel):
    """Input for shell command execution.

    Attributes:
        command: Shell command to execute.
        cwd: Working directory for command execution.
        timeout: Inactivity timeout in seconds.
    """

    command: str = Field(
        description="Shell command to execute",
    )
    cwd: str | None = Field(
        default=None,
        description="Working directory for execution",
    )
    timeout: int = Field(
        default=DEFAULT_TIMEOUT,
        ge=1,
        le=600,
        description="Inactivity timeout in seconds",
    )


class ShellCommandOutput(BaseModel):
    """Output from shell command execution.

    Attributes:
        success: Whether command executed successfully (exit code 0).
        command: The executed command string.
        error: Error message if execution failed.
        stdout: Standard output (last 256 lines).
        stderr: Standard error (last 256 lines).
        exit_code: Process exit code.
        execution_time: Total execution time in seconds.
        timeout: Whether command was terminated due to timeout.
        user_interrupted: Whether user killed the process.
    """

    success: bool = Field(default=False, description="Execution success")
    command: str | None = Field(default=None, description="Executed command")
    error: str | None = Field(default=None, description="Error message")
    stdout: str | None = Field(default=None, description="Standard output")
    stderr: str | None = Field(default=None, description="Standard error")
    exit_code: int | None = Field(default=None, description="Exit code")
    execution_time: float | None = Field(
        default=None,
        description="Execution time in seconds",
    )
    timeout: bool | None = Field(
        default=None,
        description="Terminated due to timeout",
    )
    user_interrupted: bool | None = Field(
        default=None,
        description="Killed by user",
    )


class ShellCommandTool(BaseTool[ShellCommandInput, ShellCommandOutput]):
    """Tool for executing shell commands.

    Features:
    - Streaming output capture
    - Inactivity timeout (terminates if no output for timeout seconds)
    - Output truncation (prevents token overflow)
    - Line length limits
    - Cross-platform support

    Example:
        >>> tool = ShellCommandTool()
        >>> result = await tool.run(
        ...     ShellCommandInput(
        ...         command="python --version",
        ...         timeout=30,
        ...     )
        ... )
        >>> print(result.output.stdout)
        Python 3.12.0
    """

    # Platform detection
    IS_WINDOWS: ClassVar[bool] = platform.system() == "Windows"

    def __init__(self, yolo_mode: bool = True) -> None:
        """Initialize the shell command tool.

        Args:
            yolo_mode: If False, require confirmation before execution.
        """
        super().__init__(
            ToolConfig(
                name="agent_run_shell_command",
                description=(
                    "Execute shell commands with output capture and timeout handling. "
                    "Supports cross-platform execution with safety limits."
                ),
                requires_confirmation=not yolo_mode,
                timeout=DEFAULT_TIMEOUT,
            )
        )
        self._yolo_mode = yolo_mode

    async def execute(
        self,
        input_data: ShellCommandInput,
        context: RunContext | None = None,
    ) -> ShellCommandOutput:
        """Execute a shell command.

        Args:
            input_data: Command and parameters.
            context: Optional run context.

        Returns:
            ShellCommandOutput with results.
        """
        command = input_data.command.strip()

        if not command:
            return ShellCommandOutput(
                error="Command cannot be empty",
            )

        # Resolve working directory
        cwd = input_data.cwd
        if cwd:
            cwd = os.path.expanduser(cwd)
            if not os.path.isdir(cwd):
                return ShellCommandOutput(
                    command=command,
                    error=f"Working directory does not exist: {cwd}",
                )

        start_time = time.monotonic()

        try:
            result = await self._run_command(
                command=command,
                cwd=cwd,
                timeout=input_data.timeout,
            )

            execution_time = time.monotonic() - start_time
            result.execution_time = execution_time

            return result

        except asyncio.CancelledError:
            return ShellCommandOutput(
                command=command,
                error="Command was cancelled",
                user_interrupted=True,
                execution_time=time.monotonic() - start_time,
            )
        except Exception as e:
            return ShellCommandOutput(
                command=command,
                error=f"Execution error: {e}",
                execution_time=time.monotonic() - start_time,
            )

    async def _run_command(
        self,
        command: str,
        cwd: str | None,
        timeout: int,
    ) -> ShellCommandOutput:
        """Run a command with streaming output.

        Args:
            command: Command to execute.
            cwd: Working directory.
            timeout: Inactivity timeout.

        Returns:
            ShellCommandOutput with results.
        """
        # Determine shell settings based on platform
        if self.IS_WINDOWS:
            shell_cmd = ["cmd.exe", "/c", command]
            creationflags = subprocess.CREATE_NEW_PROCESS_GROUP  # type: ignore[name-defined]
        else:
            shell_cmd = ["/bin/sh", "-c", command]
            creationflags = 0

        # Import subprocess here to handle platform-specific flags
        import subprocess as sp

        # Create process
        process = await asyncio.create_subprocess_shell(
            command,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            cwd=cwd,
            start_new_session=not self.IS_WINDOWS,
        )

        stdout_lines: list[str] = []
        stderr_lines: list[str] = []
        timed_out = False

        async def read_stream(
            stream: asyncio.StreamReader | None,
            lines: list[str],
        ) -> None:
            """Read from a stream and append to lines list."""
            if stream is None:
                return

            while True:
                try:
                    line = await asyncio.wait_for(
                        stream.readline(),
                        timeout=timeout,
                    )
                    if not line:
                        break

                    # Decode and truncate line
                    decoded = line.decode("utf-8", errors="replace").rstrip("\n\r")
                    if len(decoded) > MAX_LINE_LENGTH:
                        decoded = decoded[:MAX_LINE_LENGTH] + "..."

                    lines.append(decoded)

                    # Keep only last N lines
                    if len(lines) > MAX_OUTPUT_LINES * 2:
                        lines[:] = lines[-MAX_OUTPUT_LINES:]

                except asyncio.TimeoutError:
                    # Inactivity timeout
                    raise

        try:
            # Read both streams concurrently
            await asyncio.gather(
                read_stream(process.stdout, stdout_lines),
                read_stream(process.stderr, stderr_lines),
            )

        except asyncio.TimeoutError:
            timed_out = True
            # Terminate the process
            await self._terminate_process(process)

        # Wait for process to complete (if not already)
        try:
            await asyncio.wait_for(process.wait(), timeout=5)
        except asyncio.TimeoutError:
            # Force kill if still running
            await self._kill_process(process)

        # Truncate output to last N lines
        stdout_lines = stdout_lines[-MAX_OUTPUT_LINES:]
        stderr_lines = stderr_lines[-MAX_OUTPUT_LINES:]

        # Build output
        stdout = "\n".join(stdout_lines) if stdout_lines else None
        stderr = "\n".join(stderr_lines) if stderr_lines else None

        return ShellCommandOutput(
            success=process.returncode == 0 and not timed_out,
            command=command,
            stdout=stdout,
            stderr=stderr,
            exit_code=process.returncode,
            timeout=timed_out,
            user_interrupted=False,
        )

    async def _terminate_process(
        self,
        process: asyncio.subprocess.Process,
    ) -> None:
        """Terminate a process gracefully.

        Args:
            process: Process to terminate.
        """
        if process.returncode is not None:
            return

        try:
            if self.IS_WINDOWS:
                process.terminate()
            else:
                # Send SIGTERM to process group
                os.killpg(os.getpgid(process.pid), signal.SIGTERM)
        except (ProcessLookupError, OSError):
            pass

    async def _kill_process(
        self,
        process: asyncio.subprocess.Process,
    ) -> None:
        """Force kill a process.

        Args:
            process: Process to kill.
        """
        if process.returncode is not None:
            return

        try:
            if self.IS_WINDOWS:
                process.kill()
            else:
                # Send SIGKILL to process group
                os.killpg(os.getpgid(process.pid), signal.SIGKILL)
        except (ProcessLookupError, OSError):
            pass

    def get_schema(self) -> dict[str, Any]:
        """Get JSON schema for this tool."""
        return {
            "name": self.name,
            "description": self.description,
            "parameters": ShellCommandInput.model_json_schema(),
        }


__all__ = [
    "DEFAULT_TIMEOUT",
    "MAX_LINE_LENGTH",
    "MAX_OUTPUT_LINES",
    "ShellCommandInput",
    "ShellCommandOutput",
    "ShellCommandTool",
]
