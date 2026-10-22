//! The memory corpus, proven against the implementation it was ported from.
//!
//! Ported from `tests/memory-retrieval.test.ts`. The same cases, the same
//! claims. The TypeScript version shares one database across the whole file;
//! here each test gets its own store, so the counts are the same but the
//! isolation is stricter.

use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;

use super::*;

/// An embedder that answers from a fixed table, so the ranking is the code's
/// doing rather than the provider's.
struct FixedEmbedder {
    model: String,
    vectors: HashMap<String, Vec<f32>>,
    /// The answer for anything not named in the table. `None` is "the provider
    /// had no vector to give", which is a skip rather than a failure.
    default: Option<Vec<f32>>,
    /// Bodies that always fail, the way a provider outage does.
    failing: Vec<String>,
    /// Bodies the provider declines to embed.
    declined: Vec<String>,
}

impl FixedEmbedder {
    fn new(model: &str) -> Self {
        Self {
            model: model.to_string(),
            vectors: HashMap::new(),
            default: Some(vec![1.0, 0.0, 0.0]),
            failing: Vec::new(),
            declined: Vec::new(),
        }
    }

    /// An embedder that never produces a vector.
    fn without_vectors(model: &str) -> Self {
        Self {
            default: None,
            ..Self::new(model)
        }
    }

    fn with(mut self, body: &str, vector: Vec<f32>) -> Self {
        self.vectors.insert(body.to_string(), vector);
        self
    }

    fn failing(mut self, body: &str) -> Self {
        self.failing.push(body.to_string());
        self
    }

    fn declining(mut self, body: &str) -> Self {
        self.declined.push(body.to_string());
        self
    }
}

#[async_trait]
impl Embedder for FixedEmbedder {
    fn model(&self) -> &str {
        &self.model
    }

    async fn embed(&self, text: &str) -> std::result::Result<Option<Vec<f32>>, String> {
        if self.failing.iter().any(|body| text.contains(body)) {
            return Err("provider down".to_string());
        }
        if self.declined.iter().any(|body| text.contains(body)) {
            return Ok(None);
        }
        Ok(self
            .vectors
            .get(text)
            .cloned()
            .or_else(|| self.default.clone()))
    }
}

const MODEL: &str = "openai/text-embedding-3-small";

struct Scratch {
    state: PathBuf,
    agent: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "agent-memory-test-{name}-{}",
            uuid::Uuid::new_v4()
        ));
        let state = root.join("state");
        let agent = root.join("agent");
        std::fs::create_dir_all(&state).expect("state directory");
        std::fs::create_dir_all(&agent).expect("agent directory");
        Self { state, agent }
    }

    fn store(&self) -> MemoryStore {
        MemoryStore::open(&self.state, &self.agent).expect("the store opens")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Some(root) = self.state.parent() {
            let _ = std::fs::remove_dir_all(root);
        }
    }
}

/// Records a fact and indexes it under `model`, returning its id.
fn index_fact(store: &MemoryStore, body: &str, tags: &str, vector: &[f32], model: &str) -> String {
    let entry = store.record_fact(body, Some(tags)).expect("the fact is recorded");
    store
        .set_entry_embedding(&entry.id, vector, model)
        .expect("the vector is stored");
    entry.id
}

fn bodies(entries: &[MemoryEntry]) -> Vec<String> {
    entries.iter().map(|entry| entry.body.clone()).collect()
}

#[tokio::test]
async fn ranks_by_cosine_similarity_within_the_role_card_scope() {
    let scratch = Scratch::new("rank");
    let store = scratch.store();
    index_fact(
        &store,
        "bedroom is room 2 with 3 lights",
        "rank:home",
        &[1.0, 0.0, 0.0],
        MODEL,
    );
    index_fact(
        &store,
        "user prefers oat milk",
        "rank:grocery",
        &[0.0, 1.0, 0.0],
        MODEL,
    );
    index_fact(
        &store,
        "desk lamp is on the left",
        "rank:home",
        &[0.8, 0.2, 0.0],
        MODEL,
    );

    let embedder = FixedEmbedder::new(MODEL).with("lights", vec![1.0, 0.0, 0.0]);
    let hits = store
        .retrieve_memory(&RetrievalOptions {
            query: "lights".to_string(),
            scope: Some("rank:home".to_string()),
            embedder: &embedder,
            embedding_model: Some(MODEL.to_string()),
            limit: None,
            token_budget: None,
        })
        .await
        .expect("retrieval");

    assert_eq!(
        hits.first().map(|hit| hit.body.as_str()),
        Some("bedroom is room 2 with 3 lights")
    );
    let mut sorted = bodies(&hits);
    sorted.sort();
    assert_eq!(
        sorted,
        vec!["bedroom is room 2 with 3 lights", "desk lamp is on the left"]
    );
}

