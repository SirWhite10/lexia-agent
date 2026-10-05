---
kind: implementation
status: planned
---

# WF-IMP-009: Port the chat surface to gpui-kit, matching the Svelte reference

## Goal

Recreate the conversation surface in gpui-kit so the browser build carries the real chat, not a component gallery: message rows, run progress, composer and the states the product must answer for.

## Depends on

- [WF-IMP-008](WF-IMP-008-gpui-kit-web-build-and-host.md)

## Reference behaviour

The Svelte implementation is the specification and the parity oracle. Read it before writing Rust:

- `src/lib/components/kit/chat/chat-surface.svelte` — states, harness controls, run replay
- `src/lib/components/kit/chat/message-item.svelte` — message row, attachments, context chips
- `src/lib/components/kit/chat/run-card.svelte` — run header, action ledger, cancel, elapsed time
- `src/lib/components/kit/chat/composer.svelte` — tray, context tools, undo and redo
- `src/lib/components/kit/chat/model.ts` — the data shapes, which mirror the real server model

Two behaviours must survive the port unchanged because they are product behaviour, not styling:

- **The run ledger.** One parent run, the sub-agents it spawned, their actions nested beneath, each with capability, agent, duration and status. Cancel is offered only while a run can still change.
- **Every state is reachable.** EVE offline, turn failure, setup not finished, empty conversation, a live run, a failed action, a cancelled run. If a state cannot be produced in the gpui-kit build, it is not ported.

## Translation notes

Three places where the reference implementation uses the browser in ways GPUI Kit does not:

- **Overlays.** The Svelte surface uses the platform `<dialog>` for modal focus, inertness and Escape. The port uses `window.open_dialog` / `window.open_sheet` with `FocusTrapElement` and verifies focus returns to the trigger — that behaviour is the point of those APIs.
- **Data source.** The reference simulates the run stream locally from `src/lib/components/kit/demo/transcript.ts`. The gpui-kit surface must read the real SSE endpoint the production chat uses, with the harness toggles retained as an alpha-only affordance for review.
- **Layout.** Preserve the three-size behaviour: single column with a sheet below `md`, a rail from `md`, a wider sidebar at `xl`, and no horizontal page overflow at 360px.

## Out of scope

- The composer context model ([WF-IMP-011](WF-IMP-011-composer-context-model-and-voice.md)).
- Artifacts ([WF-IMP-010](WF-IMP-010-port-artifact-surface-to-gpui-kit.md)).
- Removing the Svelte surface.

## Acceptance criteria

- A user turn, streamed progress, cancel, and the assistant reply all work against the real run stream in the gpui-kit build.
- Each state listed above is reachable and screenshotted at phone, tablet and desktop widths.
- Focus moves predictably through the conversation, and overlays trap and restore focus.
- The message scroller owns scrolling and offers the jump-to-latest control when the reader has scrolled away.
- Side-by-side against `/kit`, the two surfaces show the same information at the same sizes; differences are recorded as parity findings rather than smoothed away.

## Progress

Not started.