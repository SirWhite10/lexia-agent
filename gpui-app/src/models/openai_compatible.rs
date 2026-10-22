//! The OpenAI chat-completions adapter.
//!
//! Most hosted inference speaks this shape, OpenRouter included, so one adapter
//! covers most providers. The value of it is not brevity — it is that this file
//! is the *only* place in the codebase that knows a vendor's wire format, so
//! changing provider or adding one is a contained change.
//!
//! Multipart bodies are assembled by hand rather than pulled in as a
//! dependency. The format is three headers and a boundary, it appears exactly
//! once in the product, and a dependency for it would be more code in the lock
//! file than code in this function.

use std::time::Duration;

use async_trait::async_trait;
use futures_util::stream::{BoxStream, StreamExt};
use serde_json::{Value, json};

use super::{
    Completion, CompletionRequest, Modality, ModelInfo, ModelProvider, StreamEvent,
    ToolCall, TranscriptionRequest, Usage,
};
use crate::error::{Error, Result, Secret};

pub struct OpenAiCompatible {
    id: String,
    endpoint: String,
    modalities: Vec<Modality>,
    client: reqwest::Client,
}

impl OpenAiCompatible {
    pub fn new(
        id: impl Into<String>,
        endpoint: impl Into<String>,
        modalities: Vec<Modality>,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            // A streaming completion is idle for long stretches between tokens,
            // so the read timeout is generous and the connect timeout is not.
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(300))
            .build()
            .map_err(|e| Error::HttpClient(e.to_string()))?;
        Ok(Self {
            id: id.into(),
            endpoint: endpoint.into().trim_end_matches('/').to_string(),
            modalities,
            client,
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.endpoint)
    }

