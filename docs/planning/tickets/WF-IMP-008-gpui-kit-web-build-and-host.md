---
kind: implementation
status: planned
---

# WF-IMP-008: Build the gpui-kit app for native and web, and host the web artifact

## Goal

Go from "a Rust crate claims wasm support" to a gpui-kit application that builds for **both** targets from one source tree, loads in a browser served by the existing SvelteKit host, and opens a native window on the desktop. The build's health must be visible without any product work.

## Depends on

- [WF-INV-002](WF-INV-002-gpui-kit-web-renderer-reality.md) reporting a workable backend — **met**
- [WF-DEC-004](WF-DEC-004-one-rust-codebase-native-and-web.md)

## Scope

- Create the Rust workspace at `gpui-app/` as a sibling of `src/`. There is no Tauri and no `src-tauri/` anywhere in this project, because a webview shell cannot host a GPUI canvas ([ADR 0004](../../adr/0004-rust-client-all-platforms.md)). It is a different target from the host and must not be entangled with it.
- One crate tree, two targets. `crate-type = ["cdylib", "rlib"]` for wasm; a binary for native. Shared view code lives in `lib`, so a screen written once is a screen on both platforms. Any `cfg(target_family = "wasm")` in application code is a finding to eliminate, not a pattern to repeat.
- Pin a nightly toolchain in `rust-toolchain.toml` with a `rust-toolchain` reason comment: `wasm_thread` requires `#![feature(stdarch_wasm_atomic_wait)]` and `gpui-kit` forces the `multithreaded` default on. Record which nightly and why, so an upgrade is a deliberate act.
- Produce both artifacts with one documented command each, reproducible from a clean checkout, and record the wall-clock build time for each.
- One window renders a known-good component set — button variants and sizes, input with a caret, message and bubble, scroller, sheet, dialog, data table — so build health is visible without product work.
- Serve the browser bundle from SvelteKit under a real route. The mount point is guarded exactly like the rest of `(app)`. The route must work on refresh, deep link and cold load, not only after a client-side navigation.
- Load fonts before the first window, including the `.SystemUIFont` alias family the theme binds to. The web platform starts with an empty font database and text measurement fails without it.
- Mark the surface alpha in the UI itself, not only in documentation.
- Record the bundle size: raw, gzip and brotli, and the post-`wasm-bindgen`/post-`wasm-opt` figure separately from the raw `rustc` output. [WF-INV-002](WF-INV-002-gpui-kit-web-renderer-reality.md) measured 24.4 MB raw / 5.95 MB gzip / 3.90 MB brotli for a near-empty probe; this is the number a real measurement replaces.
- Decide and record how updates reach the browser: cache-busted asset URLs, content hash, or a pinned release directory. A stale web bundle that the host still serves is worse than an error.

## Reference

The Svelte host surfaces this replaces are the norm, not the target: `src/routes/kit/+page.svelte` shows the shell conventions to preserve. `src/lib/components/kit/kit.css` records where every token value came from and is the parity reference for theme values.

## Out of scope

- Porting any product screen. The build-test component set exists precisely so the build can be tested without one.
- The routing and session bridge ([WF-IMP-012](WF-IMP-012-url-and-session-bridge.md)) — this ticket mounts one surface, it does not navigate.
- Removing or hiding the Svelte reference.
- Reducing the payload. Measure it, then revisit.

## Acceptance criteria

- One documented command produces a browser-loadable artifact and a native binary from a clean checkout, on the pinned toolchain.
- The route renders the component set in a real browser at phone, tablet and desktop widths, with no console errors.
- Text input takes focus and accepts a caret and selection; overlays trap focus, dismiss on Escape, and return focus to their trigger.
- Scrolling is owned by the message region, not the window.
- The native binary opens a window drawing the same component set, and the two agree on theme values.
- The alpha marker is visible on the served surface.
- Bundle size and build time are recorded in this ticket; the serving strategy is documented in the repository.

## Progress

Not started. The toolchain, flag and failure-mode groundwork is already established in [WF-INV-002](WF-INV-002-gpui-kit-web-renderer-reality.md): nightly required, `-C link-arg=-zstack-size=8388608` needed for a large render tree, and fonts must be registered before `open_window`.