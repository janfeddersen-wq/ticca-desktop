//! Custom HTTP client wrapper for OAuth header injection
//!
//! Wraps reqwest::Client to add OAuth headers to all requests.
//! This is necessary because rig's client builders don't expose
//! a public API for adding custom headers to all requests.
//!
//! Also provides `CodexHttpClient` which modifies request bodies for
//! ChatGPT Codex backend compatibility.

use bytes::Bytes;
use http::{HeaderMap, HeaderValue, Request, Response};
use reqwest::Client as ReqwestClient;
use serde_json::Value;
use std::future::Future;

use rig::http_client::{HttpClientExt, LazyBody, MultipartForm, StreamingResponse};

/// A wrapper around reqwest::Client that adds custom headers to all requests
#[derive(Clone, Debug)]
pub struct OAuthHttpClient {
    inner: ReqwestClient,
    extra_headers: HeaderMap,
}

impl Default for OAuthHttpClient {
    fn default() -> Self {
        Self {
            inner: ReqwestClient::new(),
            extra_headers: HeaderMap::new(),
        }
    }
}

impl OAuthHttpClient {
    /// Create a new OAuth HTTP client with the given headers
    pub fn new(extra_headers: HeaderMap) -> Self {
        Self {
            inner: ReqwestClient::new(),
            extra_headers,
        }
    }

    /// Create from an existing reqwest client
    pub fn with_client(client: ReqwestClient, extra_headers: HeaderMap) -> Self {
        Self {
            inner: client,
            extra_headers,
        }
    }

    /// Merge extra headers into the request
    ///
    /// Note: For OAuth-based clients, we also remove the x-api-key header
    /// that rig's providers add by default, since we're using Bearer auth.
    fn merge_headers(&self, mut headers: HeaderMap) -> HeaderMap {
        // Remove x-api-key if present (rig sets this, but we use Bearer auth)
        if headers.remove("x-api-key").is_some() {
            tracing::debug!("Removed x-api-key header in favor of Bearer auth");
        }

        // Add our custom headers
        for (key, value) in self.extra_headers.iter() {
            headers.insert(key.clone(), value.clone());
        }

        // Log all headers for debugging (mask sensitive values)
        for (name, value) in headers.iter() {
            let value_str = if name.as_str().to_lowercase().contains("auth")
                || name.as_str().to_lowercase().contains("key")
                || name.as_str().to_lowercase().contains("token")
            {
                "[REDACTED]"
            } else {
                value.to_str().unwrap_or("[non-utf8]")
            };
            tracing::trace!("Header: {}: {}", name, value_str);
        }

        headers
    }
}

impl HttpClientExt for OAuthHttpClient {
    fn send<T, U>(
        &self,
        req: Request<T>,
    ) -> impl Future<Output = rig::http_client::Result<Response<LazyBody<U>>>> + Send + 'static
    where
        T: Into<Bytes> + Send,
        U: From<Bytes> + Send + 'static,
    {
        let (mut parts, body) = req.into_parts();
        parts.headers = self.merge_headers(parts.headers);

        // Convert body to bytes so we can log it
        let body_bytes: Bytes = body.into();

        tracing::debug!(
            "OAuthHttpClient::send - {} {} (headers: {}, body: {} bytes)",
            parts.method,
            parts.uri,
            parts.headers.len(),
            body_bytes.len()
        );

        // Log the body content for debugging (first 2000 chars to avoid spam)
        if let Ok(body_str) = std::str::from_utf8(&body_bytes) {
            let preview: String = body_str.chars().take(2000).collect();
            tracing::debug!("Request body preview: {}", preview);
        }

        let req = Request::from_parts(parts, body_bytes);

        // Delegate to inner client
        self.inner.send(req)
    }

