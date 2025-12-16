"""System prompt composition for Ticca Agent.

This module handles the composition of system prompts from multiple sources:
- Base system prompt from agent configuration
- Rules from AGENTS.md files (global and project-specific)
- Callback injections from plugins
- Claude-Code prompt rewriting (when using claude-code models)

The prompt composition follows this order:
1. System Prompt (from agent)
2. Rules (from AGENTS.md files)
3. Callback Injections (from plugins)

Example:
    >>> from ticca_agent.core.prompts import PromptComposer
    >>> composer = PromptComposer()
    >>> final_prompt = await composer.compose(
    ...     base_prompt="You are a helpful assistant.",
    ...     cwd="/path/to/project",
    ... )

"""

from __future__ import annotations

import os
from pathlib import Path
from typing import TYPE_CHECKING, Protocol, runtime_checkable

from pydantic import BaseModel, Field

if TYPE_CHECKING:
    from collections.abc import Awaitable, Callable

# Claude-Code fixed instruction string
CLAUDE_CODE_INSTRUCTION = "You are Claude Code, Anthropic's official CLI for Claude."

# Default rule file names to search for
RULE_FILE_NAMES: tuple[str, ...] = ("AGENTS.md", "AGENT.md")


class PromptConfig(BaseModel):
    """Configuration for prompt composition.

    Attributes:
        app_config_dir: Application configuration directory (~/.ticca_desktop/).
        enable_rules: Whether to load rules from AGENTS.md files.
        enable_injections: Whether to allow callback injections.
        max_rules_length: Maximum characters to include from rules files.
    """

    app_config_dir: str | None = Field(
        default=None,
        description="Application configuration directory",
    )
    enable_rules: bool = Field(
        default=True,
        description="Whether to load rules from AGENTS.md files",
    )
    enable_injections: bool = Field(
        default=True,
        description="Whether to allow callback injections",
    )
    max_rules_length: int = Field(
        default=50000,
        ge=0,
        description="Maximum characters to include from rules files",
    )

    model_config = {"frozen": True}


@runtime_checkable
class PromptInjectionCallback(Protocol):
    """Protocol for prompt injection callbacks.

    Plugins can implement this to inject additional content into prompts.
    """

    def __call__(
        self,
        base_prompt: str,
        agent_name: str,
    ) -> str | Awaitable[str]:
        """Inject content into the prompt.

        Args:
            base_prompt: The current prompt being composed.
            agent_name: Name of the agent receiving the prompt.

        Returns:
            The modified prompt with injected content.
        """
        ...


class RulesContent(BaseModel):
    """Content loaded from rules files.

    Attributes:
        content: The combined content from rules files.
        sources: List of file paths that were loaded.
        truncated: Whether the content was truncated.
    """

    content: str = Field(default="", description="Combined rules content")
    sources: list[str] = Field(
        default_factory=list,
        description="File paths that were loaded",
    )
    truncated: bool = Field(
        default=False,
        description="Whether content was truncated",
    )


class ComposedPrompt(BaseModel):
    """Result of prompt composition.

    Attributes:
        system_prompt: The final composed system prompt.
        instructions: The instructions field for the agent.
        requires_rewrite: Whether Claude-Code rewriting is needed.
        original_prompt: The original base prompt before composition.
        rules_loaded: Information about loaded rules.
    """

    system_prompt: str = Field(description="Final composed system prompt")
    instructions: str = Field(description="Instructions field for the agent")
    requires_rewrite: bool = Field(
        default=False,
        description="Whether Claude-Code rewriting is needed",
    )
    original_prompt: str = Field(
        default="",
        description="Original base prompt before composition",
    )
    rules_loaded: RulesContent = Field(
        default_factory=RulesContent,
        description="Information about loaded rules",
    )

    model_config = {"frozen": True}


