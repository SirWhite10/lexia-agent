---
kind: implementation
status: planned
---

# WF-IMP-010: Port the artifact surface to gpui-kit and settle what "artifact" means there

## Goal

Carry the inline artifact surface into gpui-kit — the thing an agent hands back under a message — and resolve the one question the Svelte implementation answered by accident: what an artifact *is* when the renderer is not a browser.

## Depends on

- [WF-IMP-009](WF-IMP-009-port-chat-surface-to-gpui-kit.md)

## Reference behaviour

`src/lib/components/kit/artifacts/types.ts` is the contract: an artifact has a kind, a title, optional source, and a typed payload. `artifact-host.svelte` is the frame, with a Preview/Code toggle for source-carrying kinds. `cards/` holds one renderer per data kind. The kinds in use today are `html`, `svelte`, `react`, `code`, `table`, `record`, `products`, `link`, `metrics`, `chart`, `diff`, `files`, `checklist`, `timeline`, `json`, `terminal`, `media`.

## The fork this ticket must settle

In the browser the reference runs agent-authored source: HTML in a sandboxed `srcdoc` iframe with no `allow-same-origin`, Svelte compiled at render time with `svelte/compiler`, React rendered with React and htm. All three are browser capabilities with no GPUI Kit equivalent. The port has to choose, per kind:

1. **Render natively.** Typed data kinds become gpui-kit compositions — tables through `DataTable`, charts through `chart`, JSON and code through the crate's `text` and highlight support. This is the recommended default: it is the reason the artifact exists in gpui-kit at all.
2. **Embed a web view.** HTML artifacts render in a `WebView` with a locked-down policy, where the platform allows one. This keeps "the agent returns a document" working on native and web.
3. **Show the source.** Svelte and React artifacts degrade to a code view with no live preview, unless the investigation finds a portable evaluator.

Recording the choice per kind, with the reason, is a deliverable of this ticket. A kind that cannot be honoured should say so in the artifact frame rather than rendering an empty box.

## Scope

- Port the frame: header, kind marker, Preview/Code toggle, and the sizing contract (`auto`, tall, full).
- Port the data kinds listed above as native gpui-kit compositions, starting with the ones the product leans on: table, products, link, metrics, chart, diff, files, checklist, timeline, json, terminal, record, media.
- Port the code view with line numbers and copy, using the crate's highlight support rather than a hand-rolled tokenizer.
- Port the artifact body dispatch, including the explicit unsupported-kind state.
- Keep the source-typed kinds in the artifact contract even when they degrade to source, so the contract does not change shape when a renderer improves.

## Out of scope

- The chat surface ([WF-IMP-009](WF-IMP-009-port-chat-surface-to-gpui-kit.md)).
- Removing the Svelte artifact implementation.
- Adding new artifact kinds. If one is genuinely needed, it is a separate ticket.

## Acceptance criteria

- Every kind present in `types.ts` either renders in the gpui-kit build or shows its documented fallback; none renders blank.
- The documented per-kind rendering choice exists in this ticket and in the crate's module docs.
- Artifacts are keyboard reachable and scroll inside their own region; a wide table scrolls itself, never the page.
- An external link opens outside the surface, visibly, on both native and web.
- Side-by-side against `/kit`, both surfaces show the same information for the same artifact.

## Progress

Not started. The fork above is unresolved and is the first thing to do.