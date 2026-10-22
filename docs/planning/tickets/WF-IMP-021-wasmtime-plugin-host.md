---
kind: implementation
status: planned
---

# WF-IMP-021: The Wasmtime plugin host

## Goal

Load capabilities as WebAssembly Components through Wasmtime, with a WIT-defined interface, capabilities the host grants rather than the plugin assumes, and versioned generations that promote transactionally and roll back automatically.

## Depends on

- [WF-IMP-008](WF-IMP-008-gpui-kit-web-build-and-host.md) creating the Rust workspace
- [ADR 0005](../../adr/0005-rust-agent-host-and-wasm-plugins.md)

Independent of the agent port. This can be built and proven while [WF-IMP-019](WF-IMP-019-rust-agent-binary-and-model-layer.md) is in flight.

## The stack, and why it is not negotiable

Wasmtime, WASI, and the **Component Model** with **WIT**-defined interfaces. Not raw `wasm32` modules with a hand-rolled ABI.

That choice buys language-neutral bindings — Rust, Go, C/C++, Zig, JavaScript/TypeScript — generated from one interface definition, rather than one hand-written binding per language that then has to be kept in step. It also inherits a mature runtime's answers for sandboxing, resource limits and host linking, rather than requiring this project to invent and maintain its own. The component model is the plugin ABI.

## Keep the ABI small

Expose the minimum that makes a capability useful. Something in the shape of:

```text
initialize() · execute() · shutdown()

host.log(…)      host.memory.*      host.skill.*
host.tool.*      host.http.*        host.storage.*      host.spawn()
```

A small interface is what makes the SDKs thin in every language. The temptation to expose the whole host must be resisted: every function added to the ABI is a function agent-authored code can call.

## Capabilities are granted, not assumed

A plugin declares what it needs; the host decides what it gets.

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

The default when a plugin does not declare a capability is **denied**. This matters more here than in most plugin systems, because the agent writes its own plugins; an unrestricted default would hand agent-authored code host authority.

Concurrency is the same kind of decision. The runtime supports async and threads; a plugin is granted them under ceilings rather than being free to consume host resources because it asked for the default.

Secrets are named, not blanket-granted: `secrets = ["github_token"]` means the plugin can read that one, through a host API that logs access.

## Consent is separate from declaration

The manifest says what a plugin **asks** for. It never says what it **gets**. Those are two records, and conflating them is the mistake to avoid: a grant decided once at install time is a grant the plugin keeps forever, including after it is found to be misbehaving.

The model is `studio-platform`'s — `crates/studio-plugin-registry/src/consent.rs`, whose own module summary is the requirement restated:

> "Consent is deny-by-default: nothing is granted until an explicit user decision is recorded, every grant is scoped to `(project, plugin, capability)`, and revocation takes effect immediately by deactivating active extensions."

So:

- **Requested** lives in the plugin descriptor. Declaring is a request, never a grant.
- **Granted** lives in a consent ledger keyed by `(agent, plugin, capability)`. Lexosa's "project" is the agent, so a grant one agent made does not follow the plugin to another.
- **The default is denied**, and it is not a prompt waiting to happen — an absent decision *is* the answer.
- **Revocation is immediate and deactivating.** Removing a grant stops the active extension; it does not wait for the plugin to restart.
- **Both outcomes are recorded.** `Granted` and `Denied` are explicit entries, so "the user said no" is distinguishable from "nobody was asked".

This matters more here than in most plugin systems, and that is the whole argument for it: the agent writes its own plugins, so the thing requesting `shell.exec` may be code the agent authored five minutes ago.

Also taken from the same crate: capabilities are a **closed enum**, not free-form strings — `DeclaredCapability`, with stable serde wire names in `descriptor.rs`. A manifest naming a capability the host does not implement is a parse error that names it, not a string that quietly resolves to nothing.

## Generations, promotion and rollback

```text
github
├── v1  a81c…    ← retired, still on disk
├── v2  91bd…    ← retired, still on disk
├── v3  f03a…    ← retired, still on disk
└── v4  5d2e…    ← active
```

Promotion is transactional: generate → compile → validate against the WIT interface → load as a candidate → sandbox test → health test → promote. Any failure discards the candidate and the active generation is never touched. Rollback is instant because previous generations are never mutated.

Content-addressed storage gives immutable versions, deduplication, integrity verification and crash recovery, and it lets the agent say "promote generation 12 to 13" instead of overwriting a file.

## Scope

- Wasmtime with WASI and the Component Model, linked into the binary.
- The WIT interface and generated host bindings.
- Manifest parsing, validation, and the capability grant decision.
- Content-addressed storage under the state directory: `registry/`, `versions/`, `active/`.
- Install, load, unload, promote, rollback, and hot loading without restarting the host.
- Per-plugin memory and CPU ceilings, and a plugin panic or trap that cannot take down the host.
- The self-extension loop: the agent writes a plugin, compiles it, validates it, loads it as a candidate, health-tests it, and promotes it.
- One real capability delivered as a plugin, so the ABI is proven against something that matters rather than a demo.

