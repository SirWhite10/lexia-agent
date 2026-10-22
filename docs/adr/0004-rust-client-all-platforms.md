---
status: accepted
amended_by: 0005
---

# The client is Rust on every platform; SvelteKit is a host, not a client

Lexosa's interface is one gpui-kit codebase compiled to both a native binary and a `wasm32` browser bundle. There is no Tauri, no `src-tauri/`, and no webview shell anywhere in the project. SvelteKit remains a slim host — auth and sessions, static serving of the page and the wasm bundle, and the browser-native artifact renderers — and mounts the GPUI canvas the application renders into; on desktop the same views open a window through `gpui-pre-platform`. Agent APIs and the run stream are served by the Rust binary, not by the host; that split is decided in [ADR 0005](0005-rust-agent-host-and-wasm-plugins.md), which amends this record.

This is recorded as an ADR because it decides the client architecture for every platform, removes a technology that `PRODUCT.md` previously named, and cannot be cheaply reversed once screens have been ported. It implements [WF-DEC-004](../planning/tickets/WF-DEC-004-one-rust-codebase-native-and-web.md); the evidence that the browser target builds is [WF-INV-002](../planning/tickets/WF-INV-002-gpui-kit-web-renderer-reality.md).

## Amendment

This record was written when SvelteKit was still expected to keep the run stream and the agent surfaces. [ADR 0005](0005-rust-agent-host-and-wasm-plugins.md) moved both to the Rust agent binary. Everything else here stands: the client is Rust on every platform, the browser build is the same views compiled to wasm, SvelteKit still owns the document and therefore the URL, the browser-native artifact renderers stay, and Tauri remains excluded.

## Consequences

- **No Tauri.** A webview shell renders HTML and cannot host a GPUI canvas, so it was never a candidate. The desktop shell is a plain Rust binary linking `gpui-pre-platform` directly. `PRODUCT.md` no longer describes `src-tauri/`, and no Tauri dependency exists or is introduced.
- **The browser build is on nightly Rust, permanently.** `wasm_thread` requires `#![feature(stdarch_wasm_atomic_wait)]` and `gpui-kit` forces its `multithreaded` default on, so it cannot be disabled through the facade. This is a standing toolchain cost, not a temporary one.
- **SvelteKit owns the URL.** Rust mirrors the address bar with `pushState`/`replaceState` and listens for `popstate`; it never navigates the document. Refresh, deep link, cold load and browser back keep working because the host still owns the document and its router.
- **Some things stay browser-native.** Where the host already has the better answer — the `/kit` artifact renderers for HTML, Svelte and React — the wasm app hands the host a rectangle and receives the result back, rather than reimplementing a compiler in Rust. See [WF-IMP-018](../planning/tickets/WF-IMP-018-host-dom-island-bridge.md).
- **The Svelte reference is retained to the end.** `src/lib/components/kit/**` and `src/routes/kit/**` remain the parity oracle for the whole migration, as [WF-DEC-003](../planning/tickets/WF-DEC-003-gpui-kit-forefront-web-alpha.md) requires, and are deleted by a separate decision once parity is demonstrated across every route.
- **A `cfg(target_family = "wasm")` in application view code is a finding, not a pattern.** View code is shared; a difference between targets is a capability boundary that lives in one named place and is recorded as one.
- **The client waits for the agent.** The run and event contract is still being written, so the chat surface is ported after the agent, not before. Porting it now would mean porting it twice. See [ADR 0005](0005-rust-agent-host-and-wasm-plugins.md) and the re-ordered chain in `docs/planning/tickets/`.
- **The browser surface is not accessible by default.** The canvas carries no web semantics. What is done about that is decided in [WF-IMP-017](../planning/tickets/WF-IMP-017-accessibility-mirror-for-the-web-surface.md), not inherited.
- **Costs accepted:** upstream marks WebAssembly *showcase only* and has no wasm CI job, so every web acceptance criterion is ours to prove; and first load is multi-megabyte (24.4 MB raw / 5.95 MB gzip / 3.90 MB brotli for a near-empty probe, before `wasm-opt`, fonts and assets), to be measured properly and revisited rather than optimised pre-emptively.
- **A capability change in gpui-kit now reaches every platform at once.** That is the point of one codebase, and it is also the blast radius of an upstream regression.

Nothing here forecloses a future webview shell for something that is not a GPUI surface. It forecloses Tauri as the application shell.