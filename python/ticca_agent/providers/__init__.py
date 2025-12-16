"""AI Provider implementations for Ticca Agent.

This module contains abstract base classes and concrete implementations
for various AI providers (OpenAI, Anthropic, Gemini, etc.).

The provider system uses a plugin architecture allowing easy extension
for new AI backends.

Key Components:
- BaseProvider: Abstract base class for all providers
- ProviderConfig: Configuration for provider connections
- ProviderType: Enum of supported provider types
- Provider Registry: Factory functions for creating providers

Supported Providers:
- Anthropic (with cache control injection)
- Claude Code (with prompt rewriting)
- OpenAI (including GPT-5)
- Azure OpenAI
- Google Gemini
- Gemini OAuth (Code Assist)
- OpenRouter (65+ models)
- Cerebras
- Z.AI
- Custom (OpenAI/Anthropic/Gemini compatible)
- Round Robin (load balancing)
"""

from __future__ import annotations

from typing import Any

from ticca_agent.providers.base import (
    AuthenticationError,
    BaseProvider,
    ProviderConfig,
    ProviderError,
    ProviderType,
    RateLimitError,
    RetryConfig,
    expand_env_var,
)

# Type annotations available via lazy imports:
# - AnthropicProvider, AnthropicConfig
# - AzureOpenAIProvider, AzureOpenAIConfig
# - CerebrasProvider
# - ClaudeCodeProvider, ClaudeCodeConfig
# - CustomProvider, CustomConfig, CustomAPIFormat
# - GeminiProvider, GeminiConfig
# - GeminiOAuthProvider, GeminiOAuthConfig
# - OpenAIProvider, OpenAIConfig
# - OpenRouterProvider, OpenRouterConfig
# - RoundRobinProvider, RoundRobinConfig, ModelTarget
# - ZAIProvider, ZAIConfig

# Provider type to class mapping (lazy loaded)
_PROVIDER_CLASSES: dict[ProviderType, type[BaseProvider]] = {}


def _ensure_provider_loaded(provider_type: ProviderType) -> type[BaseProvider]:
    """Lazy load provider class.

    Args:
        provider_type: The provider type to load.

    Returns:
        The provider class.

    Raises:
        ValueError: If provider type is unknown.
    """
    if provider_type in _PROVIDER_CLASSES:
        return _PROVIDER_CLASSES[provider_type]

    # Import on demand to avoid circular imports and speed up initial load
    if provider_type == ProviderType.ANTHROPIC:
        from ticca_agent.providers.anthropic import AnthropicProvider

        _PROVIDER_CLASSES[provider_type] = AnthropicProvider

    elif provider_type == ProviderType.CUSTOM_ANTHROPIC:
        from ticca_agent.providers.custom import CustomProvider

        _PROVIDER_CLASSES[provider_type] = CustomProvider

    elif provider_type == ProviderType.CLAUDE_CODE:
        from ticca_agent.providers.claude_code import ClaudeCodeProvider

        _PROVIDER_CLASSES[provider_type] = ClaudeCodeProvider

    elif provider_type == ProviderType.OPENAI:
        from ticca_agent.providers.openai import OpenAIProvider

        _PROVIDER_CLASSES[provider_type] = OpenAIProvider

    elif provider_type == ProviderType.CUSTOM_OPENAI:
        from ticca_agent.providers.custom import CustomProvider

        _PROVIDER_CLASSES[provider_type] = CustomProvider

    elif provider_type == ProviderType.AZURE_OPENAI:
        from ticca_agent.providers.azure import AzureOpenAIProvider

        _PROVIDER_CLASSES[provider_type] = AzureOpenAIProvider

    elif provider_type == ProviderType.GEMINI:
        from ticca_agent.providers.gemini import GeminiProvider

        _PROVIDER_CLASSES[provider_type] = GeminiProvider

    elif provider_type == ProviderType.CUSTOM_GEMINI:
        from ticca_agent.providers.custom import CustomProvider

        _PROVIDER_CLASSES[provider_type] = CustomProvider

    elif provider_type == ProviderType.GEMINI_OAUTH:
        from ticca_agent.providers.gemini_oauth import GeminiOAuthProvider

        _PROVIDER_CLASSES[provider_type] = GeminiOAuthProvider

    elif provider_type == ProviderType.OPENROUTER:
        from ticca_agent.providers.openrouter import OpenRouterProvider

        _PROVIDER_CLASSES[provider_type] = OpenRouterProvider

    elif provider_type == ProviderType.CEREBRAS:
        from ticca_agent.providers.cerebras import CerebrasProvider

        _PROVIDER_CLASSES[provider_type] = CerebrasProvider

    elif provider_type in (ProviderType.ZAI_CODING, ProviderType.ZAI_API):
        from ticca_agent.providers.zai import ZAIProvider

        _PROVIDER_CLASSES[provider_type] = ZAIProvider

    elif provider_type == ProviderType.ROUND_ROBIN:
        from ticca_agent.providers.round_robin import RoundRobinProvider

        _PROVIDER_CLASSES[provider_type] = RoundRobinProvider

    else:
        raise ValueError(f"Unknown provider type: {provider_type}")

    return _PROVIDER_CLASSES[provider_type]


