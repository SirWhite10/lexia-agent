---
kind: implementation
status: planned
---

# WF-IMP-008: Build the gpui-kit web artifact and serve it from the SvelteKit host

## Goal

Go from "a Rust crate claims wasm support" to a gpui-kit application that loads in a browser, served by the existing SvelteKit host, and labelled alpha on the surface that serves it.

## Depends on

- [WF-INV-002](WF-INV-002-gpui-kit-web-renderer-reality.md) reporting a workable backend

## Scope

- Create the Rust workspace for the gpui-kit application. Keep it a separate top-level directory beside `src-tauri/`; it is a different target with a different dependency graph from the native build and must not be entangled with it.
- One window renders a known-good component set — button variants and sizes, input, message and bubble, scroller, sheet, data table — so the build's health is visible without any product work.
- Produce the browser bundle with a reproducible command documented in the repository, and record its size against the number [WF-INV-002](WF-INV-002-gpui-kit-web-renderer-reality.md) measured.
- Serve it from SvelteKit under a real route. The host owns the session: the mount point is guarded exactly like the rest of `(app)`, and the route must work on refresh, deep link and cold load, not only after a client-side navigation.
- Mark the surface alpha in the UI itself, not only in documentation: an alpha badge on the mounted window's title area, matching the marker on `/kit`.
- Decide and record how updates reach the browser: cache-busted asset URLs, content hash, or a pinned release directory. A stale web bundle that the host still serves is worse than an error.

## Reference

The Svelte host surfaces this replaces are the norm, not the target: `src/routes/kit/+page.svelte` shows the shell conventions to preserve (rail on tablet and up, sheet below it, bounded conversation column). `src/lib/components/kit/kit.css` records where every token value came from and must be treated as the parity reference for theme values.

## Out of scope

- Porting the chat or artifact surfaces. Those are separate tickets and should not be started to "test the build"; the build test component set exists precisely so it can be.
- Removing or hiding the Svelte reference.
- Any native target change.

## Acceptance criteria

- The documented command produces a browser-loadable artifact from a clean checkout.
- The route renders the component set in a real browser, at phone, tablet and desktop widths, with no console errors.
- Text input takes focus and accepts a caret and selection; overlays trap focus and dismiss on Escape, and return focus to their trigger.
- Scrolling is owned by the message region, not the window.
- The alpha marker is visible on the served surface.
- The bundle size is recorded in this ticket, and the serving strategy is documented in the repository.

## Progress

Not started.