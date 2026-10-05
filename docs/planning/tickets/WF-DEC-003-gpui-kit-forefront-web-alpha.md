---
kind: decision
status: complete
---

# WF-DEC-003: GPUI Kit becomes the forefront interface; web ships alpha

## Goal

Decide which component layer leads the product, and what "alpha" is allowed to mean for the browser build while that lead is proved rather than assumed.

## Decision

**GPUI Kit is the forefront interface.** Component geometry, theme tokens, interaction behaviour and layout are authored against `gpui-kit` and rendered by it. The SvelteKit surfaces stop being where the design lives and become the place the design is served from and, for now, the reference implementation.

**The web build is alpha.** The browser path is published as alpha: the primitives are real and the theme is real, but the web renderer is unproven in this codebase and every known gap is recorded rather than smoothed over. Alpha covers the whole web target, not one screen.

**Nothing is deleted.** `src/lib/components/kit/**` and `src/routes/kit/**` stay in the tree after this change. That code is the working reference for the interface and the parity oracle for the port: when a gpui-kit screen disagrees with it, one of them is wrong and the difference is the bug report. Deletion is a separate decision with its own ticket, after parity is demonstrated.

**The host stays SvelteKit.** Serving, sessions, the run stream and the agent surfaces are unchanged. Only the rendering layer moves. A gpui-kit surface is a window the host embeds or mounts; it does not fork the server.

## Why

- GPUI is Rust and owns the interaction model this product needs: owned focus, real keyboard navigation, region-owned scrolling, a theme that changes at runtime. Re-expressing those in DOM is ongoing cost paid per screen.
- The existing GPUI Kit component layer is broader than the parts the Svelte port needed, so moving up front stops the two from diverging further.
- The risk is the browser build, not the design. Naming that alpha up front keeps the claim honest without blocking the migration.

## Scope boundary

This decision covers which component layer leads, what alpha means for web, and what is retained. It does not choose the web render backend, the WASM packaging strategy, or the cutover date for any individual screen — those are [WF-INV-002](../tickets/WF-INV-002-gpui-kit-web-renderer-reality.md) and the implementation tickets that follow it.

## Completion criteria

- The web target is labelled alpha in the code, not only in a ticket: the `/kit` page and the kit README both carry the marker.
- The reference implementation and its crate-sourced token transcription are retained and documented.
- The migration is expressed as ordered tickets with dependencies, so an agent can start from the investigation and work forward without re-litigating the decision.
- The relationship to the `origin/lexosa` branch — which already carries a chat composer and speech providers — is recorded, so voice mode is built once.

## Follow-up

[WF-INV-002](../tickets/WF-INV-002-gpui-kit-web-renderer-reality.md) establishes what the browser build actually is. If it fails, this decision stands for native and the web direction returns to the Svelte reference, which is exactly why that reference is retained.