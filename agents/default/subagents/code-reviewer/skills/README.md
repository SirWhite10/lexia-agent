# Skills

A skill says **how and when** a capability is used. It is not the capability. The capability is a builtin in the host or a plugin under `plugins/`; the skill is the knowledge of when reaching for it is the right move.

One directory per skill, with a `SKILL.md` at its root:

```text
skills/
└── turn-off-the-lights/
    └── SKILL.md
```

```markdown
---
name: turn-off-the-lights
requires: [home.lights]
---

Use when the user asks for a light to be switched off and names a room.

1. Resolve the room against `memory/facts/`. If it is ambiguous, ask.
2. Call `home.lights` with the resolved room.
3. Confirm what changed, naming the room.
```

## Resolution

The agent searches this index when it meets something it does not know how to do. A skill's `requires` names the capabilities it needs. If a required capability is absent, that is the load-or-create path from `plugins/README.md`, not an error: the agent can build the plugin.

## On change

`skills/` is watched. A new directory enters the index immediately; nothing needs a restart or a rebuild.

Keep a skill short. It is loaded into the agent's working context, so a long skill costs context on every turn that reads it, and a skill that could apply to everything applies to nothing.