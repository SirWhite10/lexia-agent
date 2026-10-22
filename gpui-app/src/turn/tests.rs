//! The per-turn seam, proven against the implementation it was ported from.
//!
//! The claims are the ticket's own: a turn captures its message and a later
//! turn retrieves it; reflection fires on its boundary and not off it; a
//! provider failure degrades to an empty memory block rather than breaking the
//! conversation.

use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;

use super::*;
use crate::memory::Embedder;

struct FixedEmbedder {
    model: String,
    vectors: HashMap<String, Vec<f32>>,
    failing: bool,
}

impl FixedEmbedder {
    fn new(model: &str) -> Self {
        Self {
            model: model.to_string(),
            vectors: HashMap::new(),
            failing: false,
        }
    }

    fn with(mut self, body: &str, vector: Vec<f32>) -> Self {
        self.vectors.insert(body.to_string(), vector);
        self
    }

    fn broken(mut self) -> Self {
        self.failing = true;
        self
    }
}

#[async_trait]
impl Embedder for FixedEmbedder {
    fn model(&self) -> &str {
        &self.model
    }

    async fn embed(&self, text: &str) -> std::result::Result<Option<Vec<f32>>, String> {
        if self.failing {
            return Err("the provider is down".to_string());
        }
        Ok(self.vectors.get(text).cloned().or(Some(vec![1.0, 0.0, 0.0])))
    }
}

/// A reflector that answers with a fixed JSON body.
struct FixedReflector {
    reply: String,
}

#[async_trait]
impl Reflector for FixedReflector {
    async fn complete(
        &self,
        _system: &str,
        _user: &str,
    ) -> std::result::Result<String, String> {
        Ok(self.reply.clone())
    }
}

struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "agent-turn-test-{name}-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(root.join("state")).expect("state directory");
        std::fs::create_dir_all(root.join("agent")).expect("agent directory");
        Self { root }
    }

    fn store(&self) -> MemoryStore {
        MemoryStore::open(&self.root.join("state"), &self.root.join("agent")).expect("the store opens")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A turn captures its message, a later turn retrieves it, and the retrieved
/// content is the content that was captured.
#[tokio::test]
async fn a_captured_turn_is_retrieved_by_a_later_turn() {
    let scratch = Scratch::new("capture");
    let store = scratch.store();
    let embedder = FixedEmbedder::new(EMBEDDING_MODEL)
        .with("turn off the bedroom lights", vec![1.0, 0.0, 0.0]);
    let mut assembler = PromptAssembler::new();

    let indexed = capture_turn(
        &store,
        Some(&embedder),
        "message-1",
        "turn off the bedroom lights",
    )
    .await
    .expect("the turn is captured");
    assert!(indexed, "the turn was indexed");

    let turn = prepare_turn(
        &store,
        &mut assembler,
        Some(&embedder),
        "turn off the bedroom lights",
        None,
    )
    .await;
    assert!(!turn.degraded);
    assert_eq!(turn.memory.len(), 1);
    assert_eq!(turn.memory[0].body, "turn off the bedroom lights");
    assert!(
        turn.prompt.system.contains("turn off the bedroom lights"),
        "the retrieved memory is in the prompt: {}",
        turn.prompt.system
    );
}

/// Reflection fires on its boundary and not off it. The counter comes from the
/// persisted log, so a restart does not reset it.
#[tokio::test]
async fn reflection_fires_on_its_boundary_and_not_off_it() {
    let scratch = Scratch::new("reflect");
    let store = scratch.store();
    let embedder = FixedEmbedder::new(EMBEDDING_MODEL);
    let reflector = FixedReflector {
        reply: r#"{"facts":[{"body":"the user lives in Oslo","tags":["place"]}],"traits":["answers concisely"]}"#
            .to_string(),
    };

    for index in 0..(REFLECTION_INTERVAL as usize - 1) {
        capture_turn(&store, Some(&embedder), &format!("m{index}"), "hello")
            .await
            .expect("the turn is captured");
    }
    assert!(
        maybe_reflect(&store, Some(&reflector), "user: hello")
            .await
            .is_none(),
        "nine turns is not a boundary"
    );

    capture_turn(&store, Some(&embedder), "last", "hello")
        .await
        .expect("the turn is captured");
    let summary = maybe_reflect(&store, Some(&reflector), "user: hello")
        .await
        .expect("the tenth turn is a boundary");
    assert_eq!(summary.facts, 1);
    assert_eq!(summary.traits, 1);

    // The fact landed as a row and as markdown, and the traits landed in the
    // personality file.
    let facts = store.list_facts().expect("the facts");
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].body, "the user lives in Oslo");
    assert!(store.load_personality().contains("answers concisely"));

    // A restart does not reset the counter: the log is what counts, so the
    // same boundary applies to a store that was reopened.
    let reopened = scratch.store();
    assert_eq!(
        reopened.count_user_turns().expect("the count"),
        REFLECTION_INTERVAL
    );
    assert!(
        maybe_reflect(&reopened, Some(&reflector), "user: hello")
            .await
            .is_some(),
        "the boundary is the same after a restart, because the log is what counts"
    );

    // The next turn is not a boundary.
    capture_turn(&reopened, Some(&embedder), "next", "hello")
        .await
        .expect("the turn is captured");
    assert!(
        maybe_reflect(&reopened, Some(&reflector), "user: hello")
            .await
            .is_none(),
        "the eleventh turn is not a boundary"
    );
}

/// A provider failure degrades to an empty memory block and the conversation
/// continues.
#[tokio::test]
async fn a_provider_failure_degrades_to_an_empty_memory_block() {
    let scratch = Scratch::new("degrade");
    let store = scratch.store();
    let broken = FixedEmbedder::new(EMBEDDING_MODEL).broken();
    let mut assembler = PromptAssembler::new();

    let turn = prepare_turn(&store, &mut assembler, Some(&broken), "anything", None).await;
    assert!(turn.degraded, "the turn says it degraded");
    assert!(turn.memory.is_empty());
    assert!(
        !turn.prompt.system.contains("# Memory"),
        "no memory block at all: {}",
        turn.prompt.system
    );

    // The capture still records the turn, so the row is there for a later
    // re-index even though it could not be embedded.
    let indexed = capture_turn(&store, Some(&broken), "m1", "a turn worth keeping")
        .await
        .expect("the turn is captured");
    assert!(!indexed, "the row was recorded but not indexed");
    assert_eq!(store.count_user_turns().expect("the count"), 1);
}

/// With no credential there is no embedder at all, and the turn goes out
/// without memory rather than failing.
#[tokio::test]
async fn a_turn_without_a_provider_still_goes_out() {
    let scratch = Scratch::new("nokey");
    let store = scratch.store();
    let mut assembler = PromptAssembler::new();
    let turn = prepare_turn(&store, &mut assembler, None, "anything", None).await;
    assert!(turn.degraded);
    assert_eq!(turn.prompt.system, "");
}
