---
kind: decision
status: complete
---

# WF-DEC-004: One Rust codebase, both targets; SvelteKit keeps the URL

## Depends on

- [WF-DEC-003](WF-DEC-003-gpui-kit-forefront-web-alpha.md)
- [WF-INV-002](WF-INV-002-gpui-kit-web-renderer-reality.md) reporting a workable backend

Promoted to [ADR 0004](../../adr/0004-rust-client-all-platforms.md): it decides the client architecture for every platform and removes a technology the repository previously named.

## Decision

**One Rust codebase compiles to both a native binary and a `wasm32` browser artifact.** The gpui-kit views are shared; only the platform crate differs, and `gpui-kit` already selects it by target cfg. There is one product, two distributions, and no second implementation of any screen.

**Every route converts, including `/login`, `/settings/*` and `/bootstrap`.** The earlier chain covered chat and artifacts only. That was an artefact of the investigation being narrow, not a considered scope limit, and it is superseded.

**SvelteKit owns the URL.** The browser address bar is authoritative. Rust mirrors it with `history.pushState`/`replaceState` and listens for `popstate`; the SvelteKit host pushes route changes into the wasm app on a `#[wasm_bindgen]` export. Refresh, deep link, cold load and browser back all work because the host still owns the document and its router — Rust never navigates the page.

**Tauri is removed from the project entirely.** Tauri renders HTML into a webview and cannot host a GPUI canvas, so it was never a candidate for the application shell; the desktop shell is a plain Rust binary linking `gpui-pre-platform` directly. No Tauri dependency exists in `package.json` and none is introduced. `src-tauri/` is not created, and the mention of Tauri clients in `AGENTS.md` has been replaced with the Rust client.

**The Svelte reference is retained to the end.** `src/lib/components/kit/**` and `src/routes/kit/**` stay as the parity oracle for the whole migration, as [WF-DEC-003](WF-DEC-003-gpui-kit-forefront-web-alpha.md) already requires. It is not deleted when a screen lands; it is deleted by a separate decision, once parity has been demonstrated across every route.

**Payload is accepted for now.** The first-load cost is measured, recorded and revisited after `wasm-opt`, not optimised pre-emptively.

**Where the host already has the better answer, the host keeps it.** The `/kit` demo renders `html`, `svelte` and `react` artifacts with a sandboxed `srcdoc` iframe, `svelte/compiler`, and React plus `htm`. The wasm app delegates a rectangle of its own screen back to the host for those kinds rather than reimplementing a Svelte compiler in Rust. See [WF-IMP-018](WF-IMP-018-host-dom-island-bridge.md) and [WF-IMP-010](WF-IMP-010-port-artifact-surface-to-gpui-kit.md).

## Why

- [WF-INV-002](WF-INV-002-gpui-kit-web-renderer-reality.md) showed the renderer works and the component set compiles for `wasm32`. That removes the blocker that kept this direction parked.
- Making the URL authoritative to the host is what keeps deep links and refresh working, and it is the same contract the SvelteKit app already meets. The alternative — Rust owning navigation — breaks exactly those properties.
- Removing Tauri follows from what a GPUI canvas needs and from the fact that gpui-kit covers every platform wanted; it is a consequence, not a preference.
- Every route converting in one pass avoids building a routing model, a session bridge and a design-token pipeline that only work for half the product, then retrofitting the other half.

## Accepted risks

These are accepted deliberately, with the reasons recorded rather than smoothed over.

1. **Upstream calls browser support showcase-only.** *"Shipping an application in a browser is not a supported path yet"*, and upstream CI has no `wasm32` job. The migration is therefore downstream of a capability the maintainers have not validated for this use. Every web acceptance criterion in the implementation tickets is ours to prove.
2. **Nightly Rust is permanent.** `wasm_thread` requires `#![feature(stdarch_wasm_atomic_wait)]` and cannot be disabled through the `gpui-kit` facade.
3. **First load is multi-megabyte.** 24.4 MB raw, 5.95 MB gzip, 3.90 MB brotli for a near-empty probe, before `wasm-opt`, fonts and assets.
4. **Accessibility, file upload and CJK typography need explicit work.** The canvas carries no web semantics, `prompt_for_paths` panics on web, and the font database starts empty.

## Scope boundary

This ticket chooses the target shape, the route scope, the URL owner, the shell technology, the reference-retention rule and the payload posture. It does not choose per-screen implementations, the packaging strategy, or the cutover date for any individual route — those are the implementation tickets.

## Completion criteria

- The migration is expressed as ordered tickets covering every route, with dependencies, so an agent can work forward without re-litigating these decisions.
- The decision is promoted to [ADR 0004](../../adr/0004-rust-client-all-platforms.md), and `PRODUCT.md`, `OPERATIONS.md` and `AGENTS.md` no longer describe Tauri as part of this work.
- The accepted risks are recorded here rather than only in conversation.

## Progress

Decided. The ticket chain follows: [WF-IMP-008](WF-IMP-008-gpui-kit-web-build-and-host.md) builds both targets, then the routing bridge, the shell, and each surface in turn.

## Follow-up

- [WF-DEC-005](WF-DEC-005-memory-injection-and-eve-memory-provider.md) is open. It is unrelated to this direction, but it needs answering before any ticket that touches memory retrieval.