---
kind: implementation
status: implemented
---

# WF-IMP-003: Cut the state schema over to messages, sub-agents, and memory

## Goal

Replace the `agents` table (kind `chat`/`agent`, `parent_id`) with the schema [WF-DEC-002](WF-DEC-002-memory-personality-and-sub-agent-model.md) implies, so the massive chat, sub-agents, and memory have a home before any UI depends on them.

## Depends on

- [WF-DEC-002](WF-DEC-002-memory-personality-and-sub-agent-model.md)
- [WF-INV-001](WF-INV-001-eve-authored-structure-inventory.md) (memory/personality file paths)

## Scope

- Create `messages` (`id`, `author_kind` user|lexia|sub_agent, `author_id` nullable, `addressee_id` nullable for agent↔agent chat, `run_id` nullable, `body`, `created_at`) as the one chat plus peer traffic.
- Create `sub_agents` (`id`, `name`, `focus`, `memory_scope`, `capabilities`, `status` standing|retired, `created_at`, `archived_at`).
- Create `sub_agent_prompt_versions` (`sub_agent_id`, `version`, `body`, `created_at`).
- Create `memory_entries` (`id`, `kind` fact|log, `source_message_id` nullable, `body`, `tags`, `file_path`, `embedding`, `embedding_model`, `created_at`) and `memory_revisions` (`entry_id`, `revision`, `body`, `created_at`).
- Drop the `agents` table and its scratch rows.
- Rewrite `src/lib/server/agents.ts` as the data layer for the new tables and remove its old exports outright; migrate every caller with no shims.
- Keep the current `/agents` page rendering against the new model (standing sub-agents only, empty state for now) until [WF-IMP-006](WF-IMP-006-sub-agent-management-surface.md).

## Out of scope

- `runs`/`actions`/`run_events` tables — owned by [WF-IMP-001](WF-IMP-001-durable-intent-run-vertical-slice.md).
- Embedding and retrieval behavior ([WF-IMP-004](WF-IMP-004-memory-capture-distillation-and-retrieval.md)), prompt assembly ([WF-IMP-005](WF-IMP-005-cache-stable-prompt-assembly.md)), the manager UX ([WF-IMP-006](WF-IMP-006-sub-agent-management-surface.md)), and the chat surface itself.

## Acceptance criteria

- `bun run check` passes and no reference to `AgentNode` or the `agents` table remains anywhere in `src/`.
- The data layer round-trips messages (user, Lexia, and addressed sub-agent entries), standing sub-agents, prompt versions, memory facts, and fact revisions.
- Each memory fact row points at its markdown file inside the [WF-INV-001](WF-INV-001-eve-authored-structure-inventory.md) tree.
- A prompt revision creates a new version rather than overwriting, and archived sub-agents remain queryable.
- The existing scratch data (three rows) is gone and the app runs against the new schema.
