"""Claude Code OAuth implementation.

Implements PKCE-based OAuth 2.0 flow for Claude Code authentication.

Configuration values from FEATURES.md §1.2:
- Auth URL: https://claude.ai/oauth/authorize
- Token URL: https://console.anthropic.com/v1/oauth/token
- Client ID: 9d1c250a-e61b-44d9-88ed-5944d1962f5e
- Scope: org:create_api_key user:profile user:inference
- Port Range: 8765-8795
- Callback Timeout: 180 seconds
"""

from __future__ import annotations

import asyncio
import base64
import hashlib
import secrets
import webbrowser
from dataclasses import dataclass
from datetime import UTC, datetime, timedelta
from urllib.parse import parse_qs, urlencode, urlparse

import httpx
import structlog
from pydantic import BaseModel, Field, SecretStr

from ticca_agent.providers.base import ProviderError

logger = structlog.get_logger(__name__)

# Claude Code OAuth configuration constants
CLAUDE_CODE_AUTH_URL = "https://claude.ai/oauth/authorize"
CLAUDE_CODE_TOKEN_URL = "https://console.anthropic.com/v1/oauth/token"
CLAUDE_CODE_API_BASE = "https://api.anthropic.com"
CLAUDE_CODE_CLIENT_ID = "9d1c250a-e61b-44d9-88ed-5944d1962f5e"
CLAUDE_CODE_SCOPE = "org:create_api_key user:profile user:inference"
CLAUDE_CODE_REDIRECT_PATH = "callback"
CLAUDE_CODE_PORT_MIN = 8765
CLAUDE_CODE_PORT_MAX = 8795
CLAUDE_CODE_CALLBACK_TIMEOUT = 180  # seconds


class ClaudeCodeOAuthConfig(BaseModel):
    """Claude Code OAuth configuration.

    Attributes:
        auth_url: Authorization URL.
        token_url: Token exchange URL.
        client_id: OAuth client ID.
        scope: OAuth scopes.
        port_min: Minimum callback port.
        port_max: Maximum callback port.
        callback_timeout: Timeout for callback in seconds.
    """

    auth_url: str = Field(
        default=CLAUDE_CODE_AUTH_URL,
        description="OAuth authorization URL",
    )
    token_url: str = Field(
        default=CLAUDE_CODE_TOKEN_URL,
        description="Token exchange URL",
    )
    client_id: str = Field(
        default=CLAUDE_CODE_CLIENT_ID,
        description="OAuth client ID",
    )
    scope: str = Field(
        default=CLAUDE_CODE_SCOPE,
        description="OAuth scopes",
    )
    port_min: int = Field(
        default=CLAUDE_CODE_PORT_MIN,
        description="Minimum callback port",
    )
    port_max: int = Field(
        default=CLAUDE_CODE_PORT_MAX,
        description="Maximum callback port",
    )
    callback_timeout: int = Field(
        default=CLAUDE_CODE_CALLBACK_TIMEOUT,
        description="Callback timeout in seconds",
    )

    model_config = {"frozen": True}


@dataclass
class OAuthTokens:
    """OAuth token storage.

    Attributes:
        access_token: The access token for API calls (stored securely).
        refresh_token: Token for refreshing access (stored securely).
        id_token: OpenID Connect ID token.
        expires_at: When the access token expires.
        token_type: Type of token (usually "Bearer").
    """

    access_token: SecretStr
    refresh_token: SecretStr | None = None
    id_token: str | None = None
    expires_at: datetime | None = None
    token_type: str = "Bearer"

    def __repr__(self) -> str:
        """Redact sensitive tokens in repr output."""
        refresh_str = "'***'" if self.refresh_token else "None"
        return (
            f"OAuthTokens(access_token='***', refresh_token={refresh_str}, "
            f"expires_at={self.expires_at!r}, token_type={self.token_type!r})"
        )

    @property
    def is_expired(self) -> bool:
        """Check if the access token is expired."""
        if self.expires_at is None:
            return False
        # Consider expired 5 minutes before actual expiry
        now = datetime.now(UTC).replace(tzinfo=None)  # Compare naive datetimes
        return now >= (self.expires_at - timedelta(minutes=5))

    def get_access_token(self) -> str:
        """Get the access token value."""
        return self.access_token.get_secret_value()

    def get_refresh_token(self) -> str | None:
        """Get the refresh token value."""
        if self.refresh_token is None:
            return None
        return self.refresh_token.get_secret_value()


@dataclass
class PKCEChallenge:
    """PKCE code verifier and challenge.

    Attributes:
        verifier: The code verifier (kept secret).
        challenge: The challenge (sent to auth server).
        method: Challenge method (always S256).
    """

    verifier: str
    challenge: str
    method: str = "S256"

    @classmethod
    def generate(cls) -> PKCEChallenge:
        """Generate a new PKCE challenge.

        Returns:
            New PKCEChallenge instance.
        """
        # Generate 32-byte random verifier
        verifier = secrets.token_urlsafe(32)

        # Create SHA256 challenge
        digest = hashlib.sha256(verifier.encode()).digest()
        challenge = base64.urlsafe_b64encode(digest).rstrip(b"=").decode()

        return cls(verifier=verifier, challenge=challenge)


