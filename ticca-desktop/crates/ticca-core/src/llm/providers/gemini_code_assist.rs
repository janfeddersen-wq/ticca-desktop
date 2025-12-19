//! Gemini Code Assist provider (OAuth)
//!
//! Implements the Cloud Code Assist API used by gemini-cli/llxprt for OAuth tokens.
//! This bypasses the public Generative Language API, which rejects OAuth scopes.

use std::time::Duration;

use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tokio::time::sleep;
use uuid::Uuid;

use super::common::{OAuthProviderError, ProviderResult};

const CODE_ASSIST_ENDPOINT: &str = "https://cloudcode-pa.googleapis.com";
const CODE_ASSIST_API_VERSION: &str = "v1internal";
const ONBOARD_POLL_DELAY: Duration = Duration::from_secs(5);
const ONBOARD_MAX_ATTEMPTS: usize = 12;

#[derive(Debug, Deserialize)]
struct LoadCodeAssistResponse {
    #[serde(default)]
    currentTier: Option<GeminiUserTier>,
    #[serde(default)]
    allowedTiers: Option<Vec<GeminiUserTier>>,
    #[serde(default)]
    cloudaicompanionProject: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct GeminiUserTier {
    id: String,
    #[serde(default)]
    userDefinedCloudaicompanionProject: Option<bool>,
    #[serde(default)]
    isDefault: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct LongRunningOperationResponse {
    #[serde(default)]
    done: Option<bool>,
    #[serde(default)]
    response: Option<OnboardUserResponse>,
}

#[derive(Debug, Deserialize)]
struct OnboardUserResponse {
    #[serde(default)]
    cloudaicompanionProject: Option<ProjectRef>,
}

#[derive(Debug, Deserialize)]
struct ProjectRef {
    id: String,
}

#[derive(Debug, Serialize)]
struct ClientMetadata<'a> {
    ideType: &'a str,
    platform: &'a str,
    pluginType: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    duetProject: Option<&'a str>,
}

#[derive(Clone, Debug)]
pub struct GeminiCodeAssistClient {
    access_token: String,
    base_url: String,
    http: Client,
}

impl GeminiCodeAssistClient {
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            base_url: CODE_ASSIST_ENDPOINT.to_string(),
            http: Client::new(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    async fn request_post<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        body: &Value,
    ) -> ProviderResult<T> {
        let url = format!("{}/{}:{}", self.base_url, CODE_ASSIST_API_VERSION, method);
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.access_token)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(OAuthProviderError::ApiError {
                status: status.as_u16(),
                message: text,
            });
        }

