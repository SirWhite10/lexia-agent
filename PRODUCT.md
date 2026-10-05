# Product Router

Lexosa is a server-side SvelteKit agent harness. Jev makes the first typed routing decision; EVE executes the selected builtin, workflow, tool, or model route. OpenRouter is the initial model provider.

Two surfaces describe that routing rather than performing it. `/thinking` is the **thought tree**: an editable graph of the questions Lexosa asks itself, drawn blueprint-style and edited in a side panel. A new workspace opens on the pipeline the host actually runs — capture, retrieve, assemble, run, reflect — and **Restore default pipeline** adds back only the nodes a tree is missing, so the operator's own edits survive. It is the map, not the runtime: Jev, the routing decision, has no code behind it yet and the page says so. `/settings/models` maps **use-cases** (Always, Quick answers, Research and search, Heavy work, plus any the operator adds) to provider models, which the host consults; EVE's own model still comes from its authored `my-agent/agent/agent.ts`.

- `my-agent/agent/` — EVE agent source: instructions, model config, channels, tools, skills, and subagents. Edited in place and run by `eve dev`; preserve EVE’s authored structure.
- `src/` — SvelteKit application and server integration. Planned; clients contain access/UI concerns only.
- `src/lib/components/ui/` — shadcn-svelte primitives. Planned.
- `src/lib/components/kit/` — GPUI Kit browser port: component layer plus the crate's default theme, scoped to `.kit`. Web is alpha per [WF-DEC-003](docs/planning/tickets/WF-DEC-003-gpui-kit-forefront-web-alpha.md); this is the reference implementation and parity oracle for the GPUI Kit migration, not a shipping surface. Its README records the crate source for every value.
- `src/routes/kit/` — UI test page: the chat interface, the live component set and the artifact gallery, at phone, tablet and desktop widths.
- `src/lib/components/ai/` — Svelte-native AI interaction components. Planned.
- `src/lib/components/features/` — Lexosa feature compositions. Planned.
- `src-tauri/` — Tauri desktop/mobile shell and native capabilities. Planned.
- `app-data/state/` — persistent Lexosa and workflow state: SQLite run/action records and the event log, thought-tree and use-case rows, attachment metadata, plus agent memory and personality files. Uploaded bytes live beside it in `app-data/state/attachments/`.
- `docs/planning/agent-capabilities-v1.md` — current planning baseline for intent plans, durable execution, and initial integrations; distinguishes selected scope from open provider validation.
- `docs/planning/tickets/` — Wayfinder-style investigation, decision, prototype, and implementation tickets for planned agent capabilities.
- `DESIGN.md` — current normative design-system baseline; once `spec.mdx`, `spec-config.ts`, and `spec:gen` exist, update it through those sources instead of editing it directly.
