# Lexosa v0.1 — a self-extensible agent on a Rust host

Lexosa is an open-source, self-extensible agent: a permanent Rust host, dynamically loaded WebAssembly Components for every capability, provider-independent model access, persistent memory and skills, and a native and browser client built from the same views.

This document says what v0.1 contains. It is scoped to Lexosa's existing feature set — chat with a live run ledger, the thought tree, workflows, sub-agents, settings — rebuilt on the stack in [ADR 0005](../adr/0005-rust-agent-host-and-wasm-plugins.md) and [ADR 0004](../adr/0004-rust-client-all-platforms.md). It is not a rewrite of EVE, and EVE is not a dependency or a model.

## The four layers

```text
┌──────────────────────────────────────────────────────────────┐
│ Clients — one gpui-kit codebase                              │
│   native binary   ·   wasm32 bundle served by the host       │
└───────────────────────────┬──────────────────────────────────┘
                            │  HTTP + SSE
┌───────────────────────────▼──────────────────────────────────┐
│ Agent Core — small, immutable, Rust                          │
│   model runtime · agent loop · tool/skill runtime            │
│   WASM runtime · plugin registry · memory · sandbox          │
│   config · events                                            │
└───────────────────────────┬──────────────────────────────────┘
                            │  WIT interface, granted capabilities
┌───────────────────────────▼──────────────────────────────────┐
│ Plugins — WebAssembly Components, versioned generations      │
│   Plugin A · Plugin B · Plugin C · …                        │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│ Knowledge — data the host reads at runtime, never compiled in │
│   config · providers · models · memory · skills · plugins    │
│   registry/versions/active · workspaces · cache · logs       │
└──────────────────────────────────────────────────────────────┘
```

The host does not change when the agent gains a capability. That is the whole design: **self-extension without self-corruption.**

## v0.1 contains

**Core.** One distributable Rust binary. A CLI. An HTTP service the clients talk to. The agent loop. Configuration and events. One state directory, one writer.

**Models.** A `ModelProvider` abstraction, not a vendor. OpenAI-compatible endpoints, OpenRouter, and at least one direct provider adapter. Streaming, tool calls, structured output. One credential per provider, held the way they are held today — written to `<provider>.key`, owner-readable, with the environment outranking the file for the process lifetime.

**Plugins.** Wasmtime with WASI and the Component Model. A WIT-defined plugin ABI. Manifest-declared capabilities granted by the host. Content-addressed, versioned generations. Transactional promotion with automatic rollback. Per-plugin memory and CPU ceilings. Load and unload without restarting the host.

**Agent capabilities.** Tools and skills resolved at runtime rather than compiled in. Memory capture, distillation and retrieval. A workspace with explicit filesystem permissions. Shell execution through the sandbox. Sub-agents, role cards and prompt versioning.

**Self-extension.** The loop is in scope, not deferred: the agent notices a missing capability, writes a plugin, compiles it, validates it against the WIT interface, loads it as a candidate generation, health-tests it, and promotes it. If any step fails, the candidate is discarded and the active generation is untouched.

**Bootstrap.** First run asks where the agent runs, which provider to use, which models to map the use cases to, where the workspace is, and how strict the security profile is. **Skip setup is a first-class option** and the agent starts with working defaults.

**Security.** Plugin isolation. Filesystem, network, secrets and shell permissions declared and granted, never assumed. Resource limits. Content hashes on every plugin artifact. Automatic rollback.

**Clients.** The gpui-kit codebase, native and browser, sharing one set of views. The browser keeps the host's artifact renderers for the kinds the browser is better at.

## v0.1 explicitly does not contain

Distributed multi-agent orchestration. A plugin marketplace. Autonomous plugin publishing or signing. Plugin monetisation. Kubernetes or multi-cloud orchestration. Elaborate GUI beyond the product's own surfaces. Model training. A custom WASM runtime. A custom compiler toolchain.

Each of these is a real product decision rather than a deferral, and each would change the security model. They arrive behind their own decisions.

## What this replaces

| Today | Moves to |
| --- | --- |
| `my-agent/`, the `eve` npm package, `eve dev` | nothing — removed, not ported |
| `src/lib/server/eve.ts`, `eve-supervisor.ts` | the Rust binary's model layer ([WF-IMP-019](tickets/WF-IMP-019-rust-agent-binary-and-model-layer.md)) |
| `src/lib/server/runs.ts` | the Rust run coordinator ([WF-IMP-020](tickets/WF-IMP-020-durable-run-coordinator-in-rust.md)) |
| `src/lib/server/prompt.ts`, `turn.ts`, `memory.ts` | the Rust memory runtime ([WF-IMP-022](tickets/WF-IMP-022-memory-and-skills-in-rust.md)) |
| `src/lib/server/planner.ts`, `builtins.ts`, `agents.ts` | the Rust agent loop ([WF-IMP-023](tickets/WF-IMP-023-agent-loop-sub-agents-and-builtins.md)) |
| `src/lib/server/transcription.ts`, `attachments.ts`, `model-*`, `provider-*`, `thought-tree.ts` | the Rust binary ([WF-IMP-019](tickets/WF-IMP-019-rust-agent-binary-and-model-layer.md), [WF-IMP-023](tickets/WF-IMP-023-agent-loop-sub-agents-and-builtins.md)) |
| `src/lib/server/auth.ts` | **stays** — auth and sessions remain the host's |
| `src/lib/components/kit/artifacts/` | **stays** — the browser-native artifact renderers |
| `app-data/state/lexia.sqlite` | migrated to the Rust state directory |

Roughly 2,900 lines of agent TypeScript and 1,123 lines of tests move. The run coordinator's lease fencing and invariant tests are the part most likely to break silently in translation and are treated accordingly.

## Order of work

The agent moves before the client. The run and event contract is still being written, so porting a chat surface onto it now means porting it twice.

```text
in parallel   WF-IMP-008   build pipeline and toolchain for both targets
              WF-IMP-021   Wasmtime plugin host

agent         WF-IMP-019   binary, HTTP surface, model layer
              WF-IMP-020   durable run coordinator
              WF-IMP-022   memory and skills
              WF-IMP-023   agent loop, sub-agents, builtins
              WF-IMP-024   slim the host, rewire, migrate state
              WF-IMP-025   retire EVE and the TypeScript agent
      beside   WF-IMP-026   agent directory loader and watch contract

client        WF-IMP-012   URL and session bridge
              WF-IMP-013   app shell
              WF-IMP-009   chat
              WF-IMP-010   artifacts   · WF-IMP-018 host DOM islands
              WF-IMP-015   thinking, workflows, agents
              WF-IMP-016   settings, login, bootstrap
              WF-IMP-014   attachments
              WF-IMP-011   composer context model and voice
              WF-IMP-017   accessibility
```

WF-IMP-008 and WF-IMP-021 are genuinely independent of the port and can run alongside it.

The directory itself, the template and the tooling exist now — `lexosa-agent/`, `templates/agent/` and `scripts/agent.ts`. What is missing is the host side that reads them, which is `WF-IMP-026`, and it can be built as soon as `WF-IMP-019` exists.

## The open question this deliberately does not answer

Whether the agent and the client ship as one binary that serves its own wasm bundle, or as one source tree producing a library embedded in a client binary and a separate service. The build ticket decides it; nothing in the architecture above depends on the answer, which is the sign that it is a packaging question rather than an architectural one.