# Product Router

Lexia is a server-side SvelteKit agent harness. Jev makes the first typed routing decision; EVE executes the selected builtin, workflow, tool, or model route. OpenRouter is the initial model provider.

- `agent/` — EVE agent configuration, instructions, channels, tools, skills, and subagents. Planned; preserve EVE’s authored structure.
- `src/` — SvelteKit application and server integration. Planned; clients contain access/UI concerns only.
- `src/lib/components/ui/` — shadcn-svelte primitives. Planned.
- `src/lib/components/ai/` — Svelte-native AI interaction components. Planned.
- `src/lib/components/features/` — Lexia feature compositions. Planned.
- `src-tauri/` — Tauri desktop/mobile shell and native capabilities. Planned.
- `app-data/releases/` — external immutable EVE releases for installed users. Not bundled into the Lexia executable.
- `app-data/state/` — persistent Lexia and workflow state, separate from replaceable EVE releases.
- `DESIGN.md` — current normative design-system baseline; once `spec.mdx`, `spec-config.ts`, and `spec:gen` exist, update it through those sources instead of editing it directly.
