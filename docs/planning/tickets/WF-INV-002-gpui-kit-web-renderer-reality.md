---
kind: investigation
status: complete
---

# WF-INV-002: Establish what the gpui-kit browser build actually is

## Goal

Answer, with evidence rather than optimism, whether `gpui-kit` can be built for `wasm32` and rendered in a browser today, and if so through which backend. Every implementation ticket in this direction depends on that answer.

## Depends on

- [WF-DEC-003](WF-DEC-003-gpui-kit-forefront-web-alpha.md)

## The premise this ticket opened with was wrong

The original draft asserted that *"no web render backend is present in the published crates"* and that the platform surface is *"macOS, Linux and Windows"*. Both are false. `gpui-pre-web` is published, and `gpui-pre-platform` already selects it by target:

```toml
# gpui-pre-platform-0.3.7/Cargo.toml
[target.'cfg(target_family = "wasm")'.dependencies.gpui_web]
version = "=0.3.7"
package = "gpui-pre-web"
```

`gpui-kit` declares the same target-gated dependency and re-exports it as `gpui_kit::web`. Web is chosen by `target_family = "wasm"`, **never** by a Cargo feature — there is no `web` feature flag to turn on or off.

The pinned versions in this repository were also one snapshot behind: `gpui-pre 0.3.7` / `gpui-kit 0.7.0` locally, `gpui-pre 0.3.8` / `gpui-kit 0.7.1` published 2026-10-05.

## Answer

**Yes — it builds, links and renders. Upstream calls it showcase-only, and that label is load-bearing.**

## Evidence

### A minimal application compiles and links

Verified from a clean scratch crate on this host, not from upstream CI:

```
nightly-2026-03-04 rustc --release --target wasm32-unknown-unknown -- -C link-arg=-zstack-size=8388608
Finished `release` profile in 1m 17s
gpui_wasm_probe.wasm   24,412,955 B raw
                        5,951,185 B gzip -9
                        3,898,403 B brotli -11
```

The probe exercised `Button`, `Input`/`InputState`, `DataTable`/`TableDelegate`, route switching, and both bridge directions: a `#[wasm_bindgen]` export for host → Rust, and a `document.dispatch_event(CustomEvent)` for Rust → host. `gpui-component` 0.7.1 and `gpui-kit-assets` 0.7.1 compile for the target without modification.

**Bundle size caveat.** These numbers are raw `rustc` output. `wasm-bindgen` and `wasm-opt` were not applied, and no fonts or assets are embedded. The real transfer size is unmeasured and will differ.

### Nightly is a hard requirement

`wasm_thread 0.3.3` opens with `#![feature(stdarch_wasm_atomic_wait)]`, which is stable-forbidden. `gpui-kit` depends on `gpui_web` **without** `default-features = false`, so its `multithreaded` default is always on, and Cargo feature unification means a consumer cannot turn it off. Upstream's own single-threaded entry point, `gpui_platform::single_threaded_web()`, is therefore unreachable through the facade.

Without `SharedArrayBuffer` the dispatcher degrades rather than fails — it logs `SharedArrayBuffer not available; main thread mailbox waker loop disabled` (`gpui-pre-web/src/dispatcher.rs:104`). Cross-origin isolation headers would be needed to get threads back.

### Upstream's own position

`website/docs/webassembly.md` carries `maturity: [showcase-only]`:

> "this repository has not validated WebAssembly as a mature distribution path for full applications."

and the maturity table on the docs site:

> "shipping an application in a browser is not a supported path yet."

`.github/workflows/ci.yml` contains no `wasm32` job — only the website release workflow builds a browser target. There is a live gallery at `https://gpui-kit.com/gallery/`, so the rendering path is real; it is the *application* path that is unproven.

### What renders, and what is missing

Present and working: text shaping, `Scrollable` regions, overlays, text input with caret and selection, dialogs and sheets, data tables, `Message`/`Bubble` and `MessageScroller`.

Gaps, each verified in source:

| Concern | Finding | Evidence |
| --- | --- | --- |
| Windows | One document canvas, one top-level window. A second, or reopening a closed one, is unsupported. | `gpui-pre-web/src/platform.rs:116`; `docs.rs/gpui-pre-web` |
| File picker | `prompt_for_paths` and `prompt_for_new_path` **panic** on web. | `platform.rs:476,490` |
| Fonts | The font database starts empty; fonts must be added before the first window, including the `.SystemUIFont` alias or text measurement can fail. | `platform.rs:30` |
| IME | Uses a hidden HTML input for keyboard and composition. The gallery disables mobile text entry on touch-only devices. | `webassembly.md` |
| Accessibility | Painted into a canvas; no native web semantics. Upstream advises providing an accessible HTML alternative. | `webassembly.md` |
| Artifacts | `gpui-wry` (published, 0.7.1) is desktop-only — no wasm dependency, README states macOS and Windows. | `crates/webview/Cargo.toml`, `README.md` |
| Window chrome | `minimize`, `zoom` and `quit` are unsupported no-ops. | `window.rs:892,896`; `platform.rs:350` |

### Backend

WebGPU preferred by default, automatic WebGL2 fallback, selected through `WebBackendPreference::Auto`. `CanvasFontFallback::EmojiAndCjk` lets the browser draw emoji and CJK graphemes from local fonts, per grapheme, which is not a full CJK typographic solution.

### The bridge question, answered

No bespoke bridge library is required, and none exists to adopt.

- **Host → Rust**: a `#[wasm_bindgen]` export. Verified compiling in the probe.
- **Rust → host**: `web_sys::CustomEvent` dispatched on `document`, so SvelteKit can listen without linking against wasm internals. Verified compiling in the probe.
- **Auth**: `FetchHttpClient` defaults to `RequestCredentials::SameOrigin`, so the existing `lexia_session` cookie works unchanged against the same-origin host. (`gpui-pre-web/src/http_client.rs:34`)
- **Run stream**: the existing SSE endpoint needs no host change. Streaming `Request` and `ReadableStream` are enabled web-sys features.

What genuinely needs JavaScript is narrow: the file picker, an accessibility mirror, and (optionally) external-link handling.

### Router

There is no URL router in `gpui-kit`, and no third-party one is expected to exist for it.

`gpui_base::{NavStack, NavStackState}` (`crates/base/src/nav_stack.rs`) is the one navigation primitive: `push`, `pop`, `pop_to_root`, `forward`, `replace`, `clear`, `depth`, `current`, backed by a `History<NavEntry>` trail. It animates (`NavMotion::Animated`, `NavStack::transition(Transition)`, `NavPage::progress()`) and stores `AnyView`s only — no path patterns, no params, no route table, no URL read or write. `gpui-component` does not re-export it; it is reachable only as `gpui_kit::base::NavStack`.

`breadcrumb`, `tab`, `sidebar` and `dock` are presentational containers driven by an application-owned flag (`SidebarMenuItem::active(bool)`, `TabBar::selected_index(usize)`).

The only URL-adjacent code in the whole upstream repository is a read-only `new URLSearchParams(window.location.search)` in the two WASM demo loaders.

## Recommendation

Proceed with constraints. The renderer is real, the component set the chat surface needs compiles, the bridge is ~30 lines of standard wasm-bindgen, and the session and run-stream contracts transfer unchanged.

The constraints, carried into [WF-DEC-004](WF-DEC-004-one-rust-codebase-native-and-web.md):

1. Upstream does not support shipping a browser application, and does not test one in CI. Every web acceptance criterion in this chain is therefore ours to verify, not inherited.
2. Nightly Rust is a permanent toolchain requirement, not a temporary one.
3. First-load cost is multi-megabyte. Accepted for now, to be measured properly after `wasm-opt`.
4. Accessibility, file upload and CJK typography need explicit work rather than inherited behaviour.

## Progress

Complete. The build evidence was produced locally on 2026-10-05; the upstream findings were read from `longbridge/gpui-kit` at `main` on the same date.

The three implementation tickets that were blocked on this report are re-scoped by [WF-DEC-004](WF-DEC-004-one-rust-codebase-native-and-web.md), which replaces their assumed shape with one that covers all routes rather than chat and artifacts alone.