def create_provider(
    provider_type: ProviderType | str,
    config: ProviderConfig,
    *,
    provider_config: Any = None,
    **kwargs: Any,
) -> BaseProvider:
    """Factory function to create a provider by type.

    Args:
        provider_type: The type of provider to create.
        config: Base provider configuration.
        provider_config: Provider-specific configuration.
        **kwargs: Additional arguments passed to provider constructor.

    Returns:
        Configured provider instance.

    Raises:
        ValueError: If provider type is unknown.

    Example:
        >>> config = ProviderConfig.from_env("ANTHROPIC_API_KEY")
        >>> provider = create_provider(ProviderType.ANTHROPIC, config)
    """
    # Convert string to enum if needed
    if isinstance(provider_type, str):
        provider_type = ProviderType(provider_type)

    provider_class = _ensure_provider_loaded(provider_type)

    # Handle provider-specific config
    if provider_config is not None:
        return provider_class(config, provider_config, **kwargs)  # type: ignore[call-arg]

    return provider_class(config, **kwargs)


def get_provider_class(provider_type: ProviderType | str) -> type[BaseProvider]:
    """Get the provider class for a type without instantiating.

    Args:
        provider_type: The type of provider.

    Returns:
        Provider class.

    Raises:
        ValueError: If provider type is unknown.
    """
    if isinstance(provider_type, str):
        provider_type = ProviderType(provider_type)

    return _ensure_provider_loaded(provider_type)


def list_provider_types() -> list[ProviderType]:
    """List all available provider types.

    Returns:
        List of ProviderType enum values.
    """
    return list(ProviderType)


def get_provider_for_model(model_name: str) -> ProviderType | None:
    """Guess the provider type from a model name.

    Args:
        model_name: Model identifier.

    Returns:
        Likely provider type, or None if unknown.
    """
    model_lower = model_name.lower()

    # Claude Code models
    if model_lower.startswith("claude-code"):
        return ProviderType.CLAUDE_CODE

    # Anthropic models
    if model_lower.startswith("claude"):
        return ProviderType.ANTHROPIC

    # OpenAI models
    if model_lower.startswith(("gpt", "o1", "o3")):
        return ProviderType.OPENAI

    # Gemini models
    if model_lower.startswith("gemini"):
        return ProviderType.GEMINI

    # Cerebras models
    if "llama" in model_lower and "cerebras" in model_lower:
        return ProviderType.CEREBRAS

    # OpenRouter format (provider/model)
    if "/" in model_name:
        return ProviderType.OPENROUTER

    # Z.AI models
    if model_lower.startswith("zai"):
        return ProviderType.ZAI_CODING

    return None


__all__ = [
    "AuthenticationError",
    # Base classes
    "BaseProvider",
    "ProviderConfig",
    "ProviderError",
    "ProviderType",
    "RateLimitError",
    "RetryConfig",
    # Factory functions
    "create_provider",
    # Utilities
    "expand_env_var",
    "get_provider_class",
    "get_provider_for_model",
    "list_provider_types",
]
