---
kind: implementation
status: planned
---

# WF-IMP-014: The attachment and file-picker bridge

## Goal

Make file upload work on both targets. The web platform cannot do it at all — `prompt_for_paths` and `prompt_for_new_path` **panic** — so this is a JavaScript bridge by necessity, not by preference.

## Depends on

- [WF-IMP-008](WF-IMP-008-gpui-kit-web-build-and-host.md) mounting a served surface
- [WF-IMP-024](WF-IMP-024-slim-the-sveltekit-host.md) — attachment storage and upload move to the Rust binary with everything else

On both targets the upload now goes to the Rust binary rather than to a SvelteKit route, because [ADR 0005](../../adr/0005-rust-agent-host-and-wasm-plugins.md) moved attachment storage with the rest of the state. The picker problem below is unchanged by that.

## The gap


```rust
// gpui-pre-web/src/platform.rs:476 and :490
"prompt_for_paths is not supported on the web"
"prompt_for_new_path is not supported on the web"
```

Both panic rather than returning an error, so a screen that calls them on web crashes the application instead of degrading. Nothing in the ported code may call them unconditionally.

`src/lib/server/attachments.ts` and the attachment metadata in `app-data/state/attachments/` already own the storage side. This ticket is about getting bytes from a user to that server.

## Two targets, two mechanisms

**Native**: `gpui-pre-platform` provides the real picker. Use it.

**Web**: a hidden DOM `<input type="file">` created and driven from Rust through `web_sys`. The picker is a browser capability; there is no GPUI equivalent and there will not be one. The bytes then go to the server over the same `FetchHttpClient` the rest of the app uses.

The Rust side sees one interface — a request for a file, then a stream of bytes — and does not branch on target. That is the whole design constraint: the capability difference lives in one place.

## Scope

- One Rust-side file-request interface, with a native implementation and a web implementation.
- Multi-file selection and drag-and-drop, since the reference composer supports both.
- The upload itself: progress, failure, and cancellation, reported through the same mechanisms as a run action so the ledger stays honest.
- The attachment chip and its removal in the composer, matching `src/lib/components/kit/attachment.svelte`.
- Transcription already exists on the host (`OPERATIONS.md`, Settings → Models) and runs after a turn carrying audio. This ticket feeds it; it does not add a second transcription path.

## Out of scope

- A gallery or media browser.
- Changing where uploaded bytes are stored.

## Acceptance criteria

- A user can attach a file on web and on native, from the picker and by drag-and-drop, and the composer shows the chip.
- No code path can reach `prompt_for_paths` on web; the application cannot be crashed by asking for a file.
- A failed or cancelled upload says what failed and leaves the composer's text intact.
- An uploaded audio turn reaches the existing transcription path rather than a new one.
- The chip is keyboard reachable and removable, and removal restores the caret position.

## Progress

Not started.