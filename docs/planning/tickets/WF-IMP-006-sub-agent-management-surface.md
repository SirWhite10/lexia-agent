---
kind: implementation
status: implemented
---

# WF-IMP-006: Turn /agents into the sub-agent management surface

## Goal

Replace the chat-cards page with the sub-agent manager implied by the one-main-agent model: standing sub-agents with full role cards, Lexosa-authored prompt versions, focus, lifecycle controls, and the agent-task history from the run model.

## Depends on

- [WF-DEC-002](WF-DEC-002-memory-personality-and-sub-agent-model.md)
- [WF-IMP-003](WF-IMP-003-schema-cutover-messages-sub-agents-memory.md)
- [WF-IMP-001](WF-IMP-001-durable-intent-run-vertical-slice.md) (agent-task history)

## Scope

- List standing sub-agents as role cards: name, focus, memory scope, capability allowlist, status, and current prompt version.
- Edit a role card; revising the system prompt creates a new version, with prior versions viewable.
- Spawn flow: Lexosa authors the sub-agent's system prompt and role card; the host persists version 1.
- Retire and archive controls; deletion everywhere archives rather than removes.
- One-shot agent tasks rendered as history rows sourced from the run/action model (status, originating command, outcome).

## Out of scope

- The chat surface redesign.
- The peer-traffic and run-detail viewer (full transcripts stay in run detail per [WF-DEC-002](WF-DEC-002-memory-personality-and-sub-agent-model.md)).
- Memory editing (facts remain markdown files).

## Acceptance criteria

- A standing sub-agent shows its complete role card and prompt version history; a revision is a new version, not an overwrite.
- Retiring a sub-agent archives it and it remains queryable; no destructive delete path remains.
- Agent-task history lists one-shot delegations with status and outcome from the run model.
- The page renders an explicit empty state before any sub-agent exists, and `bun run check` passes.

## Progress

All acceptance criteria are implemented and verified in the running app.

- Role-card editing (focus, memory scope, capabilities), Lexosa-authored prompt versioning with visible history, rename with Enter/✓/Esc and inline errors, retire, and archive — all persisted and confirmed through the browser.
- **Agent-task history** now reads one-shot delegations from the run model added by [WF-IMP-001](WF-IMP-001-durable-intent-run-vertical-slice.md), with lifecycle status and outcome per task.
- **The chat surface exists**: a turn becomes a run, and the delegation entry streams action-level progress over SSE. The capture half of the loop is live — every user message lands in the memory log on send.

The retrieve-and-inject half still runs through injected fakes in `bun test` rather than a live provider: no `OPENROUTER_API_KEY` is configured on this host, so the embedding and reflection call shapes are covered by tests, not a real round-trip. That needs a key, not more code.

**Update — provider gap closed.** `OPENROUTER_API_KEY` is configured in `.env`; embeddings and chat completions were exercised against the live OpenRouter API, so the capture → retrieve loop is verified against the real provider, not only fakes.
