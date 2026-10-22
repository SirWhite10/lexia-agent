# Workspace

The sandboxed filesystem root for this agent. It is where the agent reads and writes; anywhere else on the machine is refused by the host, not by convention.

Nothing belongs here that is not working material. It is safe to delete and safe to `.gitignore` — anything durable belongs in `memory/facts/`, and anything reusable belongs in `skills/`.

## Why this is not watched

`workspace/` is the agent's space, not a configuration surface. A file appearing here is the agent doing work, not the operator changing how the agent behaves. Watching it would make the agent react to its own output.

## The boundary

The host resolves every path the agent touches against this root and refuses anything that escapes it, including through `..`, symlinks, or an absolute path. The boundary is enforced in the host because an agent-authored plugin is exactly the kind of code that will try to cross it.

Do not put credentials in here. Credentials are named in a plugin manifest and resolved by the host at call time, so a secret is never written to a directory the agent can enumerate.