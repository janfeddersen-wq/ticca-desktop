"""Code review agent for security, performance, and design analysis.

This module provides a specialized agent for comprehensive code review:
- Security vulnerability detection
- Performance bottleneck identification
- Design pattern evaluation
- Code quality assessment
- Best practices enforcement

The CodeReviewerAgent is read-only by design, focusing on analysis
rather than modification.

Example:
    >>> from ticca_agent.agents import CodeReviewerAgent
    >>> agent = CodeReviewerAgent(provider=my_provider)
    >>> result = await agent.review(
    ...     "src/auth.py",
    ...     focus=["security", "performance"]
    ... )

"""

from __future__ import annotations

from enum import Enum
from typing import TYPE_CHECKING, Any, ClassVar

from pydantic import Field

from ticca_agent.core.agent import Agent, AgentConfig

if TYPE_CHECKING:
    from ticca_agent.core.context import RunContext
    from ticca_agent.core.types import AgentResponse
    from ticca_agent.providers.base import BaseProvider
    from ticca_agent.tools.base import BaseTool


class ReviewFocus(str, Enum):
    """Areas to focus on during code review."""

    SECURITY = "security"
    """Security vulnerabilities and best practices."""

    PERFORMANCE = "performance"
    """Performance bottlenecks and optimizations."""

    DESIGN = "design"
    """Architecture and design patterns."""

    QUALITY = "quality"
    """Code quality, readability, maintainability."""

    TESTING = "testing"
    """Test coverage and testing practices."""

    DOCUMENTATION = "documentation"
    """Documentation completeness and accuracy."""

    ALL = "all"
    """Comprehensive review of all areas."""


# Default system prompt for code review tasks
DEFAULT_CODE_REVIEW_SYSTEM_PROMPT = """You are an expert code reviewer with deep expertise in
security, performance optimization, and software design patterns.

Your role is to provide thorough, constructive code reviews that help
improve code quality while maintaining team velocity.

## Review Principles

1. **Be Constructive:** Frame feedback positively, focusing on improvement
2. **Be Specific:** Point to exact lines and provide concrete examples
3. **Explain Why:** Don't just say what's wrong, explain the impact
4. **Prioritize:** Distinguish critical issues from nice-to-haves
5. **Acknowledge Good:** Recognize well-written code and good practices

## Security Review Focus

- SQL injection vulnerabilities
- Cross-site scripting (XSS) risks
- Authentication/authorization flaws
- Sensitive data exposure
- Input validation issues
- Dependency vulnerabilities
- Hardcoded secrets or credentials
- Insecure cryptographic practices
- OWASP Top 10 compliance

## Performance Review Focus

- N+1 query patterns
- Inefficient algorithms (O(n²) when O(n) is possible)
- Memory leaks and excessive allocation
- Blocking operations in async code
- Missing caching opportunities
- Database index usage
- Network call optimization
- Resource cleanup issues

## Design Review Focus

- SOLID principles adherence
- Appropriate abstraction levels
- Clear separation of concerns
- Interface design quality
- Dependency injection patterns
- Error handling strategy
- Code organization and modularity
- API design consistency

## Quality Review Focus

- Code readability and clarity
- Naming conventions
- Comment quality and necessity
- Dead code detection
- Code duplication (DRY violations)
- Complexity metrics (cyclomatic complexity)
- Type safety and annotations
- Test coverage adequacy

## Output Format

Structure your review with:
1. **Summary:** High-level assessment (1-2 sentences)
2. **Critical Issues:** Must-fix problems (security, bugs)
3. **Improvements:** Recommended changes
4. **Suggestions:** Nice-to-have enhancements
5. **Positive Observations:** Good practices to maintain

Available tools:
- `read_file`: Read source files
- `grep`: Search for patterns across files
- `list_files`: Explore project structure
"""


