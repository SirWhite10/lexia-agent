# {{AGENT_TITLE}}

This directory is an agent. Everything it knows lives here as data — nothing in it is compiled, and the host watches it, so a change takes effect without a restart or a rebuild.

`{{AGENT_NAME}}` · {{AGENT_DESCRIPTION}}

## Structure

```text
agent.toml      the manifest: everything that stays a single record
instructions/   system-prompt fragments, composed into this agent's prompt
memory/facts/   one file per durable fact; the vector index is host-owned
skills/         one directory per skill
subagents/      standing sub-agents, one directory each
plugins/        generation pointers and the immutable content-addressed store
workspace/      the sandboxed filesystem root for this agent
```

**The shape is hybrid, deliberately.** What grows one-file-per-thing is a directory, because a directory is where you put a file. What stays a single record is a section in `agent.toml` rather than a file in a folder of one.

`cache/`, `logs/`, `plugins/versions/` and `plugins/active/` are host-written and are not authored.

## What is watched

| Path | Change takes effect |
| --- | --- |
| `agent.toml`, `instructions/`, `memory/facts/`, `skills/`, `subagents/`, `plugins/registry/`, `plugins/active/` | immediately, on the next event |
| `plugins/versions/`, `workspace/`, `cache/`, `logs/` | not watched — host-written or the agent's own space |

Nothing here polls. A watch is an event, and the agent reacts to it.

## Make it yours

Two things, in this order:

1. `agent.toml` — the role card. Set `name`, `title`, `description`, the memory `scope`, and the narrowest `capabilities.allow` that lets this agent do its job. Leave `capabilities.require` empty until the capability registry exists; a required capability is a startup gate.
2. `instructions/core.md` — how it behaves. This is the prompt fragment, and it is read on change.

Provider credentials and the model mapping are **not** here if this agent is a sub-agent. They belong to the main agent's manifest and are inherited. Carrying your own copy would put the same credential list in every directory and hand each agent a secret it does not need.

Memory scope is the field that matters most. A sub-agent that can retrieve `all` has the main agent's private context, which is not what delegation is for.

## Validate

```sh
bun scripts/agent.ts check <this-directory>   # an agent directory, not templates/agent
```

Checks that the structure matches the template and that no `{{TOKEN}}` was left unsubstituted.