        response
            .json::<T>()
            .await
            .map_err(|e| OAuthProviderError::ParseError(e.to_string()))
    }

    async fn resolve_project_id(&self) -> ProviderResult<String> {
        let project_id = std::env::var("GOOGLE_CLOUD_PROJECT").ok();
        let project_id_clone = project_id.clone();
        let metadata = ClientMetadata {
            ideType: "IDE_UNSPECIFIED",
            platform: "PLATFORM_UNSPECIFIED",
            pluginType: "GEMINI",
            duetProject: project_id.as_deref(),
        };
        let load_req = json!({
            "cloudaicompanionProject": project_id,
            "metadata": metadata,
        });
        let load_res: LoadCodeAssistResponse = self.request_post("loadCodeAssist", &load_req).await?;

        if load_res.currentTier.is_some() {
            if let Some(project) = load_res.cloudaicompanionProject.clone().or(project_id) {
                return Ok(project);
            }

            if let Some(project) = project_id_clone {
                return Ok(project);
            }

            return Err(OAuthProviderError::ConfigError(
                "Gemini Code Assist requires GOOGLE_CLOUD_PROJECT for this tier".to_string(),
            ));
        }

        let default_tier = load_res
            .allowedTiers
            .as_ref()
            .and_then(|tiers| tiers.iter().find(|tier| tier.isDefault.unwrap_or(false)))
            .or_else(|| load_res.allowedTiers.as_ref().and_then(|tiers| tiers.first()))
            .cloned()
            .ok_or_else(|| OAuthProviderError::ConfigError("No available Gemini tiers".to_string()))?;

        if default_tier.userDefinedCloudaicompanionProject.unwrap_or(false) && project_id.is_none()
        {
            return Err(OAuthProviderError::ConfigError(
                "Gemini tier requires GOOGLE_CLOUD_PROJECT".to_string(),
            ));
        }

        let onboard_req = json!({
            "tierId": default_tier.id,
            "cloudaicompanionProject": if default_tier.id == "free-tier" { Value::Null } else { json!(project_id) },
            "metadata": metadata,
        });

        for _ in 0..ONBOARD_MAX_ATTEMPTS {
            let onboard_res: LongRunningOperationResponse =
                self.request_post("onboardUser", &onboard_req).await?;
            if onboard_res.done.unwrap_or(false) {
                if let Some(project) = onboard_res
                    .response
                    .and_then(|r| r.cloudaicompanionProject)
                    .map(|p| p.id)
                    .or(project_id)
                {
                    return Ok(project);
                }
                return Err(OAuthProviderError::ConfigError(
                    "Gemini onboarding completed without project id".to_string(),
                ));
            }
            sleep(ONBOARD_POLL_DELAY).await;
        }

        Err(OAuthProviderError::ConfigError(
            "Gemini onboarding timed out".to_string(),
        ))
    }

    pub async fn stream_generate_content(
        &self,
        model: &str,
        system_prompt: &str,
        contents: &[CodeAssistContent],
    ) -> ProviderResult<impl Stream<Item = ProviderResult<String>>> {
        let project_id = self.resolve_project_id().await?;
        tracing::debug!(
            "Gemini Code Assist request: model={}, project_id={}, system_prompt_len={}, contents_len={}",
            model,
            project_id,
            system_prompt.len(),
            contents.len()
        );
        let url = format!(
            "{}/{}:streamGenerateContent",
            self.base_url, CODE_ASSIST_API_VERSION
        );

        let mut request_map = Map::new();
        request_map.insert("contents".to_string(), json!(contents));

        if !system_prompt.is_empty() {
            request_map.insert(
                "systemInstruction".to_string(),
                json!({
                    "role": "user",
                    "parts": [{ "text": system_prompt }],
                }),
            );
        }

        let body = json!({
            "model": model,
            "project": project_id,
            "user_prompt_id": Uuid::new_v4().to_string(),
            "request": Value::Object(request_map),
        });

        let response = self
            .http
            .post(url)
            .query(&[("alt", "sse")])
            .bearer_auth(&self.access_token)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        tracing::debug!("Gemini Code Assist response status: {}", status);
        if let Some(content_type) = response.headers().get(reqwest::header::CONTENT_TYPE) {
            tracing::debug!(
                "Gemini Code Assist response content-type: {}",
                content_type.to_str().unwrap_or("[non-utf8]")
            );
        }
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(OAuthProviderError::ApiError {
                status: status.as_u16(),
                message: text,
            });
        }

        let mut bytes_stream = response.bytes_stream();
        let stream = async_stream::stream! {
            let mut buffer = String::new();
            let mut data_lines: Vec<String> = Vec::new();

            while let Some(chunk) = futures::StreamExt::next(&mut bytes_stream).await {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => {
                        yield Err(OAuthProviderError::StreamError(e.to_string()));
                        return;
                    }
                };
                buffer.push_str(&String::from_utf8_lossy(&chunk));

                while let Some(pos) = buffer.find('\n') {
                    let mut line = buffer[..pos].to_string();
                    buffer = buffer[pos + 1..].to_string();
                    if line.ends_with('\r') {
                        line.pop();
                    }

                    if line.is_empty() {
                        if data_lines.is_empty() {
                            continue;
                        }
                        let data = data_lines.join("\n");
                        data_lines.clear();

                        if data.trim().is_empty() {
                            continue;
                        }
                        if data.trim() == "[DONE]" {
                            return;
                        }

                        match serde_json::from_str::<Value>(&data) {
                            Ok(value) => {
                                for text in extract_text_parts(&value) {
                                    if !text.is_empty() {
                                        tracing::debug!(
                                            "Gemini Code Assist chunk: {} chars",
                                            text.len()
                                        );
                                        yield Ok(text);
                                    }
                                }
                            }
                            Err(e) => {
                                yield Err(OAuthProviderError::ParseError(e.to_string()));
                                return;
                            }
                        }
                    } else if let Some(rest) = line
                        .strip_prefix("data: ")
                        .or_else(|| line.strip_prefix("data:"))
                    {
                        data_lines.push(rest.trim().to_string());
                    }
                }
            }
        };

        Ok(stream)
    }
}

#[derive(Debug, Serialize)]
pub struct CodeAssistContent {
    role: String,
    parts: Vec<CodeAssistPart>,
}

#[derive(Debug, Serialize)]
pub struct CodeAssistPart {
    text: String,
}

impl CodeAssistContent {
    pub fn user(text: &str) -> Self {
        Self {
            role: "user".to_string(),
            parts: vec![CodeAssistPart {
                text: text.to_string(),
            }],
        }
    }

    pub fn model(text: &str) -> Self {
        Self {
            role: "model".to_string(),
            parts: vec![CodeAssistPart {
                text: text.to_string(),
            }],
        }
    }
}

fn extract_text_parts(value: &Value) -> Vec<String> {
    let mut texts = Vec::new();
    let Some(response) = value.get("response") else {
        return texts;
    };
    let Some(candidates) = response.get("candidates").and_then(|v| v.as_array()) else {
        return texts;
    };

    for candidate in candidates {
        if let Some(parts) = candidate
            .get("content")
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.as_array())
        {
            for part in parts {
                if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                    texts.push(text.to_string());
                }
            }
        }
    }

    texts
}
