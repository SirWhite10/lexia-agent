# Memory

Facts the agent has distilled, as markdown. Humans read and edit these directly, so they are plain prose rather than a serialisation.

- `facts/` — one file per fact. Filename is the fact's stable identity; its contents are the statement.
- The vector index and its tags are **host-owned**. They are built from `facts/`, rebuilt when a fact changes, and are not edited here.

## Writing a fact

One durable statement per file. Frontmatter carries the metadata:

```markdown
---
id: bedroom-lighting
tags: [home, lighting]
---

The bedroom has three lights on the ceiling and one switch by the door.
```

`id` is stable across edits. Renaming a file is retiring a fact and creating another, which loses the history the versioning exists to preserve — change the contents instead.

A fact is something the user told you that will still be true next week. Preferences, names, locations, standing decisions, corrections. Not a transcript: the verbatim message log is captured separately and is not this folder.

## On change

`facts/` is watched. A changed fact is re-embedded and the index updated in place. Retrieval never mixes vector spaces, so a fact re-embedded under a different model has its old row invalidated rather than left to compete with the new one.

Deleting a file retires the fact. It is not a hard delete: it leaves the index and stays queryable, the same rule sub-agents follow.