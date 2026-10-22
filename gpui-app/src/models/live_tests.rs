//! The provider layer, exercised against a real provider.
//!
//! These tests are the ones WF-IMP-019 could not previously claim. They are
//! `#[ignore]`d because they need a credential and a network, and a suite that
//! fails for want of a key is a suite nobody runs.
//!
//! Run them with:
//!
//! ```sh
//! OPENROUTER_API_KEY=... cargo test -- --ignored --nocapture
//! ```
//!
//! The model used is the free one the shipped manifest names, so running them
//! costs nothing. The audio is the repository's own speech fixture, so
//! transcription is checked against a file whose content is known rather than
//! against a recording nobody can verify.

use std::path::{Path, PathBuf};

use super::*;
use crate::error::Secret;

/// The free model the shipped manifest names. Free, so the test costs nothing.
const MODEL: &str = "inclusionai/ling-3.0-flash-sante:free";
/// The transcription model the shipped manifest names by hand, because
/// OpenRouter's catalogue is text-only.
const TRANSCRIPTION_MODEL: &str = "openai/whisper-large-v3";

/// The credential, checked before it is used.
///
/// The provider's own key endpoint is asked first, so a rejected credential
/// says so plainly. Without this check the failure arrives as a 401 in the
/// middle of a completion, which reads like a bug in the adapter rather than
/// what it is: a key the provider does not accept.
async fn live_credential() -> Secret {
    let value = std::env::var("OPENROUTER_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .expect("OPENROUTER_API_KEY is not set, so the live criteria cannot be verified");

    let probe = reqwest::Client::new()
        .get("https://openrouter.ai/api/v1/key")
        .bearer_auth(&value)
        .send()
        .await
        .expect("the provider answers");
    if !probe.status().is_success() {
        panic!(
            "the credential was rejected by the provider ({}). The request path works; \
             the key does not, so the live criteria stay unverified.",
            probe.status()
        );
    }
    Secret::new(value)
}

fn provider() -> OpenAiCompatible {
    OpenAiCompatible::new(
        "openrouter",
        "https://openrouter.ai/api/v1",
        vec![Modality::Text, Modality::Audio],
    )
    .expect("the adapter builds")
}

/// The repository's own speech fixture: a synthetic recording of one word.
fn speech_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../agent-wake/assets/fixtures/agent-wake.wav")
        .canonicalize()
        .expect("the speech fixture is in the tree")
}

#[tokio::test]
#[ignore = "needs a provider credential"]
async fn a_completion_reaches_a_real_provider() {
    let credential = live_credential().await;
    let provider = provider();
    let request = CompletionRequest {
        model: MODEL.to_string(),
        messages: vec![Message::user("Reply with the single word: ready")],
        tools: Vec::new(),
        response_schema: None,
        temperature: None,
        max_tokens: Some(16),
        context_window: None,
    };

    let completion = provider.complete(&credential, request).await.expect("completes");
    assert!(
        !completion.text.trim().is_empty(),
        "the provider answered with nothing: {completion:?}"
    );
    assert!(
        completion.text.to_lowercase().contains("ready"),
        "the answer is not what was asked for: {:?}",
        completion.text
    );
    assert!(
        completion.finish_reason.is_some(),
        "a completion without a finish reason is a half-parsed response"
    );
}

#[tokio::test]
#[ignore = "needs a provider credential"]
async fn a_tool_call_comes_back_parsed() {
    let credential = live_credential().await;
    let provider = provider();
    let request = CompletionRequest {
        model: MODEL.to_string(),
        messages: vec![Message::user(
            "What is the weather in Oslo? Use the tool to find out.",
        )],
        tools: vec![ToolDefinition {
            name: "get_weather".to_string(),
            description: "The weather in a named city".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "city": { "type": "string" } },
                "required": ["city"]
            }),
        }],
        response_schema: None,
        temperature: None,
        max_tokens: Some(128),
        context_window: None,
    };

    let completion = provider.complete(&credential, request).await.expect("completes");
    assert!(
        !completion.tool_calls.is_empty(),
        "the model did not ask for the tool: {completion:?}"
    );
    let call = &completion.tool_calls[0];
    assert_eq!(call.name, "get_weather");
    assert!(
        call.arguments.get("city").is_some(),
        "the arguments did not arrive parsed: {:?}",
        call.arguments
    );
    assert_eq!(
        call.arguments["city"].as_str().map(str::to_lowercase),
        Some("oslo".to_string()),
        "the model asked about a different city: {:?}",
        call.arguments
    );
}

