//! Gemini Code Assist provider (OAuth)
//!
//! Implements the Cloud Code Assist API used by gemini-cli/llxprt for OAuth tokens.
//! This bypasses the public Generative Language API, which rejects OAuth scopes.

use std::time::Duration;

use async_stream::stream;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tokio::time::sleep;
use uuid::Uuid;

use super::common::{OAuthProviderError, ProviderResult};
use rig::completion::{
    self, CompletionError, CompletionRequest, CompletionResponse, GetTokenUsage,
};
use rig::providers::gemini::completion::gemini_api_types::{
    AdditionalParameters, Content, ContentCandidate, FunctionCallingMode, GenerateContentRequest,
    GenerateContentResponse, Part, PartKind, Role, Tool, ToolConfig,
};
use rig::streaming;

const CODE_ASSIST_ENDPOINT: &str = "https://cloudcode-pa.googleapis.com";
const CODE_ASSIST_API_VERSION: &str = "v1internal";
const ONBOARD_POLL_DELAY: Duration = Duration::from_secs(5);
const ONBOARD_MAX_ATTEMPTS: usize = 12;
const CODE_ASSIST_MAX_RETRIES: usize = 4;

fn retry_delay_from_message(message: &str, attempt: usize) -> Duration {
    if let Some(seconds) = parse_retry_after_seconds(message) {
        return Duration::from_secs(seconds);
    }
    let backoff = 2u64.saturating_pow((attempt.saturating_sub(1)) as u32);
    Duration::from_secs(backoff.min(30))
}

