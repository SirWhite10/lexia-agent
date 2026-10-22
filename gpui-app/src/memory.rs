//! The memory corpus (WF-IMP-004), ported by WF-IMP-022.
//!
//! Verbatim user-message log entries and distilled facts, each fact mirrored
//! as human-editable markdown, with a vector index the host owns. The markdown
//! is the human source; the row is the index.
//!
//! The layout follows the agent directory's own rule. Facts grow one file per
//! thing, so they live in `memory/facts/`. The personality is a single record
//! that grows by lines, so it is one file beside them. The vector index is
//! host-owned state, so it lives in the state directory and is never authored.
//!
//! Two behaviours here are load-bearing and are easy to lose in a port:
//!
//! - **Retrieval never mixes vector spaces.** A row embedded under one model
//!   is invisible to a caller using another. Comparing them returns confident
//!   nonsense, which is worse than returning nothing.
//! - **A provider failure costs one row, not the corpus.** Embeddings are
//!   computed one body at a time, so one failure leaves the rest indexed.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use async_trait::async_trait;
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{Error, Result};

#[cfg(test)]
mod tests;

/// Where the personality file lives, relative to the agent directory.
pub const PERSONA_FILE: &str = "memory/personality.md";

const DEFAULT_RETRIEVAL_LIMIT: usize = 10;
const DEFAULT_TOKEN_BUDGET: usize = 1500;
/// Deterministic token estimate; a real tokenizer would only move the cap slightly.
const CHARS_PER_TOKEN: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryKind {
    Fact,
    Log,
}

impl MemoryKind {
    fn as_str(self) -> &'static str {
        match self {
            MemoryKind::Fact => "fact",
            MemoryKind::Log => "log",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "fact" => Some(MemoryKind::Fact),
            "log" => Some(MemoryKind::Log),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub id: String,
    pub kind: MemoryKind,
    pub source_message_id: Option<String>,
    pub body: String,
    pub tags: String,
    pub file_path: Option<String>,
    pub embedding_model: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct MemoryFactRevision {
    pub entry_id: String,
    pub revision: i64,
    pub body: String,
    pub created_at: String,
}

/// What the caller needs to embed a body.
///
/// `Ok(None)` is "the provider had no vector to give", which is a skip.
/// `Err` is a failure, which costs one row.
#[async_trait]
pub trait Embedder: Send + Sync {
    /// The model id rows are stamped with. Retrieval compares against it, so
    /// a row embedded under another model is invisible.
    fn model(&self) -> &str;
    async fn embed(&self, text: &str) -> std::result::Result<Option<Vec<f32>>, String>;
}

/// An embedder that goes through the provider abstraction.
///
/// The corpus is never indexed against one vendor's endpoint by accident: the
/// embedding goes through the same `ModelProvider` the completions do, so a
/// provider that cannot serve embeddings is a provider the host reports rather
/// than a second, hidden integration.
pub struct ProviderEmbedder<'a> {
    provider: &'a dyn crate::models::ModelProvider,
    credential: crate::error::Secret,
    model: String,
}

impl<'a> ProviderEmbedder<'a> {
    pub fn new(
        provider: &'a dyn crate::models::ModelProvider,
        credential: crate::error::Secret,
        model: &str,
    ) -> Self {
        Self {
            provider,
            credential,
            model: model.to_string(),
        }
    }
}

#[async_trait]
impl Embedder for ProviderEmbedder<'_> {
    fn model(&self) -> &str {
        &self.model
    }

    async fn embed(&self, text: &str) -> std::result::Result<Option<Vec<f32>>, String> {
        self.provider
            .embed(&self.credential, &self.model, text)
            .await
            .map(Some)
            .map_err(|error| error.to_string())
    }
}

#[derive(Debug, Default)]
pub struct ReindexResult {
    pub reindexed: usize,
    pub skipped: usize,
    pub failed: Vec<String>,
}

#[derive(Clone)]
pub struct RetrievalOptions<'a> {
    pub query: String,
    /// Comma-separated role card scope; empty is the main agent (unscoped).
    pub scope: Option<String>,
    pub embedder: &'a dyn Embedder,
    /// The model the caller's vector belongs to. Defaults to the embedder's.
    pub embedding_model: Option<String>,
    pub limit: Option<usize>,
    pub token_budget: Option<usize>,
}

