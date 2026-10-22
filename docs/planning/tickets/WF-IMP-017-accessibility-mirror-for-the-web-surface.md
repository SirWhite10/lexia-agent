---
kind: implementation
status: planned
---

# WF-IMP-017: The accessibility mirror for the web surface

## Goal

Give the browser surface semantics a screen reader and a keyboard-only user can reach. A canvas has none, so this is built rather than inherited — and it is the difference between a web product and a picture of one.

## Depends on

- [WF-IMP-012](WF-IMP-012-url-and-session-bridge.md)
- [WF-IMP-013](WF-IMP-013-port-app-shell-to-gpui-kit.md) for the shell's own affordances

## The gap

Upstream states it plainly:

> "Components can express roles and labels in GPUI, while this web example is painted into a canvas."
> "Do not assume that a native accessibility bridge or a GPUI accessibility property produces equivalent web semantics. Provide an accessible HTML alternative for content or actions that your target browser cannot expose."

`accesskit` is a GPUI-side dependency, and it produces platform accessibility trees on desktop. On web there is no DOM to expose, so those properties go nowhere a browser can read.

## What this means for the migration

The web surface is not accessible by default, and no amount of correct gpui-kit composition changes that. The options, in the order worth considering:

1. **Mirror the interactive surface into a parallel DOM layer.** Each focusable gpui-kit control also exists as a real HTML element positioned over the canvas, carrying its role, label and state, and forwarding activation into Rust. This is what makes a canvas application usable with a screen reader and is the only option that preserves the single implementation.
2. **Provide accessible HTML fallbacks per route** for content and actions, and leave the canvas as the visual layer. Cheaper, and honest about what it gives up: it is two implementations and they will drift.
3. **Accept the gap on web and ship native accessibility only.** This is the option upstream's own showcase effectively takes. It is a real product decision, not a technical workaround, and it should be written down as one rather than left unstated.

Record which is chosen here and why. If the answer is 3, say so plainly and note that the web surface is not accessible — that is a legitimate call, but it must be a call.

## Scope

- The decision above, recorded with its reason.
- The chosen mechanism, applied to the shell navigation and the primary actions of each ported route.
- Keyboard reachability for everything reachable by pointer, with a visible focus indicator that is not colour-only.
- Live-region announcements for the states a screen-reader user cannot see: run progress, upload progress, and failures.
- A statement of what remains inaccessible, if anything.

## Out of scope

- Accessibility work on the native target, which inherits platform accessibility trees from `accesskit`.
- WCAG conformance certification.
- Screen-reader testing on the Svelte reference.

## Acceptance criteria

- Every control reachable by pointer is reachable by keyboard, in a visible order, with a non-colour focus indicator.
- The chosen mechanism is implemented and exercised on every ported route, or the chosen gap is documented as a product decision.
- Run start, run failure and upload failure are announced, not only drawn.
- A screen-reader pass over the chat and settings routes finds no unnamed interactive element.
- No information is conveyed by colour alone.

## Progress

Not started. Decide the mechanism before the surfaces land — retrofitting accessibility across a ported application is far more expensive than building it in.