#[tokio::test]
async fn an_unscoped_caller_sees_every_entry_regardless_of_tags() {
    let scratch = Scratch::new("unscoped");
    let store = scratch.store();
    index_fact(
        &store,
        "bedroom is room 2 with 3 lights",
        "rank:home",
        &[1.0, 0.0, 0.0],
        MODEL,
    );
    index_fact(
        &store,
        "user prefers oat milk",
        "rank:grocery",
        &[0.0, 1.0, 0.0],
        MODEL,
    );

    let embedder = FixedEmbedder::new(MODEL).with("lights", vec![1.0, 0.0, 0.0]);
    let hits = store
        .retrieve_memory(&RetrievalOptions {
            query: "lights".to_string(),
            scope: None,
            embedder: &embedder,
            embedding_model: Some(MODEL.to_string()),
            limit: None,
            token_budget: None,
        })
        .await
        .expect("retrieval");

    let found = bodies(&hits);
    assert!(found.contains(&"bedroom is room 2 with 3 lights".to_string()));
    assert!(found.contains(&"user prefers oat milk".to_string()));
}

#[tokio::test]
async fn never_mixes_vector_spaces_a_different_embedding_model_is_invisible() {
    let scratch = Scratch::new("spaces");
    let store = scratch.store();
    index_fact(
        &store,
        "stale vector from an old model",
        "model:home",
        &[1.0, 0.0, 0.0],
        "old/model-v1",
    );

    let embedder = FixedEmbedder::new(MODEL).with("lights", vec![1.0, 0.0, 0.0]);
    let hits = store
        .retrieve_memory(&RetrievalOptions {
            query: "lights".to_string(),
            scope: Some("model:home".to_string()),
            embedder: &embedder,
            embedding_model: Some(MODEL.to_string()),
            limit: None,
            token_budget: None,
        })
        .await
        .expect("retrieval");

    assert!(hits.is_empty(), "a foreign vector space is invisible");
}

#[tokio::test]
async fn respects_the_entry_count_limit() {
    let scratch = Scratch::new("limit");
    let store = scratch.store();
    let long = "y".repeat(200);
    for index in 0..12 {
        index_fact(
            &store,
            &format!("capped fact {index} {long}"),
            "limit:bulk",
            &[1.0, 0.0, 0.0],
            MODEL,
        );
    }

    let embedder = FixedEmbedder::new(MODEL).with("bulk", vec![1.0, 0.0, 0.0]);
    let capped = store
        .retrieve_memory(&RetrievalOptions {
            query: "bulk".to_string(),
            scope: Some("limit:bulk".to_string()),
            embedder: &embedder,
            embedding_model: Some(MODEL.to_string()),
            limit: Some(3),
            token_budget: None,
        })
        .await
        .expect("retrieval");

    assert_eq!(capped.len(), 3);
}

#[tokio::test]
async fn respects_the_token_budget() {
    let scratch = Scratch::new("budget");
    let store = scratch.store();
    let long = "z".repeat(400);
    for index in 0..12 {
        index_fact(
            &store,
            &format!("budget fact {index} {long}"),
            "budget:bulk",
            &[1.0, 0.0, 0.0],
            MODEL,
        );
    }

    let embedder = FixedEmbedder::new(MODEL).with("bulk", vec![1.0, 0.0, 0.0]);
    let budgeted = store
        .retrieve_memory(&RetrievalOptions {
            query: "bulk".to_string(),
            scope: Some("budget:bulk".to_string()),
            embedder: &embedder,
            embedding_model: Some(MODEL.to_string()),
            limit: None,
            token_budget: Some(120),
        })
        .await
        .expect("retrieval");

    let spent: usize = budgeted
        .iter()
        .map(|hit| hit.body.chars().count().div_ceil(CHARS_PER_TOKEN))
        .sum();
    assert!(!budgeted.is_empty());
    assert!(spent <= 120, "spent {spent} of the budget");
}

