---
kind: implementation
status: planned
---

# WF-IMP-024: Slim the SvelteKit host and cut over

## Goal

Move SvelteKit from serving agent APIs to serving auth, static files and the browser-native artifact renderers — and move the state, so there is one writer rather than two.

## Depends on

- [WF-IMP-019](WF-IMP-019-rust-agent-binary-and-model-layer.md)
- [WF-IMP-020](WF-IMP-020-durable-run-coordinator-in-rust.md)
- [WF-IMP-022](WF-IMP-022-memory-and-skills-in-rust.md)
- [WF-IMP-023](WF-IMP-023-agent-loop-sub-agents-and-builtins.md)

## What stays

Three things, and only three:

- **Auth and sessions.** `src/lib/server/auth.ts`, the `lexia_session` cookie, and the recovery-code rules.
- **Static serving.** The page, the wasm bundle, and the client assets.
- **The browser-native artifact renderers.** `src/lib/components/kit/artifacts/` — the HTML, Svelte and React paths, which stay because the browser has the better answer and the wasm app reaches them through [WF-IMP-018](WF-IMP-018-host-dom-island-bridge.md).

## What goes

Every agent route, and every module under `src/lib/server/` that is agent logic. That is roughly 2,900 lines of TypeScript plus 1,123 lines of tests, all superseded by the binary.

## Two decisions this ticket has to make, not defer

**Who issues the session.** The `lexia_session` cookie is an unsigned user id, so both processes can read it — which is precisely why the answer has to be explicit. The clean shape is that the Rust binary owns session issuance and verification, and the host verifies by asking the binary, so there is one writer and one decision. The alternative — both reading the cookie independently — is simpler and means the recovery rules are enforced in two places. Record the choice and why.

**How the browser reaches the binary.** The browser talks to SvelteKit for the page and to the Rust binary for agent APIs, so there are two origins. That is a deployment decision: cross-origin with credentials, or a reverse proxy that puts them on one. It is not a reason to put a proxy layer in the application code.

## State cutover

`app-data/state/lexia.sqlite` migrates to the Rust state directory. The dangerous window is the one where both can write it. The cutover is therefore:

1. Migrate and verify while the TypeScript path is still authoritative.
2. Freeze: one writer, one path, no fallback writing to both.
3. Switch the clients to the binary.
4. Only then remove the TypeScript modules, in [WF-IMP-025](WF-IMP-025-retire-eve-and-the-typescript-agent.md).

No state is deleted before the Rust path has been proven against it, and nothing is written by two processes at once at any point.

## Out of scope

- Deleting the TypeScript agent modules. That is [WF-IMP-025](WF-IMP-025-retire-eve-and-the-typescript-agent.md), and it is gated on this ticket succeeding.
- The client surfaces. Nothing here changes what the UI looks like.

## Acceptance criteria

- SvelteKit serves exactly three responsibilities and no agent route answers from it.
- The session contract has one owner, recorded, with the recovery-code rules enforced in one place.
- The deployment story for the two origins is written down and works in a real browser.
- `lexia.sqlite` migrates to the Rust state directory with runs, actions, the event log and memory intact.
- At no point in the cutover are both implementations able to write the state.
- A rollback exists and has been exercised before the TypeScript modules are removed.

## Progress

Not started.