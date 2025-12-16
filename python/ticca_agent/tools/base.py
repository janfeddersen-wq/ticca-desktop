"""Base classes for tool implementations.

This module defines the abstract interface and base functionality for all tools
in the Ticca Agent system. Tools extend agent capabilities by providing
access to external systems, file operations, code execution, and more.

Tools follow PydanticAI's function-calling conventions and support:
- Schema validation via Pydantic models
- Async execution for I/O-bound operations
- Contextual execution with access to RunContext
- Retry logic for transient failures

Example:
    >>> from ticca_agent.tools.base import BaseTool, ToolConfig
    >>>
    >>> class FileReadTool(BaseTool):
    ...     async def execute(self, path: str) -> str:
    ...         async with aiofiles.open(path) as f:
    ...             return await f.read()

"""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Any, Generic, TypeVar

from pydantic import BaseModel, Field

if TYPE_CHECKING:
    from ticca_agent.core.context import RunContext

# Type variables for generic tool input/output
InputT = TypeVar("InputT", bound=BaseModel)
OutputT = TypeVar("OutputT")


class ToolConfig(BaseModel):
    """Configuration for a tool.

    Attributes:
        name: Unique tool identifier.
        description: Human-readable description for the AI model.
        enabled: Whether the tool is currently enabled.
        requires_confirmation: Whether to require user confirmation.
        timeout: Maximum execution time in seconds.
        max_retries: Maximum retry attempts on failure.
        rate_limit: Maximum calls per minute (0 = unlimited).
    """

    name: str = Field(
        min_length=1,
        max_length=64,
        pattern=r"^[a-z][a-z0-9_]*$",
        description="Unique tool identifier (snake_case)",
    )
    description: str = Field(
        min_length=1,
        max_length=1024,
        description="Human-readable description for the AI model",
    )
    enabled: bool = Field(
        default=True,
        description="Whether the tool is currently enabled",
    )
    requires_confirmation: bool = Field(
        default=False,
        description="Whether to require user confirmation before execution",
    )
    timeout: float = Field(
        default=30.0,
        ge=1.0,
        le=300.0,
        description="Maximum execution time in seconds",
    )
    max_retries: int = Field(
        default=3,
        ge=0,
        le=10,
        description="Maximum retry attempts on failure",
    )
    rate_limit: int = Field(
        default=0,
        ge=0,
        description="Maximum calls per minute (0 = unlimited)",
    )

    model_config = {"frozen": True}


class ToolResult(BaseModel, Generic[OutputT]):
    """Result from a tool execution.

    Attributes:
        success: Whether the execution succeeded.
        output: The tool's output (if successful).
        error: Error message (if failed).
        execution_time_ms: Execution time in milliseconds.
        metadata: Additional result metadata.
    """

    success: bool = Field(description="Whether the execution succeeded")
    output: OutputT | None = Field(
        default=None,
        description="Tool output if successful",
    )
    error: str | None = Field(
        default=None,
        description="Error message if failed",
    )
    execution_time_ms: int = Field(
        default=0,
        ge=0,
        description="Execution time in milliseconds",
    )
    metadata: dict[str, Any] = Field(
        default_factory=dict,
        description="Additional result metadata",
    )


