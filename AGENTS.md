## Project Configuration

- **Language**: TypeScript
- **Package Manager**: bun
- **Add-ons**: tailwindcss, playwright, ai-tools

---

# Agent instructions

@RTK.md

## Project identity

- Product and agent name: **Agent**
- Repository/project name: **agent-platform** (the GitHub remote is unchanged)
- Folder name: `Agent` (folder on disk; not the product name)

## Operating rules

- Read the relevant router file in the workspace map before opening deeper files.
- Use Bun for package management, scripts, SQLite, and process orchestration. Do not introduce Node, npm, pnpm, or yarn workflows.
- Keep agent execution server-side. Clients are access surfaces only; never place model credentials, tool execution, or agent orchestration in SvelteKit client code or in the Rust client.
- Use Svelte 5 runes mode only. Do not add legacy Svelte reactivity, event directives, slots, or component syntax.
- Comment generated or non-obvious code with the reason, lifecycle, and ownership boundary future developers and agents need to understand.
- Treat `DESIGN.md` as the normative design-system artifact once its generator sources exist. Do not edit a generated `DESIGN.md` directly.
- The Rust client under `gpui-app/` is built with cargo, not Bun, and pinned to a nightly toolchain by `rust-toolchain.toml`. Its views are shared across the native binary and the `wasm32` bundle; a `cfg(target_family = "wasm")` in view code is a finding, not a pattern, per [ADR 0004](docs/adr/0004-rust-client-all-platforms.md).
- Use Wayfinder-style tickets for feature work. Keep investigation, decisions, prototypes, and implementation tasks distinguishable.
- Use Git worktrees for agent coding tasks that need isolation.
- Never activate an imported agent extension before validation, health check, and explicit acceptance.
- A plugin is untrusted code, because the agent writes its own plugins. Never grant a capability a manifest does not request, never exceed a declared resource ceiling, and never promote a generation that has not passed validation and health checks.
- `agents/default/` is data, not source. The host watches it and a change applies on the next event. Never put logic there that needs a rebuild to take effect — that belongs in the host. Create an agent directory with `bun scripts/agent.ts new <name>`, never by hand.

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

- The agent is a **Rust binary** — the agent host. It owns the agent loop, tools, skills, memory, model calls, plugin loading, and all agent state. See [ADR 0005](./docs/adr/0005-rust-agent-host-and-wasm-plugins.md).
- Capabilities arrive as **WebAssembly Components** loaded through Wasmtime with a WIT-defined interface. Capabilities are declared in a plugin manifest and granted by the host; an undeclared capability is denied.
- Plugins are **versioned generations**, content-addressed, promoted transactionally with automatic rollback. A plugin never changes the host.
- Jev is the first routing gate, and it reads the thought tree. It moves into the host by [WF-IMP-023](./docs/planning/tickets/WF-IMP-023-agent-loop-sub-agents-and-builtins.md).
- Model access is provider-independent. OpenRouter is one provider, not the architecture.
- The client is one gpui-kit codebase compiled to a native binary and a `wasm32` bundle ([ADR 0004](./docs/adr/0004-rust-client-all-platforms.md)). SvelteKit is a slim host: auth, static serving, and the browser-native artifact renderers.
- **Retiring:** EVE, `my-agent/` and `eve dev` are superseded and are removed by [WF-IMP-025](./docs/planning/tickets/WF-IMP-025-retire-eve-and-the-typescript-agent.md). They still run today. Treat every reference to them as pending deletion; build nothing new on them.

## Order of work

The agent moves before the client. The run and event contract is still being written, so porting a surface onto it now means porting it twice. In parallel and independent of the port: [WF-IMP-008](./docs/planning/tickets/WF-IMP-008-gpui-kit-web-build-and-host.md) (build and toolchain) and [WF-IMP-021](./docs/planning/tickets/WF-IMP-021-wasmtime-plugin-host.md) (Wasmtime). Then 019 → 020 → 022 → 023 → 024 → 025, with [WF-IMP-026](./docs/planning/tickets/WF-IMP-026-agent-directory-loader-and-watch.md) alongside 019 once the binary exists, and only then the client chain. The full order is in [docs/planning/agent-runtime-v0.1.md](./docs/planning/agent-runtime-v0.1.md).

## Workspace map

Read the appropriate router first, then open only the file needed for the task:

- [PRODUCT.md](./PRODUCT.md): product architecture, runtime boundaries, and major source areas.
- [CONTENT.md](./CONTENT.md): domain language, product/design documentation, and prompt-facing content.
- [OPERATIONS.md](./OPERATIONS.md): repository rules, workflows, and operational tooling.
- [CONTEXT.md](./CONTEXT.md): canonical Agent domain glossary.
- [docs/adr/](./docs/adr/): durable architectural decisions and their rationale.
- [workflow/](./workflow/): repeatable workflows; read the workflow Markdown before its `scripts/` directory.
- [agents/default/](./agents/default/README.md): the agent data directory — structure and the watch contract.
- [templates/agent/](./templates/README.md): the agent directory template; `scripts/agent.ts` creates one from it.

When a router is insufficient, update the router as part of the same change.
