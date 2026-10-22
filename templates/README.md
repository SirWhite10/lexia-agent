# Agent templates

`agent/` is the template every agent directory is created from. `agent.zip` is the same tree packed, committed alongside it so creating an agent does not require the template directory to be present.

## Why both

The template is committed as a **readable tree**, not only as an archive, because a change to the shape of an agent directory should be a diff a reviewer can read. A zip alone makes that change opaque: the reviewer sees a binary blob change and no way to tell what moved.

The zip is committed too because a packed artifact is what you want at the point of use — handing someone a directory of scaffolding and telling them to pack it themselves is a step that will be skipped.

The two are kept identical by `check`:

```sh
bun scripts/agent.ts check agents/default     # structure parity, no unsubstituted tokens
```

That is why the default agent lives in the repository rather than only being generated: running `check` against it is what proves the template and a real instance have not drifted apart.

## Editing the template

Change `agent/`, then:

```sh
bun scripts/agent.ts pack
```

`pack` is deterministic — the same tree always produces the same bytes — so a rebuild that changes the zip means the tree changed, and a rebuild that does not change it proves the zip is current. Commit both in the same change.

## Tokens

| Token | Substituted with |
| --- | --- |
| `{{AGENT_NAME}}` | the directory name, a slug: lowercase words joined by single dashes |
| `{{AGENT_TITLE}}` | the same, title-cased |
| `{{AGENT_DESCRIPTION}}` | a placeholder line the operator replaces |

Tokens live only in `agent.toml` and `instructions/core.md`. Everything else in the template is identical across agents and carries no tokens, which is why a new directory needs exactly three edits to be real.

`check` rejects a directory where a token survived substitution. A directory with `{{AGENT_NAME}}` still in its manifest is a directory nobody finished.

## Usage

```sh
bun scripts/agent.ts new <name>                        # agents/default/subagents/<name>/
bun scripts/agent.ts new <name> --parent <dir>        # anywhere else
bun scripts/agent.ts check <dir>                       # validate
```

`new` refuses to overwrite an existing directory. An agent that already has a prompt version history, a memory index and possibly plugins is not something to replace by accident.