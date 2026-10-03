# Product Router

Lexia is a server-side SvelteKit agent harness. Jev makes the first typed routing decision; EVE executes the selected builtin, workflow, tool, or model route. OpenRouter is the initial model provider.

- `my-agent/agent/` — EVE agent source: instructions, model config, channels, tools, skills, and subagents. Edited in place and run by `eve dev`; preserve EVE’s authored structure.
- `src/` — SvelteKit application and server integration. Planned; clients contain access/UI concerns only.
- `src/lib/components/ui/` — shadcn-svelte primitives. Planned.
- `src/lib/components/ai/` — Svelte-native AI interaction components. Planned.
- `src/lib/components/features/` — Lexia feature compositions. Planned.
- `src-tauri/` — Tauri desktop/mobile shell and native capabilities. Planned.
- `app-data/state/` — persistent Lexia and workflow state: SQLite run/action records and the event log, plus agent memory and personality files.
- `docs/planning/agent-capabilities-v1.md` — current planning baseline for intent plans, durable execution, and initial integrations; distinguishes selected scope from open provider validation.
- `docs/planning/tickets/` — Wayfinder-style investigation, decision, prototype, and implementation tickets for planned agent capabilities.
- `DESIGN.md` — current normative design-system baseline; once `spec.mdx`, `spec-config.ts`, and `spec:gen` exist, update it through those sources instead of editing it directly.
