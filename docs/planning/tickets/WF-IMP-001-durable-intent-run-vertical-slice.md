---
kind: implementation
status: implemented
---

# WF-IMP-001: Build the durable intent-run vertical slice

## Goal

Demonstrate that one internal-chat request can produce multiple independently progressing actions, with the run surviving browser refresh and coordinator restart. Use a deterministic builtin/background action for the slice; do not couple this foundation task to Google, Home Assistant, or payment-provider setup.

## Depends on

- [WF-DEC-001](WF-DEC-001-durable-intent-run-contract.md)
- [ADR 0002](../../adr/0002-durable-intent-run-state.md)

## Scope

- Persist a parent run, action records, dependency edges, and append-only run events in the existing state database.
- Dispatch eligible actions from the stable Lexosa host and report progress/results from the EVE execution boundary.
- Stream events over SSE with snapshot loading and cursor-based replay.
- Demonstrate one immediate action and one background action in the same run, including partial completion.
- Recover queued and waiting actions after restart; move expired running actions to reconciliation unless safe idempotent retry is supported.
- Keep result/event payloads sanitized and make cancellation behavior explicit before enabling external side-effect actions.

## Out of scope

- Real external integrations, user-configurable workflow construction, multi-host workers, durable scheduling/cron, and autonomous memory or skill mutation.

## Acceptance criteria

- A run and each action have stable IDs and independently queryable current state.
- An action cannot run until its dependencies, required inputs, and approvals are satisfied.
- Failed/cancelled prerequisites cancel dependent actions with a recorded reason; parent lifecycle is projected from child states, including partial completion.
- State changes and corresponding events are persisted atomically and replay in sequence after reconnect.
- Worker reports are fenced by execution attempt/lease identity so expired attempts cannot overwrite recovered state.
- Refreshing the browser does not lose the run; restarting the coordinator does not erase it or silently duplicate an uncertain side effect.
- A completed run distinguishes full success, failure, cancellation, and partial completion and provides a concise result/next-step summary.
- The chat view presents one parent request with action-level progress; the mobile focused view can use the same run without creating separate execution state.

## Progress

The vertical slice is implemented and verified. `runs`, `actions`, `action_dependencies`, and the append-only `run_events` log live in the state database; every transition commits atomically with its replay event, and parent status is a derived projection over child states.

- `src/lib/server/runs.ts` — coordinator data layer: plan commit with dependency gating, input/approval waits, lease claim/renew, lease-fenced reports, cancellation, startup lease recovery into `needs_reconciliation`, and cursor-based replay.
- `src/routes/api/runs/[id]/events/+server.ts` — SSE transport: snapshot first, then every event after the client's cursor, closing on a terminal run.
- `src/lib/server/builtins.ts` — deterministic `chat.ack` and `note.write` capabilities. The dispatcher sweeps until the run settles, because completing an action unblocks dependents that the same pass has already passed.
- `src/routes/(app)/+page.svelte` — the massive chat: one turn becomes one run, shown as a delegation entry with action-level progress streamed over SSE.
- `tests/runs.test.ts` — 9 invariant tests: dependency gating, cancellation cascade, parent-status projection, lease fencing, reconciliation policy, and event sequencing.

Verified live: a chat turn produced a run where the immediate action succeeded and the dependent background action ran afterwards, writing a real note file; the run reached `succeeded`; reconnecting with `Last-Event-ID: 2` replayed events 3–12 and closed on the terminal event.

Two deviations from the original acceptance wording, both deliberate:

- The **intent planner** does not yet exist. `?/send` writes a fixed immediate-plus-background plan so the execution contract is proven end to end; deriving that graph from user intent arrives with the planner slice.
- **EVE does not execute these builtins yet.** The host dispatches its own deterministic capabilities under the same lease protocol a real EVE worker will use, so the contract is real rather than simulated.
