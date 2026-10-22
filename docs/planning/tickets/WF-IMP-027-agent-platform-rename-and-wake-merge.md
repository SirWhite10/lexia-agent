---
kind: implementation
status: planned
---

# WF-IMP-027: Rename to agent-platform and merge the wake prototype

## Goal

Make the repository `agent-platform`, the binary `agent`, and the instance `agents/default/`, and bring the wake-word prototype in as `agent-wake/`. Decisions are recorded in [ADR 0006](../../adr/0006-agent-platform-and-the-agent-name.md).

## Depends on

- The 39 uncommitted changes in this repository are committed or snapshotted first, so the rename diff can be reviewed on its own.
- [WF-IMP-019](WF-IMP-019-rust-agent-binary-and-model-layer.md) is complete or paused at a commit. Its code uses the old names.

## Scope

### Phase 1: snapshot and worktree

- Commit or snapshot the pending changes.
- Create a git worktree for the rename, per AGENTS.md.

### Phase 2: the mechanical rename

- `lexosa-agent/` becomes `agents/default/`.
- The Rust package and binary become `agent`. `gpui-app/` keeps its name until WF-IMP-008 decides its layout.
- `LEXOSA_*` and `LEXIA_*` environment variables become `AGENT_*`.
- Active docs and code use the new names. ADRs and tickets keep their history.
- Exclude `node_modules`, `target`, `.svelte-kit`, and generated artifacts from the search.

### Phase 3: import the wake prototype

- Create `agent-wake/` from `lexia-sherpa-wake-test/`.
- Do not carry `lexia-sherpa-wake-test.zip`, the prebuilt binary, `.venv-tools/`, or `target/`.
- Do not commit `models/`. Keep `scripts/prepare-model.sh` and fetch the model from it.
- Review `.scratch/` before importing it. It holds planning notes and is not carried by default.
- Rename the package `lexia-sherpa-wake-test` to `agent-wake`.
- Rename the wake keyword env vars from `LEXIA_*` to `AGENT_WAKE_*`.

### Phase 4: wake phrases per agent

- One agent holds several wake phrases. Each phrase maps to a command.
- The default phrase is `AGENT`. The setup wizard lets the user pick a name or skip it.
- Phrase changes restart the detector. They are refused while a microphone stream is active.

### Phase 5: web surface

- Port the sibling's Svelte UI into the SvelteKit host as a route.
- Replace the single env-credential login with the host's credential flow.
- Delete `web/` once the route is in place.

### Phase 6: capability

- Add `audio.capture` to the closed capability enum, granted per agent through the consent ledger.

## Out of scope

- Desktop app packaging. It follows the same surface.
- Choosing the command vocabulary. That belongs to WF-IMP-023.

## Acceptance criteria

- `grep` over active code and docs finds no `lexosa`, `lexia` or `alexia` outside ADRs and tickets.
- `cargo build` produces `agent`. `bun run check` and `bun test` pass.
- `agents/default/` passes `agent check`, and a fresh `agent new` produces a directory that passes it.
- The wake prototype runs from `agent-wake/` with the default phrase `AGENT`, and reports a detection on the bundled fixture.
- A phrase that the model cannot spell is refused with the reason, and the detector is not restarted.
- A second phrase can be added to one agent and maps to a different command.
- No credential, model or audio file is committed.

## Progress

Not started. Decisions are recorded in ADR 0006.
