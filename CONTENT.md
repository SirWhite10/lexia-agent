# Content Router

This area contains the language, product intent, design guidance, and prompt-facing material that agents use to understand Lexosa.

- `CONTEXT.md` — canonical product glossary and domain language.
- `DESIGN.md` — generated design tokens and visual guidance once the design source files exist.
- `spec.mdx` — planned design-system source document; the generated artifact must not be edited directly.
- [`lexosa-agent/instructions/`](./lexosa-agent/instructions/) — the agent's prompt fragments, composed into its system prompt. Watched: an edit changes the next turn. This replaces `agent/instructions.md`, which was EVE-authored content and is removed by [WF-IMP-025](docs/planning/tickets/WF-IMP-025-retire-eve-and-the-typescript-agent.md).
- [`lexosa-agent/memory/facts/`](./lexosa-agent/memory/README.md) — durable facts the agent has distilled, as human-editable markdown. Watched: a change re-embeds that fact.
- [`lexosa-agent/skills/`](./lexosa-agent/skills/README.md) — how and when a capability is used. Watched: a new directory enters the index.
- `docs/adr/` — durable decisions explaining architectural trade-offs.
- `workflow/*/*.md` — human-readable instructions for repeatable workflows; read before workflow scripts.
