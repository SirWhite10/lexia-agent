## Project Configuration

- **Language**: TypeScript
- **Package Manager**: bun
- **Add-ons**: tailwindcss, playwright, ai-tools

---

# Lexia-Agent instructions

@RTK.md

## Project identity

- Product and agent name: **Lexia**
- Repository/project name: **Lexia-Agent**
- Folder name: `alexia-bot` (historical; do not use it as the product name)

## Operating rules

- Read the relevant router file in the workspace map before opening deeper files.
- Use Bun for package management, scripts, SQLite, and process orchestration. Do not introduce Node, npm, pnpm, or yarn workflows.
- Keep agent execution server-side. Clients are access surfaces only; never place model credentials, tool execution, or agent orchestration in SvelteKit client code or Tauri clients.
- Use Svelte 5 runes mode only. Do not add legacy Svelte reactivity, event directives, slots, or component syntax.
- Comment generated or non-obvious code with the reason, lifecycle, and ownership boundary future developers and agents need to understand.
- Treat `DESIGN.md` as the normative design-system artifact once its generator sources exist. Do not edit a generated `DESIGN.md` directly.
- Use Wayfinder-style tickets for feature work. Keep investigation, decisions, prototypes, and implementation tasks distinguishable.
- Use Git worktrees for agent coding tasks that need isolation. User-installed EVE updates use staged release folders, not worktrees.
- Never activate an imported agent extension before validation, build, health check, and explicit acceptance.

## Dev-only page instrumentation

To inject test or diagnostic code into every dev page load — before hydration, at parse time — extend the `transformPageChunk` hook in `src/hooks.server.ts`:

```ts
transformPageChunk: ({ html }) =>
	dev && injectMyScript ? html.replace('</head>', `<script>${myScript}</script>\n</head>`) : html,
```

Rules:

- Keep the injected script as a plain-string module under `src/lib/dev/` (ES5 body, no template literals, no build step). It is inlined verbatim into an inline `<script>`; parse-time execution is the point — snippets pasted into the console arm too late to observe pre-hydration behavior.
- Gate on `dev` from `$app/env` (Kit 3 renamed `$app/environment`; importing the old name fails `svelte-check`). Dead-branch elimination keeps the payload out of `bun run build` output — verify with a `grep` over the build artifacts after changing the gate.
- Mute instrumentation with a `false` constant, never by deleting it: findings, reading guides, and the failure modes it uncovered belong in the module's comments so later testing concepts can reuse the recipe (see `src/lib/dev/sidebar-observer.ts` for the format: method notes, constraints, findings).
- Log wall-clock `HH:MM:SS.mmm` plus a parse-relative `+Nms` offset on every line so output correlates with devtools Network/Performance while still measuring the window under test.

## Runtime decisions

- Jev is the first routing gate. It may select a builtin action, tool/workflow, or EVE model route.
- EVE executes the selected route and owns tools, skills, workflows, and model calls.
- OpenRouter is the initial model provider.
- Production users run EVE from an external release folder with `eve build` and `eve start`. `eve dev` is for Lexia development only.
- The compiled Bun Lexia host remains stable while EVE releases are staged, activated, restarted, or rolled back.

## Workspace map

Read the appropriate router first, then open only the file needed for the task:

- [PRODUCT.md](./PRODUCT.md): product architecture, runtime boundaries, and major source areas.
- [CONTENT.md](./CONTENT.md): domain language, product/design documentation, and prompt-facing content.
- [OPERATIONS.md](./OPERATIONS.md): repository rules, workflows, release management, and operational tooling.
- [CONTEXT.md](./CONTEXT.md): canonical Lexia domain glossary.
- [docs/adr/](./docs/adr/): durable architectural decisions and their rationale.
- [workflow/](./workflow/): repeatable workflows; read the workflow Markdown before its `scripts/` directory.

When a router is insufficient, update the router as part of the same change.
