# Operations Router

This area describes how Lexia is developed, updated, validated, and operated locally.

- `AGENTS.md` — repository instructions and workspace map.
- `RTK.md` — required command-output proxy instructions.
- `workflow/` — repeatable operational workflows and their Bun/TypeScript scripts.
- `docs/adr/` — accepted architectural decisions that constrain operations.
- `my-agent/` — the EVE agent source tree, edited in place; `eve dev` runs it.
- `app-data/state/` — persistent Lexia state: the SQLite run/action records and event log, plus agent memory and personality files.
- `scripts/` — planned repository-level Bun automation; workflow-specific scripts belong under their workflow directory.
- `.eve/` — generated EVE runtime artifacts; never treat them as source of truth.

## Running the agent

`bun run dev` starts the host and the EVE agent together; the supervisor restarts EVE if it dies. Editing `my-agent/agent/` changes the agent's behaviour and EVE reloads it on the next turn. There is no build, stage, promote, or rollback step, and no second terminal to manage. See [ADR 0003](./docs/adr/0003-eve-development-runtime.md).

EVE runs as a Node program. The host strips Bun's Node-compatibility shim from
the child `PATH` and unsets `NODE` before spawning, because that shim cannot
evaluate one of EVE's internal modules; without the strip, `eve dev` exits on
every start and the supervisor loops on `Failed to evaluate authored module`
without EVE ever binding its port.
