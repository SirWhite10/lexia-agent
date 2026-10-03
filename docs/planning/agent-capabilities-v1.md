# Agent capabilities and execution planning baseline

Status: planning baseline, 2026-09-23

This document records the current direction for Lexia's intent planning, background execution, and initial integration scope. It is a product/architecture plan, not an implementation specification. Items called out as open still need validation before implementation or release.

## Product direction

Lexia is a self-hosted personal assistant and agent harness. It should grow through integrations, tools, skills, and user-approved memory while keeping execution and credentials on the server side. The chat interface is the primary control surface. A mobile presentation may offer a focused chat and shortcut view over the same conversations, actions, approvals, and background runs rather than a separate agent.

EVE runs from its authored source in development mode only, supervised by the host ([ADR 0003](../adr/0003-eve-development-runtime.md)); there is no packaged or production runtime for V1. Jev remains the first typed routing gate and EVE remains the execution runtime, consistent with the project runtime decisions in `AGENTS.md` and `PRODUCT.md`.

## Initial product scope

The current V1 planning choice is:

- Internal Lexia chat as the only chat surface initially. External chat channels are deferred.
- Direct Google integrations for Gmail and Google Calendar.
- Local computer and device actions, with Home Assistant as the smart-home integration boundary.
- An optional agent-related virtual-card capability, isolated behind a payment integration and strict per-purchase controls. AgentCard.ai is a candidate for a personal pilot, subject to provider and terms validation.
- Manual integrations first. Composio may be evaluated later as an additional integration source, without making it a prerequisite for the initial architecture.
- No assigned phone number or live voice channel in this scope.

This is an initial capability set, not a claim that every action or provider integration must ship in the first milestone. Permissions, platform access, and provider eligibility remain gating details.

## Intent plans

Jev should be able to route a request into a plan path when it contains one or more outcomes. A planner then decomposes the request into actions and identifies prerequisites and dependencies. The planner may use a smaller model for decomposition; model choice is configuration, not part of the intent contract.

An intent plan should represent:

- The parent user request and its conversation.
- Each action's requested outcome and selected capability family.
- Execution mode: immediate, background, scheduled, or dependent on another action.
- Dependencies and resource conflicts, so independent actions can run concurrently and conflicting work can be serialized.
- Required inputs, connected accounts, permissions, and whether user approval is needed.
- Current status, progress, structured result, errors, and next steps.

Useful intent families include answering/research, retrieving/organizing information, creating/editing content, communicating, controlling local devices or apps, scheduling/monitoring, coding/delegation, managing Lexia (connections, skills, and memory), and purchases/payments. New skills and integrations add capabilities under these families; they should not require Jev to learn each vendor's API vocabulary.

Clarification and approval should be scoped to the action that needs them. A clear independent action can proceed while another action waits for a missing detail or approval. Dependencies such as “send this after the coding run succeeds” must be represented explicitly.

## Execution and durable state

The planned server-side flow is:

1. Jev classifies the incoming request and selects a direct route or intent-planning route.
2. The planner produces a one-action plan or a multi-action dependency graph.
3. A deterministic policy layer checks required inputs, permissions, approvals, and available capabilities.
4. The run coordinator persists the plan and schedules eligible actions.
5. EVE executes the selected builtin, integration, workflow, or agent task.
6. Results and state transitions are persisted and surfaced to the chat UI; the coordinator summarizes partial or complete outcomes.

The run coordinator, not an LLM turn or an EVE release, owns durable lifecycle state. The stable Lexia host owns run/action records and scheduling; EVE executes assigned actions and returns progress/results. A run references the originating user message and contains one or more actions. A coding action may contain child agent tasks, each with its own worker/worktree details.

Persist each action transition and its event in the same database transaction. Keep an append-only event history for audit/replay alongside current run/action rows for efficient snapshots; this is not full event sourcing. Use the existing `app-data/state/lexia.sqlite` state boundary for the initial single-host coordinator. Each run event has a monotonically increasing sequence within that run. SSE replays events after the client's last event identifier; on a fresh connection, the client loads a snapshot with its latest sequence, then streams events after that cursor. SQLite remains authoritative if the browser disconnects or the SSE connection drops.