class BaseTool(ABC, Generic[InputT, OutputT]):
    """Abstract base class for tool implementations.

    All tools must inherit from this class and implement the execute method.
    The base class provides configuration management, validation, and
    common functionality.

    Type Parameters:
        InputT: Pydantic model for tool input validation.
        OutputT: Type of the tool's output.

    Attributes:
        config: Tool configuration.

    Example:
        >>> class CalculatorInput(BaseModel):
        ...     expression: str
        >>>
        >>> class CalculatorTool(BaseTool[CalculatorInput, float]):
        ...     async def execute(
        ...         self,
        ...         input_data: CalculatorInput,
        ...         context: RunContext | None = None,
        ...     ) -> float:
        ...         # Safe evaluation here
        ...         return result
    """

    def __init__(self, config: ToolConfig) -> None:
        """Initialize the tool.

        Args:
            config: Tool configuration.
        """
        self._config = config
        self._call_count: int = 0
        self._error_count: int = 0

        # TODO: Initialize rate limiter if configured
        # TODO: Set up telemetry hooks

    @property
    def name(self) -> str:
        """Get the tool's unique identifier."""
        return self._config.name

    @property
    def description(self) -> str:
        """Get the tool's description."""
        return self._config.description

    @property
    def config(self) -> ToolConfig:
        """Get the tool's configuration."""
        return self._config

    @property
    def is_enabled(self) -> bool:
        """Check if the tool is enabled."""
        return self._config.enabled

    @property
    def call_count(self) -> int:
        """Get the total number of calls to this tool."""
        return self._call_count

    @property
    def error_count(self) -> int:
        """Get the total number of errors from this tool."""
        return self._error_count

    @abstractmethod
    async def execute(
        self,
        input_data: InputT,
        context: RunContext | None = None,
    ) -> OutputT:
        """Execute the tool with the given input.

        This is the main method that subclasses must implement. It should
        perform the tool's action and return the result.

        Args:
            input_data: Validated input data.
            context: Optional run context for access to dependencies.

        Returns:
            The tool's output.

        Raises:
            ToolExecutionError: If execution fails.
        """
        ...

    @abstractmethod
    def get_schema(self) -> dict[str, Any]:
        """Get the JSON schema for this tool's input.

        This schema is used by the AI model to understand how to call
        the tool correctly.

        Returns:
            JSON schema dictionary describing the tool's parameters.
        """
        ...

    async def run(
        self,
        input_data: InputT,
        context: RunContext | None = None,
    ) -> ToolResult[OutputT]:
        """Run the tool with error handling and telemetry.

        This method wraps execute() with error handling, timing,
        and retry logic. Use this instead of calling execute() directly.

        Args:
            input_data: Input data for the tool.
            context: Optional run context.

        Returns:
            ToolResult containing success status and output or error.
        """
        import time

        if not self.is_enabled:
            return ToolResult(
                success=False,
                error=f"Tool '{self.name}' is disabled",
            )

        # TODO: Check rate limit
        # TODO: Request user confirmation if required

        start_time = time.monotonic()
        self._call_count += 1

        try:
            # TODO: Implement retry logic
            # TODO: Implement timeout handling
            output = await self.execute(input_data, context)

            execution_time_ms = int((time.monotonic() - start_time) * 1000)
            return ToolResult(
                success=True,
                output=output,
                execution_time_ms=execution_time_ms,
            )

        except ToolExecutionError as e:
            self._error_count += 1
            execution_time_ms = int((time.monotonic() - start_time) * 1000)
            return ToolResult(
                success=False,
                error=str(e),
                execution_time_ms=execution_time_ms,
            )

        except Exception as e:
            self._error_count += 1
            execution_time_ms = int((time.monotonic() - start_time) * 1000)
            return ToolResult(
                success=False,
                error=f"Unexpected error: {e}",
                execution_time_ms=execution_time_ms,
            )

    def __repr__(self) -> str:
        """Return string representation of the tool."""
        return (
            f"{self.__class__.__name__}("
            f"name={self.name!r}, "
            f"enabled={self.is_enabled})"
        )


class ToolExecutionError(Exception):
    """Exception raised when tool execution fails.

    Attributes:
        message: Human-readable error description.
        tool_name: Name of the tool that failed.
        retryable: Whether the error is safe to retry.
    """

    def __init__(
        self,
        message: str,
        *,
        tool_name: str | None = None,
        retryable: bool = False,
    ) -> None:
        """Initialize ToolExecutionError.

        Args:
            message: Human-readable error description.
            tool_name: Name of the tool that failed.
            retryable: Whether the error is safe to retry.
        """
        super().__init__(message)
        self.message = message
        self.tool_name = tool_name
        self.retryable = retryable

    def __str__(self) -> str:
        """Return string representation of the error."""
        if self.tool_name:
            return f"[{self.tool_name}] {self.message}"
        return self.message


class ToolRegistry:
    """Registry for managing available tools.

    The registry provides tool discovery, registration, and lookup
    functionality for agents.

    Example:
        >>> registry = ToolRegistry()
        >>> registry.register(file_read_tool)
        >>> tool = registry.get("file_read")
    """

    def __init__(self) -> None:
        """Initialize an empty registry."""
        self._tools: dict[str, BaseTool[Any, Any]] = {}

    def register(self, tool: BaseTool[Any, Any]) -> None:
        """Register a tool.

        Args:
            tool: The tool to register.

        Raises:
            ValueError: If a tool with the same name exists.
        """
        if tool.name in self._tools:
            raise ValueError(f"Tool '{tool.name}' is already registered")
        self._tools[tool.name] = tool

    def unregister(self, tool_name: str) -> bool:
        """Unregister a tool by name.

        Args:
            tool_name: Name of the tool to unregister.

        Returns:
            True if found and removed, False otherwise.
        """
        if tool_name in self._tools:
            del self._tools[tool_name]
            return True
        return False

    def get(self, tool_name: str) -> BaseTool[Any, Any] | None:
        """Get a tool by name.

        Args:
            tool_name: Name of the tool.

        Returns:
            The tool if found, None otherwise.
        """
        return self._tools.get(tool_name)

    def list_tools(self) -> list[BaseTool[Any, Any]]:
        """List all registered tools.

        Returns:
            List of all registered tools.
        """
        return list(self._tools.values())

    def get_enabled_tools(self) -> list[BaseTool[Any, Any]]:
        """Get all enabled tools.

        Returns:
            List of enabled tools.
        """
        return [t for t in self._tools.values() if t.is_enabled]

    def get_schemas(self) -> list[dict[str, Any]]:
        """Get JSON schemas for all enabled tools.

        Returns:
            List of tool schemas for AI model consumption.
        """
        return [t.get_schema() for t in self.get_enabled_tools()]

    def __len__(self) -> int:
        """Return the number of registered tools."""
        return len(self._tools)

    def __contains__(self, tool_name: str) -> bool:
        """Check if a tool is registered."""
        return tool_name in self._tools


# Global default registry
default_registry = ToolRegistry()


__all__ = [
    "BaseTool",
    "InputT",
    "OutputT",
    "ToolConfig",
    "ToolExecutionError",
    "ToolRegistry",
    "ToolResult",
    "default_registry",
]