fn parse_retry_after_seconds(message: &str) -> Option<u64> {
    let lower = message.to_lowercase();
    let after_idx = lower.find("after")?;
    let tail = &lower[after_idx + "after".len()..];
    for token in tail.split_whitespace() {
        let cleaned = token
            .trim_matches(|c: char| !c.is_ascii_digit())
            .to_string();
        if cleaned.is_empty() {
            continue;
        }
        if let Ok(value) = cleaned.parse::<u64>() {
            return Some(value);
        }
    }
    None
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodeAssistResponseEnvelope<T> {
    response: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodeAssistStreamResponse {
    candidates: Vec<ContentCandidate>,
    usage_metadata: Option<CodeAssistPartialUsage>,
}

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodeAssistPartialUsage {
    #[serde(default)]
    total_token_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    cached_content_token_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    candidates_token_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thoughts_token_count: Option<i32>,
    #[serde(default)]
    prompt_token_count: i32,
}

impl GetTokenUsage for CodeAssistPartialUsage {
    fn token_usage(&self) -> Option<completion::Usage> {
        let mut usage = completion::Usage::new();
        usage.input_tokens = self.prompt_token_count as u64;
        usage.output_tokens = (self.cached_content_token_count.unwrap_or_default()
            + self.candidates_token_count.unwrap_or_default()
            + self.thoughts_token_count.unwrap_or_default()) as u64;
        usage.total_tokens = if self.total_token_count > 0 {
            self.total_token_count as u64
        } else {
            usage.input_tokens + usage.output_tokens
        };
        Some(usage)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodeAssistStreamingCompletionResponse {
    pub(crate) usage_metadata: CodeAssistPartialUsage,
}

impl GetTokenUsage for CodeAssistStreamingCompletionResponse {
    fn token_usage(&self) -> Option<completion::Usage> {
        let mut usage = completion::Usage::new();
        usage.total_tokens = self.usage_metadata.total_token_count as u64;
        usage.output_tokens = self
            .usage_metadata
            .candidates_token_count
            .map(|x| x as u64)
            .unwrap_or(0);
        usage.input_tokens = self.usage_metadata.prompt_token_count as u64;
        Some(usage)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoadCodeAssistResponse {
    #[serde(default)]
    current_tier: Option<GeminiUserTier>,
    #[serde(default)]
    allowed_tiers: Option<Vec<GeminiUserTier>>,
    #[serde(default)]
    cloudaicompanion_project: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct GeminiUserTier {
    id: String,
    #[serde(default)]
    user_defined_cloudaicompanion_project: Option<bool>,
    #[serde(default)]
    is_default: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct LongRunningOperationResponse {
    #[serde(default)]
    done: Option<bool>,
    #[serde(default)]
    response: Option<OnboardUserResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OnboardUserResponse {
    #[serde(default)]
    cloudaicompanion_project: Option<ProjectRef>,
}

#[derive(Debug, Deserialize)]
struct ProjectRef {
    id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClientMetadata<'a> {
    ide_type: &'a str,
    platform: &'a str,
    plugin_type: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    duet_project: Option<&'a str>,
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

    async fn request_stream<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        body: Value,
    ) -> ProviderResult<impl Stream<Item = ProviderResult<T>>> {
        let url = format!("{}/{}:{}", self.base_url, CODE_ASSIST_API_VERSION, method);
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
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(OAuthProviderError::ApiError {
                status: status.as_u16(),
                message: text,
            });
        }

        let mut bytes_stream = response.bytes_stream();
        let stream = stream! {
            let mut buffer = String::new();
            let mut data_lines: Vec<String> = Vec::new();

            while let Some(chunk) = StreamExt::next(&mut bytes_stream).await {
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

                        match serde_json::from_str::<T>(&data) {
                            Ok(value) => yield Ok(value),
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

    async fn resolve_project_id(&self) -> ProviderResult<String> {
        let project_id = std::env::var("GOOGLE_CLOUD_PROJECT").ok();
        let project_id_clone = project_id.clone();
        let metadata = ClientMetadata {
            ide_type: "IDE_UNSPECIFIED",
            platform: "PLATFORM_UNSPECIFIED",
            plugin_type: "GEMINI",
            duet_project: project_id.as_deref(),
        };
        let load_req = json!({
            "cloudaicompanionProject": project_id,
            "metadata": metadata,
        });
        let load_res: LoadCodeAssistResponse =
            self.request_post("loadCodeAssist", &load_req).await?;

        if load_res.current_tier.is_some() {
            if let Some(project) = load_res.cloudaicompanion_project.clone().or(project_id) {
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
            .allowed_tiers
            .as_ref()
            .and_then(|tiers| tiers.iter().find(|tier| tier.is_default.unwrap_or(false)))
            .or_else(|| {
                load_res
                    .allowed_tiers
                    .as_ref()
                    .and_then(|tiers| tiers.first())
            })
            .cloned()
            .ok_or_else(|| {
                OAuthProviderError::ConfigError("No available Gemini tiers".to_string())
            })?;

        if default_tier
            .user_defined_cloudaicompanion_project
            .unwrap_or(false)
            && project_id.is_none()
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
                    .and_then(|r| r.cloudaicompanion_project)
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

    pub async fn generate_content(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> ProviderResult<GenerateContentResponse> {
        let project_id = self.resolve_project_id().await?;
        let body = json!({
            "model": model,
            "project": project_id,
            "user_prompt_id": Uuid::new_v4().to_string(),
            "request": request,
        });
        let response: CodeAssistResponseEnvelope<GenerateContentResponse> =
            self.request_post("generateContent", &body).await?;
        Ok(response.response)
    }

    pub(crate) async fn stream_generate_content_raw(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> ProviderResult<impl Stream<Item = ProviderResult<CodeAssistStreamResponse>>> {
        let project_id = self.resolve_project_id().await?;
        let body = json!({
            "model": model,
            "project": project_id,
            "user_prompt_id": Uuid::new_v4().to_string(),
            "request": request,
        });
        let stream = self
            .request_stream::<CodeAssistResponseEnvelope<CodeAssistStreamResponse>>(
                "streamGenerateContent",
                body,
            )
            .await?
            .map(|item| item.map(|envelope| envelope.response));
        Ok(stream)
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

#[derive(Clone, Debug)]
pub struct GeminiCodeAssistRigClient {
    inner: GeminiCodeAssistClient,
}

impl GeminiCodeAssistRigClient {
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            inner: GeminiCodeAssistClient::new(access_token),
        }
    }

    pub fn completion_model(&self, model: &str) -> GeminiCodeAssistCompletionModel {
        GeminiCodeAssistCompletionModel {
            client: self.inner.clone(),
            model: model.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct GeminiCodeAssistCompletionModel {
    client: GeminiCodeAssistClient,
    model: String,
}

impl rig::completion::CompletionModel for GeminiCodeAssistCompletionModel {
    type Response = GenerateContentResponse;
    type StreamingResponse = CodeAssistStreamingCompletionResponse;
    type Client = GeminiCodeAssistRigClient;

    fn make(client: &Self::Client, model: impl Into<String>) -> Self {
        Self {
            client: client.inner.clone(),
            model: model.into(),
        }
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse<Self::Response>, CompletionError> {
        let body = create_request_body(request)?;
        let mut attempt = 0usize;
        let response = loop {
            attempt += 1;
            match self.client.generate_content(&self.model, &body).await {
                Ok(response) => break response,
                Err(OAuthProviderError::ApiError {
                    status: 429,
                    message,
                }) => {
                    if attempt >= CODE_ASSIST_MAX_RETRIES {
                        return Err(CompletionError::ProviderError(format!(
                            "Rate limited after {} attempts: {}",
                            attempt, message
                        )));
                    }
                    let delay = retry_delay_from_message(&message, attempt);
                    sleep(delay).await;
                }
                Err(e) => {
                    return Err(CompletionError::ProviderError(e.to_string()));
                }
            }
        };
        response.try_into()
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<streaming::StreamingCompletionResponse<Self::StreamingResponse>, CompletionError>
    {
        let body = create_request_body(request)?;
        let client = self.client.clone();
        let model = self.model.clone();

        let stream = stream! {
            let mut attempt = 0usize;
            let stream = loop {
                attempt += 1;
                match client.stream_generate_content_raw(&model, &body).await {
                    Ok(stream) => break stream,
                    Err(OAuthProviderError::ApiError { status: 429, message }) => {
                        if attempt >= CODE_ASSIST_MAX_RETRIES {
                            yield Err(CompletionError::ProviderError(format!(
                                "Rate limited after {} attempts: {}",
                                attempt, message
                            )));
                            return;
                        }
                        let delay = retry_delay_from_message(&message, attempt);
                        sleep(delay).await;
                    }
                    Err(e) => {
                        yield Err(CompletionError::ProviderError(e.to_string()));
                        return;
                    }
                }
            };
            futures::pin_mut!(stream);
            let mut final_usage = None;
            while let Some(chunk_result) = stream.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        yield Err(CompletionError::ProviderError(e.to_string()));
                        break;
                    }
                };

                if let Some(usage) = chunk.usage_metadata.clone() {
                    final_usage = Some(usage);
                }

                let Some(choice) = chunk.candidates.into_iter().next() else {
                    continue;
                };
                let Some(content) = choice.content else {
                    continue;
                };

                for part in content.parts {
                    match part {
                        Part {
                            part: PartKind::Text(text),
                            thought: Some(true),
                            ..
                        } => {
                            yield Ok(streaming::RawStreamingChoice::ReasoningDelta {
                                id: None,
                                reasoning: text,
                            });
                        }
                        Part {
                            part: PartKind::Text(text),
                            ..
                        } => {
                            yield Ok(streaming::RawStreamingChoice::Message(text));
                        }
                        Part {
                            part: PartKind::FunctionCall(function_call),
                            thought_signature,
                            ..
                        } => {
                            yield Ok(streaming::RawStreamingChoice::ToolCall(
                                streaming::RawStreamingToolCall::new(
                                    function_call.name.clone(),
                                    function_call.name.clone(),
                                    function_call.args.clone(),
                                )
                                .with_signature(thought_signature),
                            ));
                        }
                        _ => {}
                    }
                }

                if choice.finish_reason.is_some() {
                    break;
                }
            }

            yield Ok(streaming::RawStreamingChoice::FinalResponse(
                CodeAssistStreamingCompletionResponse {
                    usage_metadata: final_usage.unwrap_or_default(),
                },
            ));
        };

        Ok(streaming::StreamingCompletionResponse::stream(Box::pin(
            stream,
        )))
    }
}

fn create_request_body(
    completion_request: CompletionRequest,
) -> Result<GenerateContentRequest, CompletionError> {
    let mut full_history = Vec::new();
    full_history.extend(completion_request.chat_history);

    let additional_params = completion_request
        .additional_params
        .unwrap_or_else(|| Value::Object(Map::new()));

    let AdditionalParameters {
        mut generation_config,
        additional_params,
    } = serde_json::from_value::<AdditionalParameters>(additional_params)?;

    generation_config = generation_config.map(|mut cfg| {
        if let Some(temp) = completion_request.temperature {
            cfg.temperature = Some(temp);
        }

        if let Some(max_tokens) = completion_request.max_tokens {
            cfg.max_output_tokens = Some(max_tokens);
        }

        cfg
    });

    let system_instruction = completion_request.preamble.clone().map(|preamble| Content {
        parts: vec![preamble.into()],
        role: Some(Role::Model),
    });

    let tools = if completion_request.tools.is_empty() {
        None
    } else {
        Some(Tool::try_from(completion_request.tools)?)
    };

    let tool_config = if let Some(cfg) = completion_request.tool_choice {
        Some(ToolConfig {
            function_calling_config: Some(FunctionCallingMode::try_from(cfg)?),
        })
    } else {
        None
    };

    let request = GenerateContentRequest {
        contents: full_history
            .into_iter()
            .map(|msg| {
                msg.try_into()
                    .map_err(|e| CompletionError::RequestError(Box::new(e)))
            })
            .collect::<Result<Vec<_>, _>>()?,
        generation_config,
        safety_settings: None,
        tools,
        tool_config,
        system_instruction,
        additional_params,
    };

    Ok(request)
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
