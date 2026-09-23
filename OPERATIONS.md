# Operations Router

This area describes how Lexia is developed, updated, validated, and operated locally.

- `AGENTS.md` — repository instructions and workspace map.
- `RTK.md` — required command-output proxy instructions.
- `workflow/` — repeatable operational workflows and their Bun/TypeScript scripts.
- `docs/adr/` — accepted architectural decisions that constrain operations.
- `app-data/releases/` — staged and active production EVE releases on user installations.
- `app-data/state/` — workflow state, release metadata, and persistent user data.
- `scripts/` — planned repository-level Bun automation; workflow-specific scripts belong under their workflow directory.
- `.eve/` and `.output/` — generated EVE build/runtime artifacts; never treat them as source of truth.

## Release lifecycle

`download → stage → inspect → validate → eve build → health check → accept → promote → restart EVE → retain rollback release`
