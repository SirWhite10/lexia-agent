---
status: accepted
---

# Keep intent-run execution durable in the stable Lexia host

The stable Lexia host owns durable run/action projections and an append-only per-run event log in `app-data/state/lexia.sqlite`; each state transition and its replay event commit atomically. EVE executes assigned actions under renewable leases, while SSE replays persisted events by per-run sequence so browser reconnects and EVE/host restarts do not erase run history. After an expired lease, actions with uncertain external side effects enter reconciliation instead of being blindly retried; automatic retries require a provider-backed idempotency key. This keeps the initial single-host design operationally simple while avoiding a false exactly-once guarantee; the dispatcher may later move to a separate queue without changing the run/action contract.

## Consequences

- A run is a durable parent record linked to the originating user message; actions are independently tracked children, and coding actions may contain worker tasks.
- Actions move through explicit planned, input/approval waiting, dependency-blocked, queued, running, reconciliation, and terminal states. If a prerequisite fails, dependent actions are cancelled with a reason; fallback behavior requires an explicit plan branch or replan. Parent status is a derived projection (`planning`, `active`, `waiting`, `succeeded`, `failed`, `cancelled`, or `partially_complete`).
- SQLite is the initial persistence and event-replay boundary. The current-status rows are query projections; the event log supports audit and replay without making every read rebuild state.
- SSE and in-memory pub/sub are delivery mechanisms only; the database remains authoritative.
- Execution attempts carry lease identity; stale worker reports cannot overwrite state after recovery or a later attempt.
- Provider calls cannot be made exactly once by database transactions. Reconciliation is required when a crash leaves an external result uncertain and the provider offers no idempotency contract.
- The current design targets one stable Lexia host. Multi-host dispatch, event retention policy, and cancellation semantics during execution remain open.
