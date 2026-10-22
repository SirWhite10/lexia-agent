---
kind: implementation
status: planned
---

# WF-IMP-023: The agent loop, sub-agents and builtins

## Goal

Build the loop that decides and executes: the router that classifies a turn, the run it produces, the sub-agents that run beneath it, and the capabilities — builtin or plugin — that carry out its actions.

## Depends on

- [WF-IMP-019](WF-IMP-019-rust-agent-binary-and-model-layer.md) for the model layer
- [WF-IMP-020](WF-IMP-020-durable-run-coordinator-in-rust.md) for the coordinator these actions run in
- [WF-IMP-021](WF-IMP-021-wasmtime-plugin-host.md) for the capability interface
- [WF-IMP-022](WF-IMP-022-memory-and-skills-in-rust.md) for memory and skills

## Reference behaviour

`src/lib/server/planner.ts`, `builtins.ts` and `agents.ts`, plus `tests/planner.test.ts`. [WF-IMP-001](WF-IMP-001-durable-intent-run-vertical-slice.md) and [WF-IMP-006](WF-IMP-006-sub-agent-management-surface.md) are `implemented` and verified in the running app.

## The router, honestly

`planner.ts` says so itself: it recognises exactly one intent — delegate this to a sub-agent — from wording, and *"the router that replaces it will read the thought tree in `/thinking` instead of a list of patterns"*. That was written before the thought tree had a runtime behind it.

The port is an opportunity to finish what the comment promises: the router reads the thought tree rather than matching phrases, and the thing a new workspace opens on is the pipeline the host actually runs — capture, retrieve, assemble, run, reflect. Jev exists and has no code behind it; this ticket is where it stops being a label.

What must not change is the *shape* the decision produces: one typed plan, one real run graph, and swapping the decision out later changes nothing downstream. That property is why the seam was drawn in the first place.

## Sub-agents

A *standing sub-agent* persists until explicitly retired; a one-shot *agent task* dies with its run and auto-archives. Standing sub-agents carry a role card — authored system prompt, focus, memory scope, capability allowlist — and a prompt that is versioned, so "what did it run with?" stays answerable. Revising the prompt creates a version rather than overwriting one. Retirement archives; nothing is hard-deleted.

Sub-agents run their own personality, receive role-scoped retrieval and explicit hand-off notes, and may message each other: commands are structured action requests in the action model, chat is plain text, and both route through the coordinator.

## Capabilities

Two kinds, one interface:

- **Builtins** are what the product needs regardless of plugins — recording a note, acknowledging a turn. They run in-process.
- **Plugins** arrive through [WF-IMP-021](WF-IMP-021-wasmtime-plugin-host.md) and are sandboxed.

Both appear to the agent loop as the same capability, which is what lets a builtin be replaced by a plugin later without the loop changing. Every builtin is a real side effect, not a stub; the current ones are deliberately boring precisely so the execution contract can be proved without coupling to a provider.

## Scope

- The routing decision, reading the thought tree.
- Plan → run graph → actions, through the Rust coordinator.
- Standing sub-agents, role cards, prompt versioning, lifecycle, and the agent-task history sourced from the run model.
- The builtin registry behind the same capability interface plugins use.
- **The capability registry, and the vocabulary it defines.** Every `agent.toml` grants capabilities by name, and the host refuses a name it does not recognise — so the registry, not any manifest, is the authority on what exists.

  **Decided.** The canonical vocabulary names the action, not the implementation:

  | Capability | Meaning |
  | --- | --- |
  | `chat.reply` | stream a reply back to the user |
  | `note.write` | record something as a note |
  | `subagent.spawn` | fan work out to a sub-agent |
  | `file.read` | read a file into the turn |
  | `artifact.write` | emit an inline artifact |
  | `memory.write` | write a durable fact |

  That is the composer's list, plus `note.write` which both sources already agreed on. So the **runner** was the thing that was wrong, not the composer: `chat.ack` becomes `chat.reply` and `subagent.delegate` becomes `subagent.spawn`, and four capabilities the composer offered but nothing implemented become real.

  It is also why neither existing list could simply be adopted. `chat.ack` describes what the code does rather than what the user gets, and a name that appears in the composer and in the run ledger is product surface, not an internal identifier.

  Because these six are what exist, `capabilities.allow` in a manifest naming anything else is refused with that name in the error — which is why `require` stays empty in the shipped example agent: a required capability is a startup gate, and a manifest must not be unloadable merely because the registry has not landed yet.
