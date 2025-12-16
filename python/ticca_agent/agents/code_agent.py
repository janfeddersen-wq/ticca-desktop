"""Default code agent implementation.

This module provides a specialized agent for code-related tasks including:
- Code generation and completion
- Code review and analysis
- Refactoring suggestions
- Bug detection and fixing
- Documentation generation

The CodeAgent is optimized for programming tasks with:
- Language-aware system prompts
- Code-specific tool integrations
- Syntax-aware context management
- Best practices enforcement

Example:
    >>> from ticca_agent.agents import CodeAgent
    >>> agent = CodeAgent(provider=my_provider)
    >>> result = await agent.run(
    ...     "Write a Python function to calculate fibonacci numbers"
    ... )

"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, ClassVar

from pydantic import Field

from ticca_agent.core.agent import Agent, AgentConfig

if TYPE_CHECKING:
    from ticca_agent.core.context import RunContext
    from ticca_agent.core.types import AgentResponse
    from ticca_agent.providers.base import BaseProvider
    from ticca_agent.tools.base import BaseTool


# Default system prompt for coding tasks
DEFAULT_CODE_SYSTEM_PROMPT = """You are an expert software engineer with deep knowledge
across multiple programming languages and frameworks. Your responses should:

1. Follow best practices and established patterns for the relevant language/framework
2. Include proper error handling and edge case consideration
3. Be well-documented with clear comments where appropriate
4. Prioritize readability and maintainability
5. Consider performance implications
6. Use idiomatic code style for the language

When writing code:
- Use meaningful variable and function names
- Keep functions focused and single-purpose (SRP)
- Add type hints where the language supports them
- Include docstrings for public APIs
- Handle errors gracefully

When reviewing code:
- Identify potential bugs and security issues
- Suggest improvements for readability and performance
- Point out violations of best practices
- Provide constructive feedback with examples

