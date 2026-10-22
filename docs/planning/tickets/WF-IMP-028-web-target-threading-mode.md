---
kind: implementation
status: planned
---

# WF-IMP-028: Decide the web target's threading mode, and get threads working if they are needed

## Goal

Settle whether the browser build runs single-threaded or multi-threaded, and either enable threads or record, with evidence, why they stay off.

This is the one part of [WF-INV-002](WF-INV-002-gpui-kit-web-renderer-reality.md)'s answer that is still open. Everything else in that report — that `gpui-kit` builds for `wasm32`, links, and paints real components into a browser canvas — was verified by building and running it.

## Depends on

- [WF-INV-002](WF-INV-002-gpui-kit-web-renderer-reality.md) — the evidence this ticket starts from
- [WF-IMP-008](WF-IMP-008-gpui-kit-web-build-and-host.md) — the build that must adopt the decision

## The problem

`gpui-pre-web` 0.3.7 decides at runtime whether to use threads, and it needs three things at once:

```rust
// gpui-pre-web-0.3.7/src/dispatcher.rs:168
let supports_threads = cfg!(feature = "multithreaded")
    && allow_threads
    && shared_memory_supported()
    && wait_async_supported();
```

The first is not negotiable from outside: `gpui-kit` depends on `gpui_web` without `default-features = false`, so `gpui_web`'s `multithreaded` default is always on and Cargo feature unification means a consumer cannot turn the feature off. The second is a runtime choice, and it is the only one this project controls: `WebPlatform::new(false)` disables multithreading, and `gpui_platform::single_threaded_web()` is that constructor. **That function is reachable through the facade** — `gpui_kit::platform::single_threaded_web()` compiles for `wasm32-unknown-unknown`, contrary to an earlier reading of this investigation.

The third is the hard one. `shared_memory_supported()` checks that the wasm memory buffer really is a `SharedArrayBuffer`, which needs the module built with shared memory — cross-origin isolation headers alone are not enough. The build flags get the module that far:

```
RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals
           -C link-arg=--shared-memory
           -C link-arg=--import-memory'
cargo build --release --target wasm32-unknown-unknown -Z build-std=std,panic_abort
```

That build succeeds. `wasm-bindgen` then refuses the module:

```
error: failed to prepare module for threading

Caused by:
    failed to find `__heap_base` for injecting thread id     # wasm-bindgen-cli 0.2.121
    failed to find `__tls_size`                              # wasm-bindgen-cli 0.2.129
```

`wasm-bindgen`'s threads transform rewrites `__wasm_init_tls`, `__tls_size`, `__tls_align`, `__heap_base` and `__stack_pointer` as locals (`wasm-bindgen-cli-support-0.2.129/src/transforms/threads/mod.rs:88`), and Rust's `wasm32-unknown-unknown` link does not emit them in that shape.

Meanwhile the nightly toolchain is required either way, because `wasm_thread` 0.3.3 opens with `#![cfg_attr(target_arch = "wasm32", feature(stdarch_wasm_atomic_wait))]` and `wasm_thread` compiles whether or not threads are used at runtime. Building on stable fails with `error[E0554]: `#![feature]` may not be used on the stable release channel`.

## Why this is a ticket and not a note

The default build is not neutral. `gpui_kit::application()` selects the multithreaded dispatcher, and the browser console says so:

```
[WARN] gpui_web::dispatcher: Required WebAssembly threading APIs are unavailable; falling back to single-threaded dispatcher
```

Upstream's own gallery produces no such warning, because it constructs `WebPlatform::new_with_backend_and_font_fallback(false, ...)` directly. So the choice is real, and today the honest answer is "threads are requested and refused".

## Scope

- Decide the mode for this project's web target and record the decision with the console output that shows it took effect. If single-threaded is chosen, the fallback warning must be gone, not merely tolerated.
- If threads are wanted, find a way to produce a module `wasm-bindgen` accepts: the missing globals exported under the names the transform rewrites, a different `wasm-bindgen-cli` version, or a `-Z build-std` configuration that emits them.
- Record the cost of each outcome. Threads bought means a second toolchain (`-Z build-std`), a different `wasm-bindgen` invocation, and a shared-memory build that cannot be served without cross-origin isolation headers.
- Do not spend this on the feature if the application's workload does not need it. GPUI's browser path is single-threaded in upstream's own shipped example.

## Out of scope

- Any product screen. This is a build-configuration question.
- Removing the nightly requirement. `wasm_thread` compiles unconditionally through the facade's dependency on `gpui_web`; that would mean upstream changes.

## Acceptance criteria

- The web target's threading mode is chosen and recorded in this ticket.
- The browser console for the served bundle shows the chosen mode, with no unexplained warning.
- If threads are enabled, the module passes `wasm-bindgen` and the dispatcher logs no single-threaded fallback.
- If threads stay disabled, the reason is recorded against the evidence above, and the nightly requirement is justified on its own terms rather than by the threading path.

## Progress

Not started. The evidence above was gathered on 2026-10-22 while re-verifying [WF-INV-002](WF-INV-002-gpui-kit-web-renderer-reality.md): a shared-memory build of the probe succeeded and `wasm-bindgen` 0.2.121 and 0.2.129 each rejected it with a different missing global, and the ordinary build logged the single-threaded fallback in the browser.
