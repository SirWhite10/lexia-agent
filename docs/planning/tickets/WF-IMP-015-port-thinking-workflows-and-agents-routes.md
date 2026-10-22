---
kind: implementation
status: planned
---

# WF-IMP-015: Port `/thinking`, `/workflows` and `/agents`

## Goal

Convert the three remaining content routes to gpui-kit. They are display- and list-shaped rather than stream-shaped, so they are the cheapest evidence that the shell, route table and theme carry beyond the chat surface.

## Depends on

- [WF-IMP-013](WF-IMP-013-port-app-shell-to-gpui-kit.md)

## Reference behaviour

| Route | Reference | Character |
| --- | --- | --- |
| `/thinking` | `src/routes/(app)/thinking/+page.svelte`, `src/lib/components/features/thought-tree/` | An editable graph drawn blueprint-style, edited in a side panel. Opens on the pipeline the host actually runs — capture, retrieve, assemble, run, reflect — and **Restore default pipeline** re-adds only the nodes a tree is missing, so the operator's own edits survive. |
| `/workflows` | `src/routes/(app)/workflows/+page.svelte` | Workflow list and detail. |
| `/agents` | `src/routes/(app)/agents/+page.svelte`, `src/routes/(app)/agents/[id]/+page.svelte` | Standing sub-agent role cards, prompt version history, focus, lifecycle controls, and agent-task history from the run model. |

`/agents` is already complete per [WF-IMP-006](WF-IMP-006-sub-agent-management-surface.md) and is the parity oracle for that surface.

## Translation notes

- **Thought tree.** The graph is drawn, not laid out by the DOM. `gpui-component` has `tree` for hierarchical disclosure and `dock` for a resizable split workspace; the blueprint canvas is a custom element over those. Draw it once, in the shared view code, so native and web agree.
- **Restore default pipeline** is product behaviour, not styling: it must add only missing nodes and never remove an operator's edit. That rule is the acceptance criterion, not the node list.
- **Dynamic route.** `/agents/[id]` depends on [WF-IMP-012](WF-IMP-012-url-and-session-bridge.md) resolving the id from the path. The header title is the sub-agent name, which the route table already looks up.
- **Data.** All three are server-loaded today. They cross the same bridge as everything else and must render their own loading and empty states — an empty sub-agent list before any exists is a real state, not an edge case.

## Out of scope

- Editing agent memory, which stays markdown files.
- The peer-traffic and run-detail viewer.
- New workflow capabilities.

## Acceptance criteria

- All three routes render in gpui-kit, at phone, tablet and desktop widths, against real server data.
- `/agents/<id>` resolves on a direct deep link and the header shows that sub-agent's name.
- A thought tree with operator edits survives **Restore default pipeline**; only missing nodes are added.
- Empty states render for an empty sub-agent list and an empty workflow set, and are distinguishable from loading.
- Prompt version history shows prior versions; a revision creates a new version rather than overwriting.
- Keyboard reachable throughout; the thought-tree side panel traps and restores focus.

## Progress

Not started.