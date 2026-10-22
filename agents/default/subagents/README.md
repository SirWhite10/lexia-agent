# Standing sub-agents

One directory per standing sub-agent, made from the template:

```sh
bun scripts/agent.ts new researcher
```

Each is a complete agent directory with the same shape as `agents/default/` itself — its own manifest, instructions, memory scope, skills and capability list — living under a narrower grant.

## Standing and one-shot

A **standing sub-agent** persists until explicitly retired. It has a role card, a prompt with versions, and its own memory scope.

A **one-shot agent task** dies with its run and auto-archives. It is an action in a run, not a directory here. The history of both is read from the run model, not from these directories.

## Role card

The manifest is the role card: authored system prompt, focus, memory scope, capability allowlist.

```toml
[agent]
name = "researcher"
title = "Researcher"
description = "Gathers and cross-checks sources for a question."

[memory]
scope = "research"      # not "all" — never the main agent's private context
```

Memory scope is the field that matters most. A sub-agent that can retrieve `all` has the main agent's context, which is not what delegation is for. Give it a scope of its own.

Capabilities are narrower than the parent's by design. A sub-agent exists to do one job well; a capability list as broad as the parent's means the delegation was unnecessary.

## Lifecycle

Revising `instructions/core.md` creates a **new prompt version**. Prior versions stay viewable, so "what did it run with?" is answerable after the fact. Never overwrite a version in place.

Retiring archives. The directory stays and stays queryable; nothing here is ever hard-deleted.

## On change

`subagents/` is watched. A new directory is available immediately; a prompt edit takes effect on the next task that agent runs, not mid-run.