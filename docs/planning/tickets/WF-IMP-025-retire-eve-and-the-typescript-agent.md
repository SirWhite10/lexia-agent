---
kind: implementation
status: planned
---

# WF-IMP-025: Retire EVE and the TypeScript agent

## Goal

Remove EVE, `my-agent/`, the supervisor, and every agent module in `src/lib/server/` that the Rust binary has replaced — cleanly, in one change, once the binary has been answering for all of them.

## Depends on

- [WF-IMP-024](WF-IMP-024-slim-the-sveltekit-host.md) succeeding, with a rollback that has been exercised

## Why this is gated

The order matters more than the work. EVE and the TypeScript agent are removed **after** the Rust binary answers for them, not alongside it and not before. Until this ticket, both paths run and must agree; a divergence found here is a bug in the port, not a reason to keep two runtimes.

Deleting earlier would leave a product that can answer nothing, and deleting piecemeal would leave a second runtime nobody is maintaining.

## What is removed

| Removed | Notes |
| --- | --- |
| `my-agent/` | the authored agent tree, including `instructions.md` and `channels/eve.ts`. Its content is superseded, not ported: instructions become the Rust core's prompt and skills, channels become capabilities. |
| the `eve` npm dependency | `my-agent/package.json`, `bun.lock`, and every install path that reaches it |
| `src/lib/server/eve.ts` | the EVE client |
| `src/lib/server/eve-supervisor.ts` | the process supervisor — a single binary has nothing to supervise |
| `src/lib/server/openrouter.ts`, `model-routes.ts`, `model-catalog.ts`, `provider-registry.ts`, `provider-key.ts` | superseded by the `ModelProvider` layer |
| `src/lib/server/runs.ts`, `memory.ts`, `prompt.ts`, `turn.ts`, `planner.ts`, `builtins.ts`, `agents.ts`, `thought-tree.ts`, `transcription.ts`, `attachments.ts` | ported to Rust |
| `src/routes/api/runs/[id]/events/+server.ts` | the run stream moves to the binary |
| the ported tests | moved with the code, not deleted from under it |
| the agent-facing `+page.server.ts` actions | superseded by the binary's endpoints |
| `auth-recovery.txt` handling | moves with the session owner decided in [WF-IMP-024](WF-IMP-024-slim-the-sveltekit-host.md) |

## What stays

- `src/lib/server/auth.ts` and the session contract.
- The browser-native artifact renderers.
- The `/kit` reference implementation and its parity oracle role ([WF-DEC-003](WF-DEC-003-gpui-kit-forefront-web-alpha.md)).
- The client routes, until [WF-IMP-016](WF-IMP-016-settings-login-and-bootstrap-routes.md) and its siblings replace them with native ones.

## Documentation that changes with it

Documentation that describes EVE as running is wrong the moment this lands, and it is currently wrong in several places:

- `AGENTS.md` — the workspace map, the operating rules about `my-agent/`, `eve dev`, and the supervisor.
- `OPERATIONS.md` — "Running the agent", the PATH-strip note, the provider-restart note.
- `PRODUCT.md` — Jev as router, EVE as executor, the `my-agent/` entry.
- `CONTEXT.md` — the glossary entries **EVE runtime**, **Jev gate**, and **EVE source tree**.
- `docs/adr/0003-eve-development-runtime.md` — already marked superseded by [ADR 0005](../../adr/0005-rust-agent-host-and-wasm-plugins.md); its title and body stay as the record of what was decided and then reversed.

That glossary is `CONTEXT.md`, and it is the canonical domain vocabulary. If EVE is gone, the terms that named it go with it rather than lingering as descriptions of something that no longer runs.

## Out of scope

- Rewriting what EVE's instructions said. They are superseded by the Rust core's prompt and skills; preserving the text would preserve a shape that no longer applies.
- The client migration, which is the GPUI chain.
- The state migration, which [WF-IMP-024](WF-IMP-024-slim-the-sveltekit-host.md) already did.

## Acceptance criteria

- No file in the repository references EVE as something that runs, other than the superseded ADR and this ticket.
- `CONTEXT.md` has no entry for a runtime that does not exist.
- `bun run dev` starts the host and the Rust binary, and nothing supervises a child agent process.
- `bun run check` and `bun test` pass with no reference to removed modules.
- The reference-code strip: no `eve`, `eve dev`, or `my-agent` import survives in `src/`.
- The password-recovery flow still works end to end, and its recovery code still never reaches the browser.
- Provider credential handling still follows the rules in `OPERATIONS.md`: environment outranks file, empty removes, and saving a key is reported rather than silent.

## Progress

Not started. Blocked on [WF-IMP-024](WF-IMP-024-slim-the-sveltekit-host.md).