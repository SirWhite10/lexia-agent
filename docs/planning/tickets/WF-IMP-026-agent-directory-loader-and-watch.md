---
kind: implementation
status: planned
---

# WF-IMP-026: Load and watch the agent directory

## Goal

Make the host read `lexosa-agent/` at start and apply changes to it while it runs, so an agent's identity, prompt, memory, skills, sub-agents, credentials and plugin pointers can all change without a restart or a rebuild.

## Depends on

- [WF-IMP-019](WF-IMP-019-rust-agent-binary-and-model-layer.md) for the binary and its state directory
- [WF-IMP-021](WF-IMP-021-wasmtime-plugin-host.md) for plugin generation pointers

## The contract is already written

`lexosa-agent/README.md` is the specification, including which paths are watched and which are not, and why. Do not renegotiate it during implementation; if it is wrong, fix it there first.

The short version:

| Watched | Not watched |
| --- | --- |
| `agent.toml`, `instructions/`, `memory/facts/`, `skills/`, `subagents/`, `config/`, `plugins/registry/`, `plugins/active/` | `plugins/versions/`, `workspace/`, `cache/`, `logs/` |

The unwatched set is not a configuration choice. `plugins/versions/` and `cache/` are written by this host, and watching a path the host writes to means reacting to its own writes. `workspace/` is the agent's space, not a configuration surface.

## Scope

- Read and validate `agent.toml` at start; refuse to start an agent whose manifest is invalid rather than running it with defaults. An agent silently running with a different prompt than its file says is worse than one that refuses.
- Watch the paths above and apply changes on the event. No polling — the whole point of a watch is that it is an event.
- Re-embed changed facts and update the vector index in place, invalidating rows whose embedding model no longer matches rather than leaving them to compete.
- Enter a new skill into the index the moment its directory appears.
- Version a revised sub-agent prompt rather than overwriting it, so "what did it run with?" stays answerable.
- Re-read provider and model configuration, with the environment credential still outranking the stored file.
- React to a plugin generation pointer change by promoting or rolling back through the path [WF-IMP-021](WF-IMP-021-wasmtime-plugin-host.md) defines.
- Resolve a skill whose required plugin is absent through the load-or-create path rather than failing the turn.

## Two failure modes worth designing for

**A partial write.** An editor saving a manifest can produce a read of a truncated file. A watcher that applies every event will happily load half a manifest. Debounce, and validate before applying — a rejected change leaves the previous state in force and says so, rather than dropping the agent to defaults.

**A burst.** A `git checkout` touches every file at once. Applying each event separately would re-embed the whole memory index several times. Coalesce a burst, then apply once.

## Out of scope

- Creating an agent directory. `scripts/agent.ts new` does that, and its output is what this ticket loads.
- The prompt cache itself, which is [WF-IMP-022](WF-IMP-022-memory-and-skills-in-rust.md)'s measurement.
- Plugin compilation and validation, which are [WF-IMP-021](WF-IMP-021-wasmtime-plugin-host.md)'s.

## Acceptance criteria

- Editing `instructions/core.md` changes the next turn's prompt, with no restart.
- Adding a skill directory makes the skill available to the next turn.
- Editing a fact re-embeds exactly that fact; the rest of the index is untouched.
- Revising a sub-agent's prompt creates a version; the prior version is still readable.
- Changing a provider credential or a model mapping takes effect without a restart.
- Promoting a plugin generation through `plugins/active/` changes what the next action loads.
- A burst of changes — a checkout touching the whole tree — applies once, not once per file.
- A truncated or invalid manifest is rejected and the previous state stays in force, with the reason reported.
- Nothing the host writes to `cache/`, `logs/` or `plugins/versions/` triggers a reload.
- The watch contract in `lexosa-agent/README.md` and this implementation agree; if they diverge, the ticket is the stale one.

## Progress

Not started. The directory, the template and the tooling exist; only the host side that reads them is missing.