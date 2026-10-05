---
kind: decision
status: complete
---

# WF-DEC-001: Define the durable intent-run contract

## Goal

Settle how a compound user request becomes persistent actions, how clients replay progress, and what recovery means after a process crash. This decision is the common foundation for background coding, Google actions, Home Assistant, and purchases.

## Decision

The stable Lexosa host owns run/action state, SQLite persistence, scheduling, and recovery. EVE executes assigned actions. Each state transition and replay event is committed atomically; SSE replays persisted events by a per-run sequence. Expired execution leases enter reconciliation unless the capability supports a reusable idempotency key. Parent run status is derived from child actions, and partial completion is represented explicitly.

Full consequences and status definitions are recorded in [ADR 0002](../../adr/0002-durable-intent-run-state.md).

## Scope boundary

This decision covers persistence ownership, action lifecycle, event replay, and crash recovery. It does not select Google OAuth scopes, define every local-computer permission, select the final virtual-card provider, or specify multi-host scheduling.

## Completion criteria

- Parent run and child action responsibilities are explicit.
- Dependencies, input/approval waits, partial outcomes, retries, and reconciliation have defined semantics.
- Refresh/reconnect replay does not depend on process memory.
- The ADR and canonical glossary record the decision.

## Follow-up

Implement the bounded end-to-end slice in [WF-IMP-001](WF-IMP-001-durable-intent-run-vertical-slice.md).