/// The memory store: the index, the facts and the personality.
pub struct MemoryStore {
    /// One writer, as the run store has. The connection is behind a lock
    /// because a transaction needs `&mut`, and the store is shared.
    connection: Mutex<Connection>,
    agent_dir: PathBuf,
    facts_dir: PathBuf,
    personality_path: PathBuf,
}

impl MemoryStore {
    /// Open the store. The index lives in the state directory; the facts and
    /// the personality live in the agent's own directory.
    pub fn open(state_root: &Path, agent_dir: &Path) -> Result<Self> {
        let memory_dir = state_root.join("memory");
        fs::create_dir_all(&memory_dir)
            .map_err(|e| Error::State(format!("could not create {}: {e}", memory_dir.display())))?;
        let database = Connection::open(memory_dir.join("memory.sqlite"))
            .map_err(|e| Error::State(format!("could not open the memory index: {e}")))?;
        database
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA foreign_keys = ON;",
            )
            .map_err(|e| Error::State(format!("could not configure the memory index: {e}")))?;
        database
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS memory_entries (
                    id TEXT PRIMARY KEY,
                    kind TEXT NOT NULL CHECK (kind IN ('fact', 'log')),
                    source_message_id TEXT,
                    body TEXT NOT NULL,
                    tags TEXT NOT NULL DEFAULT '',
                    file_path TEXT,
                    embedding BLOB,
                    embedding_model TEXT,
                    created_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS memory_revisions (
                    entry_id TEXT NOT NULL REFERENCES memory_entries(id),
                    revision INTEGER NOT NULL,
                    body TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    PRIMARY KEY (entry_id, revision)
                );",
            )
            .map_err(|e| Error::State(format!("could not create the memory schema: {e}")))?;

