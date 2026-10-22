# The agent data directory

`agents/default/` is the default agent's directory. Everything the agent knows lives here as data — nothing in it is compiled into the host, and changing it changes agent behaviour without a rebuild. That separation is the point: the host is permanent, this tree is not.

Per [ADR 0005](../../docs/adr/0005-rust-agent-host-and-wasm-plugins.md), this replaces `my-agent/`, which was the EVE project. The structure is different on purpose — EVE's was a package with a runtime inside it; this is a data tree read by a binary that does not change.

## Structure

```text
agents/default/
├── agent.toml      the manifest: everything that stays a single record
├── instructions/   system-prompt fragments, composed into the agent's prompt
├── memory/facts/   one file per durable fact; the vector index is host-owned
├── skills/         one directory per skill
├── subagents/      standing sub-agents, one directory each
├── plugins/        generation pointers and the immutable content-addressed store
└── workspace/      the sandboxed filesystem root for this agent
```

**The shape is hybrid, deliberately.** What grows one-file-per-thing is a directory, because a directory is where you put a file. What stays a single record — identity, prompt composition, memory scope, capabilities, sandbox, providers, the use-case → model mapping — is a section in `agent.toml` rather than a file in a folder of one.

Provider credentials and the model mapping belong to the **main** agent's manifest and are inherited. A sub-agent carries neither: duplicating them would put the same credential list in every directory and give each agent a secret it does not need.

`cache/`, `logs/`, `plugins/versions/` and `plugins/active/` exist but are host-written. They are created on first run and are not authored.

## What is watched

The host watches these directories and applies changes without a restart. A change is picked up on the next event; nothing here polls.

| Path | Change takes effect | Notes |
| --- | --- | --- |
| `agent.toml` | immediately | Identity, capabilities and memory scope re-resolve. A capability *narrowing* applies to the next action; a widening applies once validated. |
| `instructions/` | next turn | Prompt fragments are re-read and recomposed. The system prompt is not cached across this boundary. |
| `memory/facts/` | immediately | Changed facts are re-embedded and the index updated in place. Retrieval never mixes vector spaces, so a fact re-embedded under a new model invalidates its old row. |
| `skills/` | immediately | A new skill enters the index. A skill whose required plugin is absent resolves to the load-or-create path, not an error. |
| `subagents/` | immediately | Role card and prompt version. A revised prompt is a new version, never an overwrite. |
| `agent.toml` → `providers` | immediately | Credential list re-read. An environment credential still outranks a stored one. Only the main agent carries this section. |
| `agent.toml` → `use_cases` | immediately | The use-case → model mapping re-read. |
| `plugins/registry/` | immediately | A plugin appearing or being withdrawn enters or leaves the registry. |
| `plugins/active/` | immediately | A generation pointer change promotes or rolls back. |
| `plugins/versions/` | **not watched** | Immutable content-addressed store, written by the host. Watching it would mean reacting to our own writes. |
| `workspace/` | **not watched** | The sandbox root. It is the agent's workspace, not a configuration surface. |
| `cache/`, `logs/` | **not watched** | Host-written. |

## Why some paths are not watched

Watching a path that only this host writes to produces feedback: the host writes, the watcher fires, the host reacts to its own write. `plugins/versions/` and `cache/` are therefore outside the watch set by construction, not by configuration.

## New agents

Every agent directory has the same shape. A new one is made from the template, not by hand:

```sh
bun scripts/agent.ts new <name>          # creates agents/default/subagents/<name>/
bun scripts/agent.ts pack                # rebuilds templates/agent.zip from templates/agent/
bun scripts/agent.ts check <name>        # validates a directory against the manifest schema
```

The template lives at `templates/agent/` as a reviewable tree and ships as `templates/agent.zip`. `{{AGENT_NAME}}`, `{{AGENT_TITLE}}` and `{{AGENT_DESCRIPTION}}` are substituted at creation; a directory containing an unsubstituted token is rejected by `check`.

## What this tree is not

It is not source code. Nothing here is compiled, and nothing here should contain logic that belongs in the host. A skill says *how and when* to use a capability; the capability itself is a builtin or a plugin. If a change requires a rebuild to take effect, it is in the wrong place.