- Cancellation, and the run ledger the UI reads: one parent run, its spawned sub-agents, their nested actions, each with capability, agent, duration and status.
- Provider failures and turn failures surfacing as states the UI can render, not as a blank conversation.

## Out of scope

- The chat surface ([WF-IMP-009](WF-IMP-009-port-chat-surface-to-gpui-kit.md)).
- Building the plugin *writer* — the agent's side of self-extension is in [WF-IMP-021](WF-IMP-021-wasmtime-plugin-host.md); this ticket consumes what it loads.
- New providers or integrations beyond the model layer's.

## Acceptance criteria

- A turn produces a run whose actions the coordinator tracks, and a dependent action runs only after its dependency succeeds.
- The run ledger is complete: parent run, sub-agents, nested actions, with capability, agent, duration and status.
- Cancel is offered only while a run can still change, and cascades.
- A revised sub-agent prompt creates a new version; prior versions stay viewable.
- A retired sub-agent is archived and remains queryable; no destructive delete path exists.
- A builtin and a plugin are interchangeable behind one capability interface.
- A provider failure, a turn failure and a failed action each produce a distinct, reachable state — because "if a state cannot be produced, it was not ported" applies to the runtime too.
- `tests/planner.test.ts` passes against the Rust router.
- Every capability name offered by the composer either resolves through the registry or is removed from the composer; the two are one list, not two that drifted.
- A manifest naming a capability the registry does not know is refused with a message naming the unknown capability, not silently narrowed to the ones that exist.

## Progress

The foundations are in the binary and tested. The loop itself is not: nothing
drives a turn from a request yet.

**Done and verified**

- **`gpui-app/src/registry.rs` — the capability registry.** The authority on
  what exists. It holds the six capabilities the composer already offers, plus
  `note.write`, so the composer and the registry are one list rather than two
  that drifted. A manifest naming a capability the registry does not know is
  refused with that name in the error, in `allow`, `deny` and `require` alike:
  a silent narrowing would leave a manifest that says one thing and means
  another, and nothing would report it.
- **`gpui-app/src/builtins.rs` — the builtin capabilities.** `note.write`
  appends a file a person can read; `chat.reply` records the reply. Both are
  real local side effects, deliberately boring so the execution contract can be
  proved without coupling to a provider. A capability the host does not
  provide resolves to nothing rather than to an error, because it may be a
  plugin's.
- **`gpui-app/src/router.rs` — the turn router.** It reads a request and
  decides the run graph behind it. `tests/planner.test.ts` passes against it,
  case for case. The decision is one typed value behind a seam, so the router
  that replaces it can read the thought tree and nothing downstream changes.
  The original was regular expressions with lazy quantifiers the `regex` crate
  cannot express, so this matches word sequences instead; the one behaviour
  that differs is that whitespace inside a phrase is not significant here.
- **`gpui-app/src/subagents.rs` — the standing sub-agents.** The role card, the
  prompt versions and the message log the reflection reads. A revised prompt
  creates a version and never overwrites one, and retirement archives: there
  is no delete path in the module at all, deliberately.

- **`gpui-app/src/agent_loop.rs` — the plan into a run graph, and the
  dispatch.** A reply action, a delegation behind it when the decision asked
  for one, and a note behind the reply. The dispatch claims queued actions under
  a lease and runs them; a capability this host cannot run is left queued
  rather than failed, because a plugin worker may claim it later.
- **`POST /api/turn` — one turn, end to end.** Capture it, assemble the prompt
  around it, plan the run, answer it, and run whatever the plan left behind.
  It is the first thing that puts the whole stack — the memory corpus, the
  prompt's cache contract, the router, the run graph, the builtins and the
  coordinator — behind one request.

**Verified by running it**, not by inspection: a turn creates the run and its
actions, the reply action carries the provider's own failure (`401 User not
found`, because this host's key is not accepted), and the note behind it is
cancelled by the dependency. A turn with no usable provider still goes out: it
has no memory and no answer, and it says so, so a failed turn is a reachable
state rather than a blank conversation. An empty turn is refused with 422.

**Not done**

- **The sub-agent execution.** A delegation action names a role card and a
  task; the action is planned and the role card is created, but nothing runs
  the sub-agent yet.
- **Cancellation from the loop.** The coordinator can cancel a run and cascade;
  the loop does not yet offer it, and the run ledger the UI reads is not
  assembled from the loop's own state.
- **A provider failure, a turn failure and a failed action are distinct
  states, but only two of the three are produced.** A failed action and a
  failed reply are; a turn that fails before it plans anything is not, because
  the turn always plans first.