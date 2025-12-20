//! Claude API client for chat completions
//!
//! Uses OAuth tokens to authenticate with the Anthropic API.

use anyhow::Result;
use futures::stream::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

const CLAUDE_API_URL: &str = "https://api.anthropic.com/v1/messages";
const CLAUDE_MODELS_URL: &str = "https://api.anthropic.com/v1/models";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    stream: bool,
}

/// A content block in the response
#[derive(Debug, Clone, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: Option<String>,
}

/// The full chat response (non-streaming)
#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
}

/// SSE stream events from Claude API
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: serde_json::Value },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: u32,
        content_block: serde_json::Value,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: u32, delta: Delta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: u32 },
    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: serde_json::Value,
        usage: Option<serde_json::Value>,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "error")]
    Error { error: ErrorInfo },
}

/// Delta content in streaming response
#[derive(Debug, Clone, Deserialize)]
pub struct Delta {
    #[serde(rename = "type")]
    pub delta_type: String,
    pub text: Option<String>,
}

/// Error information from the API
#[derive(Debug, Clone, Deserialize)]
pub struct ErrorInfo {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

/// Claude API client
pub struct ClaudeClient {
    access_token: String,
    client: Client,
    model: String,
}

impl ClaudeClient {
    /// Create a new Claude client with the given OAuth access token
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            client: Client::new(),
            model: DEFAULT_MODEL.to_string(),
        }
    }

    /// Create a new Claude client with a specific model
    pub fn with_model(access_token: String, model: impl Into<String>) -> Self {
        Self {
            access_token,
            client: Client::new(),
            model: model.into(),
        }
    }

    /// Get the current model
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Set the model to use
    pub fn set_model(&mut self, model: impl Into<String>) {
        self.model = model.into();
    }

    /// Send a chat message and get a non-streaming response
    pub async fn chat(
        &self,
        messages: Vec<Message>,
        system_prompt: Option<&str>,
    ) -> Result<String> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            max_tokens: 4096,
            system: system_prompt.map(|s| s.to_string()),
            stream: false,
        };

        tracing::debug!("Sending chat request to Claude API (model: {})", self.model);

        let response = self
            .client
            .post(CLAUDE_API_URL)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header(
                "anthropic-beta",
                "oauth-2025-04-20,interleaved-thinking-2025-05-14",
            )
            .header("x-app", "cli")
            .header("User-Agent", "claude-cli/2.0.61 (external, cli)")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Claude API error {}: {}", status, body);
            anyhow::bail!("Claude API error {}: {}", status, body);
        }

        let chat_response: ChatResponse = response.json().await?;

        // Extract text from content blocks
        let text = chat_response
            .content
            .iter()
            .filter_map(|block| block.text.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join("");

        tracing::debug!("Received response from Claude ({} chars)", text.len());

        Ok(text)
    }

    /// Send a chat message and stream the response
    /// Returns a stream of text chunks
    pub async fn chat_stream(
        &self,
        messages: Vec<Message>,
        system_prompt: Option<&str>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            max_tokens: 4096,
            system: system_prompt.map(|s| s.to_string()),
            stream: true,
        };

        tracing::debug!(
            "Sending streaming chat request to Claude API (model: {})",
            self.model
        );

        let response = self
            .client
            .post(CLAUDE_API_URL)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header(
                "anthropic-beta",
                "oauth-2025-04-20,interleaved-thinking-2025-05-14",
            )
            .header("x-app", "cli")
            .header("User-Agent", "claude-cli/2.0.61 (external, cli)")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Claude API error {}: {}", status, body);
            anyhow::bail!("Claude API error {}: {}", status, body);
        }

        let byte_stream = response.bytes_stream();

        // Convert bytes stream to string chunks, parsing SSE events
        let stream = byte_stream
            .map(
                |result: std::result::Result<bytes::Bytes, reqwest::Error>| -> Result<String> {
                    match result {
                        Ok(bytes) => Ok(String::from_utf8_lossy(&bytes).to_string()),
                        Err(e) => Err(anyhow::anyhow!("Stream error: {}", e)),
                    }
                },
            )
            .scan(String::new(), |buffer, result| {
                // Use a synchronous closure that returns a ready future
                let output = match result {
                    Ok(chunk) => {
                        buffer.push_str(&chunk);

                        // Process complete lines
                        let mut texts = Vec::new();
                        while let Some(pos) = buffer.find('\n') {
                            let line = buffer[..pos].to_string();
                            *buffer = buffer[pos + 1..].to_string();

                            if let Some(data) = line.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    continue;
                                }
                                if let Ok(event) = serde_json::from_str::<StreamEvent>(data) {
                                    match event {
                                        StreamEvent::ContentBlockDelta { delta, .. } => {
                                            if let Some(text) = delta.text {
                                                texts.push(text);
                                            }
                                        }
                                        StreamEvent::Error { error } => {
                                            tracing::error!(
                                                "Stream error: {} - {}",
                                                error.error_type,
                                                error.message
                                            );
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }

                        if texts.is_empty() {
                            Some(Ok(String::new()))
                        } else {
                            Some(Ok(texts.join("")))
                        }
                    }
                    Err(e) => Some(Err(e)),
                };
                std::future::ready(output)
            })
            .filter(|result| {
                std::future::ready(match result {
                    Ok(s) => !s.is_empty(),
                    Err(_) => true,
                })
            });

        Ok(Box::pin(stream))
    }

    /// Simple helper to send a single user message
    pub async fn send_message(&self, message: &str, system_prompt: Option<&str>) -> Result<String> {
        self.chat(vec![Message::user(message)], system_prompt).await
    }

    /// Fetch available models from the Claude API
    pub async fn fetch_models(&self) -> Result<Vec<String>> {
        tracing::debug!("Fetching available models from Claude API");

        let response = self
            .client
            .get(CLAUDE_MODELS_URL)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("anthropic-beta", "oauth-2025-04-20")
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Failed to fetch models {}: {}", status, body);
            anyhow::bail!("Failed to fetch models {}: {}", status, body);
        }

        let data: ModelsResponse = response.json().await?;
        let models: Vec<String> = data
            .data
            .into_iter()
            .filter_map(|m| m.id.or(m.name))
            .collect();

        tracing::info!("Fetched {} models from Claude API", models.len());
        Ok(models)
    }

    /// Fetch and filter models to only the latest versions
    pub async fn fetch_latest_models(&self) -> Result<Vec<String>> {
        let all_models = self.fetch_models().await?;
        let filtered = filter_latest_claude_models(&all_models);
        tracing::info!(
            "Filtered to {} latest models: {:?}",
            filtered.len(),
            filtered
        );
        Ok(filtered)
    }
}

