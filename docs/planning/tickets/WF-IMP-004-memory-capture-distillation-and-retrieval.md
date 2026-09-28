---
kind: implementation
status: implemented
---

# WF-IMP-004: Build the memory capture, distillation, and retrieval pipeline

## Goal

Make the memory corpus real: every user message is embedded and retrievable, the agent distills durable facts from conversation into the mirrored file tree, and a user request retrieves exactly the memory and capabilities it needs.

## Depends on

- [WF-DEC-002](WF-DEC-002-memory-personality-and-sub-agent-model.md)
- [WF-IMP-003](WF-IMP-003-schema-cutover-messages-sub-agents-memory.md)

## Scope

- On message write, embed the text through OpenRouter's `/api/v1/embeddings` (server-side only; configured model, default `openai/text-embedding-3-small`) and store the vector with its model id in `memory_entries` as kind `log`.
- Run the reflection step at turn boundaries (~10 turns or session end): distill concise user facts, write them to markdown in the mirrored state tree, and upsert `memory_entries`/`memory_revisions` accordingly.
- Implement role-scoped retrieval: filter by the requesting identity's memory scope/tags, rank by vector similarity, and return within the ~10-entry / ~1.5k-token budget.
- Use `sqlite-vec` when the extension loads under `bun:sqlite`; otherwise store vectors as BLOBs and rank in process. Either path records the embedding model id per row and triggers a re-index when the configured model changes.
- Surface the bedroom-lights scenario end to end: "turn off the lights in the bedroom" retrieves the stored fact (room 2, light count) alongside the relevant smart-home capability.

## Out of scope

- Skills mutation and its review lifecycle.
- A user-facing memory editor (facts stay human-editable files; the editor is a later slice).
- Retrieval-quality tuning beyond a deterministic cap.

## Acceptance criteria

- A user message is retrievable by meaning, not only by keyword, and its embedding model id is queryable.
- Distilled facts are readable as markdown, versioned in SQLite, and revertible to a prior revision.
- Sub-agent retrieval never returns entries outside its role card's memory scope.
- A configured embedding-model change re-indexes the corpus instead of mixing vector spaces.
- Embedding calls run server-side; the browser receives no provider credentials.

## Progress

All acceptance criteria are implemented and covered by `bun test` (15 tests, including the re-index recovery path and the never-mix-vector-spaces invariant).

One caveat is deliberate, not an omission: the provider call shape is exercised through injected fakes, not a live `OPENROUTER_API_KEY` round-trip, because no key is configured on this host. The embedding and reflection functions take the provider call as an injected dependency precisely so the pipeline holds no credential of its own; wiring them to `embedOpenRouterText` and a chat model call is the runtime's job.

**Update — the live provider round-trip is now verified.** With `OPENROUTER_API_KEY` set in `.env`, a real `openai/text-embedding-3-small` request returned 1536 dimensions in ~194ms, and a role-scoped retrieval using that live vector ranked "bedroom is room 2 with 3 lights" first for the query "turn off the lights in the bedroom" — the exact scenario in this ticket's scope. A live chat completion also returned cleanly through the existing client. The injected-fake tests remain as the deterministic regression net.

**Update — the loop is wired, not just built.** `src/lib/server/turn.ts` binds the live provider to the pipeline and is called by the chat action: `captureTurn` writes the verbatim log entry and embeds it, `prepareTurn` retrieves role-scoped memory and assembles the cache-stable prompt, and `maybeReflect` fires on a turn boundary (every 10th turn, derived from the persisted log so it survives restarts). Verified live: a chat turn embedding its message, a later turn retrieving it, and the reflection gate correctly declining off-boundary. Provider failures degrade to an empty memory block rather than breaking the conversation.
