---
kind: decision
status: complete
---

# WF-DEC-002: Adopt the Jarvis-shaped memory, personality, and sub-agent model

## Goal

Replace the abandoned "many chats, many top-level sessions" model with one main agent, one massive chat, growing memory and personality, and Lexosa-authored sub-agents — and settle how each is stored and injected so the prompt prefix stays cache-stable. The product framing is a Jarvis-like experience: the user talks to one main agent that spins up sub-agents for multitasking and parallel commands.

## Decision

**Conversation.** One main agent (Lexosa) and one massive chat per installation, single user; the `users` table remains authentication-only. ADR 0002's run/action contract is unchanged: a run references the originating user message. Sub-agent work surfaces in the chat as compact delegation entries, streamed progress, and summaries; full transcripts live in run detail.

**Memory.** The corpus is the verbatim user-message log (the `messages` table) plus distilled facts the agent maintains. Storage follows EVE's authored files-and-folders structure mirrored under the mutable home `app-data/state/agent/…`; facts and personality are human-editable markdown there, while `app-data/state/lexia.sqlite` holds the vector index and tags. Every vector row stores its embedding model id. Retrieval is vector search via OpenRouter embeddings (default `openai/text-embedding-3-small`), role-scoped for sub-agents, with an injection budget of roughly ten entries / 1.5k tokens.

**Writes.** The agent writes and updates distilled facts automatically at turn boundaries. Every change is versioned, so the corpus is inspectable and revertible rather than approve-first.

**Personality.** One markdown file of timestamped trait entries. The agent self-reflects every ~10 turns or at session end (whichever comes first) and appends or rewrites traits freely.

**Prompt order.** The assembled system prompt is `[personality → all skills → retrieved memory → resolved tools]`. Everything above the memory block is byte-stable across turns; skills are always complete, never per-turn selected; personality rewrites are batched at boundaries so the cached prefix re-warms once per change. Jev classifies before capabilities are resolved and injected.

**Sub-agents.** A *standing sub-agent* ("forever") is distinct from a one-shot *agent task* (dies with its run, auto-archives; standing sub-agents persist until explicitly retired). Lexosa authors each sub-agent's system prompt at spawn and may revise it later, versioned, so "what did it run with?" is answerable. The role card is: authored system prompt + focus (its assignment) + memory scope + capability allowlist. Sub-agents run their own personality — derived from Lexosa through Lexosa's authored prompt — and receive role-scoped retrieval plus explicit hand-off notes. Sub-agents may message each other: commands are structured action requests in the action model, chat is plain text, and both are routed through the run coordinator. The main chat stays filtered to user↔Lexosa plus delegation entries and summaries.

**Surfaces.** The chat is home. `/agents` becomes sub-agent management: role cards, focus, standing/retired state, prompt versions, and agent-task history. Delete becomes archive everywhere.

**Legacy.** The old `agents` table (kind `chat`/`agent`, `parent_id`) and its scratch rows are dropped.

## Flagged deviation

`docs/planning/agent-capabilities-v1.md` says memory grows as "user-approved" memory through a reviewable update lifecycle. This decision replaces approve-first writes with automatic, versioned, revertible writes. The baseline wording must be aligned to match; the skills-side review lifecycle remains an open baseline decision.

## Scope boundary

This decision covers conversation shape, memory corpus and write policy, personality, prompt assembly order, sub-agent identity and communication, and the archive-over-delete rule. It does not settle embedding-provider choice beyond the default, the skills update lifecycle, or multi-user support.

## Completion criteria

- The canonical glossary (`CONTEXT.md`) records the settled terms: massive chat, memory fact, personality trait, standing sub-agent, role card, focus, command, and archive.
- The baseline deviation above is recorded rather than silent.
- Implementation proceeds through the tickets below.

## Follow-up

- [WF-INV-001](WF-INV-001-eve-authored-structure-inventory.md) for the exact EVE tree.
- [WF-IMP-003](WF-IMP-003-schema-cutover-messages-sub-agents-memory.md) → [WF-IMP-004](WF-IMP-004-memory-capture-distillation-and-retrieval.md) → [WF-IMP-005](WF-IMP-005-cache-stable-prompt-assembly.md); [WF-IMP-006](WF-IMP-006-sub-agent-management-surface.md) for the manager surface.
- Promote this decision to ADR-0004 if desired: the data model is hard to reverse, surprising without context, and a real trade-off. Left to an explicit call. (0003 was taken by [the EVE development runtime](../../adr/0003-eve-development-runtime.md).)
