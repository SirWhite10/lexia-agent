# Agent Context

This glossary defines the product-specific language used by Agent and its agents. Use these terms consistently in code, tickets, documentation, and prompts.

## Product and runtime

**Agent**:
The user-facing agent harness and the single main agent the user talks to; it coordinates routing, execution, tools, skills, workflows, sub-agents, and persistent memory.
_Avoid_: Agent, Agent, agent-platform when referring to the product itself.

**thought tree**:
The editable graph of questions Agent asks itself while routing a request, drawn at `/thinking`. Each node is a question, a route decision, or an action; its answers open the branch below. Edited by the operator, and later by the agent.
_Avoid_: mind map, decision tree, flowchart.

**provider**:
An outside service Agent can be configured to call — OpenRouter, OpenAI, ElevenLabs, Deepgram, Stability AI, Runway. Each has exactly one credential, held in Settings → Providers, and declares the modalities it serves.
_Avoid_: integration, connector, vendor, channel.

**modality**:
The kind of work a model does: text, spoken replies, transcriptions, images, or video. Every use-case belongs to one modality, which is what decides which providers and models may be assigned to it.
_Avoid_: capability, media type, content type.

**use case**:
A named job a model is chosen for — Always, Quick answers, Research and search, Heavy work, Spoken replies, Transcriptions, Images, Video, or one the operator adds. The fallback is Always; anything unset falls through to it. An assignment names a model and the provider that serves it.
_Avoid_: tier, mode, persona.

**attachment**:
A file or voice note sent with one chat message. Bytes are stored under the state directory and referenced from the message; the agent is told what arrived, not handed the contents.
_Avoid_: upload batch, blob.

**EVE runtime** _(retiring)_:
The TypeScript execution runtime that ran an agent turn, tools, workflows, skills and model calls after routing. Replaced by the **agent host** per [ADR 0005](docs/adr/0005-rust-agent-host-and-wasm-plugins.md); the term survives only until [WF-IMP-025](docs/planning/tickets/WF-IMP-025-retire-eve-and-the-typescript-agent.md) removes it.
_Avoid_: frontend agent, client agent.

**agent host**:
The Rust binary that runs an agent turn, resolves tools and skills, calls models, loads plugins and owns all agent state. The permanent product; it does not change when the agent gains a capability.
_Avoid_: agent project, EVE runtime, backend script.

**plugin**:
A WebAssembly Component loaded by the agent host that provides one capability. Written in any language the Component Model binds, sandboxed, granted its capabilities by the host, and versioned.
_Avoid_: skill, tool script, extension.

**capability**:
Something the agent can do, whether builtin or supplied by a plugin. One interface, so a builtin can be replaced by a plugin without the agent loop changing.
_Avoid_: permission, privilege.

**generation**:
One immutable, content-addressed version of a plugin. Exactly one is active; promotion is transactional and rollback is instant because earlier generations are never mutated.
_Avoid_: version, release.

**host**:
The SvelteKit application that serves auth, static files and the browser-native artifact renderers, and mounts the client. It is not the agent and does not serve agent APIs.
_Avoid_: server, backend, app.

**Jev gate**:
The typed decision stage that classifies a request before execution and may select a builtin action, a plugin capability, or a model route. Reads the thought tree; moved into the agent host by [WF-IMP-023](docs/planning/tickets/WF-IMP-023-agent-loop-sub-agents-and-builtins.md).
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
The single continuous conversation between the user and Agent on this installation; it grows for the life of the install and is never split into separate chats.
_Avoid_: chat, session, thread when referring to the one conversation.

**command**:
A request to perform work, issued by the user or by one agent to another. A user command enters the massive chat as a message; an inter-agent command becomes a structured action request.
_Avoid_: message, action.

**delegation entry**:
The compact record of a sub-agent's work shown in the massive chat — progress and summary — while the full transcript stays in the run detail.
_Avoid_: transcript, log.

**memory**:
Agent's durable user-specific context carried across turns: the verbatim user-message log plus distilled facts.
_Avoid_: conversation history, transcript, context window.

**memory fact**:
A concise durable fact distilled from conversation and stored in memory, versioned so it can be inspected and reverted.
_Avoid_: note, raw log, message.

**reflection**:
The turn-boundary step that distills memory facts and updates Agent's personality.
_Avoid_: summarization, compaction.

**personality**:
Agent's evolving set of timestamped traits, injected as a main block of its system prompt and updated by reflection.
_Avoid_: persona, profile, preferences.

**archive**:
Soft removal: the entry leaves active lists but stays queryable; user data is never hard-deleted.
_Avoid_: delete, remove.

## Agent runtime

**agent directory**:
A directory holding one agent's data — manifest, instructions, memory, skills, sub-agents, plugin pointers, configuration, workspace. Not source: nothing in it is compiled, and a change takes effect on the next watch event without a rebuild. `agents/default/` is the default agent's.
_Avoid_: agent project, agent source, config file.

**agent manifest**:
`agent.toml` in an agent directory: identity, prompt fragments, memory scope, capability grant, sandbox profile. It is the role card for a sub-agent. Read at start and re-read on change; an invalid manifest is rejected and the previous state stays in force.
_Avoid_: config, settings, metadata.

**skill**:
How and when a capability is used, as opposed to the capability itself. A skill names the plugins it requires; the capability is a builtin in the host or a plugin. Loaded into working context, so a skill that could apply to everything applies to nothing.
_Avoid_: capability, tool, instruction.

**fact**:
A durable statement distilled from conversation, held as human-editable markdown under an agent's `memory/facts/`. Versioned so it can be inspected and reverted. Not a transcript.
_Avoid_: memory, note, log entry.

**watch contract**:
Which paths in an agent directory the host observes, and what a change to each one does. Recorded in [agents/default/README.md](agents/default/README.md). Paths the host writes to — `cache/`, `logs/`, `plugins/versions/` — are excluded by construction, because a host that watches its own writes reacts to them.
_Avoid_: hot reload, file watching.

**agent project** _(retiring)_:
The EVE source tree the host ran with `eve dev` — `my-agent/` by default, `AGENT_EVE_ROOT` overrides. Removed by [WF-IMP-025](docs/planning/tickets/WF-IMP-025-retire-eve-and-the-typescript-agent.md) once the agent host answers for it.
_Avoid_: release, bundle, production agent, deployed agent.

**agent source** _(retiring)_:
The authored content under the agent project's `agent/` directory: instructions, model configuration, channels, tools, skills and subagents. Superseded by the agent host's prompt and skills; removed by [WF-IMP-025](docs/planning/tickets/WF-IMP-025-retire-eve-and-the-typescript-agent.md).
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
An agent Agent delegates work to — either a standing sub-agent or a one-shot agent task.
_Avoid_: worker, helper, process.

**standing sub-agent**:
A sub-agent with a role card and focus that persists across conversations until explicitly retired.
_Avoid_: sub-process, session.

**role card**:
A standing sub-agent's identity record: its Agent-authored system prompt, focus, memory scope, and capability allowlist.
_Avoid_: profile, config.

**focus**:
A standing sub-agent's ongoing assignment, recorded on its role card.
_Avoid_: priority, current task.
