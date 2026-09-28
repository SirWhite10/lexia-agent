---
kind: implementation
status: implemented
---

# WF-IMP-007: Add a first-run bootstrap for instructions and personality

## Goal

A new install has an EVE scaffold whose instructions are placeholder text and no personality at
all, so the first turn runs against a generic agent. Give the user a short flow that writes the
two files that make the agent theirs, and get out of the way. Memory is deliberately not seeded:
the agent builds that itself as it works, and writing facts the user never said would be fiction.

## Depends on

- [WF-INV-001](WF-INV-001-eve-authored-structure-inventory.md) (where authored files live)
- [WF-IMP-004](WF-IMP-004-memory-capture-distillation-and-retrieval.md) (personality file format)

## Scope

- `src/lib/server/bootstrap.ts` composes `agent/instructions.md` from what the user said and seeds
  personality traits through the existing `savePersonality` writer, so the file keeps the format
  reflection already maintains.
- `/bootstrap` asks for name, purpose, tone, and working-style lines, then redirects home.
- The chat shows a setup card until the install is bootstrapped.

## Out of scope

- Seeding memory.
- Editing the files after the fact; both remain hand-editable.
- Validating a model or a provider during setup.

## Acceptance criteria

- The flow writes a readable `instructions.md` with identity, purpose, voice, working style, and
  an explicit note that memory is not hand-maintained.
- Personality traits land in the reflection-maintained file format, appended to any traits that
  already exist rather than replacing them.
- Re-running the flow is idempotent: no duplicate traits, no spurious file change.
- Once bootstrapped, `/bootstrap` redirects home and the chat no longer offers setup.
- The bootstrapped personality reaches the assembled system prompt.

## Notes

`instructions.md` is authored EVE content, so the project root is configuration
(`LEXIA_EVE_ROOT`, defaulting to `my-agent`) rather than an assumption. An authored EVE module
cannot read the filesystem — EVE rejects `node:fs` imports at evaluation time — so the provider
credential must come from the process environment, not a file read inside `agent.ts`.