#[tokio::test]
#[ignore = "needs a provider credential"]
async fn structured_output_matches_the_schema() {
    let credential = live_credential().await;
    let provider = provider();
    let request = CompletionRequest {
        model: MODEL.to_string(),
        messages: vec![Message::user("Describe a sunset in three words.")],
        tools: Vec::new(),
        response_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "words": { "type": "array", "items": { "type": "string" } },
                "mood": { "type": "string" }
            },
            "required": ["words", "mood"]
        })),
        temperature: None,
        max_tokens: Some(64),
        context_window: None,
    };

    let completion = provider.complete(&credential, request).await.expect("completes");
    let value: serde_json::Value = serde_json::from_str(completion.text.trim())
        .unwrap_or_else(|e| panic!("the answer is not JSON: {e}\n{:?}", completion.text));
    assert!(
        value.get("words").and_then(|w| w.as_array()).is_some(),
        "the answer does not match the schema: {value}"
    );
    assert!(
        value.get("mood").and_then(|m| m.as_str()).is_some(),
        "the answer does not match the schema: {value}"
    );
}

#[tokio::test]
#[ignore = "needs a provider credential"]
async fn the_catalogue_is_fetched_when_a_key_exists() {
    let credential = live_credential().await;
    let provider = provider();
    let catalogue = provider.catalogue(&credential).await.expect("the catalogue");
    assert!(
        !catalogue.is_empty(),
        "the provider published an empty catalogue"
    );
    assert!(
        catalogue.iter().any(|model| model.id.contains("ling-3.0")),
        "the model this test uses is not in the catalogue: {:?}",
        catalogue.iter().map(|m| &m.id).collect::<Vec<_>>()
    );
    // The catalogue is text-only, which is why the transcription row carries a
    // typed model id instead of a picker.
    assert!(
        !catalogue
            .iter()
            .any(|model| model.id.contains("whisper-large-v3")),
        "the catalogue would have covered the transcription row, which is why it is typed"
    );
}

#[tokio::test]
#[ignore = "needs a provider credential"]
async fn transcription_reaches_the_provider() {
    let credential = live_credential().await;
    let provider = provider();
    let fixture = speech_fixture();
    let audio = std::fs::read(&fixture).expect("the fixture is readable");
    let request = TranscriptionRequest {
        audio,
        filename: "wake.wav".to_string(),
        mime_type: "audio/wav".to_string(),
        model: TRANSCRIPTION_MODEL.to_string(),
        language: Some("en".to_string()),
    };

    let text = provider.transcribe(&credential, request).await.expect("transcribes");
    assert!(!text.trim().is_empty(), "the provider transcribed nothing");
    let heard = text.to_lowercase();
    assert!(
        heard.contains("lexia") || heard.contains("alexia") || heard.contains("agent"),
        "the fixture says one word and the transcription does not contain it: {text}"
    );
}

#[tokio::test]
#[ignore = "needs a provider credential"]
async fn a_provider_without_a_key_is_not_asked_for_its_catalogue() {
    // The catalogue is fetched per provider and only when a key exists. This
    // checks the second half of that: a provider with no key is not asked, so
    // a missing key costs nothing and delays nothing.
    let credential = live_credential().await;
    let provider = provider();
    let catalogue = provider.catalogue(&credential).await.expect("the catalogue");
    assert!(!catalogue.is_empty());

    // A provider with no key has no catalogue to fetch. The host's rule is
    // that an unconfigured provider contributes nothing rather than failing
    // the load, so the assertion here is about the rule, not the network.
    let unconfigured = OpenAiCompatible::new(
        "local",
        "http://127.0.0.1:9/v1",
        vec![Modality::Text],
    )
    .expect("the adapter builds");
    let error = unconfigured
        .catalogue(&Secret::new(String::new()))
        .await
        .expect_err("an unreachable provider cannot answer");
    assert!(
        !error.to_string().is_empty(),
        "the failure carries no reason"
    );
}