#[tokio::test]
async fn returns_nothing_when_the_query_cannot_be_embedded() {
    let scratch = Scratch::new("noembed");
    let store = scratch.store();
    let embedder = FixedEmbedder::without_vectors(MODEL);
    let hits = store
        .retrieve_memory(&RetrievalOptions {
            query: "anything".to_string(),
            scope: None,
            embedder: &embedder,
            embedding_model: None,
            limit: None,
            token_budget: None,
        })
        .await
        .expect("retrieval");
    assert!(hits.is_empty());
}

/// Re-index is corpus-global, so these assert per-row outcomes rather than
/// global counts: a fact is retrievable on the new model only if it was
/// re-embedded, and a fact that failed keeps its old (hidden) vector.
#[tokio::test]
async fn re_embeds_foreign_model_and_missing_rows_and_a_failed_embed_stays_unindexed() {
    let scratch = Scratch::new("reindex");
    let store = scratch.store();

    let stale = store
        .record_fact("reindex stale foreign model row", Some("reindex:foreign"))
        .expect("the fact is recorded");
    store
        .set_entry_embedding(&stale.id, &[0.0, 0.0, 1.0], "old/model-v1")
        .expect("the vector is stored");
    let missing = store
        .record_fact("reindex row with no vector", Some("reindex:missing"))
        .expect("the fact is recorded");
    let broken = store
        .record_fact("reindex row whose embed always fails", Some("reindex:broken"))
        .expect("the fact is recorded");
    let current = store
        .record_fact("reindex row already on new model", Some("reindex:current"))
        .expect("the fact is recorded");
    store
        .set_entry_embedding(&current.id, &[0.0, 1.0, 0.0], MODEL)
        .expect("the vector is stored");

    let embedder = FixedEmbedder::new(MODEL)
        .with("reindex stale foreign model row", vec![1.0, 0.0, 0.0])
        .with("reindex row with no vector", vec![1.0, 0.0, 0.0])
        .with("reindex row already on new model", vec![1.0, 0.0, 0.0])
        .failing("always fails");
    let result = store.reindex_memory(&embedder).await.expect("the reindex");

    assert!(
        result.failed.contains(&broken.id),
        "the failed row is reported, not silently dropped"
    );
    // Two rows could be embedded; the third always fails. The original
    // asserted a count of three or more because its corpus was shared with
    // the earlier tests, so the number carried rows this test did not create.
    // The claim that matters is per row, and the assertions below are that.
    assert_eq!(result.reindexed, 2, "reindexed {}", result.reindexed);

    let hits = store
        .retrieve_memory(&RetrievalOptions {
            query: "reindex".to_string(),
            scope: Some("reindex:foreign".to_string()),
            embedder: &embedder,
            embedding_model: Some(MODEL.to_string()),
            limit: None,
            token_budget: None,
        })
        .await
        .expect("retrieval");
    assert!(hits.iter().any(|hit| hit.id == stale.id));

    let missing_hits = store
        .retrieve_memory(&RetrievalOptions {
            query: "reindex".to_string(),
            scope: Some("reindex:missing".to_string()),
            embedder: &embedder,
            embedding_model: Some(MODEL.to_string()),
            limit: None,
            token_budget: None,
        })
        .await
        .expect("retrieval");
    assert!(missing_hits.iter().any(|hit| hit.id == missing.id));

    let broken_hits = store
        .retrieve_memory(&RetrievalOptions {
            query: "reindex".to_string(),
            scope: Some("reindex:broken".to_string()),
            embedder: &embedder,
            embedding_model: Some(MODEL.to_string()),
            limit: None,
            token_budget: None,
        })
        .await
        .expect("retrieval");
    assert!(
        !broken_hits.iter().any(|hit| hit.id == broken.id),
        "a fact whose embed failed is not retrievable on the new model"
    );

    let current_hits = store
        .retrieve_memory(&RetrievalOptions {
            query: "reindex".to_string(),
            scope: Some("reindex:current".to_string()),
            embedder: &embedder,
            embedding_model: Some(MODEL.to_string()),
            limit: None,
            token_budget: None,
        })
        .await
        .expect("retrieval");
    assert!(current_hits.iter().any(|hit| hit.id == current.id));
}

