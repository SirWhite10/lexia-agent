---
kind: implementation
status: planned
---

# WF-IMP-022: Memory, skills and prompt assembly in Rust

## Goal

Move the capture → retrieve → assemble → reflect loop into the Rust binary, and give the agent a skills system that resolves at runtime rather than being compiled in.

## Depends on

- [WF-IMP-019](WF-IMP-019-rust-agent-binary-and-model-layer.md) for the model layer
- [WF-IMP-021](WF-IMP-021-wasmtime-plugin-host.md) for the capability interface these call through

## Reference behaviour

`src/lib/server/prompt.ts`, `turn.ts` and `memory.ts` are the specification, and `tests/memory-retrieval.test.ts` and `tests/prompt-assembly.test.ts` are the parity oracle. [WF-IMP-004](WF-IMP-004-memory-capture-distillation-and-retrieval.md) and [WF-IMP-005](WF-IMP-005-cache-stable-prompt-assembly.md) are `implemented` and verified against the live provider, so this is a port of working code, not a rewrite.

The loop is live today: `captureTurn` writes the verbatim log entry and embeds it, `prepareTurn` retrieves role-scoped memory and assembles the cache-stable prompt, and `maybeReflect` fires on a turn boundary derived from the persisted log, so it survives restarts.

## The injection decision is already made, by removal

[WF-DEC-005](WF-DEC-005-memory-injection-and-eve-memory-provider.md) recorded an open contradiction: EVE injected recalled content as **user-role messages attributed to a memory slot**, while [WF-DEC-002](WF-DEC-002-memory-personality-and-sub-agent-model.md) put it in the **system prompt** for prompt-cache reuse. That contradiction existed because two runtimes both wanted to own memory. With EVE removed there is one owner, so the cache-stable system-prompt injection stands and the question closes.

The system prompt order stays `[personality → all skills → retrieved memory → resolved tools]`, with everything above the memory block byte-stable across turns so the cached prefix re-warms once per change rather than once per turn.

That was the cache-first bet. State it as a bet: measure the actual cache behaviour once the model layer is running, rather than continuing to assume it.

## Storage

Facts and personality are human-editable markdown under the state directory; the vector index and tags stay in SQLite. Every vector row stores its embedding model id, and retrieval never mixes vector spaces — an index built with one model is not queried with another.

## Skills

Skills describe how and when capabilities are used. They resolve at runtime: the agent searches the skill index, a skill names the plugin it requires, the plugin is loaded, and the skill runs. A skill whose plugin is missing is a prompt for the self-extension loop in [WF-IMP-021](WF-IMP-021-wasmtime-plugin-host.md), not an error.

Skills live in the state directory as data, so adding one does not require rebuilding the binary.

## Scope

- The capture → retrieve → assemble → reflect loop, with role-scoped retrieval and the injection budget preserved.
- Embeddings through the `ModelProvider` abstraction, never against one vendor.
- The thought tree, which is what the router reads and what `/thinking` edits.
- The skill index and the resolve → require plugin → load → execute path.
- Provider failure behaviour: degrade to an empty memory block rather than breaking the conversation, which is what happens today.

## Out of scope

- Memory editing. Facts remain human-editable markdown.
- Retrieval quality. This is about moving working code, not improving it.
- The `/thinking` surface, which is client work ([WF-IMP-015](WF-IMP-015-port-thinking-workflows-and-agents-routes.md)).

## Acceptance criteria

- Both ported test files pass against the Rust implementation, ported rather than rewritten.
- A turn captures its message, a later turn retrieves it, and the retrieved content is the content that was captured.
- Reflection fires on its boundary and not off it, derived from the persisted log so a restart does not reset the counter.
- A provider failure degrades to an empty memory block and the conversation continues.
- Retrieval is role-scoped and never mixes vector spaces across embedding models.
- The system prompt keeps its byte-stable prefix across turns, and the cache behaviour is measured rather than assumed.
- A skill whose plugin is absent produces a load-or-create path, not a dead end.
- Adding a skill file changes agent behaviour with no rebuild.

## Progress

Not started.