Use an explicit action lifecycle: `planned`, `waiting_for_input`, `waiting_for_approval`, `blocked_by_dependency`, `queued`, `running`, `needs_reconciliation`, `succeeded`, `failed`, or `cancelled`. The coordinator queues an action only after its dependencies have succeeded and required inputs/approvals are present. Parent run status is a projection from child action states, including partial completion; it is not a second independently mutable truth. A worker returning a result is not automatically equivalent to the user's goal being complete; integration and review may have separate actions.

If a prerequisite fails or is cancelled, dependent actions are cancelled with a dependency-failure reason; continuing with a fallback requires an explicit plan branch or replanning. Parent status is `planning` while the plan is not committed, `active` while work can progress, `waiting` when progress requires input/approval/reconciliation, `succeeded` when all actions succeed, `failed` when all actions are terminal with no successes and at least one failure, `cancelled` when all actions are cancelled, and `partially_complete` when terminal outcomes are mixed. The coordinator records each execution attempt and fences worker reports by attempt/lease identity so late results from an expired attempt cannot overwrite newer state.

On host startup, queued actions remain eligible for dispatch and approval/input waits remain pending. A running action has a renewable execution lease. If the lease expires after a crash, mark the action `needs_reconciliation`; do not blindly repeat an external side effect whose outcome is unknown. Retry automatically only when the capability provides an idempotency mechanism and the same idempotency key can be reused. Otherwise require reconciliation or user direction. This avoids claiming exactly-once delivery across external APIs, which Lexia cannot guarantee.

The run coordinator persists before notifying subscribers. An SSE reconnect uses the last delivered event sequence and replays later records from SQLite, so in-memory notifications are only a latency optimization. Store sanitized event/result data; never write model credentials, card credentials, or private chain-of-thought into the event log. A disconnected browser must not stop or erase background work.

For coding delegation, workers should use separate Git worktrees and have shared task context. Shared files should normally be read-only across workers; overlapping edits should be sequenced or handed to an integration task. The main run should report changed files, validation performed, conflicts, and remaining work.

## Integration model

Keep user intent and run state provider-neutral. An integration advertises its capability, supported operations, authentication requirements, permissions/scopes, and event behavior. The planner selects a capability; a server-side adapter invokes its provider-specific API. MCP can later be one connector protocol, but it is not itself an authorization policy or a guarantee of equivalent action coverage.

Treat these as different concepts:

- **Integration** connects an account or external system and exposes capabilities.
- **Tool** performs one typed operation.
- **Skill** teaches the agent when and how to use a bounded set of capabilities.
- **Memory** stores durable user-specific context or preferences.
- **Workflow** performs a repeatable multi-step process.
- **Agent task** is a delegated unit of coding, research, or operational work.

Manual integrations should start with the narrow operations the V1 experience needs. Grant provider access progressively: prefer read access initially, then request send/edit scopes only when the user enables those actions. Do not load every integration's full tool catalog into every model call; resolve relevant capabilities after intent classification.

### Google

Implement Gmail and Google Calendar as direct Google integrations first. Keep the OAuth client and consent flow server-owned, store tokens securely, request only needed scopes, handle refresh and revocation, and record which connected Google account an action uses. Gmail permissions should be separated by capability where available (for example, read versus compose/send); calendar read and calendar write should likewise be independently selectable.

### Local computer and Home Assistant

Local computer capabilities execute through a server-side/local host boundary, never from browser code. Each operation should have a narrow schema and an explicit permission policy. Browser, file, shell, and desktop/device access are separate capabilities; avoid giving the model one unrestricted host-control tool.

Home Assistant is the initial smart-home adapter so Lexia can address entities and services through one local control plane rather than implement individual device brands. Connection setup, token storage, network reachability, allowed entities/domains, and confirmation rules are open implementation details. Device actions should return the concrete entity and resulting state where available.