class ClaudeCodeOAuth:
    """Claude Code OAuth 2.0 PKCE flow implementation.

    Handles the complete OAuth flow:
    1. Generate PKCE challenge
    2. Start local callback server
    3. Open browser for authorization
    4. Handle callback and exchange code for tokens
    5. Store and refresh tokens

    Example:
        >>> oauth = ClaudeCodeOAuth()
        >>> tokens = await oauth.authenticate()
        >>> # Use tokens.access_token for API calls
    """

    def __init__(
        self,
        config: ClaudeCodeOAuthConfig | None = None,
    ) -> None:
        """Initialize Claude Code OAuth.

        Args:
            config: OAuth configuration.
        """
        self.config = config or ClaudeCodeOAuthConfig()
        self._tokens: OAuthTokens | None = None
        self._state: str | None = None
        self._pkce: PKCEChallenge | None = None

    @property
    def tokens(self) -> OAuthTokens | None:
        """Get current tokens."""
        return self._tokens

    @property
    def is_authenticated(self) -> bool:
        """Check if we have valid (non-expired) tokens."""
        return self._tokens is not None and not self._tokens.is_expired

    def _generate_state(self) -> str:
        """Generate a random state parameter."""
        return secrets.token_urlsafe(32)

    def _build_auth_url(self, redirect_uri: str) -> str:
        """Build the authorization URL.

        Args:
            redirect_uri: Callback URL.

        Returns:
            Full authorization URL with parameters.
        """
        self._state = self._generate_state()
        self._pkce = PKCEChallenge.generate()

        params = {
            "response_type": "code",
            "client_id": self.config.client_id,
            "redirect_uri": redirect_uri,
            "scope": self.config.scope,
            "state": self._state,
            "code_challenge": self._pkce.challenge,
            "code_challenge_method": self._pkce.method,
        }

        return f"{self.config.auth_url}?{urlencode(params)}"

    async def _find_available_port(self) -> int:
        """Find an available port in the configured range.

        Returns:
            Available port number.

        Raises:
            ProviderError: If no ports are available.
        """
        import socket

        for port in range(self.config.port_min, self.config.port_max + 1):
            try:
                sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                sock.settimeout(1)
                sock.bind(("localhost", port))
                sock.close()
                return port
            except OSError:
                continue

        raise ProviderError(
            f"No available ports in range {self.config.port_min}-{self.config.port_max}"
        )

    async def _exchange_code_for_tokens(
        self,
        code: str,
        redirect_uri: str,
    ) -> OAuthTokens:
        """Exchange authorization code for tokens.

        Args:
            code: Authorization code from callback.
            redirect_uri: The redirect URI used in authorization.

        Returns:
            OAuth tokens.

        Raises:
            ProviderError: If token exchange fails.
        """
        if not self._pkce:
            raise ProviderError("PKCE challenge not initialized")

        async with httpx.AsyncClient(timeout=30.0) as client:
            response = await client.post(
                self.config.token_url,
                data={
                    "grant_type": "authorization_code",
                    "client_id": self.config.client_id,
                    "code": code,
                    "redirect_uri": redirect_uri,
                    "code_verifier": self._pkce.verifier,
                },
                headers={
                    "Content-Type": "application/x-www-form-urlencoded",
                },
            )

            if response.status_code != 200:
                error_data = response.json() if response.content else {}
                error_msg = error_data.get("error_description", response.text)
                raise ProviderError(
                    f"Token exchange failed: {error_msg}",
                    status_code=response.status_code,
                )

            data = response.json()

            expires_in = data.get("expires_in")
            expires_at = None
            if expires_in:
                expires_at = datetime.now(UTC).replace(tzinfo=None) + timedelta(seconds=expires_in)

            refresh_token = data.get("refresh_token")
            return OAuthTokens(
                access_token=SecretStr(data["access_token"]),
                refresh_token=SecretStr(refresh_token) if refresh_token else None,
                id_token=data.get("id_token"),
                expires_at=expires_at,
                token_type=data.get("token_type", "Bearer"),
            )

    async def refresh_tokens(self) -> OAuthTokens:
        """Refresh the access token using the refresh token.

        Returns:
            New OAuth tokens.

        Raises:
            ProviderError: If refresh fails or no refresh token available.
        """
        if not self._tokens or not self._tokens.refresh_token:
            raise ProviderError("No refresh token available")

        async with httpx.AsyncClient(timeout=30.0) as client:
            response = await client.post(
                self.config.token_url,
                data={
                    "grant_type": "refresh_token",
                    "client_id": self.config.client_id,
                    "refresh_token": self._tokens.get_refresh_token(),
                },
                headers={
                    "Content-Type": "application/x-www-form-urlencoded",
                },
            )

            if response.status_code != 200:
                error_data = response.json() if response.content else {}
                error_msg = error_data.get("error_description", response.text)
                raise ProviderError(
                    f"Token refresh failed: {error_msg}",
                    status_code=response.status_code,
                )

            data = response.json()

            expires_in = data.get("expires_in")
            expires_at = None
            if expires_in:
                expires_at = datetime.now(UTC).replace(tzinfo=None) + timedelta(seconds=expires_in)

            new_refresh = data.get("refresh_token")
            refresh_token = (
                SecretStr(new_refresh) if new_refresh
                else self._tokens.refresh_token
            )
            self._tokens = OAuthTokens(
                access_token=SecretStr(data["access_token"]),
                refresh_token=refresh_token,
                id_token=data.get("id_token"),
                expires_at=expires_at,
                token_type=data.get("token_type", "Bearer"),
            )

            logger.info("claude_code_tokens_refreshed")
            return self._tokens

    async def authenticate(
        self,
        *,
        open_browser: bool = True,
    ) -> OAuthTokens:
        """Perform the full OAuth authentication flow.

        This will:
        1. Find an available port
        2. Start a local callback server
        3. Open the browser for authorization
        4. Wait for the callback
        5. Exchange the code for tokens

        Args:
            open_browser: Whether to automatically open the browser.

        Returns:
            OAuth tokens.

        Raises:
            ProviderError: If authentication fails.
        """
        # Find available port
        port = await self._find_available_port()
        redirect_uri = f"http://localhost:{port}/{CLAUDE_CODE_REDIRECT_PATH}"

        # Build auth URL
        auth_url = self._build_auth_url(redirect_uri)

        # Create callback future
        callback_received: asyncio.Future[dict[str, str]] = asyncio.Future()

        # Simple HTTP server for callback
        async def handle_callback(
            reader: asyncio.StreamReader,
            writer: asyncio.StreamWriter,
        ) -> None:
            try:
                # Read request
                request_line = await reader.readline()
                request_str = request_line.decode()

                # Parse GET request
                if request_str.startswith("GET"):
                    path = request_str.split()[1]
                    parsed = urlparse(path)

                    if parsed.path == f"/{CLAUDE_CODE_REDIRECT_PATH}":
                        params = parse_qs(parsed.query)

                        # Extract single values from lists
                        result = {k: v[0] if v else "" for k, v in params.items()}

                        if not callback_received.done():
                            callback_received.set_result(result)

                        # Send success response
                        response = (
                            "HTTP/1.1 200 OK\r\n"
                            "Content-Type: text/html\r\n"
                            "\r\n"
                            "<html><body>"
                            "<h1>Authentication Successful!</h1>"
                            "<p>You can close this window and return to the application.</p>"
                            "</body></html>"
                        )
                    else:
                        response = "HTTP/1.1 404 Not Found\r\n\r\n"

                    writer.write(response.encode())
                    await writer.drain()

            except Exception as e:
                logger.error("oauth_callback_error", error=str(e))
            finally:
                writer.close()
                await writer.wait_closed()

        # Start server
        server = await asyncio.start_server(
            handle_callback,
            "localhost",
            port,
        )

        try:
            logger.info(
                "claude_code_oauth_started",
                port=port,
                redirect_uri=redirect_uri,
            )

            # Open browser
            if open_browser:
                logger.info("opening_browser", url=auth_url)
                webbrowser.open(auth_url)
            else:
                logger.info(
                    "manual_auth_required",
                    url=auth_url,
                    message="Please open this URL in your browser to authenticate",
                )

            # Wait for callback
            try:
                params = await asyncio.wait_for(
                    callback_received,
                    timeout=self.config.callback_timeout,
                )
            except TimeoutError as e:
                raise ProviderError(
                    f"OAuth callback timed out after {self.config.callback_timeout} seconds"
                ) from e

            # Validate state
            if params.get("state") != self._state:
                raise ProviderError("OAuth state mismatch - possible CSRF attack")

            # Check for errors
            if "error" in params:
                error_desc = params.get("error_description", params["error"])
                raise ProviderError(f"OAuth error: {error_desc}")

            # Get authorization code
            code = params.get("code")
            if not code:
                raise ProviderError("No authorization code in callback")

            # Exchange code for tokens
            self._tokens = await self._exchange_code_for_tokens(code, redirect_uri)

            logger.info("claude_code_oauth_complete")
            return self._tokens

        finally:
            server.close()
            await server.wait_closed()

    async def ensure_valid_token(self) -> OAuthTokens:
        """Ensure we have a valid (non-expired) access token.

        Will refresh if the current token is expired.

        Returns:
            Valid OAuth tokens.

        Raises:
            ProviderError: If no tokens and can't authenticate.
        """
        if not self._tokens:
            raise ProviderError("Not authenticated - call authenticate() first")

        if self._tokens.is_expired:
            logger.info("claude_code_token_expired_refreshing")
            return await self.refresh_tokens()

        return self._tokens

    def set_tokens(self, tokens: OAuthTokens) -> None:
        """Set tokens from external source (e.g., loaded from storage).

        Args:
            tokens: OAuth tokens to set.
        """
        self._tokens = tokens

    def clear_tokens(self) -> None:
        """Clear stored tokens (logout)."""
        self._tokens = None
        self._state = None
        self._pkce = None
