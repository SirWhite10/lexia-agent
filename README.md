# Agent

Agent is a self-extensible agent: a permanent Rust host, capabilities supplied as WebAssembly Components, provider-independent model access, and an interface built once and shipped as both a native binary and a browser bundle ([ADR 0005](./docs/adr/0005-rust-agent-host-and-wasm-plugins.md), [ADR 0004](./docs/adr/0004-rust-client-all-platforms.md)).

**That architecture is being built, not running.** What runs today is the previous one: a SvelteKit harness with a TypeScript agent runtime (`EVE`) executed from `my-agent/`. That path is superseded and is removed by [WF-IMP-025](./docs/planning/tickets/WF-IMP-025-retire-eve-and-the-typescript-agent.md). Do not build on it.

## Development

Install dependencies and start the app with Bun:

```sh
bun install
bun run dev
```

This starts the SvelteKit development server and, under it, the **retiring** EVE runtime. Both go away when the Rust agent host replaces them.

Run type checks and unit tests:

```sh
bun run check
bun run test:unit
```

## The agent

The agent's data lives in [`agents/default/`](./agents/default/README.md) — its manifest, instructions, memory, skills, sub-agents, plugin pointers and configuration. It is data, not source: the host watches it, so a change applies on the next event with no rebuild.

Create an agent from the template rather than by hand:

```sh
bun scripts/agent.ts new <name>      # agents/default/subagents/<name>/
bun scripts/agent.ts check <dir>     # structure and substitution check
```

The application uses Svelte 5 runes mode, shadcn-svelte primitives, Quicksand for interface text, and IA Writer Duo for code and operational output.

## Repository orientation

Read [AGENTS.md](./AGENTS.md) first. It carries the operating rules and the order of work. The workspace routers provide focused entry points:

- [PRODUCT.md](./PRODUCT.md) — product architecture and source areas
- [CONTENT.md](./CONTENT.md) — language, intent, prompt-facing material
- [OPERATIONS.md](./OPERATIONS.md) — rules, workflows, tooling
- [CONTEXT.md](./CONTEXT.md) — canonical domain glossary
- [agents/default/README.md](./agents/default/README.md) — the agent data directory and its watch contract
- [DESIGN.md](./DESIGN.md)