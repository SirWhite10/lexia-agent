---
kind: implementation
status: implemented
---

# WF-IMP-005: Assemble prompts in cache-stable order

## Goal

Make prompt assembly follow the [WF-DEC-002](WF-DEC-002-memory-personality-and-sub-agent-model.md) slot order so provider prompt caching hits on every turn that does not change the prefix: `[personality → all skills → retrieved memory → resolved tools]`.

## Depends on

- [WF-DEC-002](WF-DEC-002-memory-personality-and-sub-agent-model.md)
- [WF-IMP-004](WF-IMP-004-memory-capture-distillation-and-retrieval.md)

## Scope

- Build the EVE-side prompt assembler with four fixed slots; everything above the memory slot is byte-stable across turns.
- Load the complete skill set every turn; never select skills per request, and never reorder them.
- Append retrieved memory, then the capabilities Jev's classification resolved, as the volatile suffix.
- Gate personality writes to the reflection boundaries so a trait change re-warms the cache once instead of per turn.
- Assemble sub-agent prompts from the Lexia-authored prompt version plus role-scoped memory; the main Lexia identity block is never injected into a sub-agent prompt.
- Record a cheap prefix digest with each assembled prompt so a test can assert prefix stability.

## Out of scope

- Provider cache dashboards or accounting UI.
- Prompt-optimization research beyond the cache contract.

## Acceptance criteria

- Two consecutive turns with no personality or skill change produce an identical prefix digest; only the suffix differs.
- A memory write changes only the volatile suffix; a personality rewrite is the only event that changes the prefix, and it does so at a boundary.
- Skills are injected in full and in a stable order on every turn.
- A standing sub-agent's assembled prompt equals its authored prompt version plus scoped retrieved memory, and differs from Lexia's own prompt.
