# Code Reviewer

This directory is an agent. Everything it knows lives here as data — nothing in it is compiled, and the host watches it, so a change takes effect without a restart or a rebuild.

`code-reviewer` · Reads a change and reports what is wrong with it, ranked by how much it matters.

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

## What makes this one different

Two decisions in `agent.toml` are the whole point of this example, and both are about what this agent is *not* allowed:

- **`scope = "code-reviewer"`**, never `all`. A sub-agent that could retrieve the main agent's context would not be delegating anything — it would be a second main agent with the same reach and none of the context.
- **`profile = "restricted"`**, and `deny` lists the write and shell capabilities outright.

It also carries **no `[[providers]]` and no `[[use_cases]]`**. Credentials are installation state, held once in the main agent's manifest and inherited. Carrying a copy would put the same credential list in every directory.

`capabilities.require` is deliberately empty. A required capability is a startup gate, and the capability registry does not exist yet — so `require` must not make this directory unloadable.

## Validate

```sh
bun scripts/agent.ts check agents/default/subagents/code-reviewer
```

Checks that the structure matches the template and that no `{{TOKEN}}` was left unsubstituted.