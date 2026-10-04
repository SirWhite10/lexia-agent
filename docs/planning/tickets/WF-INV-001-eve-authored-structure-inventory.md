---
kind: investigation
status: complete
---

# WF-INV-001: Inventory EVE's authored files-and-folders structure

## Goal

Record the canonical EVE directory layout so Lexosa's mutable agent state (memory, personality, sub-agent prompts) can mirror it name-for-name instead of inventing folder names. Nothing on this installation defines it yet: `agent/` is planned in `PRODUCT.md`, `app-data/releases/` has no staged release, and the `eve` CLI is not installed here.

## Questions

- What is the canonical EVE tree, and which folders hold configuration, instructions, channels, tools, skills, and subagents?
- Which parts of that tree are authored release content (immutable per ADR 0001) versus mutable per-installation state?
- Where do a sub-agent's authored system prompt and per-installation memory/personality belong under that layout?
- What is a sub-agent's stable identity across releases: id, folder, or manifest entry?

## Method

- Locate the EVE source, release artifact, scaffold, or documentation that defines the layout.
- Record the tree verbatim with a one-line purpose per top-level folder.
- Map the mutable state home proposed in [WF-DEC-002](WF-DEC-002-memory-personality-and-sub-agent-model.md) onto the tree and flag any folder that would violate release immutability.

## Acceptance criteria

- The canonical tree is recorded in this ticket with an agreed mutable state home under `app-data/state/agent/…`.
- Names for memory, personality, and sub-agent prompt locations are fixed, or explicitly deferred with a named follow-up.
- [WF-IMP-003](WF-IMP-003-schema-cutover-messages-sub-agents-memory.md) no longer depends on unverified structure guesses.

## Findings

EVE is the `eve` npm package (`bunx eve@latest`, v0.66.3), authored by Vercel. `bunx eve@latest init my-agent` scaffolds the authored tree, and the full version-matched docs ship inside the package at `node_modules/eve/docs/`.

**Authored tree (single agent):**

```text
my-agent/
├── package.json          # "imports": { "#*": "./agent/*" }
├── agent/
│   ├── instructions.md   # the always-on system prompt
│   ├── agent.ts          # defineAgent({ model })
│   ├── tools/            # typed tools, defineTool + Zod inputSchema
│   ├── subagents/<name>/ # declared specialists, own prompts/tools/sandboxes
│   ├── memory.ts         # single memory slot
│   │                     #   OR memory/<slot>.ts for named slots (exclusive)
│   ├── connections/      # MCP + OpenAPI servers
│   ├── channels/         # how users reach the agent
│   └── skills/
├── evals/
└── apps/web/             # optional Next.js frontend
```

Several separately addressable agents instead use `agents/<name>/agent/` — a workspace sharing one package, dependency set, and deployment. A project with a root `agent/` is treated as single-agent even if `agents/` also exists.

**Answering the questions:**

- **Immutable vs mutable.** Everything above is authored release content. It ships in the release folder and is immutable per ADR 0001.
- **Memory has no authored home.** EVE does not store memory in the authored tree at all. A memory slot is code (`agent/memory/<slot>.ts`) that binds a *provider* to a scope; the provider decides storage, so memory lands in whatever backend the provider uses. Our `app-data/state/agent/…` mirror does not correspond to any EVE path.
- **Personality has no EVE concept.** The personality block belongs to `instructions.md`, which is authored and immutable — so a personality that grows per user cannot live there.
- **Sub-agent identity is the directory** `agent/subagents/<name>/`. EVE states specialists "do not inherit the parent's authored capabilities" and have no separate channel endpoint, which matches WF-DEC-002's model where Lexosa authors each sub-agent's system prompt.

## Consequence for WF-DEC-002

The decision recorded memory as an EVE-style files-and-folders mirror under `app-data/state/agent/…`. The authored tree does not support that: there is no memory directory to mirror, and no mutable slot for a growing personality. This does not invalidate what IMP-003/004 built — the per-row `file_path` column means the on-disk home can move without a schema change — but the premise that we were mirroring EVE's structure was wrong, and the layout should be re-decided against EVE's actual slot/provider split rather than a guessed tree.

## Follow-up

- EVE ships a first-class memory-slot system whose recall/capture lifecycle is owned by the runtime. Re-decide whether Lexosa keeps its own memory pipeline or implements EVE's `MemoryProvider` contract so EVE owns the turn lifecycle.
- EVE injects recalled content as **user-role messages attributed to the slot, never as system instructions**. WF-DEC-002 instead injects memory at the bottom of the system prompt for prompt-cache reuse. These are incompatible; the cache-first decision needs revisiting or an explicit override.
