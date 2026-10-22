---
status: accepted
supersedes: 0003
---

# The agent is a Rust host; capabilities are WebAssembly Components

Lexosa's agent is a single Rust binary — the permanent product, not a scaffold for one. Everything it can *do* arrives through dynamically loaded WebAssembly **Components**, and everything it *knows* lives in data the binary reads at runtime. The binary does not change when the agent gains a capability.

EVE and the TypeScript agent runtime are removed. SvelteKit remains as a slim host and stops serving agent APIs.

This supersedes [ADR 0003](0003-eve-development-runtime.md), which made `eve dev` the only supported way to run the agent. It amends [ADR 0004](0004-rust-client-all-platforms.md), which described SvelteKit as keeping "the run stream and the agent surfaces"; both of those move to Rust.

## The plugin ABI is Wasmtime and WASI, on a documented boundary

Plugins are WebAssembly loaded through **Wasmtime** with **WASI**. The target is the **Component Model** with interfaces defined in **WIT**, which is what makes the ABI language-neutral by construction — one interface definition rather than one hand-written binding per language. The sandbox, the resource limits and the host-linking surface are then a mature runtime's problems rather than ones this project invents and maintains.

The first implementation uses **core `wasm32` modules** speaking a documented string-over-linear-memory ABI, because a Component-Model guest cannot be built on this host: `wit-bindgen` 0.62 cannot resolve the WIT layout, and `clang` carries the `wasm32-wasi` target with no `wasm-ld` to link it. Core modules are Wasmtime's own default path. See the amendment below.

What is *not* provisional: the sandbox, the resource ceilings, the capability model, the consent ledger, content-addressed generations and transactional promotion are all independent of the binary format.

## Capabilities are declared and granted, never assumed

A plugin does not receive filesystem, network, shell, secrets or memory access by being loaded. It requests them in a manifest and the host decides:

```toml
[plugin]
name = "github"
version = "1.0.0"

[capabilities]
network = true
filesystem = false
shell = false
memory = true
secrets = ["github_token"]

[resources]
memory_mb = 256
cpu_ms = 5000
```

This matters more here than in most plugin systems, because the agent can write its own plugins. An unrestricted default would mean agent-authored code inheriting host authority.

Concurrency is the same kind of decision. The runtime supports async and threads; a plugin is *granted* them, with memory and CPU ceilings, rather than being free to consume host resources because it asked for the default.

Declaring and granting are separate records, and that separation is the point. A plugin's descriptor says what it **asks** for; a consent ledger says what it **has**. The ledger is deny-by-default, keyed by `(agent, plugin, capability)`, records `Granted` and `Denied` as distinct outcomes, and treats an absent decision as a denial rather than a pending prompt. Revoking a grant deactivates the running extension immediately rather than at the next load. A grant one agent made does not follow the plugin to another agent.

Capabilities are a closed set, not free-form strings, so a manifest naming one the host does not implement fails at parse with the name in the error rather than resolving to nothing.

This is modelled on `studio-platform`'s plugin registry, whose consent module states the same rule: *"nothing is granted until an explicit user decision is recorded… revocation takes effect immediately by deactivating active extensions."*

## Amendment — core modules, not Components

Decided 2026-10-05. The first implementation uses core `wasm32` modules rather than WebAssembly Components. The reason is tooling, not preference: a Component-Model guest cannot be built on this host. Wasmtime, WASI, the capability model, the consent ledger, content-addressed generations and transactional promotion are all unaffected.

The deviation is in what a *guest* is compiled to, not in how the host treats it. When a component toolchain is available the guests change shape; the host, the manifest, the consent ledger and the generation store do not. [WF-IMP-021](../planning/tickets/WF-IMP-021-wasmtime-plugin-host.md) records what switching costs and keeps the language-neutral proof outstanding until it is met — a Rust guest demonstrates the host, not the claim.

## Plugins are versioned generations, promoted transactionally

A plugin is not replaced in place. Generations accumulate, are content-addressed by hash, and exactly one is active:

```text
github
├── v1  a81c…
├── v2  91bd…
├── v3  f03a…
└── v4  5d2e…   ← active
```

Promotion runs generate → compile → validate against the ABI → load → sandbox test → health test → promote. Failure discards the candidate; the active generation is untouched. Rollback is instant because the previous generation is still on disk and never mutated.

Content addressing gives immutable versions, deduplication, integrity verification and crash recovery for free, and it lets the agent say "promote generation 12 to 13" rather than "overwrite this file".

The point is self-extension without self-corruption. **The agent can upgrade itself without the host restarting.**

## The core is small and the rest is data

The immutable core is a runtime: model access, the agent loop, tool and skill runtimes, the WASM runtime, the plugin registry, memory, sandbox, configuration, events.

Everything else is data under the agent's state directory — config, providers, models, memory, skills, plugins (registry, versions, active), workspaces, cache, logs. Changing the agent's behaviour should not require rebuilding the binary.

## Model access is provider-independent

The model layer is an abstraction over providers — OpenAI-compatible endpoints, OpenRouter, and direct provider adapters — with a single credential per provider and a per-provider model catalogue. Nothing in the agent loop is written against one vendor.

## Harnesses are adapters

Codex, MCP, or any future harness is a way to *drive* the agent, not the architecture. The agent's internal interface is this project's own; an external harness is one more adapter on the edge.

## SvelteKit becomes a slim host

SvelteKit keeps three things: **auth and sessions**, **static serving** of the page and the wasm bundle, and **the browser-native artifact renderers** — the `/kit` HTML, Svelte and React paths, which stay because the browser has the better answer for them ([WF-IMP-018](../planning/tickets/WF-IMP-018-host-dom-island-bridge.md)).

It stops serving agent APIs. The browser reaches the Rust binary directly for those, and the host is reachable on its own origin. Everything in `src/lib/server/` that is agent logic moves to Rust; `auth.ts` remains, but the session it protects becomes a contract between two processes rather than a table in one.

## All state moves to Rust

One store, one writer. Runs, actions, the append-only event log, memory, sub-agents, model configuration and attachments live in the Rust binary's state directory. `app-data/state/lexia.sqlite` is migrated rather than read from two places.

## Consequences

- **The agent moves before the UI does.** The run and event contract is still being written; porting the chat surface onto it now means porting it twice. The GPUI chain is re-ordered behind the agent work, and only the build and toolchain work proceeds in parallel.
- **A real port, not a rewrite.** Roughly 2,900 lines of agent TypeScript and 1,123 lines of its tests move to Rust, including the run coordinator's lease fencing and its invariant tests, which are the part most likely to break silently in translation.
- **Existing state needs migrating**, and the two stores must never be writable at once during the cutover.
- **Two origins.** The browser reaches SvelteKit and the Rust binary, so cross-origin setup or a reverse proxy becomes a deployment concern it was not before.
- **Session verification is now a two-party question.** The `lexia_session` cookie is an unsigned user id, so both processes can read it; which one issues it, and which one asks whom, is settled by the first implementation ticket rather than by this record.
- **Deletion is gated on replacement.** EVE, `my-agent/`, the supervisor and the agent modules are removed by an explicit retirement ticket once the Rust binary answers for them — not before, and not piecemeal.
- **The plugin system is a security surface.** It is the first thing in this project that executes code the agent itself wrote. The capability model, the resource limits and the promotion gate are load-bearing, not hardening to add later.
- **Rust is now the only language that can change the agent's core.** That is the intended trade: a stable host with a replaceable surface, in exchange for a single-language core.