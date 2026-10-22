---
kind: implementation
status: planned
---

# WF-IMP-010: Port the artifact surface to gpui-kit

## Goal

Carry the inline artifact surface into gpui-kit — the thing an agent hands back under a message — and resolve the one question the Svelte implementation answered by accident: what an artifact *is* when the renderer is not a browser.

## Depends on

- [WF-IMP-009](WF-IMP-009-port-chat-surface-to-gpui-kit.md) — artifacts render under a message
- [WF-IMP-018](WF-IMP-018-host-dom-island-bridge.md) — the delegation mechanism for `html`, `svelte` and `react` on web

## Reference behaviour

`src/lib/components/kit/artifacts/types.ts` is the contract: an artifact has a kind, a title, optional source, and a typed payload. `artifact-host.svelte` is the frame, with a Preview/Code toggle for source-carrying kinds. `cards/` holds one renderer per data kind. The kinds in use today are `html`, `svelte`, `react`, `code`, `table`, `record`, `products`, `link`, `metrics`, `chart`, `diff`, `files`, `checklist`, `timeline`, `json`, `terminal`, `media`.

## The rendering choice, per kind

Three options exist. The choice is now made; it is recorded here per kind, with the reason, because that record is a deliverable of this ticket.

1. **Render natively in gpui-kit.** The recommended default for typed data, and the reason the artifact surface exists in gpui-kit at all. Applies to: `table`, `record`, `products`, `link`, `metrics`, `chart`, `diff`, `files`, `checklist`, `timeline`, `json`, `terminal`, `media`, `code`.
2. **Embed a web view.** `gpui-wry` on native only — it is published but carries no wasm dependency, so the browser has no equivalent.
3. **Delegate to a host island.** On web, `html`, `svelte` and `react` are rendered by the host's existing renderers through [WF-IMP-018](WF-IMP-018-host-dom-island-bridge.md), unchanged. The host already has the better answer for all three: a sandboxed opaque-origin `srcdoc` iframe, `svelte/compiler` evaluated against the internals the page already ships, and React plus `htm` loaded on first use. Reimplementing a Svelte compiler in Rust to reach parity with `render-svelte.svelte` would be the wrong trade by a wide margin.

So `html`, `svelte` and `react` take **option 2 on native and option 3 on web**, and every data kind takes **option 1 on both**. None degrades to source-only.

What is shared across targets is the frame: header, kind marker, Preview/Code toggle and the `auto`/tall/full sizing contract. Only the body is delegated, and only where the host is genuinely better. That split is a capability boundary in the bridge, not a second implementation in view code ([ADR-0004](../../adr/0004-rust-client-all-platforms.md)).

A kind that cannot be honoured says so in the artifact frame rather than rendering an empty box.

## Scope

- Port the frame: header, kind marker, Preview/Code toggle, and the sizing contract (`auto`, tall, full).
- Port the data kinds listed above as native gpui-kit compositions, starting with the ones the product leans on: table, products, link, metrics, chart, diff, files, checklist, timeline, json, terminal, record, media.
- Port the code view with line numbers and copy, using the crate's highlight support rather than a hand-rolled tokenizer.
- Port the artifact body dispatch, including the explicit unsupported-kind state, behind the capability that [WF-IMP-018](WF-IMP-018-host-dom-island-bridge.md) defines.
- Keep the source-typed kinds in the artifact contract even when they degrade to source, so the contract does not change shape when a renderer improves.

## Out of scope

- The chat surface ([WF-IMP-009](WF-IMP-009-port-chat-surface-to-gpui-kit.md)).
- Removing the Svelte artifact implementation.
- Splitting the implementation per target. One view code serves both ([WF-DEC-004](WF-DEC-004-one-rust-codebase-native-and-web.md)); where a kind renders differently, the difference is a capability boundary and is recorded above, not a second implementation.
- Adding new artifact kinds. If one is genuinely needed, it is a separate ticket.

## Acceptance criteria

- Every kind present in `types.ts` either renders in the gpui-kit build or shows its documented fallback; none renders blank.
- The documented per-kind rendering choice exists in this ticket and in the crate's module docs.
- Artifacts are keyboard reachable and scroll inside their own region; a wide table scrolls itself, never the page.
- An external link opens outside the surface, visibly, on both native and web.
- Side-by-side against `/kit`, both surfaces show the same information for the same artifact.
- On web, `html`, `svelte` and `react` artifacts render live through the unchanged `/kit` renderers, with no edit to `render-html.svelte`, `render-svelte.svelte` or `react-runtime.ts`.
- The HTML island keeps the reference's security posture exactly: `sandbox="allow-scripts"` with no `allow-same-origin`, opaque origin, no access to the host page, its storage or its cookies.

## Progress

Not started. The rendering fork above is now settled; what remains is the frame, the data kinds, the code view and the dispatch.