### Agent-related virtual card

Represent purchasing as a separate `purchase` action with an explicit merchant or allowed merchant set, amount ceiling, currency, task purpose, funding source, approval state, expiry, and receipt/result. A provider must enforce limits outside the model. Card numbers, CVV, and payment credentials must not enter prompts, ordinary run logs, or general memory. The payment adapter should return opaque authorization/transaction identifiers and sanitized status.

AgentCard.ai advertises direct single-use virtual cards with per-card amount limits. Its current FAQ states that direct accounts require identity verification, are limited to US residents and supported merchants, and have a $150 per-card ceiling and $200 daily ceiling. Its terms page describes itself as a summary while full terms are being finalized. Treat it as a candidate, not a selected dependency, until its terms, API, data handling, account eligibility, and integration method are confirmed. Sources: [AgentCard FAQ](https://agentcard.ai/faq), [AgentCard terms summary](https://agentcard.ai/terms).

Do not confuse this direct personal offering with Agentcard.sh, which advertises a product wallet/API for embedding payments and currently lists a $5,000 monthly company plan. Stripe Issuing is another route but is card-program infrastructure with bank partners and program requirements, not a simple personal-card OAuth integration. Sources: [Agentcard.sh pricing and FAQ](https://www.agentcard.sh/), [Stripe Issuing](https://stripe.com/issuing).

V1 can include the purchase plan, approval UI, limits, audit trail, and receipt lifecycle while provider enablement remains opt-in and subject to due diligence. No unrestricted autonomous spending is implied by adding a card provider.

## Long-term learning and self-updates

Run state and long-term agent state are separate. Run/task progress is written automatically as operational state. A run should not silently rewrite durable memory, skills, or agent configuration merely because a model or worker produced new text.

The agent may propose a memory or skill update from successful work, including a short rationale and source. Apply such updates through a reviewable lifecycle with versioning and rollback. Store concise user-relevant facts, not raw worker transcripts. Imported extensions follow the existing rule in `AGENTS.md`: validate, build, health-check, and receive explicit acceptance before activation.

## Open decisions before implementation

- Confirm exact V1 actions for Gmail and Calendar, and the least-privilege OAuth scopes each requires.
- Decide whether Gmail sending and calendar writes are enabled initially or added after read-only operation is established.
- Define the first set of local computer operations and which actions require confirmation.
- Define Home Assistant connection onboarding and entity/service allowlisting.
- Verify AgentCard.ai's current full terms, API stability, supported jurisdictions/merchants, credential boundary, and account-level limits; decide whether V1 enables real spending or only prepares approved purchase requests.
- Specify cancellation behavior while EVE is executing an action and retention limits for run events/results.
- Define how coding workers are reviewed and how overlapping worktree changes are integrated.
- Define memory/skill proposal review, provenance, revision history, and rollback.

The core persistence, replay, and restart contract is recorded in [ADR 0002](../adr/0002-durable-intent-run-state.md). The first step toward testing routing with a real provider is [WF-IMP-002](tickets/WF-IMP-002-openrouter-model-test.md), a small server-side OpenRouter model test panel; it does not yet connect EVE execution or the logical model tiers. The next durable execution slice remains [WF-IMP-001](tickets/WF-IMP-001-durable-intent-run-vertical-slice.md).

## Current provider research references

These are primary product/documentation pages consulted for the planning snapshot; capabilities, eligibility, and pricing may change:

- [Google Gmail API sending](https://developers.google.com/workspace/gmail/api/guides/sending)
- [Google OAuth scopes](https://developers.google.com/identity/protocols/oauth2/scopes)
- [Home Assistant REST API](https://developers.home-assistant.io/docs/api/rest/)
- [AgentCard.ai FAQ](https://agentcard.ai/faq)
- [AgentCard.ai terms summary](https://agentcard.ai/terms)
- [Agentcard.sh](https://www.agentcard.sh/)
- [Stripe Issuing](https://stripe.com/issuing)
