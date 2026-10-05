---
status: accepted
supersedes: 0001
---

# Run EVE from source in development mode only

Lexosa runs EVE straight from its authored source tree with `eve dev`, supervised by the host, and that is the only supported way to run the agent. There is no `eve build` bundle, no versioned release folder, no staging/inspection/promotion/rollback lifecycle, and no production packaging of the Bun host. EVE's source lives in the working tree at `my-agent/` (configurable through `LEXIA_EVE_ROOT`), the host spawns it on `LEXIA_EVE_PORT`, and the supervisor starts on every boot rather than only under `bun run dev`. Agent work is therefore an ordinary edit-and-reload loop: change a file under `my-agent/agent/`, and EVE picks it up on the next turn. This supersedes ADR 0001, whose external-release model was speculative for a version whose actual problem is getting the agent to behave correctly, not shipping it.

## Consequences

- Updating the agent is editing `my-agent/agent/` in the working tree; there is no update, validation, or activation step to run.
- The host supervises EVE unconditionally. The previous `dev`-only gate existed because a production build expected an externally promoted release to already be running; with no production path that expectation is gone.
- `eve build`, `eve start`, `eve deploy`, and `eve eval` are not part of the operating procedure. The scaffold scripts remain in `my-agent/package.json` because they are EVE-authored content, but nothing in Lexosa calls them.
- Settings → Releases and the release lifecycle in `OPERATIONS.md` are removed; they described a workflow that no longer exists.
- ADR 0002 is unaffected. Durable runs, leases, the append-only event log, and SSE replay belong to the host's own state and do not depend on how EVE was started.
- Cost accepted: no reproducibility or rollback for the agent's code. A bad edit to `my-agent/agent/` is reverted with version control, not with a release promotion. Reproducing an old agent state later requires reconstructing from git.
- Reinstating a packaged or production runtime is a future decision, not a pending task. Nothing here forecloses it, but no code or documentation preserves the seams for it.