        let facts_dir = agent_dir.join("memory").join("facts");
        let personality_path = agent_dir.join(PERSONA_FILE);
        Ok(Self {
            connection: Mutex::new(database),
            agent_dir: agent_dir.to_path_buf(),
            facts_dir,
            personality_path,
        })
    }

    /// The agent directory this store writes into.
    pub fn agent_dir(&self) -> &Path {
        &self.agent_dir
    }

    /// The connection, locked. A poisoned lock is recovered rather than
    /// propagated: the data is consistent, and a panic in one caller should
    /// not take the store down for the rest.
    fn connection(&self) -> MutexGuard<'_, Connection> {
        self.connection.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// One timestamp source for every write; callers never format their own.
    fn now() -> String {
        Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
    }

    /// One verbatim user-message log entry; pass the embedding once it is
    /// computed.
    pub fn record_log_entry(
        &self,
        source_message_id: Option<&str>,
        body: &str,
        tags: Option<&str>,
        embedding: Option<&[f32]>,
        embedding_model: Option<&str>,
    ) -> Result<MemoryEntry> {
        let entry = MemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            kind: MemoryKind::Log,
            source_message_id: source_message_id.map(str::to_string),
            body: body.to_string(),
            tags: tags.unwrap_or_default().trim().to_string(),
            file_path: None,
            embedding_model: embedding_model.map(str::to_string),
            created_at: Self::now(),
        };
        self.connection()
            .execute(
                "INSERT INTO memory_entries (id, kind, source_message_id, body, tags, file_path, embedding, embedding_model, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    entry.id,
                    entry.kind.as_str(),
                    entry.source_message_id,
                    entry.body,
                    entry.tags,
                    entry.file_path,
                    embedding.map(vector_bytes),
                    entry.embedding_model,
                    entry.created_at,
                ],
            )
            .map_err(sqlite)?;
        Ok(entry)
    }

    pub fn set_entry_embedding(&self, id: &str, embedding: &[f32], model: &str) -> Result<()> {
        self.connection()
            .execute(
                "UPDATE memory_entries SET embedding = ?1, embedding_model = ?2 WHERE id = ?3",
                params![vector_bytes(embedding), model, id],
            )
            .map_err(sqlite)?;
        Ok(())
    }

    /// Number of captured user turns, the reflection trigger. Derived from the
    /// persisted log so the cadence survives restarts without scheduler state.
    pub fn count_user_turns(&self) -> Result<u64> {
        let count: i64 = self
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM memory_entries WHERE kind = 'log'",
                [],
                |row| row.get(0),
            )
            .map_err(sqlite)?;
        Ok(count.max(0) as u64)
    }

    /// The markdown mirror is the human source; the row is the index.
    fn write_fact_file(&self, id: &str, body: &str, tags: &str, created_at: &str) -> Result<String> {
        fs::create_dir_all(&self.facts_dir)
            .map_err(|e| Error::State(format!("could not create the facts directory: {e}")))?;
        let relative_path = format!("memory/facts/{id}.md");
        let file = format!("---\ntags: {tags}\nupdated: {created_at}\n---\n\n{body}\n");
        fs::write(self.agent_dir.join(&relative_path), file)
            .map_err(|e| Error::State(format!("could not write the fact file: {e}")))?;
        Ok(relative_path)
    }

    /// Distilled facts start at revision 1 and gain revisions instead of overwrites.
    pub fn record_fact(&self, body: &str, tags: Option<&str>) -> Result<MemoryEntry> {
        let entry = MemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            kind: MemoryKind::Fact,
            source_message_id: None,
            body: body.to_string(),
            tags: tags.unwrap_or_default().trim().to_string(),
            file_path: None,
            embedding_model: None,
            created_at: Self::now(),
        };
        let file_path = self.write_fact_file(&entry.id, &entry.body, &entry.tags, &entry.created_at)?;
        let mut connection = self.connection();
        let transaction = connection
            .transaction()
            .map_err(|e| Error::State(format!("could not begin the fact transaction: {e}")))?;
        transaction
            .execute(
                "INSERT INTO memory_entries (id, kind, source_message_id, body, tags, file_path, embedding, embedding_model, created_at)
                 VALUES (?1, ?2, NULL, ?3, ?4, ?5, NULL, ?6, ?7)",
                params![
                    entry.id,
                    entry.kind.as_str(),
                    entry.body,
                    entry.tags,
                    file_path,
                    entry.embedding_model,
                    entry.created_at,
                ],
            )
            .map_err(sqlite)?;
        transaction
            .execute(
                "INSERT INTO memory_revisions (entry_id, revision, body, created_at) VALUES (?1, 1, ?2, ?3)",
                params![entry.id, entry.body, entry.created_at],
            )
            .map_err(sqlite)?;
        transaction
            .commit()
            .map_err(|e| Error::State(format!("could not commit the fact: {e}")))?;
        Ok(MemoryEntry {
            file_path: Some(file_path),
            ..entry
        })
    }

    pub fn revise_fact(&self, id: &str, body: &str, tags: Option<&str>) -> Result<i64> {
        let next: i64 = self
            .connection()
            .query_row(
                "SELECT COALESCE(MAX(revision), 0) + 1 FROM memory_revisions WHERE entry_id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(sqlite)?;
        let stamp = Self::now();
        // The transaction is scoped, so the connection is released before the
        // row is read back below. A lock taken twice is a deadlock: the mutex
        // is not reentrant, and the second request would wait for the first.
        {
            let mut connection = self.connection();
            let transaction = connection
                .transaction()
                .map_err(|e| Error::State(format!("could not begin the revision: {e}")))?;
            transaction
                .execute(
                    "UPDATE memory_entries SET body = ?2, tags = COALESCE(?3, tags), file_path = COALESCE(file_path, ?4) WHERE id = ?1",
                    params![id, body, tags.map(|t| t.trim().to_string()), format!("memory/facts/{id}.md")],
                )
                .map_err(sqlite)?;
            transaction
                .execute(
                    "INSERT INTO memory_revisions (entry_id, revision, body, created_at) VALUES (?1, ?2, ?3, ?4)",
                    params![id, next, body, stamp],
                )
                .map_err(sqlite)?;
            transaction
                .commit()
                .map_err(|e| Error::State(format!("could not commit the revision: {e}")))?;
        }

        // The markdown is rewritten from the row, so the file and the index
        // cannot drift.
        if let Some(row) = self.entry_row(id)? {
            self.write_fact_file(id, &row.body, &row.tags, &row.created_at)?;
        }
        Ok(next)
    }

    pub fn list_facts(&self) -> Result<Vec<MemoryEntry>> {
        let connection = self.connection();
        let mut statement = connection
            .prepare(
                "SELECT id, kind, source_message_id, body, tags, file_path, embedding_model, created_at
                 FROM memory_entries WHERE kind = 'fact' ORDER BY created_at DESC",
            )
            .map_err(sqlite)?;
        let facts = statement
            .query_map([], row_to_entry)
            .map_err(sqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite)?;
        Ok(facts)
    }

    pub fn list_fact_revisions(&self, id: &str) -> Result<Vec<MemoryFactRevision>> {
        let connection = self.connection();
        let mut statement = connection
            .prepare(
                "SELECT entry_id, revision, body, created_at FROM memory_revisions WHERE entry_id = ?1 ORDER BY revision ASC",
            )
            .map_err(sqlite)?;
        let revisions = statement
            .query_map(params![id], |row| {
                Ok(MemoryFactRevision {
                    entry_id: row.get(0)?,
                    revision: row.get(1)?,
                    body: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .map_err(sqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite)?;
        Ok(revisions)
    }

    fn entry_row(&self, id: &str) -> Result<Option<MemoryEntry>> {
        self.connection()
            .query_row(
                "SELECT id, kind, source_message_id, body, tags, file_path, embedding_model, created_at
                 FROM memory_entries WHERE id = ?1",
                params![id],
                row_to_entry,
            )
            .optional()
            .map_err(sqlite)
    }

    /// The injected personality block; empty before the first reflection.
    pub fn load_personality(&self) -> String {
        fs::read_to_string(&self.personality_path).unwrap_or_default()
    }

    /// Rewrites the trait file, keeping the original date on traits that
    /// survive, so the file records when each trait was first observed rather
    /// than when it was touched.
    pub fn save_personality(&self, traits: &[String]) -> Result<String> {
        if let Some(parent) = self.personality_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| Error::State(format!("could not create the memory directory: {e}")))?;
        }
        let mut previous: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for line in self.load_personality().lines() {
            if let Some(rest) = line.strip_prefix("- [")
                && let Some((stamp, trait_text)) = rest.split_once("] ")
            {
                previous.insert(trait_text.to_string(), stamp.to_string());
            }
        }
        let stamp = Self::now();
        let mut lines: Vec<String> = vec!["# Agent personality".to_string(), String::new()];
        for trait_text in traits {
            let trait_text = trait_text.trim();
            if trait_text.is_empty() {
                continue;
            }
            let when = previous
                .get(trait_text)
                .cloned()
                .unwrap_or_else(|| stamp.clone());
            lines.push(format!("- [{when}] {trait_text}"));
        }
        lines.push(String::new());
        let file = lines.join("\n");
        fs::write(&self.personality_path, &file)
            .map_err(|e| Error::State(format!("could not write the personality file: {e}")))?;
        Ok(file)
    }

    /// Re-embeds every row whose vector is missing or was produced by a
    /// different model. Retrieval already hides foreign-model rows, so a changed
    /// embedding model degrades to "no memory" rather than to confident
    /// nonsense; this is the recovery path that fills the gap again. Bodies are
    /// re-embedded one at a time, so a single provider failure costs one row.
    pub async fn reindex_memory(&self, embedder: &dyn Embedder) -> Result<ReindexResult> {
        let model = embedder.model();
        let rows: Vec<(String, String)> = {
            let connection = self.connection();
            let mut statement = connection
                .prepare(
                    "SELECT id, body FROM memory_entries
                     WHERE embedding IS NULL OR embedding_model IS NULL OR embedding_model <> ?1",
                )
                .map_err(sqlite)?;
            statement
                .query_map(params![model], |row| -> rusqlite::Result<(String, String)> {
                    Ok((row.get(0)?, row.get(1)?))
                })
                .map_err(sqlite)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(sqlite)?
        };

        let mut result = ReindexResult::default();
        for (id, body) in rows {
            match embedder.embed(&body).await {
                Ok(Some(vector)) => {
                    self.set_entry_embedding(&id, &vector, model)?;
                    result.reindexed += 1;
                }
                Ok(None) => result.skipped += 1,
                Err(_) => result.failed.push(id),
            }
        }
        Ok(result)
    }

    /// Role-scoped vector retrieval ranked by cosine similarity. Rows embedded
    /// with a different model are invisible: vector spaces are not comparable,
    /// and mixing them returns confident nonsense. Budget-capped by a
    /// deterministic token estimate; a single oversized entry is truncated
    /// rather than dropped.
    pub async fn retrieve_memory(
        &self,
        options: &RetrievalOptions<'_>,
    ) -> Result<Vec<MemoryEntry>> {
        let query_vector = match options.embedder.embed(&options.query).await {
            Ok(Some(vector)) => vector,
            // A provider that cannot embed leaves the conversation without
            // memory rather than broken. The caller logs the reason.
            Ok(None) => return Ok(Vec::new()),
            Err(reason) => {
                return Err(Error::State(format!("the embedding provider failed: {reason}")));
            }
        };
        let model = options
            .embedding_model
            .clone()
            .unwrap_or_else(|| options.embedder.model().to_string());

        let scope_tags: Vec<String> = options
            .scope
            .as_deref()
            .unwrap_or("")
            .split(',')
            .map(|tag| tag.trim().to_string())
            .filter(|tag| !tag.is_empty())
            .collect();

        let rows: Vec<(MemoryEntry, Vec<f32>)> = {
            let connection = self.connection();
            let mut statement = connection
                .prepare(
                    "SELECT id, kind, source_message_id, body, tags, file_path, embedding_model, created_at, embedding
                     FROM memory_entries WHERE embedding IS NOT NULL",
                )
                .map_err(sqlite)?;
            let raw = statement
                .query_map([], |row| {
                    let entry = row_to_entry(row)?;
                    let blob: Vec<u8> = row.get(8)?;
                    Ok((entry, blob))
                })
                .map_err(sqlite)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(sqlite)?;
            raw.into_iter()
                .map(|(entry, blob)| (entry, bytes_to_vector(&blob)))
                .collect()
        };

        let mut ranked: Vec<(MemoryEntry, f32)> = Vec::new();
        for (entry, vector) in rows {
            if !model.is_empty() && entry.embedding_model.as_deref() != Some(model.as_str()) {
                continue;
            }
            if !scope_tags.is_empty() {
                let entry_tags: Vec<String> = entry
                    .tags
                    .split(',')
                    .map(|tag| tag.trim().to_string())
                    .filter(|tag| !tag.is_empty())
                    .collect();
                if !scope_tags.iter().any(|tag| entry_tags.contains(tag)) {
                    continue;
                }
            }
            if vector.len() != query_vector.len() {
                continue;
            }
            let mut dot = 0.0_f32;
            let mut query_norm = 0.0_f32;
            let mut entry_norm = 0.0_f32;
            for index in 0..vector.len() {
                dot += vector[index] * query_vector[index];
                query_norm += query_vector[index] * query_vector[index];
                entry_norm += vector[index] * vector[index];
            }
            if query_norm == 0.0 || entry_norm == 0.0 {
                continue;
            }
            let score = dot / (query_norm.sqrt() * entry_norm.sqrt());
            ranked.push((entry, score));
        }
        // A stable sort keeps the order of equally-scored rows the order they
        // were read in, so a tie does not shuffle between calls.
        ranked.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let limit = options.limit.unwrap_or(DEFAULT_RETRIEVAL_LIMIT);
        let budget = options.token_budget.unwrap_or(DEFAULT_TOKEN_BUDGET);
        let mut results: Vec<MemoryEntry> = Vec::new();
        let mut spent = 0_usize;
        for (entry, _) in ranked.into_iter().take(limit) {
            let tokens = entry.body.chars().count().div_ceil(CHARS_PER_TOKEN);
            if spent + tokens > budget {
                let remaining = budget - spent;
                if remaining > 0 {
                    let mut truncated = entry.clone();
                    truncated.body = entry.body.chars().take(remaining * CHARS_PER_TOKEN).collect();
                    results.push(truncated);
                }
                break;
            }
            spent += tokens;
            results.push(entry);
        }
        Ok(results)
    }
}

fn sqlite(error: rusqlite::Error) -> Error {
    Error::State(format!("memory store: {error}"))
}


/// The instruction the reflection model is given. Ported verbatim: the model
/// is asked for JSON only, and the rules are part of the contract.
pub const REFLECTION_SYSTEM: &str = "You maintain Agent's long-term memory. Reply with JSON only, no prose and no code fences:\n{\"facts\": [{\"body\": \"one concise durable fact about the user or their world\", \"tags\": [\"comma\",\"separated\"]}], \"traits\": [\"one personality trait Agent has observed\"]}\nRules: facts are durable (preferences, facts about the user's home, work, people), never transient chit-chat, never secrets; reuse the wording of an existing fact when you revise it; traits are short lowercase phrases describing how Agent should behave for this user.";

/// A model call the reflection makes. The caller supplies it, so the memory
/// store never holds a provider credential of its own.
#[async_trait]
pub trait Reflector: Send + Sync {
    async fn complete(
        &self,
        system: &str,
        user: &str,
    ) -> std::result::Result<String, String>;
}

#[derive(Debug, Clone)]
pub struct ReflectionResult {
    pub facts: Vec<MemoryEntry>,
    pub traits: Vec<String>,
    /// The personality file as it was rewritten.
    pub file: String,
}

/// One reflection step: the model proposes distilled facts and the full trait
/// set; facts land as versioned rows plus markdown, traits rewrite the
/// personality file.
pub async fn reflect(
    store: &MemoryStore,
    reflector: &dyn Reflector,
    transcript: &str,
    known_facts: &str,
) -> Result<ReflectionResult> {
    let reply = reflector
        .complete(
            REFLECTION_SYSTEM,
            &format!(
                "Known memory facts:\n{}\n\nRecent conversation:\n{}",
                if known_facts.is_empty() { "(none)" } else { known_facts },
                if transcript.is_empty() { "(none)" } else { transcript }
            ),
        )
        .await
        .map_err(|reason| Error::State(format!("the reflection model failed: {reason}")))?;

    // The model is asked for JSON only, but it is a model: the first brace to
    // the last one is taken, and a reply with no object in it is an error
    // rather than a silent no-op.
    let start = reply
        .find('{')
        .ok_or_else(|| Error::State("the reflection returned no JSON object".into()))?;
    let end = reply
        .rfind('}')
        .ok_or_else(|| Error::State("the reflection returned no JSON object".into()))?;
    if end < start {
        return Err(Error::State("the reflection returned no JSON object".into()));
    }
    let parsed: serde_json::Value = serde_json::from_str(&reply[start..=end])
        .map_err(|e| Error::State(format!("the reflection returned unreadable JSON: {e}")))?;

    let known = store.list_facts()?;
    let mut facts: Vec<MemoryEntry> = Vec::new();
    if let Some(proposed) = parsed.get("facts").and_then(|value| value.as_array()) {
        for item in proposed {
            let body = item
                .get("body")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .trim()
                .to_string();
            if body.is_empty() {
                continue;
            }
            let tags = item
                .get("tags")
                .and_then(|value| value.as_array())
                .map(|values| {
                    values
                        .iter()
                        .filter_map(|value| value.as_str())
                        .collect::<Vec<&str>>()
                        .join(",")
                })
                .unwrap_or_default();
            match known.iter().find(|fact| fact.body == body) {
                // The wording of an existing fact is reused, so the fact gains
                // a revision rather than a second row.
                Some(prior) => {
                    store.revise_fact(&prior.id, &body, Some(&tags))?;
                    facts.push(MemoryEntry {
                        body,
                        tags,
                        file_path: prior.file_path.clone(),
                        ..prior.clone()
                    });
                }
                None => facts.push(store.record_fact(&body, Some(&tags))?),
            }
        }
    }

    let traits: Vec<String> = parsed
        .get("traits")
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let file = store.save_personality(&traits)?;
    Ok(ReflectionResult {
        facts,
        traits,
        file,
    })
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryEntry> {
    let kind: String = row.get(1)?;
    Ok(MemoryEntry {
        id: row.get(0)?,
        kind: MemoryKind::parse(&kind).unwrap_or(MemoryKind::Log),
        source_message_id: row.get(2)?,
        body: row.get(3)?,
        tags: row.get(4)?,
        file_path: row.get(5)?,
        embedding_model: row.get(6)?,
        created_at: row.get(7)?,
    })
}

/// The stored bytes are little-endian f32, the same layout the original wrote.
fn vector_bytes(vector: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vector.len() * 4);
    for value in vector {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn bytes_to_vector(bytes: &[u8]) -> Vec<f32> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}
