# Lexia Context

This glossary defines the product-specific language used by Lexia and its agents. Use these terms consistently in code, tickets, documentation, and prompts.

## Product and runtime

**Lexia**:
The user-facing agent harness and the single main agent the user talks to; it coordinates routing, execution, tools, skills, workflows, sub-agents, and persistent memory.
_Avoid_: Alexia, Lexia-Agent when referring to the product itself.

**EVE runtime**:
The server-side execution runtime that runs an agent turn, tools, workflows, skills, and model calls after routing.
_Avoid_: frontend agent, client agent.

**Jev gate**:
The typed decision stage that classifies a request before execution and may select a builtin action, tool/workflow, or model route.
_Avoid_: Jev agent, Jev chatbot.

**intent plan**:
A structured plan representing one or more user-requested outcomes as independently executable actions, with dependencies, execution mode, required inputs, permission requirements, and status. A request with one clear action may have a one-action plan.
_Avoid_: single route when the request contains independent outcomes.

**action**:
One bounded outcome within an intent plan, executed by a builtin capability, integration, workflow, or agent task. Actions can succeed, fail, wait for input, or be cancelled independently.
_Avoid_: tool call when referring to the user's requested outcome rather than its implementation.

**run**:
A durable execution record for a user request, containing one or more actions and their outcomes.
_Avoid_: turn when referring to work that can continue after the response or browser session ends.

**run coordinator**:
The owner of durable run and action lifecycle, scheduling, dependency handling, recovery, and progress publication.
_Avoid_: model-owned background state.

**model tier**:
A logical capability class selected by Jev, such as `small`, `general`, `complex`, or `deep`; provider-specific model IDs belong in configuration.
_Avoid_: hard-coded model name when discussing routing policy.

## Conversation and memory

**massive chat**:
The single continuous conversation between the user and Lexia on this installation; it grows for the life of the install and is never split into separate chats.
_Avoid_: chat, session, thread when referring to the one conversation.

**command**:
A request to perform work, issued by the user or by one agent to another. A user command enters the massive chat as a message; an inter-agent command becomes a structured action request.
_Avoid_: message, action.

**delegation entry**:
The compact record of a sub-agent's work shown in the massive chat — progress and summary — while the full transcript stays in the run detail.
_Avoid_: transcript, log.

**memory**:
Lexia's durable user-specific context carried across turns: the verbatim user-message log plus distilled facts.
_Avoid_: conversation history, transcript, context window.

**memory fact**:
A concise durable fact distilled from conversation and stored in memory, versioned so it can be inspected and reverted.
_Avoid_: note, raw log, message.

**reflection**:
The turn-boundary step that distills memory facts and updates Lexia's personality.
_Avoid_: summarization, compaction.

**personality**:
Lexia's evolving set of timestamped traits, injected as a main block of its system prompt and updated by reflection.
_Avoid_: persona, profile, preferences.

**archive**:
Soft removal: the entry leaves active lists but stays queryable; user data is never hard-deleted.
_Avoid_: delete, remove.

## Agent runtime

**agent project**:
The EVE source tree the host runs with `eve dev` — `my-agent/` by default, `LEXIA_EVE_ROOT` overrides. It is edited in place and has no packaged or released form.
_Avoid_: release, bundle, production agent, deployed agent.

**agent source**:
The authored content under the agent project's `agent/` directory: instructions, model configuration, channels, tools, skills, and subagents. Editing it changes agent behaviour on the next turn.
_Avoid_: generated output, build artifact.

## Work execution

**workflow**:
A documented, repeatable multi-step operation with scripts stored under a named directory in `workflow/`.
_Avoid_: ad hoc script, command bundle.

**skill**:
An instruction and capability package that teaches an agent how to perform a bounded class of work.
_Avoid_: model, workflow.

**agent task**:
A user-approved unit of coding, research, or operational work that may run in an isolated Git worktree.
_Avoid_: background job when user review and promotion are required.

**sub-agent**:
An agent Lexia delegates work to — either a standing sub-agent or a one-shot agent task.
_Avoid_: worker, helper, process.

**standing sub-agent**:
A sub-agent with a role card and focus that persists across conversations until explicitly retired.
_Avoid_: sub-process, session.

**role card**:
A standing sub-agent's identity record: its Lexia-authored system prompt, focus, memory scope, and capability allowlist.
_Avoid_: profile, config.

**focus**:
A standing sub-agent's ongoing assignment, recorded on its role card.
_Avoid_: priority, current task.