class PromptComposer:
    """Composes system prompts from multiple sources.

    The composer handles:
    - Loading and combining rules from AGENTS.md files
    - Applying callback injections from plugins
    - Claude-Code prompt rewriting for claude-code models

    Example:
        >>> composer = PromptComposer(config=PromptConfig())
        >>> result = await composer.compose(
        ...     base_prompt="You are a coding assistant.",
        ...     agent_name="code-agent",
        ...     model_name="claude-code-sonnet",
        ... )
        >>> print(result.requires_rewrite)  # True for claude-code models
    """

    def __init__(
        self,
        config: PromptConfig | None = None,
    ) -> None:
        """Initialize the prompt composer.

        Args:
            config: Configuration for prompt composition.
        """
        self._config = config or PromptConfig()
        self._injection_callbacks: list[PromptInjectionCallback] = []
        self._rules_cache: dict[str, RulesContent] = {}

    @property
    def config(self) -> PromptConfig:
        """Get the composer configuration."""
        return self._config

    def register_injection(
        self,
        callback: PromptInjectionCallback,
    ) -> None:
        """Register a prompt injection callback.

        Args:
            callback: Callback to inject content into prompts.
        """
        self._injection_callbacks.append(callback)

    def unregister_injection(
        self,
        callback: PromptInjectionCallback,
    ) -> bool:
        """Unregister a prompt injection callback.

        Args:
            callback: Callback to remove.

        Returns:
            True if callback was found and removed.
        """
        try:
            self._injection_callbacks.remove(callback)
            return True
        except ValueError:
            return False

    def _find_rule_files(self, directory: str | Path) -> list[Path]:
        """Find rules files in a directory.

        Args:
            directory: Directory to search.

        Returns:
            List of found rule file paths.
        """
        directory = Path(directory)
        if not directory.exists():
            return []

        found: list[Path] = []
        for name in RULE_FILE_NAMES:
            path = directory / name
            if path.is_file():
                found.append(path)
                break  # Use first found (AGENTS.md before AGENT.md)

        return found

    def _load_rule_file(self, path: Path) -> str:
        """Load content from a rules file.

        Args:
            path: Path to the rules file.

        Returns:
            File content or empty string if loading fails.
        """
        try:
            content = path.read_text(encoding="utf-8")
            return content.strip()
        except (OSError, UnicodeDecodeError):
            return ""

    def load_rules(
        self,
        cwd: str | Path | None = None,
    ) -> RulesContent:
        """Load rules from AGENTS.md files.

        Rules are loaded from:
        1. Global: app_config_dir/AGENTS.md
        2. Project: cwd/AGENTS.md

        Args:
            cwd: Current working directory for project rules.

        Returns:
            Combined rules content.
        """
        if not self._config.enable_rules:
            return RulesContent()

        # Generate cache key
        cache_key = f"{self._config.app_config_dir or ''}:{cwd or ''}"
        if cache_key in self._rules_cache:
            return self._rules_cache[cache_key]

        sources: list[str] = []
        contents: list[str] = []

        # Load global rules from app config dir
        if self._config.app_config_dir:
            global_files = self._find_rule_files(self._config.app_config_dir)
            for path in global_files:
                content = self._load_rule_file(path)
                if content:
                    contents.append(f"# Global Rules\n\n{content}")
                    sources.append(str(path))

        # Load project rules from cwd
        if cwd:
            project_files = self._find_rule_files(cwd)
            for path in project_files:
                content = self._load_rule_file(path)
                if content:
                    contents.append(f"# Project Rules\n\n{content}")
                    sources.append(str(path))

        # Combine and truncate if needed
        combined = "\n\n".join(contents)
        truncated = False
        if len(combined) > self._config.max_rules_length:
            combined = combined[: self._config.max_rules_length]
            truncated = True

        result = RulesContent(
            content=combined,
            sources=sources,
            truncated=truncated,
        )

        self._rules_cache[cache_key] = result
        return result

    async def apply_injections(
        self,
        prompt: str,
        agent_name: str,
    ) -> str:
        """Apply all registered injection callbacks.

        Args:
            prompt: Current prompt to inject into.
            agent_name: Name of the agent.

        Returns:
            Prompt with all injections applied.
        """
        if not self._config.enable_injections:
            return prompt

        import asyncio

        result = prompt
        for callback in self._injection_callbacks:
            try:
                injection_result = callback(result, agent_name)
                if asyncio.iscoroutine(injection_result):
                    result = await injection_result
                else:
                    result = injection_result  # type: ignore[assignment]
            except Exception:  # noqa: BLE001
                # Continue with other callbacks if one fails
                continue

        return result

    async def compose(
        self,
        base_prompt: str,
        *,
        agent_name: str = "agent",
        model_name: str | None = None,
        cwd: str | Path | None = None,
    ) -> ComposedPrompt:
        """Compose the final system prompt.

        This method:
        1. Starts with the base prompt
        2. Appends rules from AGENTS.md files
        3. Applies injection callbacks
        4. Detects if Claude-Code rewriting is needed

        Args:
            base_prompt: Base system prompt from agent.
            agent_name: Name of the agent.
            model_name: Model identifier (for Claude-Code detection).
            cwd: Current working directory for project rules.

        Returns:
            ComposedPrompt with final prompt and metadata.
        """
        # Load rules
        rules = self.load_rules(cwd=cwd)

        # Combine base prompt with rules
        parts = [base_prompt]
        if rules.content:
            parts.append(rules.content)

        combined = "\n\n".join(filter(None, parts))

        # Apply injection callbacks
        final_prompt = await self.apply_injections(combined, agent_name)

        # Check if Claude-Code rewriting is needed
        requires_rewrite = self._is_claude_code_model(model_name)

        # Determine instructions
        instructions = CLAUDE_CODE_INSTRUCTION if requires_rewrite else final_prompt

        return ComposedPrompt(
            system_prompt=final_prompt,
            instructions=instructions,
            requires_rewrite=requires_rewrite,
            original_prompt=base_prompt,
            rules_loaded=rules,
        )

    def _is_claude_code_model(self, model_name: str | None) -> bool:
        """Check if the model requires Claude-Code rewriting.

        Args:
            model_name: Model identifier.

        Returns:
            True if the model name starts with 'claude-code'.
        """
        if model_name is None:
            return False
        return model_name.lower().startswith("claude-code")

    def rewrite_for_claude_code(
        self,
        composed: ComposedPrompt,
        user_message: str,
    ) -> tuple[str, str]:
        """Rewrite prompt and message for Claude-Code models.

        Claude-Code models don't support custom system prompts, so we:
        1. Prepend the system prompt to the first user message
        2. Replace instructions with the fixed Claude-Code string

        Args:
            composed: The composed prompt.
            user_message: The user's first message.

        Returns:
            Tuple of (instructions, modified_user_message).
        """
        if not composed.requires_rewrite:
            return composed.instructions, user_message

        # Prepend system prompt to user message
        modified_message = f"{composed.system_prompt}\n\n{user_message}"

        return CLAUDE_CODE_INSTRUCTION, modified_message

    def clear_cache(self) -> None:
        """Clear the rules cache."""
        self._rules_cache.clear()


# Convenience function for quick prompt composition
async def compose_prompt(
    base_prompt: str,
    *,
    agent_name: str = "agent",
    model_name: str | None = None,
    cwd: str | Path | None = None,
    app_config_dir: str | None = None,
) -> ComposedPrompt:
    """Compose a system prompt (convenience function).

    Args:
        base_prompt: Base system prompt.
        agent_name: Agent name.
        model_name: Model identifier.
        cwd: Current working directory.
        app_config_dir: Application config directory.

    Returns:
        Composed prompt.
    """
    # Use default app config dir if not provided
    if app_config_dir is None:
        app_config_dir = str(Path.home() / ".ticca_desktop")

    config = PromptConfig(app_config_dir=app_config_dir)
    composer = PromptComposer(config=config)

    return await composer.compose(
        base_prompt=base_prompt,
        agent_name=agent_name,
        model_name=model_name,
        cwd=cwd or os.getcwd(),
    )


__all__ = [
    "CLAUDE_CODE_INSTRUCTION",
    "ComposedPrompt",
    "PromptComposer",
    "PromptConfig",
    "PromptInjectionCallback",
    "RULE_FILE_NAMES",
    "RulesContent",
    "compose_prompt",
]
