# Content Router

This area contains the language, product intent, design guidance, and prompt-facing material that agents use to understand Lexia.

- `CONTEXT.md` — canonical product glossary and domain language.
- `DESIGN.md` — generated design tokens and visual guidance once the design source files exist.
- `spec.mdx` — planned design-system source document; the generated artifact must not be edited directly.
- `agent/instructions.md` — stable human-maintained EVE runtime instructions; dynamic runtime data belongs in TypeScript/state, not by silently rewriting this file.
- `docs/adr/` — durable decisions explaining architectural trade-offs.
- `workflow/*/*.md` — human-readable instructions for repeatable workflows; read before workflow scripts.