    fn send_multipart<U>(
        &self,
        req: Request<MultipartForm>,
    ) -> impl Future<Output = rig::http_client::Result<Response<LazyBody<U>>>> + Send + 'static
    where
        U: From<Bytes> + Send + 'static,
    {
        let (mut parts, body) = req.into_parts();
        parts.headers = self.merge_headers(parts.headers);
        let req = Request::from_parts(parts, body);

        tracing::debug!(
            "OAuthHttpClient::send_multipart - {} {} (headers: {})",
            req.method(),
            req.uri(),
            req.headers().len()
        );

        self.inner.send_multipart(req)
    }

    fn send_streaming<T>(
        &self,
        req: Request<T>,
    ) -> impl Future<Output = rig::http_client::Result<StreamingResponse>> + Send
    where
        T: Into<Bytes>,
    {
        let (mut parts, body) = req.into_parts();
        parts.headers = self.merge_headers(parts.headers);

        // Convert body to bytes so we can log it
        let body_bytes: Bytes = body.into();

        tracing::debug!(
            "OAuthHttpClient::send_streaming - {} {} (headers: {}, body: {} bytes)",
            parts.method,
            parts.uri,
            parts.headers.len(),
            body_bytes.len()
        );

        // Log the body content for debugging (first 2000 chars to avoid spam)
        if let Ok(body_str) = std::str::from_utf8(&body_bytes) {
            let preview: String = body_str.chars().take(2000).collect();
            tracing::debug!("Streaming request body preview: {}", preview);
        }

        let req = Request::from_parts(parts, body_bytes);

        self.inner.send_streaming(req)
    }
}

// ============================================================================
// CodexHttpClient - Specialized client for ChatGPT Codex backend
// ============================================================================

/// Codex CLI system prompt
/// Required by the Codex API for gpt-5.x models
/// Source: https://github.com/openai/codex/blob/main/codex-rs/core/prompt.md
const CODEX_INSTRUCTIONS: &str = include_str!("codex_prompt.txt");

/// A wrapper around reqwest::Client that adds Codex-specific headers and
/// modifies request bodies for ChatGPT Codex backend compatibility.
///
/// The Codex backend at `chatgpt.com/backend-api/codex` requires:
/// - Removal of `max_output_tokens` (not supported)
/// - Addition of `store: false` (required)
/// - Removal of system messages from input (they go in instructions)
/// - Specific headers (ChatGPT-Account-ID, originator, etc.)
#[derive(Clone, Debug)]
pub struct CodexHttpClient {
    inner: ReqwestClient,
    extra_headers: HeaderMap,
}

impl Default for CodexHttpClient {
    fn default() -> Self {
        Self {
            inner: ReqwestClient::new(),
            extra_headers: HeaderMap::new(),
        }
    }
}

impl CodexHttpClient {
    /// Create a new Codex HTTP client with the given headers
    pub fn new(extra_headers: HeaderMap) -> Self {
        Self {
            inner: ReqwestClient::new(),
            extra_headers,
        }
    }

    /// Merge extra headers into the request
    fn merge_headers(&self, mut headers: HeaderMap) -> HeaderMap {
        // Remove x-api-key if present (rig sets this, but we use Bearer auth)
        if headers.remove("x-api-key").is_some() {
            tracing::debug!("Removed x-api-key header in favor of Bearer auth");
        }

        // Add our custom headers
        for (key, value) in self.extra_headers.iter() {
            headers.insert(key.clone(), value.clone());
        }

        headers
    }