Always explain your reasoning and provide context for your suggestions.
"""


class CodeAgentConfig(AgentConfig):
    """Configuration specific to the CodeAgent.

    Extends AgentConfig with code-specific settings.

    Attributes:
        preferred_languages: Languages the agent should prioritize.
        enforce_type_hints: Whether to enforce type hints in generated code.
        max_code_block_tokens: Maximum tokens per code block.
        include_tests: Whether to include test suggestions.
        style_guide: Optional style guide to follow.
    """

    preferred_languages: list[str] = Field(
        default_factory=lambda: ["python", "typescript", "rust"],
        description="Languages the agent should prioritize",
    )
    enforce_type_hints: bool = Field(
        default=True,
        description="Whether to enforce type hints in generated code",
    )
    max_code_block_tokens: int = Field(
        default=2000,
        ge=100,
        le=10000,
        description="Maximum tokens per code block",
    )
    include_tests: bool = Field(
        default=True,
        description="Whether to include test suggestions",
    )
    style_guide: str | None = Field(
        default=None,
        description="Optional style guide to follow (e.g., 'pep8', 'google')",
    )

    model_config = {"frozen": True}


class CodeAgent(Agent):
    """Specialized agent for code-related tasks.

    The CodeAgent extends the base Agent with code-specific functionality
    and optimizations for programming tasks.

    Class Attributes:
        SUPPORTED_LANGUAGES: Languages this agent can work with.
        DEFAULT_MODEL: Default model for code generation.

    Example:
        >>> config = CodeAgentConfig(name="code-assistant")
        >>> agent = CodeAgent(config=config, provider=anthropic_provider)
        >>> result = await agent.generate_code(
        ...     "Create a REST API endpoint",
        ...     language="python",
        ...     framework="fastapi",
        ... )
    """

    SUPPORTED_LANGUAGES: ClassVar[list[str]] = [
        "python",
        "typescript",
        "javascript",
        "rust",
        "go",
        "java",
        "c",
        "cpp",
        "csharp",
        "ruby",
        "php",
        "swift",
        "kotlin",
        "scala",
        "sql",
        "html",
        "css",
        "bash",
    ]

    DEFAULT_MODEL: ClassVar[str | None] = None  # Use provider default

    def __init__(
        self,
        config: CodeAgentConfig | None = None,
        provider: BaseProvider | None = None,
        tools: list[BaseTool[Any, Any]] | None = None,
    ) -> None:
        """Initialize the CodeAgent.

        Args:
            config: Agent configuration (uses defaults if not provided).
            provider: AI provider for generation.
            tools: Optional list of additional tools.
        """
        if config is None:
            config = CodeAgentConfig(
                name="code-agent",
                system_prompt=DEFAULT_CODE_SYSTEM_PROMPT,
            )

        # Type narrowing - we need a provider
        if provider is None:
            # TODO: Create a default provider or raise
            raise ValueError("Provider is required for CodeAgent")

        super().__init__(
            config=config,
            provider=provider,
            tools=tools,
        )

        # TODO: Register code-specific tools (file_read, file_write, etc.)
        # TODO: Set up language detection
        # TODO: Configure code-aware tokenization

    @property
    def code_config(self) -> CodeAgentConfig:
        """Get the code-specific configuration."""
        # Safe cast since we control initialization
        return self._config  # type: ignore[return-value]

    async def generate_code(
        self,
        prompt: str,
        *,
        language: str | None = None,
        framework: str | None = None,
        context: RunContext | None = None,
        include_tests: bool | None = None,
    ) -> AgentResponse:
        """Generate code based on a prompt.

        This is a convenience method that constructs an optimized prompt
        for code generation tasks.

        Args:
            prompt: Description of the code to generate.
            language: Target programming language.
            framework: Optional framework to use.
            context: Optional run context.
            include_tests: Whether to include unit tests.

        Returns:
            Response containing generated code.
        """
        # TODO: Build enhanced prompt with language/framework context
        # TODO: Apply code-specific system prompt modifications
        # TODO: Set appropriate model parameters for code generation
        # TODO: Call self.run() with enhanced prompt
        raise NotImplementedError("CodeAgent.generate_code() not yet implemented")

    async def review_code(
        self,
        code: str,
        *,
        language: str | None = None,
        focus_areas: list[str] | None = None,
        context: RunContext | None = None,
    ) -> AgentResponse:
        """Review code and provide feedback.

        Args:
            code: The code to review.
            language: Programming language of the code.
            focus_areas: Specific areas to focus on (security, performance, etc.).
            context: Optional run context.

        Returns:
            Response containing code review feedback.
        """
        # TODO: Detect language if not provided
        # TODO: Build review-focused prompt
        # TODO: Include style guide checks
        # TODO: Call self.run() with review prompt
        raise NotImplementedError("CodeAgent.review_code() not yet implemented")

    async def explain_code(
        self,
        code: str,
        *,
        language: str | None = None,
        detail_level: str = "medium",
        context: RunContext | None = None,
    ) -> AgentResponse:
        """Explain what code does.

        Args:
            code: The code to explain.
            language: Programming language of the code.
            detail_level: Level of detail (brief, medium, detailed).
            context: Optional run context.

        Returns:
            Response containing code explanation.
        """
        # TODO: Build explanation prompt based on detail level
        # TODO: Include context about common patterns
        # TODO: Call self.run() with explanation prompt
        raise NotImplementedError("CodeAgent.explain_code() not yet implemented")

    async def refactor_code(
        self,
        code: str,
        *,
        language: str | None = None,
        goals: list[str] | None = None,
        context: RunContext | None = None,
    ) -> AgentResponse:
        """Suggest refactoring improvements.

        Args:
            code: The code to refactor.
            language: Programming language of the code.
            goals: Refactoring goals (readability, performance, etc.).
            context: Optional run context.

        Returns:
            Response containing refactored code and explanation.
        """
        # TODO: Analyze code structure
        # TODO: Build refactoring prompt with goals
        # TODO: Include before/after comparison
        # TODO: Call self.run() with refactoring prompt
        raise NotImplementedError("CodeAgent.refactor_code() not yet implemented")

    def _detect_language(self, code: str) -> str | None:
        """Attempt to detect the programming language of code.

        Args:
            code: Code snippet to analyze.

        Returns:
            Detected language name or None if unknown.
        """
        # TODO: Implement language detection heuristics
        # TODO: Check for common patterns (shebang, imports, syntax)
        # TODO: Consider using a library like pygments
        return None

    def _build_language_context(self, language: str) -> str:
        """Build language-specific context for prompts.

        Args:
            language: The target programming language.

        Returns:
            Context string with language-specific guidance.
        """
        # TODO: Include language-specific best practices
        # TODO: Add framework conventions if applicable
        # TODO: Include style guide requirements
        return f"Target language: {language}"


# Factory function for easy agent creation
def create_code_agent(
    provider: BaseProvider,
    *,
    name: str = "code-agent",
    system_prompt: str | None = None,
    preferred_languages: list[str] | None = None,
    tools: list[BaseTool[Any, Any]] | None = None,
) -> CodeAgent:
    """Create a CodeAgent with common configuration.

    This factory function provides a convenient way to create a CodeAgent
    with sensible defaults.

    Args:
        provider: AI provider for generation.
        name: Agent name.
        system_prompt: Custom system prompt (uses default if not provided).
        preferred_languages: Languages to prioritize.
        tools: Additional tools to register.

    Returns:
        Configured CodeAgent instance.
    """
    config = CodeAgentConfig(
        name=name,
        system_prompt=system_prompt or DEFAULT_CODE_SYSTEM_PROMPT,
        preferred_languages=preferred_languages
        or ["python", "typescript", "rust"],
    )

    return CodeAgent(
        config=config,
        provider=provider,
        tools=tools,
    )


__all__ = [
    "CodeAgent",
    "CodeAgentConfig",
    "DEFAULT_CODE_SYSTEM_PROMPT",
    "create_code_agent",
]