class CodeReviewerConfig(AgentConfig):
    """Configuration specific to the CodeReviewerAgent.

    Extends AgentConfig with review-specific settings.

    Attributes:
        default_focus: Default review focus areas.
        include_line_numbers: Include line numbers in feedback.
        severity_threshold: Minimum severity to report.
        max_file_size_kb: Maximum file size to review.
        check_dependencies: Whether to check for known vulnerabilities.
    """

    default_focus: list[ReviewFocus] = Field(
        default_factory=lambda: [ReviewFocus.ALL],
        description="Default review focus areas",
    )
    include_line_numbers: bool = Field(
        default=True,
        description="Include line numbers in feedback",
    )
    severity_threshold: str = Field(
        default="low",
        description="Minimum severity: critical, high, medium, low",
    )
    max_file_size_kb: int = Field(
        default=500,
        ge=10,
        le=10000,
        description="Maximum file size to review in KB",
    )
    check_dependencies: bool = Field(
        default=True,
        description="Check for known dependency vulnerabilities",
    )

    model_config = {"frozen": True}


class CodeReviewerAgent(Agent):
    """Specialized agent for code review.

    The CodeReviewerAgent is optimized for comprehensive code analysis:
    - Security vulnerability detection
    - Performance issue identification
    - Design pattern evaluation
    - Quality metric assessment

    This agent is read-only and does not modify files.

    Class Attributes:
        DEFAULT_MODEL: Default model for reviews.
        READ_ONLY_TOOLS: Tools that this agent can use (read operations only).

    Example:
        >>> config = CodeReviewerConfig(name="reviewer")
        >>> agent = CodeReviewerAgent(config=config, provider=my_provider)
        >>> result = await agent.review_file(
        ...     "auth/login.py",
        ...     focus=[ReviewFocus.SECURITY],
        ... )
    """

    DEFAULT_MODEL: ClassVar[str | None] = None

    # Read-only tools for code review
    READ_ONLY_TOOLS: ClassVar[list[str]] = [
        "read_file",
        "grep",
        "list_files",
    ]

    def __init__(
        self,
        config: CodeReviewerConfig | None = None,
        provider: BaseProvider | None = None,
        tools: list[BaseTool[Any, Any]] | None = None,
    ) -> None:
        """Initialize the CodeReviewerAgent.

        Args:
            config: Agent configuration (uses defaults if not provided).
            provider: AI provider for generation.
            tools: Optional list of additional tools.
        """
        if config is None:
            config = CodeReviewerConfig(
                name="code-reviewer",
                system_prompt=DEFAULT_CODE_REVIEW_SYSTEM_PROMPT,
            )

        if provider is None:
            raise ValueError("Provider is required for CodeReviewerAgent")

        super().__init__(
            config=config,
            provider=provider,
            tools=tools,
        )

        # TODO: Register read-only tools
        # TODO: Set up security scanning

    @property
    def review_config(self) -> CodeReviewerConfig:
        """Get the review-specific configuration."""
        return self._config  # type: ignore[return-value]

    async def review_file(
        self,
        file_path: str,
        *,
        focus: list[ReviewFocus] | None = None,
        context: RunContext | None = None,
    ) -> AgentResponse:
        """Review a single file.

        Args:
            file_path: Path to the file to review.
            focus: Areas to focus on (uses default if not specified).
            context: Optional run context.

        Returns:
            Response containing the code review.
        """
        # TODO: Read the file
        # TODO: Build review prompt with focus areas
        # TODO: Execute review
        # TODO: Format and return results
        raise NotImplementedError(
            "CodeReviewerAgent.review_file() not yet implemented"
        )

    async def review_diff(
        self,
        diff_content: str,
        *,
        focus: list[ReviewFocus] | None = None,
        context: RunContext | None = None,
    ) -> AgentResponse:
        """Review a diff/patch.

        Args:
            diff_content: The diff content to review.
            focus: Areas to focus on.
            context: Optional run context.

        Returns:
            Response containing the review.
        """
        # TODO: Parse diff content
        # TODO: Build review prompt focused on changes
        # TODO: Execute review
        raise NotImplementedError(
            "CodeReviewerAgent.review_diff() not yet implemented"
        )

    async def review_directory(
        self,
        directory: str,
        *,
        focus: list[ReviewFocus] | None = None,
        file_patterns: list[str] | None = None,
        context: RunContext | None = None,
    ) -> AgentResponse:
        """Review all matching files in a directory.

        Args:
            directory: Directory to review.
            focus: Areas to focus on.
            file_patterns: Glob patterns for files to include.
            context: Optional run context.

        Returns:
            Response containing aggregated review.
        """
        # TODO: List files in directory
        # TODO: Filter by patterns
        # TODO: Review each file
        # TODO: Aggregate results
        raise NotImplementedError(
            "CodeReviewerAgent.review_directory() not yet implemented"
        )

    async def security_scan(
        self,
        path: str,
        *,
        context: RunContext | None = None,
    ) -> AgentResponse:
        """Perform a focused security scan.

        Args:
            path: File or directory to scan.
            context: Optional run context.

        Returns:
            Response containing security findings.
        """
        return await self.review_file(
            path,
            focus=[ReviewFocus.SECURITY],
            context=context,
        )

    async def performance_analysis(
        self,
        path: str,
        *,
        context: RunContext | None = None,
    ) -> AgentResponse:
        """Perform a focused performance analysis.

        Args:
            path: File or directory to analyze.
            context: Optional run context.

        Returns:
            Response containing performance findings.
        """
        return await self.review_file(
            path,
            focus=[ReviewFocus.PERFORMANCE],
            context=context,
        )

    def _build_focus_prompt(self, focus: list[ReviewFocus]) -> str:
        """Build focus-specific prompt additions.

        Args:
            focus: Review focus areas.

        Returns:
            Additional prompt text for focus areas.
        """
        if ReviewFocus.ALL in focus:
            return "Perform a comprehensive review of all aspects."

        focus_prompts = {
            ReviewFocus.SECURITY: "Focus on security vulnerabilities and risks.",
            ReviewFocus.PERFORMANCE: "Focus on performance issues and optimization.",
            ReviewFocus.DESIGN: "Focus on architecture and design patterns.",
            ReviewFocus.QUALITY: "Focus on code quality and maintainability.",
            ReviewFocus.TESTING: "Focus on test coverage and testing practices.",
            ReviewFocus.DOCUMENTATION: "Focus on documentation completeness.",
        }

        parts = [focus_prompts[f] for f in focus if f in focus_prompts]
        return " ".join(parts)


# Factory function for easy agent creation
def create_code_reviewer(
    provider: BaseProvider,
    *,
    name: str = "code-reviewer",
    system_prompt: str | None = None,
    tools: list[BaseTool[Any, Any]] | None = None,
    default_focus: list[ReviewFocus] | None = None,
) -> CodeReviewerAgent:
    """Create a CodeReviewerAgent with common configuration.

    Args:
        provider: AI provider for generation.
        name: Agent name.
        system_prompt: Custom system prompt.
        tools: Additional tools to register.
        default_focus: Default review focus areas.

    Returns:
        Configured CodeReviewerAgent instance.
    """
    config = CodeReviewerConfig(
        name=name,
        system_prompt=system_prompt or DEFAULT_CODE_REVIEW_SYSTEM_PROMPT,
        default_focus=default_focus or [ReviewFocus.ALL],
    )

    return CodeReviewerAgent(
        config=config,
        provider=provider,
        tools=tools,
    )


__all__ = [
    "CodeReviewerAgent",
    "CodeReviewerConfig",
    "DEFAULT_CODE_REVIEW_SYSTEM_PROMPT",
    "ReviewFocus",
    "create_code_reviewer",
]