    /// Modify the request body for Codex compatibility
    ///
    /// The Codex backend requires specific request format:
    /// - Removes `max_output_tokens` (Codex doesn't support it)
    /// - Adds `store: false` (required by Codex)
    /// - Adds `parallel_tool_calls: false` if tools are present
    /// - Overwrites `instructions` field with official Codex prompt
    /// - Removes system messages from input/messages array
    fn modify_body_for_codex(&self, body_bytes: Bytes) -> Bytes {
        // Try to parse as JSON
        let Ok(body_str) = std::str::from_utf8(&body_bytes) else {
            tracing::warn!("Codex: request body is not valid UTF-8");
            return body_bytes;
        };

        // Log the original body for debugging
        tracing::debug!("Codex: ORIGINAL request body: {}", body_str);

        let Ok(mut json) = serde_json::from_str::<Value>(body_str) else {
            tracing::warn!("Codex: request body is not valid JSON");
            return body_bytes;
        };

        if let Some(obj) = json.as_object_mut() {
            // Log what fields are present
            let fields: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
            tracing::debug!("Codex: request has fields: {:?}", fields);

            // Remove unsupported fields
            if obj.remove("max_output_tokens").is_some() {
                tracing::debug!("Codex: removed unsupported max_output_tokens");
            }
            if obj.remove("max_tokens").is_some() {
                tracing::debug!("Codex: removed unsupported max_tokens");
            }
            if obj.remove("temperature").is_some() {
                tracing::debug!("Codex: removed unsupported temperature");
            }

            // Add required fields
            obj.insert("store".to_string(), Value::Bool(false));
            tracing::debug!("Codex: set store=false");

            // If tools are present, disable parallel tool calls
            if obj.contains_key("tools") {
                obj.insert("parallel_tool_calls".to_string(), Value::Bool(false));
                tracing::debug!("Codex: set parallel_tool_calls=false");
            }

            // CRITICAL: Overwrite the instructions field with official Codex prompt
            // Codex API requires this exact prompt for gpt-5.x models.
            if obj.contains_key("instructions") {
                tracing::debug!("Codex: overwriting existing instructions field");
            }
            obj.insert(
                "instructions".to_string(),
                Value::String(CODEX_INSTRUCTIONS.to_string()),
            );
            tracing::debug!(
                "Codex: set instructions to official Codex prompt ({} chars)",
                CODEX_INSTRUCTIONS.len()
            );

            // Remove system messages from `input` array (Responses API format)
            // The Codex API uses the `instructions` field for system prompts, not input messages
            if let Some(Value::Array(input)) = obj.get_mut("input") {
                let original_len = input.len();
                input.retain(|msg| {
                    // Keep messages that don't have role=system
                    if let Some(role) = msg.get("role").and_then(|v| v.as_str())
                        && role == "system"
                    {
                        tracing::debug!("Codex: removing system message from input array");
                        return false;
                    }
                    true
                });
                let removed = original_len - input.len();
                if removed > 0 {
                    tracing::debug!(
                        "Codex: removed {} system message(s) from input array",
                        removed
                    );
                }
            }

            // Also check `messages` array (Chat Completions API format - just in case)
            if let Some(Value::Array(messages)) = obj.get_mut("messages") {
                let original_len = messages.len();
                messages.retain(|msg| {
                    if let Some(role) = msg.get("role").and_then(|v| v.as_str())
                        && role == "system"
                    {
                        tracing::debug!("Codex: removing system message from messages array");
                        return false;
                    }
                    true
                });
                let removed = original_len - messages.len();
                if removed > 0 {
                    tracing::debug!(
                        "Codex: removed {} system message(s) from messages array",
                        removed
                    );
                }
            }
        }

        // Re-serialize
        match serde_json::to_vec(&json) {
            Ok(new_body) => {
                // Log the modified body for debugging
                if let Ok(modified_str) = std::str::from_utf8(&new_body) {
                    tracing::debug!("Codex: MODIFIED request body: {}", modified_str);
                }
                Bytes::from(new_body)
            }
            Err(e) => {
                tracing::warn!("Failed to re-serialize Codex request body: {}", e);
                body_bytes
            }
        }
    }
}

impl HttpClientExt for CodexHttpClient {
    fn send<T, U>(
        &self,
        req: Request<T>,
    ) -> impl Future<Output = rig::http_client::Result<Response<LazyBody<U>>>> + Send + 'static
    where
        T: Into<Bytes> + Send,
        U: From<Bytes> + Send + 'static,
    {
        let (mut parts, body) = req.into_parts();
        parts.headers = self.merge_headers(parts.headers);

        // Convert body to bytes and modify for Codex
        let body_bytes: Bytes = body.into();
        let body_bytes = self.modify_body_for_codex(body_bytes);

        tracing::debug!(
            "CodexHttpClient::send - {} {} (headers: {}, body: {} bytes)",
            parts.method,
            parts.uri,
            parts.headers.len(),
            body_bytes.len()
        );

        let req = Request::from_parts(parts, body_bytes);
        self.inner.send(req)
    }

