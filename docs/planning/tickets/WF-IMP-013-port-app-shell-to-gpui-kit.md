---
kind: implementation
status: planned
---

# WF-IMP-013: Port the app shell

## Goal

Recreate the authenticated shell in gpui-kit — sidebar, header, mobile tab bar, user menu — so every later surface lands into a container that already works.

## Depends on

- [WF-IMP-012](WF-IMP-012-url-and-session-bridge.md)

## Reference behaviour

The Svelte implementation is the specification and the parity oracle:

- `src/lib/components/features/app-shell/app-sidebar.svelte` — desktop rail, collapsible groups
- `src/lib/components/features/app-shell/app-header.svelte` — route title
- `src/lib/components/features/app-shell/app-bottom-nav.svelte` — mobile tab bar
- `src/lib/components/features/app-shell/user-menu.svelte` — sign out
- `src/lib/components/features/app-shell/nav-items.ts` — the route model, already ported to Rust by [WF-IMP-012](WF-IMP-012-url-and-session-bridge.md)

`gpui-component` supplies `sidebar`, `tab`, `breadcrumb`, `dock` and `title_bar`. All four sidebar/tab components are presentational: they take an application-owned selection flag (`SidebarMenuItem::active(bool)`, `TabBar::selected_index(usize)`) and never derive state from a location. That is the correct fit — the route table from [WF-IMP-012](WF-IMP-012-url-and-session-bridge.md) is the only thing that decides what is active.

The sidebar needs the standing sub-agent list that `(app)/+layout.server.ts` loads on every navigation. That data crosses the bridge like any other.

## Translation notes

- **Icons.** gpui-kit assets are fetched by HTTP from a configured endpoint, concatenated with the asset path. An endpoint with a trailing slash produces `//assets/` and blank icons. Set an endpoint without one, and host the paths.
- **Collapsible groups.** `SidebarCollapsible` takes `Icon`, `Offcanvas` or `None`. The reference's group open/closed state is application state, not a route concern.
- **Title bar.** `title_bar` is a native window-drag region. On web there is one document canvas and no window chrome, so the same component must render the title without a drag affordance. That difference is expected, not a parity bug.

## Out of scope

- The surfaces that render inside the shell.
- The mobile tab bar's bottom-inset handling on web, beyond preserving the layout.

## Acceptance criteria

- The shell renders at phone, tablet and desktop widths with no horizontal page overflow at 360px, matching the reference's three-size behaviour.
- Every navigation entry reaches its route, and the active entry is correct on every route including the `/settings` section and `/agents/<id>`.
- The header title matches `titleForPath` on every route, including the sub-agent name case.
- Group collapse state survives a client-side navigation and a reload.
- The user menu signs out and lands on the login surface.
- Icons render; no blank-icon cases reach the console.
- Keyboard reachable throughout, with a visible focus indicator.

## Progress

Not started.