---
kind: implementation
status: implemented
---

# WF-IMP-020: The durable run coordinator in Rust

## Goal

Port the run coordinator — the piece that makes a run survive a dropped browser, a crashed client and a restarted process — from TypeScript to Rust, unchanged in behaviour and fully covered by the invariants that already exist.

## Depends on

- [WF-IMP-019](WF-IMP-019-rust-agent-binary-and-model-layer.md) for the binary and its HTTP surface

## Why this is its own ticket

`src/lib/server/runs.ts` is 739 lines and 9 invariant tests, and it is the most load-bearing code in the product. It has already been through a schema cutover ([WF-IMP-003](WF-IMP-003-schema-cutover-messages-sub-agents-memory.md)) and a live verification. Porting it in passing, alongside the agent loop, would put the one component whose failure loses user data in the diff that has the most surface. It gets its own ticket and its own review.

## Reference behaviour

`src/lib/server/runs.ts` is the specification, and so are `tests/runs.test.ts`. The behaviours that must survive unchanged:

- Dependency gating. An action whose dependencies are unmet does not start.
- Lease claim, renew, and **lease-fenced reports**. A report from a lease that has been reclaimed must not land. This is what stops two processes writing one action's outcome.
- Startup lease recovery into `needs_reconciliation`. A process that dies mid-action leaves a lease that nobody will renew, and recovery is what makes that visible rather than silent.
- Parent status as a derived projection over child states, never a stored value that can drift.
- Cancellation cascading to children.
- The append-only `run_events` log, written in the same transaction as the state transition it records.
- SSE replay: snapshot first, then every event after the client's cursor, closing on a terminal run, honouring `Last-Event-ID`.

## Scope

- The same data model: `runs`, `actions`, `action_dependencies`, `run_events`.
- SQLite as the store, in the Rust binary's state directory, migrated from `app-data/state/lexia.sqlite`.
- The HTTP and SSE surface the clients consume.
- The transaction boundaries. A transition and its replay event commit together or not at all.

## The trap in this port

SQLite gives weaker-enough guarantees that it is easy to port code that "looks right" and quietly loses a transition. Two specific hazards:

- **Fenced writes.** A lease check and the write it guards must be in one transaction. Checking and then writing in two is a different, weaker system.
- **Event ordering.** `run_events` is append-only and its sequence is the replay cursor. A port that assigns sequence numbers outside the inserting transaction can produce a log that replays into a different history than the one that happened.

Port the tests first. They run against the TypeScript implementation today and are the parity oracle for the port.

## Out of scope

- The agent loop that produces the actions ([WF-IMP-023](WF-IMP-023-agent-loop-sub-agents-and-builtins.md)).
- Plugin capabilities.
- The client.

## Acceptance criteria

- Every one of the 9 invariant tests in `tests/runs.test.ts` passes against the Rust implementation, ported rather than rewritten, and the ported tests are retained alongside the originals until the cutover.
- A run survives: the browser reloads mid-run and resumes from `Last-Event-ID` with no gap and no duplicate.
- A process killed mid-action leaves a lease that recovers into `needs_reconciliation` on the next start.
- Two processes racing for one action produce exactly one winner; the loser's report is rejected.
- A cancelled run cascades and every child reaches a terminal state.
- Replaying a full event log reconstructs the same run state the live process held.
- The existing `lexia.sqlite` migrates to the Rust state directory, and after cutover only one process can write it.

## Progress

Ported and verified. `gpui-app/src/runs.rs` plus its tests, over a `rusqlite` store at `<state>/runs.sqlite`.

**All 9 invariants ported**, keeping their names and their claims: dependency gating, a failed dependency cancelling dependents with a recorded reason, input and approval waits, partial completion, cancellation cascade, lease fencing across a recovery, expiry becoming `needs_reconciliation` with retry gated on an idempotency key, sequenced events with cursor replay, and redaction. 29 tests in the crate pass; clippy is clean.

**Verified by running the server**, not only by unit test:

- A submitted plan gates correctly: `first → queued`, `second → blocked_by_dependency`, run `active`, events `run.created, action.planned ×2, action.queued, action.blocked`.
- Cancellation cascades and every child reaches `cancelled` with the reason recorded.
- The event stream sends a snapshot, then `id:`/`event:` frames, then `run.terminal`. Resuming with `Last-Event-ID: 3` returns frames 4–8; a fresh client gets the whole log in its snapshot and no redundant frames.
- An unknown run is `404` on every run route, not a hang and not a 500.
- State survives reopening the file, with the same statuses, progress and attempt count.

**Three bugs the smoke test caught**, none of which a unit test would have:

- A missing `rename = "dependsOn"` on the plan request made serde accept `depends_on` and ignore the camelCase clients actually send. Every submitted plan silently lost its dependencies — and the out-of-range validation that should have rejected one never fired. Fixed, with a test that reads a plan from the wire names.
- The stream resumed from the snapshot head instead of the caller's cursor, so a reconnecting client skipped the events it asked for.
- The replay frame was built as one formatted string containing newlines. SSE gives every field its own line, so the frame was unparseable and the client saw nothing. Now built field by field, so `Last-Event-ID` has something to resume from.

**Two things this is stronger than the TypeScript, deliberately.** Fenced writes put `WHERE status = 'running' AND lease_id = ?` in the statement itself, so the check and the write cannot be separated by another worker; the TypeScript reads, checks and *then* updates, with the read outside the transaction. Event sequences are assigned inside the writing transaction rather than read and computed around it. The port's tests cannot observe either difference — it takes a race — so the fencing tests assert the observable contract rather than claiming to test atomicity.

**A gap in the TypeScript found and closed.** `recoverExpiredLeases` was exported and tested but **never called anywhere** — startup recovery was documented in ADR 0002 and never wired. The Rust host calls it before the listener binds and warns when anything needed reconciling.

**One acceptance criterion deliberately not met.** `lexia.sqlite` is not migrated; the Rust host writes its own store beside it rather than over it. Migrating while the TypeScript host still runs would put two writers on one database, which is the failure this migration exists to prevent. The move belongs to the cutover in [WF-IMP-024](WF-IMP-024-slim-the-sveltekit-host.md), and the two paths are deliberately non-overlapping until then.