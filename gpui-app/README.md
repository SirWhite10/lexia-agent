# `lexosa` — the agent host

One crate, two distributions. The `cdylib` is the browser client; the binary is the agent host and, later, the native window. Shared code lives in `lib`, so a module written once is present in both.

This directory implements [WF-IMP-019](../docs/planning/tickets/WF-IMP-019-rust-agent-binary-and-model-layer.md), [WF-IMP-020](../docs/planning/tickets/WF-IMP-020-durable-run-coordinator-in-rust.md) and [WF-IMP-021](../docs/planning/tickets/WF-IMP-021-wasmtime-plugin-host.md): the binary, the HTTP surface, the model layer, the durable run coordinator, and the plugin host. Memory, the agent loop and the client are separate tickets.

## Run it

```sh
cargo build
cargo run -- models          # use-cases and what each resolves to
cargo run -- providers       # providers and which credential is in force
cargo run -- serve           # the HTTP surface, on 127.0.0.1:8787
```

From the repository root, so `lexosa-agent/agent.toml` resolves. Override with `--agent-dir`, `--state-dir`, or `LEXOSA_AGENT_DIR` / `LEXOSA_STATE_DIR`.

```sh
cargo test
cargo clippy --all-targets
```

## Layout

```text
src/
├── state.rs          where the host writes. One writer, always.
├── config.rs         agent.toml — the authored data that is not state
├── credentials.rs    one key per provider, and which one is in force
├── models/
│   ├── mod.rs        the provider abstraction. Nothing above this names a vendor.
│   └── openai_compatible.rs   the only file that knows a wire format
├── runs.rs          the durable run coordinator and its event log
│   └── tests.rs     the 9 invariants ported from tests/runs.test.ts
├── plugins/
│   ├── capabilities.rs   the closed capability enum and the manifest
│   ├── consent.rs        deny-by-default, per agent, revocable
│   ├── store.rs          content-addressed generations, promote, roll back
│   ├── runtime.rs        Wasmtime, the sandbox, the linked imports
│   └── tests.rs          the lifecycle, against a real artifact
└── guests/echo/     a real plugin, built to wasm32-unknown-unknown
├── http.rs           the surface the clients are written against
├── error.rs          the error type, and `Secret`
└── main.rs           the CLI
```

Each layer knows only the ones below it. `models` knows nothing about HTTP, and nothing in the crate builds a vendor's request shape outside `models/openai_compatible.rs`.

## Three decisions worth knowing about

**The state directory is separate from the old one.** It defaults to `app-data/state/agent-host`, not `app-data/state`. The TypeScript host still runs and still owns `lexia.sqlite` until WF-IMP-025 removes it. Pointing both at one directory would put two implementations in a position to write the same file — the failure this whole migration is arranged to avoid.

**An environment credential outranks a stored one, and that is visible.** `<PROVIDER>_API_KEY` beats `<state>/credentials/<provider>.key` for the life of the process, so a shell configuration is never silently overridden from a browser. The settings surface can therefore say *which* credential is in force, and reports the shadow rather than showing a masked file that is not in use. Removing a stored key does not remove an environment one, and the API says so.

**A missing credential is not a load failure.** `deny_unknown_fields` rejects a mistyped manifest key, because a setting that silently never applies is worse than a startup error. A provider that is declared but has no key is different: the use-case row is reported `inert` and falls back to `always`, so a missing transcription key degrades one row instead of breaking the conversation.

## Plugins

```
./target/debug/lexosa plugins list
./target/debug/lexosa plugins granted <agent> <plugin>
```

Build the bundled guest — the tests do this for you if it is missing:

```sh
cd guests/echo && cargo +nightly-2026-10-04 build --release --target wasm32-unknown-unknown
```

**The ABI is a boundary, not a Rust interface.** Strings cross as UTF-8 in linear
memory; both buffers are allocated by the host through `lexosa_alloc`. An earlier
shape had the guest return a pointer it had allocated itself, which put the
response at the mercy of the guest's allocator — a misallocating guest could
make the host read a wild address. Now a guest bug can produce a wrong answer
but not a wild read.

**WASI is not linked on purpose.** Filesystem access is a granted capability
whose paths resolve against the sandbox root, not a preopen. With preopens a
plugin granted nothing still receives a `path_open` import, and the safety
argument becomes "the preopen is narrow"; here it is "there is nothing to
call". The consequence is what makes the consent ledger enforceable: **an
import the host did not link fails to instantiate.**

Plugins are core `wasm32` modules, not Components. The target is the Component
Model; a component toolchain does not build on this host. See ADR 0005.

## The run coordinator

**Fenced writes are atomic.** `complete_action` in the TypeScript reads the action, checks the lease, and only then updates — with the read outside the transaction, so a second worker can pass the same check in between. Here the guard is in the statement (`WHERE status = 'running' AND lease_id = ?`) and the row count is the answer. Event sequences are assigned inside the writing transaction for the same reason: a log whose sequence is assigned outside the inserting transaction can replay into a different history than the one that happened. The tests cannot observe either difference — it takes a race — so the ported invariants assert the observable contract rather than claiming to test atomicity.

**Startup recovery now actually runs.** `recoverExpiredLeases` was exported and tested in the TypeScript but never called by anything. ADR 0002 called for it and it was never wired. The host calls it before the listener binds, and warns when anything needed reconciling.

**The two stores do not overlap.** The run store is `<state>/runs.sqlite`, beside the credential directory and deliberately *not* the old `app-data/state/lexia.sqlite`, which the TypeScript host still owns. Migrating is part of the cutover; until then there is one writer each.

## What is deliberately absent

No turn endpoint, no attachments, and no claim/report/complete routes. Those belong to the agent loop and the capability runtime ([WF-IMP-023](../docs/planning/tickets/WF-IMP-023-agent-loop-sub-agents-and-builtins.md)), and a route that answers "not implemented" is worse than one that does not exist — a client will build against it.


Nightly Rust is required and the reason is in [`rust-toolchain.toml`](./rust-toolchain.toml).