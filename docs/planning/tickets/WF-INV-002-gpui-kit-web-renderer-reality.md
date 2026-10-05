---
kind: investigation
status: planned
---

# WF-INV-002: Establish what the gpui-kit browser build actually is

## Goal

Answer, with evidence rather than optimism, whether `gpui-kit` can be built for `wasm32` and rendered in a browser today, and if so through which backend. Every implementation ticket in this direction depends on that answer, and the current evidence in this repository is one code comment.

## Depends on

- [WF-DEC-003](WF-DEC-003-gpui-kit-forefront-web-alpha.md)

## The evidence that exists

- `gpui-component-0.7.0/src/theme/mod.rs::init` carries the comment *"Ensure theme is loaded directly on startup for WASM compatibility"* — the only acknowledgement of the browser target inside the component crate.
- `gpui-pre-0.3.7/Cargo.toml` carries `target_arch = "wasm32"` dependency entries (`async-channel`, `getrandom` with `wasm_js`, `uuid`), so parts of the platform crate have been touched for the target.
- `gpui-kit-0.7.0`, `gpui-component-0.7.0`, `gpui-base-0.7.0` and `gpui-pre-0.3.7` are already in the local cargo registry at `~/.cargo/registry`, so source inspection costs nothing.

## The evidence that is missing

- No web render backend is present in the published crates. `gpui-pre`'s platform surface is macOS, Linux and Windows; a browser needs a drawing path (WebGPU via a compatible GPUI backend, canvas, or something else) and the ticket author must find which one is intended.
- No `wasm32` build has been attempted from this repository, so build time, toolchain requirements and failure mode are unknown.
- Whether the components that matter here work under that backend at all — text shaping, `Scrollable` regions, overlays, text input with a real caret — is unverified.

## Scope

- Determine the intended web backend and its maturity, from upstream sources first: the crate repository, its release notes and its issue tracker. Cite what is found.
- Attempt a real `wasm32` build of a minimal `gpui-kit` application in this workspace. Record the toolchain, the flags, the build time and the first failure, if any.
- Exercise the components the chat surface actually depends on: `Message`/`Bubble`, `MessageScroller`, `Input`/`Textarea`, `Sheet`/`Dialog`, `DataTable`, `Spinner`/`Shimmer`. A build that compiles but cannot draw text is not a pass.
- Record bundle size for that minimal application. It is the number that decides whether artifacts like the Svelte-side `svelte/compiler` dependency are affordable.
- Report the fallback if it fails: the Svelte reference remains the web surface, native proceeds, and [WF-DEC-003](WF-DEC-003-gpui-kit-forefront-web-alpha.md) is amended rather than abandoned.

## Out of scope

- Porting any screen. No interface work happens until this investigation reports.
- Choosing the packaging or hosting strategy beyond what the build itself forces.
- Deciding the cutover date.

## Completion criteria

- A written answer: which backend, whether a minimal application builds today, what renders and what does not, and the bundle size.
- Every claim carries a citation or a build log, not a recollection of upstream intent.
- An explicit recommendation: proceed, proceed with constraints, or fall back.
- If the answer is "not today", the fallback is written down as its own ticket rather than left as a footnote.

## Progress

Not started. The three implementation tickets below are blocked on this report.