## Out of scope

- Signing or publishing plugins. Content hashes now, signatures later, and a marketplace not at all.
- Cross-language SDK packages. The WIT interface is the contract; SDKs follow once it is stable.
- Bundled tooling for plugin authors beyond what validation needs.

## Acceptance criteria

- A plugin written in a language other than Rust loads, calls a host capability, and returns — proving the ABI is language-neutral and not a Rust-shaped interface in disguise.
- A plugin declaring no capabilities cannot touch the filesystem, the network, the shell or any secret.
- A plugin declaring `secrets = ["github_token"]` can read that secret and cannot read another, and the access is logged.
- A plugin exceeding its memory or CPU ceiling is stopped, and the host survives it.
- A plugin that traps, panics or hangs does not take down the host or any other plugin.
- A failed candidate generation is discarded with the active generation untouched; a deliberate rollback restores the previous generation without a host restart.
- The self-extension loop completes end to end: the agent creates a plugin, and a capability that did not exist before now exists.
- Plugin artifacts are content-addressed and verifiable by hash.
- A plugin's declared capability and its granted capability are separate records; declaring is not granting.
- Nothing is granted without a recorded decision, and an absent decision is a denial rather than an implicit grant.
- A grant made by one agent does not follow the plugin to another agent.
- Revoking a grant stops the active extension immediately, with no restart and no reload.
- A `Denied` decision is distinguishable from an absent one.
- A capability the host does not implement is a parse error naming it, not a silently ignored string.

## Progress

The host is built and proven against a **real WebAssembly artifact**. `gpui-app/src/plugins/` holds the capability enum, the consent ledger, the generation store and the Wasmtime runtime; `guests/echo` is a genuine `no_std` plugin compiled to `wasm32-unknown-unknown` and loaded by the tests. 53 tests in the crate pass, clippy is clean.

**Deliberately different from the original draft, twice:**

- **Core modules, not Components.** A Component-Model guest cannot be built on this host: `wit-bindgen` 0.62 cannot resolve the WIT layout in any arrangement tried, and `clang` has the `wasm32-wasi` target with no `wasm-ld` to link it. [ADR 0005](../../adr/0005-rust-agent-host-and-wasm-plugins.md) is amended to record the deviation. The sandbox, ceilings, capability model, consent ledger and generation store are unaffected — only what a guest is compiled to.
- **WASI is not linked.** A plugin reaches the filesystem through a granted host capability whose paths resolve against the sandbox root, not through a preopen. That is stricter: with preopens a plugin granted nothing still receives a `path_open` import, so the safety argument is "the preopen is narrow". Here the import does not exist, so the argument is "there is nothing to call". The consequence is that **an ungranted import fails to instantiate**, which is what makes the consent ledger enforceable rather than advisory.

**Proven by running it:**

- A candidate grants nothing until promoted; a promoted bad build fails loudly, and `rollback` restores the previous generation with no rebuild because the bytes were never mutated.
- A promoted plugin that was never granted anything **cannot instantiate** — its `lexosa_host_log` import is absent.
- A granted plugin loads, answers the ABI, and returns a response the host reads back correctly.
- Revoking a grant stops the next call, not the next restart, and reports what to deactivate.
- A grant made by one agent does not carry to another.
- A capability the host cannot serve (`memory_*`, until [WF-IMP-022](WF-IMP-022-memory-and-skills-in-rust.md)) cannot be granted, and a grant for something the plugin never requested is refused — both where a person is looking.
- A tampered artifact is detected on read: the hash is the identity, so "the file changed under us" is a detected fact rather than an assumption.
- A manifest cannot raise its own ceiling; requested resources are clamped at install.
- Installing a non-wasm artifact, or a manifest that does not parse, is refused by name.

**The language-neutral proof is outstanding, and stays outstanding.** `clang` can target wasm here but cannot link without `wasm-ld`, so no C guest can be built. The guest that exists is Rust. That demonstrates the host works; it does **not** demonstrate the ABI is language-neutral, and this ticket does not claim it does.

**Not delivered, and named rather than left as a silent gap:**

- The self-extension loop — the agent deciding a capability is missing, writing the plugin, compiling it and promoting it — belongs to the agent loop ([WF-IMP-023](WF-IMP-023-agent-loop-sub-agents-and-builtins.md)), which consumes what this loads. Install and promote are exposed; nothing yet *authors* a plugin.
- No route invokes a plugin. The agent loop drives calls and the tests prove the path; a route exposing `execute` with no loop deciding to call it would be a capability with no caller.
- `http_*` and `shell` are implemented behind a grant but were not exercised against a live endpoint or a real command in this session.

The next step this unblocks is [WF-IMP-022](WF-IMP-022-memory-and-skills-in-rust.md), which retires the `memory_*` refusal and gives skills something to resolve to.
