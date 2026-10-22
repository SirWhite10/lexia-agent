//! The model layer: a provider abstraction, not a vendor.
//!
//! Nothing above this module may name a vendor's request shape. That is the
//! whole point — `OPERATIONS.md` treats OpenRouter as the initial provider, not
//! as the architecture, and the moment a call site builds a `chat/completions`
//! body by hand the abstraction has already failed.
//!
//! Two consequences worth stating, because they are what the abstraction buys:
//!
//! - A provider that is not OpenAI-shaped implements the same trait and nothing
//!   above it changes.
//! - A row in the manifest can name a model the provider's catalogue does not
//!   list, because the request carries an id rather than a catalogue entry.

mod openai_compatible;

#[cfg(test)]
mod live_tests;

use async_trait::async_trait;
use futures_util::stream::BoxStream;
use serde::{Deserialize, Serialize};

use crate::error::{Result, Secret};

pub use openai_compatible::OpenAiCompatible;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Modality {
    Text,
    Embeddings,
    Audio,
    Image,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    /// Present on an assistant message that asked for tools.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// Present on a tool result, naming the call it answers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// Parsed arguments, so a caller never re-parses a string to find out what
    /// the model actually asked for.
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    /// JSON Schema for the arguments.
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDefinition>,
    /// Forces the model to answer with a value matching this JSON Schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// A provider whose catalogue cannot cover the row's modality gets a bare
    /// id; an override is the only way to give the host a context window for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "kind", content = "detail")]
pub enum StreamEvent {
    TextDelta(String),
    /// A tool call arriving in pieces. Providers stream these incrementally, so
    /// the caller accumulates until `Completed`.
    ToolCallDelta {
        index: usize,
        id: Option<String>,
        name: Option<String>,
        arguments_delta: String,
    },
    Completed(Completion),
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Completion {
    pub text: String,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default)]
    pub usage: Usage,
    /// `stop`, `tool_calls`, `length`, or whatever the provider calls it.
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    #[serde(default)]
    pub context_window: Option<u64>,
    /// What the provider says this model serves. Empty where the provider does
    /// not say, which is the usual case and not an error.
    #[serde(default)]
    pub modalities: Vec<Modality>,
}

#[derive(Debug, Clone)]
pub struct TranscriptionRequest {
    /// Raw audio bytes. Not decoded here: formats are the provider's problem,
    /// and a wrong guess about one is a bug the host should not carry.
    pub audio: Vec<u8>,
    pub filename: String,
    pub mime_type: String,
    /// The transcription model. Typed rather than chosen, because a provider
    /// whose catalogue is text-only cannot offer a picker for it.
    pub model: String,
    pub language: Option<String>,
}

#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Stable identifier, matching the manifest.
    fn id(&self) -> &str;

    fn modalities(&self) -> Vec<Modality>;

    async fn complete(
        &self,
        credential: &Secret,
        request: CompletionRequest,
    ) -> Result<Completion>;

    /// Token-by-token events. Takes the credential by value so the returned
    /// stream is `'static`: the caller cannot drop the secret while the provider
    /// is still reading from it.
    fn stream(
        &self,
        credential: Secret,
        request: CompletionRequest,
    ) -> BoxStream<'static, Result<StreamEvent>>;

    /// What the provider says it serves. Empty is a legitimate answer.
    async fn catalogue(&self, credential: &Secret) -> Result<Vec<ModelInfo>>;

    async fn transcribe(
        &self,
        credential: &Secret,
        request: TranscriptionRequest,
    ) -> Result<String>;
}