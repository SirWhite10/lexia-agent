# Plugins

Plugins are WebAssembly Components loaded through Wasmtime. They provide capabilities the host does not have built in. See [ADR 0005](../../../docs/adr/0005-rust-agent-host-and-wasm-plugins.md) for the architecture and [WF-IMP-021](../../../docs/planning/tickets/WF-IMP-021-wasmtime-plugin-host.md) for the implementation.

```text
plugins/
├── registry/    name → active generation, and the metadata beside it
├── versions/    immutable, content-addressed .wasm artifacts
└── active/      the pointer the host actually loads
```

## Generations

A plugin is never replaced in place. Every build becomes a new generation, content-addressed by hash, and exactly one is active.

```text
github
├── v1  a81c…    retired, still on disk
├── v2  91bd…    retired, still on disk
└── v3  f03a…    ← active
```

Promotion is transactional: generate → compile → validate against the WIT interface → load as a candidate → sandbox test → health test → promote. Any failure discards the candidate and the active generation is untouched. Rollback is instant because earlier generations are never mutated.

Keeping the previous generations is the whole mechanism. A plugin that works today still works at 3am without the host having been rebuilt.

## What lives where

`registry/` and `active/` are **watched**: a pointer change promotes or rolls back immediately. `versions/` is **not watched** — it is an immutable store this host writes to, and watching it would mean reacting to our own writes.

## Writing one

This is the one place agent-authored code lands, so it is the security surface of the product. A plugin declares what it needs and the host decides what it gets; an undeclared capability is denied, not defaulted.

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

Ask for the narrowest set that works. A plugin that does not need the filesystem should not request it, because a manifest is the only thing standing between agent-authored code and the user's files.