/// Response from /v1/models endpoint
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelInfo>,
}

/// Model information from the API
#[derive(Debug, Deserialize)]
struct ModelInfo {
    id: Option<String>,
    name: Option<String>,
}

/// Filter models to keep only the latest haiku, sonnet, and opus versions.
///
/// Parses model names in the format `claude-{family}-{major}-{minor}-{date}`
/// and returns only the latest version of each family.
pub fn filter_latest_claude_models(models: &[String]) -> Vec<String> {
    use regex::Regex;
    use std::collections::HashMap;

    // Dictionary to store the latest model for each family
    // family -> (model_name, major, minor, date)
    let mut latest_models: HashMap<String, (String, i32, i32, i32)> = HashMap::new();

    // Pattern: claude-{family}-{major}-{minor}-{date}
    // Also handles claude-{family}-{major}.{minor}-{date}
    let pattern1 = Regex::new(r"claude-(haiku|sonnet|opus)-(\d+)-(\d+)-(\d+)").unwrap();
    let pattern2 = Regex::new(r"claude-(haiku|sonnet|opus)-(\d+)\.(\d+)-(\d+)").unwrap();

    for model_name in models {
        let captures = pattern1
            .captures(model_name)
            .or_else(|| pattern2.captures(model_name));

        if let Some(caps) = captures {
            let family = caps.get(1).unwrap().as_str().to_string();
            let major: i32 = caps.get(2).unwrap().as_str().parse().unwrap_or(0);
            let minor: i32 = caps.get(3).unwrap().as_str().parse().unwrap_or(0);
            let date: i32 = caps.get(4).unwrap().as_str().parse().unwrap_or(0);

            if let Some((_, cur_major, cur_minor, cur_date)) = latest_models.get(&family) {
                if (major, minor, date) > (*cur_major, *cur_minor, *cur_date) {
                    latest_models.insert(family, (model_name.clone(), major, minor, date));
                }
            } else {
                latest_models.insert(family, (model_name.clone(), major, minor, date));
            }
        }
    }

    // Return only the model names
    latest_models
        .values()
        .map(|(name, _, _, _)| name.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let user_msg = Message::user("Hello");
        assert_eq!(user_msg.role, "user");
        assert_eq!(user_msg.content, "Hello");

        let assistant_msg = Message::assistant("Hi there!");
        assert_eq!(assistant_msg.role, "assistant");
        assert_eq!(assistant_msg.content, "Hi there!");
    }

    #[test]
    fn test_client_creation() {
        let client = ClaudeClient::new("test-token".to_string());
        assert_eq!(client.model(), DEFAULT_MODEL);

        let client = ClaudeClient::with_model("test-token".to_string(), "claude-3-haiku-20240307");
        assert_eq!(client.model(), "claude-3-haiku-20240307");
    }
}
