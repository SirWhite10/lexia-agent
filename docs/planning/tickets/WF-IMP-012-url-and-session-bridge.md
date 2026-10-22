---
kind: implementation
status: planned
---

# WF-IMP-012: The URL and session bridge

## Goal

Make one wasm application behave like a routed web app: SvelteKit owns the address bar, Rust owns what is on screen, and the two stay in agreement through refresh, deep link, cold load and browser back. Every later surface depends on this, so it lands before any screen is ported.

## Depends on

- [WF-IMP-008](WF-IMP-008-gpui-kit-web-build-and-host.md) mounting a served surface
- [WF-IMP-024](WF-IMP-024-slim-the-sveltekit-host.md) — the bridge is written against the Rust binary's HTTP surface, not against TypeScript routes

This ticket runs **after the agent**, not before it. The contract it bridges to is defined by [WF-IMP-019](WF-IMP-019-rust-agent-binary-and-model-layer.md) and [WF-IMP-020](WF-IMP-020-durable-run-coordinator-in-rust.md); building it against routes that are about to move would mean building it twice. See [ADR 0004](../../adr/0004-rust-client-all-platforms.md).

## Decision this implements

[SvelteKit owns the URL; Rust mirrors it with `pushState`/`replaceState` and listens for `popstate`.](WF-DEC-004-one-rust-codebase-native-and-web.md) The wasm app never navigates the document. It tells the host where it wants to be, and the host decides whether that happens.

## The two directions

**Host → Rust.** One `#[wasm_bindgen]` export taking the current path. The host calls it on initial mount, on every SvelteKit navigation, and on `popstate`. Verified compiling in the probe described by [WF-INV-002](WF-INV-002-gpui-kit-web-renderer-reality.md).

**Rust → host.** One `document.dispatch_event(CustomEvent)` carrying the path the Rust view wants. The host applies it through SvelteKit's own router so the address bar, history and link interception all behave normally. Verified compiling in the same probe.

No bespoke bridge library exists or is needed. What is needed is a rule about who wins when the two disagree.

## Route table

There is no router in `gpui-kit`; this is the part that must be written. `gpui_base::NavStack`/`NavStackState` gives a `History`-backed LIFO stack of `AnyView`s with `push`, `pop`, `forward`, `replace`, `pop_to_root` and animated transitions, and knows nothing about URLs — which is exactly the split this ticket wants.

So: **NavStack owns the view stack, a route table owns the URL mapping, and nothing else does either.**

The route table mirrors `src/lib/components/features/app-shell/nav-items.ts`, which is already the project's flat path model: `chatItem`, `workflowsItem`, `settingsItem` with its `/settings` section prefix, the `settingsItems` sub-sections, `isNavActive` and `titleForPath`. It is ~30 lines of matching logic and it ports directly. It is the specification, not a design to improve on.

Decide and record in this ticket:

- **Matching**: exact match, or prefix for sections. `isNavActive` already encodes the one subtlety — `/settings/general` and the Settings tab share an href, so only the entry knows which is meant.
- **Params**: `/agents/[id]` is the only dynamic route. Decide whether `NavStack` entries carry the id, or whether the route is resolved from the path on each read.
- **Unknown paths**: they currently fall back to the chat surface (`titleForPath` returns `'Lexosa'`). Decide whether that stays or becomes an explicit not-found view.
- **Transitions**: `NavMotion::Animated` with `NavStack::transition(Transition)` for pushes, `Immediate` for pops. Record the chosen timing rather than leaving it at the default.

## Session

The `(app)` guard lives in `src/routes/(app)/+layout.server.ts:11`: no `locals.user` means `redirect(303, '/login?returnTo=…')`. The session cookie is the user id in `lexia_session`, unsigned.

- `FetchHttpClient` defaults to `RequestCredentials::SameOrigin` (`gpui-pre-web/src/http_client.rs:34`), so the cookie is sent unchanged against the same-origin host. No auth work is required for this to pass.
- The host must still own the guard, not the Rust app: an unauthenticated cold load must never paint the authenticated shell before the server redirects. Loading state matters here — see the "loader can disappear before the first frame" hazard in [WF-INV-002](WF-INV-002-gpui-kit-web-renderer-reality.md).
- `returnTo` must survive the round trip through the wasm app unchanged, including the query string.

## Native target

`pushState` has no meaning natively. The route table is the same, `NavStack` is the same, and the top of the app owns a plain location value instead of the browser's. The native binary must not grow a browser-only branch to make this work.

## Out of scope

- Porting any screen.
- The `Login`, `Settings` and `Bootstrap` screens themselves ([WF-IMP-016](WF-IMP-016-settings-login-and-bootstrap-routes.md)) — this ticket makes them reachable, not ported.
- Accessibility of the mount point ([WF-IMP-017](WF-IMP-017-accessibility-mirror-for-the-web-surface.md)).

## Acceptance criteria

- A cold load of any route paints that route, not the chat surface and not a blank canvas.
- Client-side navigation within the app updates the address bar without a document load, and a reload of the resulting URL paints the same route.
- Browser back and forward move through visited routes correctly, in both the wasm view and the address bar.
- An unauthenticated cold load of any `(app)` route redirects to `/login?returnTo=…` and lands back on the requested route after sign-in.
- `/agents/<id>` resolves to that sub-agent on a direct deep link.
- Rapid repeated navigation does not desynchronise the URL and the rendered view; a stale async route resolution cannot overwrite a newer one.
- The same route table drives the native binary, where the top of the app holds the location instead of the browser.

## Progress

Not started.