---
kind: decision
status: complete
---

# WF-DEC-005: Where memory lives, and how it is injected

## Goal

Settle two questions that [WF-INV-001](WF-INV-001-eve-authored-structure-inventory.md) surfaced and that are still open. They are recorded here because a footnote in a completed investigation is not a place a decision can live, and because one of them is a straight contradiction between two accepted decisions.

## Depends on

- [WF-DEC-002](WF-DEC-002-memory-personality-and-sub-agent-model.md) — the decision this one amends
- [WF-INV-001](WF-INV-001-eve-authored-structure-inventory.md) — the findings below

## The contradiction

Two accepted decisions disagree, and only one of them can hold.

**[WF-DEC-002](WF-DEC-002-memory-personality-and-sub-agent-model.md)** fixes the system prompt order as `[personality → all skills → retrieved memory → resolved tools]`, with memory injected at the **bottom of the system prompt**. The stated reason is prompt-cache reuse: everything above the memory block is byte-stable across turns, so the cached prefix re-warms once per change rather than once per turn.

**[WF-INV-001](WF-INV-001-eve-authored-structure-inventory.md)** found that EVE ships a first-class memory-slot system and injects recalled content as **user-role messages attributed to the slot, never as system instructions**.

These are incompatible. A recalled memory cannot be both a system-prompt block inside the cached prefix and a user-role turn attributed to a slot. One of them is wrong, and which one is wrong changes the prompt assembly in `src/lib/server/prompt.ts`.

## The ownership question

The same investigation found that **memory has no authored home in EVE**. A memory slot is code — `agent/memory/<slot>.ts` — that binds a *provider* to a scope, and the provider decides where the bytes land. Lexosa's `app-data/state/agent/…` mirror corresponds to no EVE path.

So the second question: does Lexosa keep the pipeline it already has, or hand the lifecycle to EVE?

## What is already built

Recorded here so the decision starts from facts, not from a blank page. [WF-IMP-003](WF-IMP-003-schema-cutover-messages-sub-agents-memory.md) → [WF-IMP-004](WF-IMP-004-memory-capture-distillation-and-retrieval.md) → [WF-IMP-005](WF-IMP-005-cache-stable-prompt-assembly.md) are `implemented`, and the loop is verified against the live OpenRouter API: `captureTurn` writes the verbatim log entry and embeds it, `prepareTurn` retrieves role-scoped memory and assembles the cache-stable prompt, and `maybeReflect` fires on a turn boundary. The scenario in [WF-IMP-004](WF-IMP-004-memory-capture-distillation-and-retrieval.md) was exercised live, not only through injected fakes.

Adopting EVE's contract means giving up ownership of a working, verified pipeline. That is a real cost and should be weighed as one.

## The options

**A. Keep the cache-stable system-prompt injection; override EVE.** Lexosa owns the turn lifecycle for memory. The prompt prefix stays stable and the cache saving is real. The cost is that EVE's memory-slot mechanism is bypassed, and any future EVE upgrade that assumes its own injection would have to be re-checked against this override every time.

**B. Adopt EVE's `MemoryProvider` and user-role injection.** EVE owns recall and capture. The cost is direct: prompt-cache reuse is lost for the memory region, and the verified pipeline from IMP-003/004/005 is replaced by an integration with a contract that has one implementation to model on and no second example in the tree.

**C. Split the two.** System-prompt block for the stable prefix, EVE slots for anything EVE itself recalls. The cost is two memory systems with two storage models and two injection paths, and a rule for deciding which memory a given fact belongs to — which is a worse problem than either option above.

## What to record when decided

- The choice, with its reason.
- Whether EVE's memory-slot mechanism is bypassed, and where that override is documented so the next EVE upgrade check has something to read.
- If the answer changes the prompt order in `src/lib/server/prompt.ts`, whether [WF-DEC-002](WF-DEC-002-memory-personality-and-sub-agent-model.md) is amended or superseded.
- Whether [WF-DEC-002](WF-DEC-002-memory-personality-and-sub-agent-model.md) should be promoted to an ADR. It asked for that itself: the data model is hard to reverse, surprising without context, and a real trade-off. Answered below.

## Out of scope

- Memory editing. Facts remain human-editable markdown.
- Retrieval quality. This is about ownership and injection, not about what is retrieved.

## Resolution

**Resolved by removal, not by choosing.** [ADR 0005](../../adr/0005-rust-agent-host-and-wasm-plugins.md) removes EVE, and with it the second runtime that wanted to own memory. There is now one owner — the Rust binary — so the contradiction has no second side to be incompatible with.

The cache-stable system-prompt injection from [WF-DEC-002](WF-DEC-002-memory-personality-and-sub-agent-model.md) stands. It was always the bet that prompt-cache reuse was worth more than EVE's slot attribution, and that bet is now the only position rather than one of two.

What it costs is stated plainly: it was a bet, not a measurement. [WF-IMP-022](WF-IMP-022-memory-and-skills-in-rust.md) measures the actual cache behaviour once the model layer runs, rather than leaving the assumption standing because it was comfortable.

The ADR question this ticket carried is also answered: [WF-DEC-002](WF-DEC-002-memory-personality-and-sub-agent-model.md) is **not** promoted, because the agent model it describes now lives in [ADR 0005](../../adr/0005-rust-agent-host-and-wasm-plugins.md) and the runtime that contradicted it is gone. Keeping a second record of a superseded model would be two sources of truth.


Nothing here changes [WF-IMP-003](WF-IMP-003-schema-cutover-messages-sub-agents-memory.md) → [WF-IMP-004](WF-IMP-004-memory-capture-distillation-and-retrieval.md) → [WF-IMP-005](WF-IMP-005-cache-stable-prompt-assembly.md). They are `implemented` and verified against the live provider, and they are now the specification the Rust port is written against.

## Progress

Complete. Raised as an open contradiction between two accepted decisions, and closed when the second runtime behind it was removed.