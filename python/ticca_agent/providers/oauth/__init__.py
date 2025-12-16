"""OAuth authentication support for AI providers.

This module provides OAuth 2.0 authentication flows for providers
that support OAuth-based authentication.

Currently supported:
- Claude Code OAuth (PKCE flow)
"""

from __future__ import annotations

from ticca_agent.providers.oauth.claude_code import (
    ClaudeCodeOAuth,
    ClaudeCodeOAuthConfig,
    OAuthTokens,
)

__all__ = [
    "ClaudeCodeOAuth",
    "ClaudeCodeOAuthConfig",
    "OAuthTokens",
]
