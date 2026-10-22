---
kind: implementation
status: planned
---

# WF-IMP-018: The host DOM island bridge

## Goal

Let the GPUI app delegate a rectangle of its own screen back to the host, where a real DOM island renders it. This is what keeps the `/kit` demo's HTML, Svelte and React artifact renderers working on web instead of degrading them to source.

## Depends on

- [WF-IMP-008](WF-IMP-008-gpui-kit-web-build-and-host.md) mounting a served surface

## Why this exists

[WF-INV-002](WF-INV-002-gpui-kit-web-renderer-reality.md) found that `gpui-wry` is desktop-only, so the browser has no WebView and gpui-kit has no equivalent of a browser rendering agent-authored source. That left `html`, `svelte` and `react` artifacts with no honest option on web.

But the host already has the best possible implementation of all three, and it is the parity oracle for this migration:

| Kind | Reference | Why it is the better answer |
| --- | --- | --- |
| `html` | `src/lib/components/kit/artifacts/render-html.svelte` | Sandboxed `srcdoc` iframe, `sandbox="allow-scripts"` with **no** `allow-same-origin`, so the frame gets an opaque origin. The document measures itself and posts its height up through `postMessage`. A Rust HTML engine would be strictly worse. |
| `svelte` | `src/lib/components/kit/artifacts/render-svelte.svelte` | Compiles the source with `svelte/compiler` (`runes: true`, `css: 'injected'`), evaluates it with `new Function` against the internals this page already ships, and mounts it. Compilation is memoised by source text. |
| `react` | `src/lib/components/kit/artifacts/render-react.svelte` and `react-runtime.ts` | `react`, `react-dom` and `htm` load on first use through dynamic `import()`, so a session that never shows a React artifact never downloads them. `htm` is bound to `React.createElement`. |

Reimplementing a Svelte compiler and a React runtime in Rust to reach parity with these would be the wrong trade by a wide margin. The island bridge makes the host's answer reachable instead.

## Mechanism

Rust owns the artifact **frame** — header, kind marker, Preview/Code toggle, the `auto`/tall/full sizing contract, scroll containment, and the loading and error states that belong to the conversation. Rust delegates only the **body**, for the kinds the host renders better.

1. Rust resolves the frame's on-screen rectangle from its own layout and emits a mount request: island id, kind, source, and the rect.
2. The host places an absolutely positioned overlay element above the canvas at that rect and mounts the matching renderer into it.
3. The host reports back: measured size, ready, or a compile/runtime error.
4. Rust resizes its own frame to the reported size and clears its placeholder.

The existing height protocol is reused unchanged, including the `MIN_HEIGHT`/`MAX_HEIGHT` clamps and the graceful-grace-then-timeout failure path already in `render-html.svelte`.

## Design constraints

- **One implementation of the frame.** The overlay must not become a second copy of the artifact chrome. Anything the conversation needs to draw — a loading state, a failure notice, a frame border — is drawn by Rust; only third-party document content is delegated.
- **Security posture is unchanged.** The island lives in the same host document, under the same CSP, as it did when the Svelte chat page rendered it. `sandbox="allow-scripts"` without `allow-same-origin` stays exactly as it is. Nothing in this ticket may weaken it, and the recovery code rule from [WF-IMP-016](WF-IMP-016-settings-login-and-bootstrap-routes.md) is unaffected because no island ever renders host state.
- **Coordinates.** GPUI works in `Pixels`; the overlay works in CSS pixels. The conversion has to account for device pixel ratio and for whatever offset the canvas sits at in the document. This is the fiddly part of the ticket and it is where it will go wrong first.
- **Scrolling.** The message region owns scrolling, so a rect computed once goes stale immediately. Rust re-emits rects as the region scrolls, throttled to a frame, and the host must not animate its own island position.
- **Pointer events.** The overlay is `pointer-events: none` except where an island actually is, so the canvas keeps receiving everything else. An island that fails to mount must not swallow clicks meant for the conversation.
- **The split lives here, not in view code.** [ADR-0004](../../adr/0004-rust-client-all-platforms.md) holds that a `cfg(target_family = "wasm")` in view code is a finding. This ticket is that finding's one permitted home: the artifact body dispatch sits behind a capability, native takes the `gpui-wry` path or the source path, and the frame is shared.

## Scope

- The capability interface, its web implementation and its native implementation.
- Island lifecycle: mount, resize, unmount, and teardown when the message unmounts or the route changes. A leaked island is a leaked `new Function` and a leaked React root.
- Re-emission on scroll and on resize, including the zoom and device-pixel-ratio changes that make stored rects wrong.
- The three kinds wired to the existing renderers. `html` is also the case to settle for native, where `gpui-wry` is available.
- A fourth kind if one is found where the host is clearly better; record the reason, as [WF-IMP-010](WF-IMP-010-port-artifact-surface-to-gpui-kit.md) requires for every kind.

## Out of scope

- Rendering the artifact frame itself on the host. That is Rust's, on both targets.
- The artifact contract in `src/lib/components/kit/artifacts/types.ts`; a kind's shape does not change because it renders through an island.
- The accessibility mirror ([WF-IMP-017](WF-IMP-017-accessibility-mirror-for-the-web-surface.md)), which may build on this mechanism but is a separate decision.

## Acceptance criteria

- An `html`, `svelte` and `react` artifact renders on web through its existing renderer, with no change to the renderer source.
- The artifact frame — header, toggle, sizing, scroll containment — is drawn once, by Rust, on both targets.
- Islands track the frame correctly while the conversation scrolls, and after a resize or a zoom.
- Unmounting the conversation, or navigating away, leaves no island, no React root and no iframe behind.
- A compile or runtime failure in an island surfaces in the Rust frame as a failure state; it does not blank the artifact and does not take the conversation down.
- The HTML island remains sandboxed identically to the reference: opaque origin, no access to the host page, its storage or its cookies.
- Nothing renders inside an island can reach a host credential, because no credential is in scope inside one.

## Progress

Not started.