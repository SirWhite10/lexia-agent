//! The per-turn seam (WF-IMP-004 + WF-IMP-005), ported by WF-IMP-022.
//!
//! This is where the live provider is bound to the pure pieces: embed the turn,
//! retrieve role-scoped memory, assemble the cache-stable prompt, and — at
//! reflection boundaries — let the model distil new facts and traits. Without
//! this module the pipeline exists but no request ever reaches it.
//!
//! Two rules run through the whole file, and both are load-bearing:
//!
//! - **A provider failure degrades, it does not break the turn.** Retrieval
//!   that cannot embed leaves the conversation without memory, and a capture
//!   that cannot embed leaves the row for a later re-index. The turn proceeds.
//! - **Reflection fires on a boundary, not per turn.** The trigger is derived
//!   from the persisted log, so it survives a restart and needs no scheduler
//!   state.

use crate::memory::{reflect, Embedder, MemoryEntry, MemoryStore, Reflector};

#[cfg(test)]
mod tests;
use crate::prompt::{AssembledPrompt, PromptAssembler, PromptSlots};

/// The embedding model the corpus is indexed with.
pub const EMBEDDING_MODEL: &str = "openai/text-embedding-3-small";
/// The model that distils facts and traits.
pub const REFLECTION_MODEL: &str = "openai/gpt-4o-mini";
/// WF-DEC-002: reflect every ~10 turns, never on every turn.
pub const REFLECTION_INTERVAL: u64 = 10;

#[derive(Debug, Clone)]
pub struct Turn {
    pub prompt: AssembledPrompt,
    pub memory: Vec<MemoryEntry>,
    /// True when no memory was injected because the provider was unavailable.
    pub degraded: bool,
}

/// Builds the system prompt for a user turn. The personality block is the only
/// slot that changes between reflection boundaries, so the cache prefix stays
/// byte-stable while memory varies per request.
///
/// The skills are injected in full, in the prefix, and the tools slot is
/// empty because nothing populates it yet: the resolved tools belong to the
/// agent loop
/// ([WF-IMP-023](../../docs/planning/tickets/WF-IMP-023-agent-loop-sub-agents-and-builtins.md)),
/// which is what decides what a turn may do.
pub async fn prepare_turn(
    store: &MemoryStore,
    assembler: &mut PromptAssembler,
    embedder: Option<&dyn Embedder>,
    skills: &[String],
    query: &str,
    scope: Option<&str>,
) -> Turn {
    let Some(embedder) = embedder else {
        // No credential, so no memory. The turn still goes out.
        return Turn {
            prompt: assembler.assemble(&PromptSlots::default()),
            memory: Vec::new(),
            degraded: true,
        };
    };
    let options = crate::memory::RetrievalOptions {
        query: query.to_string(),
        scope: scope.map(str::to_string),
        embedder,
        embedding_model: Some(EMBEDDING_MODEL.to_string()),
        limit: None,
        token_budget: None,
    };
    match store.retrieve_memory(&options).await {
        Ok(memory) => {
            let bodies: Vec<String> = memory.iter().map(|entry| entry.body.clone()).collect();
            Turn {
                prompt: assembler.assemble(&PromptSlots {
                    personality: Some(store.load_personality()),
                    skills: skills.to_vec(),
                    memory: bodies,
                    tools: Vec::new(),
                }),
                memory,
                degraded: false,
            }
        }
        // Retrieval must never take a turn down: a provider failure degrades
        // to an empty memory block rather than a broken conversation.
        Err(_) => Turn {
            prompt: assembler.assemble(&PromptSlots::default()),
            memory: Vec::new(),
            degraded: true,
        },
    }
}

/// Captures a user turn into the verbatim log and embeds it for later retrieval.
///
/// Returns whether the turn was indexed. A false answer means the row exists
/// but has no vector yet; a later re-index picks it up.
pub async fn capture_turn(
    store: &MemoryStore,
    embedder: Option<&dyn Embedder>,
    message_id: &str,
    body: &str,
) -> Result<bool, crate::error::Error> {
    let entry = store.record_log_entry(Some(message_id), body, None, None, None)?;
    let Some(embedder) = embedder else {
        return Ok(false);
    };
    match embedder.embed(body).await {
        Ok(Some(vector)) => {
            store.set_entry_embedding(&entry.id, &vector, embedder.model())?;
            Ok(true)
        }
        // The row exists and is picked up by a later re-index; the turn
        // proceeds.
        Ok(None) | Err(_) => Ok(false),
    }
}

/// What a reflection produced.
#[derive(Debug, Clone)]
pub struct ReflectionSummary {
    pub facts: usize,
    pub traits: usize,
}

/// Reflection runs on a turn boundary, not per turn. The trigger is derived from
/// the persisted log count, so it survives restarts and needs no scheduler
/// state. Returns `None` when this turn is not a boundary, and a failed
/// reflection never breaks the turn that triggered it.
///
/// The transcript is supplied by the caller: the agent's message log belongs to
/// the agent loop, and the boundary is the only thing this module decides.
pub async fn maybe_reflect(
    store: &MemoryStore,
    reflector: Option<&dyn Reflector>,
    transcript: &str,
) -> Option<ReflectionSummary> {
    let reflector = reflector?;
    let turns = store.count_user_turns().ok()?;
    if turns == 0 || turns % REFLECTION_INTERVAL != 0 {
        return None;
    }
    match reflect(store, reflector, transcript, "").await {
        Ok(result) => Some(ReflectionSummary {
            facts: result.facts.len(),
            traits: result.traits.len(),
        }),
        // A failed reflection never breaks the turn that triggered it.
        Err(_) => None,
    }
}