    /// Turn the vendor-neutral request into the wire shape.
    ///
    /// Structured output is sent as a tool with a single choice, because that is
    /// how this API expresses it; the provider-neutral `response_schema` is not
    /// a concept on the wire.
    fn wire_body(request: &CompletionRequest) -> Value {
        let mut messages: Vec<Value> = Vec::with_capacity(request.messages.len());
        for message in &request.messages {
            let mut wire = json!({ "role": message.role, "content": message.content });
            if !message.tool_calls.is_empty() {
                wire["tool_calls"] = Value::Array(
                    message
                        .tool_calls
                        .iter()
                        .map(|call| {
                            json!({
                                "id": call.id,
                                "type": "function",
                                "function": {
                                    "name": call.name,
                                    "arguments": call.arguments.to_string(),
                                }
                            })
                        })
                        .collect(),
                );
            }
            if let Some(id) = &message.tool_call_id {
                wire["tool_call_id"] = Value::String(id.clone());
            }
            messages.push(wire);
        }

        let mut body = json!({ "model": request.model, "messages": messages });

        let mut tools: Vec<Value> = request
            .tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters,
                    }
                })
            })
            .collect();

        if let Some(schema) = &request.response_schema {
            tools.push(json!({
                "type": "function",
                "function": {
                    "name": "structured_response",
                    "description": "Return the result in the requested shape.",
                    "parameters": schema,
                }
            }));
        }
        if !tools.is_empty() {
            body["tools"] = Value::Array(tools);
        }
        if let Some(temperature) = request.temperature {
            body["temperature"] = json!(temperature);
        }
        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        body
    }

    fn parse_completion(payload: &Value) -> Completion {
        let choice = payload
            .pointer("/choices/0")
            .cloned()
            .unwrap_or(Value::Null);

        let text = choice
            .pointer("/message/content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let tool_calls = choice
            .pointer("/message/tool_calls")
            .and_then(Value::as_array)
            .map(|calls| {
                calls
                    .iter()
                    .map(|call| ToolCall {
                        id: call
                            .get("id")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        name: call
                            .pointer("/function/name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        // A model that produced unparseable arguments is a
                        // real possibility; carry it as a string rather than
                        // dropping the call, so the caller can report it.
                        arguments: call
                            .pointer("/function/arguments")
                            .and_then(Value::as_str)
                            .and_then(|raw| serde_json::from_str(raw).ok())
                            .unwrap_or_else(|| Value::String(raw_string(call))),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Completion {
            text,
            tool_calls,
            usage: Usage {
                prompt_tokens: payload
                    .pointer("/usage/prompt_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
                completion_tokens: payload
                    .pointer("/usage/completion_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
            },
            finish_reason: choice
                .get("finish_reason")
                .and_then(Value::as_str)
                .map(str::to_string),
        }
    }

    async fn send(&self, credential: &Secret, request: &CompletionRequest) -> Result<reqwest::Response> {
        self.client
            .post(self.url("/chat/completions"))
            .bearer_auth(credential.expose())
            .json(&Self::wire_body(request))
            .send()
            .await
            .map_err(|e| Error::ProviderUnreachable {
                provider: self.id.clone(),
                reason: e.to_string(),
            })
    }

    fn status_error(&self, status: u16, body: String) -> Error {
        Error::ProviderStatus {
            provider: self.id.clone(),
            status,
            body,
        }
    }
}

fn raw_string(call: &Value) -> String {
    call.pointer("/function/arguments")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// Pull a complete message out of accumulated streaming pieces.
///
/// Tool calls arrive as fragments keyed by index, so the accumulation is by
/// index rather than by arrival order — a provider is free to interleave them.
fn finish_streamed(text: &str, calls: &[(usize, String, String, String)]) -> Completion {
    // The stream loop already keyed and accumulated these by index, because a
    // provider is free to interleave tool calls. So this only has to parse what
    // arrived, not reassemble it.
    let tool_calls: Vec<ToolCall> = calls
        .iter()
        .filter(|(_, id, name, _)| !id.is_empty() || !name.is_empty())
        .map(|(_, id, name, raw)| ToolCall {
            id: id.clone(),
            name: name.clone(),
            // Unparseable arguments stay verbatim. A model that emitted a
            // malformed tool call is reportable; a call that vanished is not.
            arguments: serde_json::from_str::<Value>(raw)
                .unwrap_or_else(|_| Value::String(raw.clone())),
        })
        .collect();

    Completion {
        text: text.to_string(),
        finish_reason: Some(
            if tool_calls.is_empty() { "stop" } else { "tool_calls" }.to_string(),
        ),
        tool_calls,
        usage: Usage::default(),
    }
}

#[async_trait]
impl ModelProvider for OpenAiCompatible {
    fn id(&self) -> &str {
        &self.id
    }

    fn modalities(&self) -> Vec<Modality> {
        self.modalities.clone()
    }

    async fn complete(
        &self,
        credential: &Secret,
        request: CompletionRequest,
    ) -> Result<Completion> {
        let response = self.send(credential, &request).await?;
        let status = response.status().as_u16();
        let text = response.text().await.unwrap_or_default();

        if !(200..300).contains(&status) {
            return Err(self.status_error(status, truncate(&text)));
        }
        let payload: Value =
            serde_json::from_str(&text).map_err(|e| self.status_error(status, e.to_string()))?;
        Ok(Self::parse_completion(&payload))
    }

    fn stream(
        &self,
        credential: Secret,
        request: CompletionRequest,
    ) -> BoxStream<'static, Result<StreamEvent>> {
        let this = self.clone_client();
        let id = self.id.clone();
        let url = self.url("/chat/completions");
        let mut body = Self::wire_body(&request);
        body["stream"] = json!(true);

        let stream = async_stream::stream! {
            let response = this
                .post(&url)
                .bearer_auth(credential.expose())
                .json(&body)
                .send()
                .await;

            let response = match response {
                Ok(response) if response.status().is_success() => response,
                Ok(response) => {
                    let status = response.status().as_u16();
                    let text = response.text().await.unwrap_or_default();
                    yield Err(Error::ProviderStatus {
                        provider: id.clone(),
                        status,
                        body: truncate(&text),
                    });
                    return;
                }
                Err(e) => {
                    yield Err(Error::ProviderUnreachable {
                        provider: id.clone(),
                        reason: e.to_string(),
                    });
                    return;
                }
            };

            let mut bytes = response.bytes_stream();
            let mut buffer = String::new();
            let mut text = String::new();
            let mut calls: Vec<(usize, String, String, String)> = Vec::new();

            while let Some(chunk) = bytes.next().await {
                let chunk = match chunk {
                    Ok(chunk) => chunk,
                    Err(e) => {
                        yield Err(Error::ProviderUnreachable {
                            provider: id.clone(),
                            reason: e.to_string(),
                        });
                        return;
                    }
                };
                buffer.push_str(&String::from_utf8_lossy(&chunk));

                // Frames are separated by a blank line. Whatever follows the
                // last separator is a partial frame and stays buffered.
                while let Some(split) = buffer.find("\n\n") {
                    let frame: String = buffer.drain(..split + 2).collect();
                    for event in parse_sse(&frame) {
                        if event != "[DONE]" {
                            continue;
                        }
                        yield Ok(StreamEvent::Completed(finish_streamed(&text, &calls)));
                        return;
                    }
                    let Some(payload) = event_payload(&frame) else {
                        continue;
                    };
                    let Ok(value) = serde_json::from_str::<Value>(&payload) else {
                        continue;
                    };
                    if let Some(error) = value.get("error").and_then(|e| e.get("message")).and_then(Value::as_str) {
                        yield Ok(StreamEvent::Failed(error.to_string()));
                        return;
                    }

                    let delta = value.pointer("/choices/0/delta").cloned().unwrap_or(Value::Null);
                    if let Some(fragment) = delta.get("content").and_then(Value::as_str)
                        && !fragment.is_empty()
                    {
                        text.push_str(fragment);
                        yield Ok(StreamEvent::TextDelta(fragment.to_string()));
                    }
                    if let Some(tool_calls) = delta.get("tool_calls").and_then(Value::as_array) {
                        for (position, call) in tool_calls.iter().enumerate() {
                            let index = call
                                .get("index")
                                .and_then(Value::as_u64)
                                .map(|i| i as usize)
                                .unwrap_or(position);
                            let id = call.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
                            let name = call
                                .pointer("/function/name")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string();
                            let arguments_delta = call
                                .pointer("/function/arguments")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string();
                            while calls.len() <= index {
                                calls.push((index, String::new(), String::new(), String::new()));
                            }
                            calls[index].1.push_str(&id);
                            calls[index].2.push_str(&name);
                            calls[index].3.push_str(&arguments_delta);
                            yield Ok(StreamEvent::ToolCallDelta {
                                index,
                                id: (!id.is_empty()).then_some(id),
                                name: (!name.is_empty()).then_some(name),
                                arguments_delta,
                            });
                        }
                    }
                }
            }

            yield Ok(StreamEvent::Completed(finish_streamed(&text, &calls)));
        };

        Box::pin(stream)
    }

    async fn catalogue(&self, credential: &Secret) -> Result<Vec<ModelInfo>> {
        let response = self
            .client
            .get(self.url("/models"))
            .bearer_auth(credential.expose())
            .send()
            .await
            .map_err(|e| Error::ProviderUnreachable {
                provider: self.id.clone(),
                reason: e.to_string(),
            })?;

        let status = response.status().as_u16();
        let text = response.text().await.unwrap_or_default();
        if !(200..300).contains(&status) {
            return Err(self.status_error(status, truncate(&text)));
        }

        let payload: Value = serde_json::from_str(&text)
            .map_err(|e| self.status_error(status, e.to_string()))?;

        // A catalogue that does not list context windows is normal, not an
        // error. The manifest's `model_overrides` is how the host learns one.
        Ok(payload
            .get("data")
            .and_then(Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| {
                        let id = entry.get("id").and_then(Value::as_str)?;
                        Some(ModelInfo {
                            id: id.to_string(),
                            context_window: entry
                                .pointer("/context_length")
                                .or_else(|| entry.pointer("/context_window"))
                                .and_then(Value::as_u64),
                            modalities: Vec::new(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn transcribe(
        &self,
        credential: &Secret,
        request: TranscriptionRequest,
    ) -> Result<String> {
        let boundary = format!("agent{}", request.audio.len());
        let body = multipart(
            &boundary,
            &[
                Part::text("model", &request.model),
                match &request.language {
                    Some(language) => Part::text("language", language),
                    None => Part::text("response_format", "text"),
                },
            ],
            &request,
        );

        let response = self
            .client
            .post(self.url("/audio/transcriptions"))
            .bearer_auth(credential.expose())
            .header(
                reqwest::header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(body)
            .send()
            .await
            .map_err(|e| Error::ProviderUnreachable {
                provider: self.id.clone(),
                reason: e.to_string(),
            })?;

        let status = response.status().as_u16();
        let text = response.text().await.unwrap_or_default();
        if !(200..300).contains(&status) {
            return Err(self.status_error(status, truncate(&text)));
        }

        // Providers answer either plain text or `{ "text": "…" }` depending on
        // the requested response format; accept both rather than forcing the
        // caller to know which provider did what.
        match serde_json::from_str::<Value>(&text) {
            Ok(value) => Ok(value
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or(&text)
                .to_string()),
            Err(_) => Ok(text),
        }
    }
}

impl OpenAiCompatible {
    /// The streaming path moves the client into the stream, so the adapter needs
    /// a handle it can put behind `'static`.
    fn clone_client(&self) -> reqwest::Client {
        self.client.clone()
    }
}

// ---------------------------------------------------------------- streaming

/// Every `data:` payload in one SSE frame.
fn parse_sse(frame: &str) -> impl Iterator<Item = &str> {
    frame.lines().filter_map(|line| {
        line.strip_prefix("data:").map(str::trim_start)
    })
}

/// The first payload in a frame, or `None` for a keep-alive comment.
fn event_payload(frame: &str) -> Option<String> {
    parse_sse(frame).next().map(str::to_string)
}

/// Provider errors can be long and can echo the request. Keep enough to diagnose
/// and no more than that in an error that ends up in a log.
fn truncate(text: &str) -> String {
    const LIMIT: usize = 512;
    if text.len() <= LIMIT {
        return text.to_string();
    }
    let mut end = LIMIT;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}… ({} bytes total)", &text[..end], text.len())
}

// ----------------------------------------------------------------- multipart

enum Part<'a> {
    Text(&'a str, &'a str),
}

impl<'a> Part<'a> {
    fn text(name: &'a str, value: &'a str) -> Part<'a> {
        Part::Text(name, value)
    }
}

fn multipart(boundary: &str, fields: &[Part<'_>], file: &TranscriptionRequest) -> Vec<u8> {
    let mut body: Vec<u8> = Vec::with_capacity(file.audio.len() + 1024);

    for Part::Text(name, value) in fields {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
        );
        body.extend_from_slice(value.as_bytes());
        body.extend_from_slice(b"\r\n");
    }

    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n",
            // A quote in a filename would end the header parameter early.
            file.filename.replace('"', "")
        )
        .as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {}\r\n\r\n", file.mime_type).as_bytes());
    body.extend_from_slice(&file.audio);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    body
}