    fn send_multipart<U>(
        &self,
        req: Request<MultipartForm>,
    ) -> impl Future<Output = rig::http_client::Result<Response<LazyBody<U>>>> + Send + 'static
    where
        U: From<Bytes> + Send + 'static,
    {
        let (mut parts, body) = req.into_parts();
        parts.headers = self.merge_headers(parts.headers);
        let req = Request::from_parts(parts, body);

        tracing::debug!(
            "CodexHttpClient::send_multipart - {} {} (headers: {})",
            req.method(),
            req.uri(),
            req.headers().len()
        );

        self.inner.send_multipart(req)
    }

    fn send_streaming<T>(
        &self,
        req: Request<T>,
    ) -> impl Future<Output = rig::http_client::Result<StreamingResponse>> + Send
    where
        T: Into<Bytes>,
    {
        let (mut parts, body) = req.into_parts();

        // ===== AGGRESSIVE DEBUG LOGGING =====
        tracing::warn!("========================================");
        tracing::warn!("===== CODEX STREAMING REQUEST =====");
        tracing::warn!("========================================");
        tracing::warn!("URL: {} {}", parts.method, parts.uri);

        // Log all headers
        tracing::warn!("--- HEADERS ({}) ---", parts.headers.len());
        for (name, value) in parts.headers.iter() {
            let value_str = if name.as_str().to_lowercase().contains("auth") {
                "[REDACTED]"
            } else {
                value.to_str().unwrap_or("[non-utf8]")
            };
            tracing::warn!("  {}: {}", name, value_str);
        }

        // Convert body to bytes BEFORE modification
        let body_bytes: Bytes = body.into();

        // Log ORIGINAL body
        tracing::warn!("--- ORIGINAL BODY ({} bytes) ---", body_bytes.len());
        if let Ok(body_str) = std::str::from_utf8(&body_bytes) {
            // Check if it has instructions field
            let has_instructions = body_str.contains("\"instructions\"");
            tracing::warn!(
                "Has 'instructions' field BEFORE modification: {}",
                has_instructions
            );
            tracing::warn!("{}", body_str);
        } else {
            tracing::warn!("[Body is not UTF-8]");
        }

        // Merge headers
        parts.headers = self.merge_headers(parts.headers);

        // Modify body for Codex
        let body_bytes = self.modify_body_for_codex(body_bytes);

        // Log MODIFIED body
        tracing::warn!("--- MODIFIED BODY ({} bytes) ---", body_bytes.len());
        if let Ok(body_str) = std::str::from_utf8(&body_bytes) {
            // Check if it has instructions field
            let has_instructions = body_str.contains("\"instructions\"");
            tracing::warn!(
                "Has 'instructions' field AFTER modification: {}",
                has_instructions
            );
            tracing::warn!("{}", body_str);
        } else {
            tracing::warn!("[Body is not UTF-8]");
        }
        tracing::warn!("========================================");

        let req = Request::from_parts(parts, body_bytes);
        let client = self.inner.clone();
        async move {
            let mut response = client.send_streaming(req).await?;
            if response.headers().get(http::header::CONTENT_TYPE).is_none() {
                response.headers_mut().insert(
                    http::header::CONTENT_TYPE,
                    HeaderValue::from_static("text/event-stream"),
                );
            }
            tracing::warn!("Codex streaming response status: {}", response.status());
            for (name, value) in response.headers().iter() {
                tracing::warn!(
                    "Codex streaming response header: {}: {}",
                    name,
                    value.to_str().unwrap_or("[non-utf8]")
                );
            }
            Ok(response)
        }
    }
}