#[tokio::test]
async fn a_null_embed_skips_a_row_and_never_clears_its_existing_vector() {
    let scratch = Scratch::new("null-embed");
    let store = scratch.store();
    let entry = store
        .record_fact("reindex null embed survivor", Some("reindex:null"))
        .expect("the fact is recorded");
    store
        .set_entry_embedding(&entry.id, &[0.0, 0.0, 1.0], "old/model-v1")
        .expect("the vector is stored");

    let embedder = FixedEmbedder::new(MODEL).declining("null embed survivor");
    let result = store.reindex_memory(&embedder).await.expect("the reindex");
    assert_eq!(result.skipped, 1);

    // Still indexed, but under the old model: invisible to new-model retrieval.
    let hidden = store
        .retrieve_memory(&RetrievalOptions {
            query: "null embed survivor".to_string(),
            scope: Some("reindex:null".to_string()),
            embedder: &embedder,
            embedding_model: Some(MODEL.to_string()),
            limit: None,
            token_budget: None,
        })
        .await
        .expect("retrieval");
    assert!(!hidden.iter().any(|hit| hit.id == entry.id));

    // Visible again to a caller still on the old model: the vector survived.
    let old_embedder = FixedEmbedder::new("old/model-v1")
        .with("null embed survivor", vec![0.0, 0.0, 1.0]);
    let visible_to_old = store
        .retrieve_memory(&RetrievalOptions {
            query: "null embed survivor".to_string(),
            scope: Some("reindex:null".to_string()),
            embedder: &old_embedder,
            embedding_model: Some("old/model-v1".to_string()),
            limit: None,
            token_budget: None,
        })
        .await
        .expect("retrieval");
    assert!(visible_to_old.iter().any(|hit| hit.id == entry.id));
}

/// The fact is the human source; the row is the index. Both must exist, or a
/// person editing the markdown has edited nothing.
#[test]
fn a_fact_is_written_as_markdown_beside_the_row() {
    let scratch = Scratch::new("mirror");
    let store = scratch.store();
    let entry = store
        .record_fact("the user prefers oat milk", Some("food, drink"))
        .expect("the fact is recorded");

    let path = scratch.agent.join(entry.file_path.clone().expect("a file path"));
    let text = std::fs::read_to_string(&path).expect("the fact file exists");
    assert!(text.contains("the user prefers oat milk"), "{text}");
    assert!(text.contains("tags: food, drink"), "{text}");

    // A revision keeps the history rather than overwriting it.
    let revision = store
        .revise_fact(&entry.id, "the user prefers almond milk", None)
        .expect("the revision is recorded");
    assert_eq!(revision, 2);
    let revisions = store.list_fact_revisions(&entry.id).expect("the revisions");
    assert_eq!(revisions.len(), 2);
    assert_eq!(revisions[0].body, "the user prefers oat milk");
}

/// The personality file records when a trait was *first* observed, not when it
/// was last touched. A trait that survives a rewrite keeps its date.
#[test]
fn the_personality_file_keeps_the_date_a_trait_was_first_observed() {
    let scratch = Scratch::new("personality");
    let store = scratch.store();
    let first = store
        .save_personality(&["answers concisely".to_string()])
        .expect("the personality is saved");
    assert!(first.contains("answers concisely"), "{first}");

    let second = store
        .save_personality(&[
            "answers concisely".to_string(),
            "asks before acting".to_string(),
        ])
        .expect("the personality is saved");
    let first_date = first
        .lines()
        .find(|line| line.contains("answers concisely"))
        .and_then(|line| line.split(']').next())
        .map(|part| part.trim_start_matches("- [").to_string())
        .expect("the first date");
    assert!(
        second.contains(&format!("- [{first_date}] answers concisely")),
        "the surviving trait keeps its original date: {second}"
    );
    assert!(second.contains("asks before acting"), "{second}");
}
