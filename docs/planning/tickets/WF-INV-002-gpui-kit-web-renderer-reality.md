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

The pinned versions in this repository are behind published upstream: `gpui-pre 0.3.7` / `gpui-kit 0.7.0` are vendored locally, while the crates index now lists `gpui-pre`/`gpui-pre-web` up to `0.3.9` and `gpui-kit` up to `0.8.0` (checked against `https://index.crates.io/` on 2026-10-22). All findings below were verified against the **locally vendored** `0.7.0` / `0.3.7`, so they describe the version this repository actually builds. The upstream gallery runs a newer release, which is why its console shows `crates/component/src/theme/mono_font.rs` paths rather than this crate set's `gpui-component`.

## Answer

**Yes — it builds, links and renders. Upstream calls it showcase-only, and that label is load-bearing.**

Verified independently on 2026-10-22 against the versions this repository vendors: the wasm32 build compiles, links, and paints real kit components into a browser canvas; the native build compiles, links, runs, and paints the same components into a desktop window. The backend is `gpui-pre-web`'s `WebPlatform`, WebGPU preferred with an automatic WebGL2 fallback.

## Evidence

### A minimal application compiles and links

Re-verified on 2026-10-22 from a clean scratch crate outside this repository (`/tmp/gpui-kit-probe`, `gpui-kit = "=0.7.0"`, default features), against the toolchain this repository pins. The probe renders `Button` (primary) and `Input`/`InputState`, and exercises both bridge directions: a `#[wasm_bindgen]` export for host → Rust, and a `document.dispatch_event(CustomEvent)` for Rust → host.

**Native build — succeeds, with one host prerequisite.**

```
$ RUSTFLAGS="-L native=/tmp/gpui-kit-probe/native-libs" \
  RUSTUP_TOOLCHAIN=nightly-2026-10-04 cargo build
   Compiling gpui-kit-probe v0.0.0 (/tmp/gpui-kit-probe)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 20s
```

The whole `gpui-kit` / `gpui-component` / `gpui-pre-linux` tree compiles unmodified for the host. Linking fails first with:

```
rust-lld: error: unable to find library -lxkbcommon-x11
```

That is a **host packaging gap, not a `gpui-kit` gap**: this Fedora host has `libxkbcommon-x11.so.0` but not the `libxkbcommon-x11-devel` symlink, and `gpui-pre-linux` links `-lxkbcommon-x11` for its X11 backend. `dnf install libxkbcommon-x11-devel` resolves it; a local symlink in a `-L native=` path also works. **WF-IMP-008 will hit this on the same host and should install the package rather than work around it.**

The resulting binary runs and paints. Against the local Xwayland (`:0`, `XAUTHORITY=/run/user/1000/.mutter-Xwaylandauth.L1NIX3`):

```
probe pid=776004
RESULT: probe still running after 18s (event loop alive, no panic)
--- 0x2600001 ---
  Width: 1920   Height: 1011   Map State: IsViewable
```

A screenshot of that window shows the rendered kit components — a full-width primary `Button` reading “Let’s Go!” above an `Input` showing the placeholder “type here” (573 unique colours). The native path is not a compile-only claim.

**wasm32 build — succeeds.**

```
$ RUSTFLAGS="-C link-arg=-zstack-size=8388608" \
  RUSTUP_TOOLCHAIN=nightly-2026-10-04 cargo build --release --target wasm32-unknown-unknown
    Finished `release` profile [optimized] target(s) in 3m 06s
```

`gpui-component`, `gpui-pre-web`, `gpui-pre-wgpu` and `gpui-kit` all compile for `wasm32-unknown-unknown` without modification. `wasm-bindgen 0.2.129` then produces browser-loadable bindings.

**It renders in a real browser.** Served over HTTP with `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`, loaded in Chromium 155, the page creates the canvas, the Rust side paints into it, and both bridge legs execute:

```
<div id="status">rust_notify detail=hello-from-rust host_ping=pong</div>
```

The status line is the browser's own text: Rust dispatched the `CustomEvent` (Rust → host) and the page then called the exported `host_ping()` (host → Rust) and got `pong`. The screenshot shows the same “Let’s Go!” button and “type here” input as the native window.

**Bundle size.** Measured 2026-10-22 on this probe (gpui-kit 0.7.0, one 410 KB embedded font, no `wasm-opt`):

| Artifact | Raw | gzip -9 | brotli q11 |
| --- | --- | --- | --- |
| `rustc` release output | 16,149,993 B | 4,927,299 B | 3,345,695 B |
| post-`wasm-bindgen` `.wasm` | 13,978,618 B | 4,689,053 B | 3,184,969 B |
| post-`wasm-bindgen` `.js` | 172,830 B | 25,140 B | — |

The 2026-10-05 figures recorded above (24.4 MB / 5.95 MB / 3.90 MB) were taken against `gpui-kit 0.7.1` / `gpui-pre 0.3.8` on `nightly-2026-03-04`; these are `0.7.0` / `0.3.7` on `nightly-2026-10-04`. Different versions, so the two are not comparable — treat the table above as the current measurement and re-measure on the version actually shipped.

For scale, upstream's own live gallery bundle (`https://gpui-kit.com/gallery/`, the full component gallery with embedded font subsets) measures **28,794,977 B raw / 8,760,516 B gzip / 5,548,655 B brotli**, plus 111,205 B of JavaScript. A near-empty probe and the real showcase are within a factor of two of each other; the payload is dominated by the component library, not by the application.

**Bundle size caveat.** No `wasm-opt` was applied and no icon assets are embedded. The real transfer size will differ.

### Nightly is a hard requirement — but the previous reasoning was wrong

`wasm_thread 0.3.3` opens with `#![cfg_attr(target_arch = "wasm32", feature(stdarch_wasm_atomic_wait))]`, which is stable-forbidden. This is verified directly: the same crate that builds on nightly fails on stable with

```
error[E0554]: `#![feature]` may not be used on the stable release channel
1 | #![cfg_attr(target_arch = "wasm32", feature(stdarch_wasm_atomic_wait))]
error: could not compile `wasm_thread` (lib) due to 1 previous error
```

**But the earlier claim that `single_threaded_web()` is unreachable through the facade is wrong.** It is reachable and it compiles. A scratch crate that calls `gpui_kit::platform::single_threaded_web()`, `gpui_kit::web::WebBackendPreference::Auto` and `gpui_kit::platform::application_with_web_backend(...)` type-checks cleanly for `wasm32-unknown-unknown` on nightly. `gpui_kit::platform` is `gpui_platform` re-exported unconditionally (`gpui-kit-0.7.0/src/lib.rs:105`), and `single_threaded_web()` is a plain `pub fn` there (`gpui-pre-platform-0.3.7/src/gpui_platform.rs:42`).

What is actually true is narrower and more specific:

- `gpui-kit` depends on `gpui_web` without `default-features = false`, so `gpui_web`'s `multithreaded` default is always on, and Cargo feature unification means a consumer cannot turn the *feature* off. `wasm_thread` therefore compiles unconditionally, and **that** is what forces nightly — not the runtime threading mode.
- Upstream's own gallery sidesteps the runtime cost by constructing `WebPlatform::new_with_backend_and_font_fallback(false, ...)` directly (`crates/story-web/src/lib.rs`), i.e. it opts out of multithreading at runtime while still paying the nightly-compile tax.

So nightly is required either way; what is optional is whether the dispatcher actually uses threads at runtime.

### Threads do not come back with headers alone

Cross-origin isolation headers are **necessary but not sufficient**. `WebDispatcher::new` requires three things at once (`gpui-pre-web-0.3.7/src/dispatcher.rs:168`):

```rust
let supports_threads = cfg!(feature = "multithreaded")
    && allow_threads
    && shared_memory_supported()
    && wait_async_supported();
```

`shared_memory_supported()` checks that the wasm memory buffer really is a `SharedArrayBuffer` — which needs the module built with shared memory, not just the page served with COOP/COEP. With COOP/COEP headers served and an ordinary (non-shared-memory) wasm build, the browser console shows the dispatcher degrading:

```
[WARN] gpui_web::dispatcher: Required WebAssembly threading APIs are unavailable; falling back to single-threaded dispatcher
```

The rendering still works — that is the point of the fallback — but it is single-threaded.

Getting real threads needs `-C target-feature=+atomics,+bulk-memory,+mutable-globals` plus `-C link-arg=--shared-memory -C link-arg=--import-memory` and a `-Z build-std` std rebuild. That build succeeds here, but `wasm-bindgen` then refuses the module:

```
error: failed to prepare module for threading
Caused by:
    failed to find `__heap_base` for injecting thread id     # wasm-bindgen-cli 0.2.121
    failed to find `__tls_size`                              # wasm-bindgen-cli 0.2.129
```

`wasm-bindgen`'s threads transform wants `__wasm_init_tls`, `__tls_size`, `__tls_align`, `__heap_base` and `__stack_pointer` as *locals* it can rewrite, and Rust's `wasm32-unknown-unknown` link does not emit them in that shape. **This is an open blocker, not a solved problem, and it is the concrete cost of the `multithreaded` default.** The workaround upstream uses is to construct the platform with multithreading disabled.

**Conclusion for the implementation:** build the web target single-threaded, exactly as upstream's gallery does, and stop paying for the threads we cannot use. Nightly remains required because `wasm_thread` compiles regardless — a separate, unavoidable cost. The open question of whether threads can be enabled at all is tracked in [WF-IMP-028](WF-IMP-028-web-target-threading-mode.md).

### Upstream's own position

`website/docs/webassembly.md` carries `maturity: [showcase-only]`:

> "this repository has not validated WebAssembly as a mature distribution path for full applications."

and the maturity table on the docs site:

> "shipping an application in a browser is not a supported path yet."

`.github/workflows/ci.yml` does contain a `wasm32` job — but it is a `cargo check`, not a build-and-run:

```yaml
  gpui-fast-web:
    name: GPUI Fast (WebAssembly)
    runs-on: ubuntu-latest
    steps:
      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: nightly
          target: wasm32-unknown-unknown
      - name: Check GPUI Fast WebAssembly
        run: cargo check -p gpui-kit --features gpui-fast --target wasm32-unknown-unknown --locked
```

So upstream type-checks the browser target on every push, and the website release workflow builds the WASM examples. **No upstream CI job runs a browser.** There is a live gallery at `https://gpui-kit.com/gallery/`, so the rendering path is real; it is the *application* path that is unproven.

Loading the live gallery in Chromium reproduces the same backend selection this repository's build would see:

```
"WebGPU initialization failed; falling back to WebGL2: browser WebGPU probe did not return a usable adapter"
"Browser graphics initialized: requested=WebGl, selected=Gl, adapter=\"ANGLE (NVIDIA, Vulkan 1.4.351 ...)\""
"Browser graphics initialized successfully with Gl"
```

Note the gallery's console contains **no** threading warning: upstream builds it single-threaded, exactly as recommended above. This repository's own build, using `gpui_kit::application()`, does log the single-threaded fallback.

### What renders, and what is missing

Present and working: text shaping, `Scrollable` regions, overlays, text input with caret and selection, dialogs and sheets, data tables, `Message`/`Bubble` and `MessageScroller`.

Gaps, each verified in source:

| Concern | Finding | Evidence |
| --- | --- | --- |
| Windows | One document canvas, one top-level window. A second, or reopening a closed one, is unsupported. | `gpui-pre-web/src/platform.rs:116`; `docs.rs/gpui-pre-web` |
| File picker | `prompt_for_paths` and `prompt_for_new_path` return an `Err` on web — they do not panic. | `platform.rs:470-495` |
| Fonts | The font database starts empty; fonts must be added before the first window, including the `.SystemUIFont` alias or text measurement can fail. The alias resolves to the literal family name **`IBM Plex Sans`** — a font registered under any other name will not satisfy it. | `platform.rs:30,179`; `gpui-pre-wgpu/src/cosmic_text_system.rs:147` |
| IME | Uses a hidden HTML input for keyboard and composition. The gallery disables mobile text entry on touch-only devices. | `webassembly.md` |
| Accessibility | Painted into a canvas; no native web semantics. Upstream advises providing an accessible HTML alternative. | `webassembly.md` |
| Artifacts | `gpui-wry` (published, 0.7.1) is desktop-only — no wasm dependency, README states macOS and Windows. | `crates/webview/Cargo.toml`, `README.md` |
| Window chrome | `minimize`, `zoom` and `quit` are unsupported no-ops. | `window.rs:892,896`; `platform.rs:350` |

### Backend

WebGPU preferred by default, automatic WebGL2 fallback, selected through `WebBackendPreference::Auto`. This is now confirmed at runtime rather than read from source — the browser console during the probe's load:

```
"No available adapters."
"[WARN] gpui_web::platform: WebGPU initialization failed; falling back to WebGL2: browser WebGPU probe did not return a usable adapter"
"[WARN] gpui_wgpu::wgpu_context: Dual-source blending not available on this GPU. Subpixel text antialiasing will be disabled."
"[INFO] gpui_wgpu::wgpu_context: Browser graphics initialized: requested=WebGl, selected=Gl, adapter=\"ANGLE (NVIDIA, Vulkan 1.4.351 (NVIDIA NVIDIA GeForce RTX 3060 Laptop GPU (0x00002520)), NVIDIA)\""
"[INFO] gpui_web::platform: Browser graphics initialized successfully with Gl"
```

WebGPU was unavailable in this headless Chromium, so the **WebGL2** path ran and rendered correctly. The upstream gallery log shows the identical sequence, so both backends are exercised in the wild.

`CanvasFontFallback::EmojiAndCjk` lets the browser draw emoji and CJK graphemes from local fonts, per grapheme, which is not a full CJK typographic solution. Its default is `Emoji` only (`canvas_fallback.rs:23`); the gallery opts into `EmojiAndCjk` explicitly.

### The bridge question, answered

No bespoke bridge library is required, and none exists to adopt.

- **Host → Rust**: a `#[wasm_bindgen]` export. Verified in the probe — the browser called `host_ping()` and received `pong`.
- **Rust → host**: `web_sys::CustomEvent` dispatched on `document`, so SvelteKit can listen without linking against wasm internals. Verified in the probe — the Rust side dispatched the event and the page's listener read `event.detail`.
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

1. Upstream does not support shipping a browser application, and no upstream CI job runs a browser. Every web acceptance criterion in this chain is therefore ours to verify, not inherited.
2. Nightly Rust is a permanent toolchain requirement, not a temporary one — forced by `wasm_thread`'s feature gate, independently of whether threads are used at runtime.
3. **Build the web target single-threaded**, as upstream's gallery does. The `multithreaded` default cannot be disabled and its threads cannot be enabled anyway (see above), so paying for them buys nothing.
4. First-load cost is multi-megabyte. Accepted for now, to be measured properly after `wasm-opt`.
5. Accessibility, file upload and CJK typography need explicit work rather than inherited behaviour.
6. The host needs `libxkbcommon-x11-devel` before the native target links.

## Progress

**Re-verified 2026-10-22.** The original evidence was produced on 2026-10-05; the upstream findings were read from `longbridge/gpui-kit` at `main` on that date.

The question is answered and the answer is unchanged: **`gpui-kit` 0.7.0 builds for `wasm32-unknown-unknown`, links, and renders in a browser, through `gpui-pre-web`'s `WebPlatform` with WebGPU preferred and an automatic WebGL2 fallback.** This re-verification was independent — a fresh scratch crate outside the repository, the pinned `nightly-2026-10-04`, and the versions this repository vendors — and it upgraded three claims from "compiles" to "compiles and demonstrably paints", and corrected four that were wrong.

**What the re-verification established**

| Claim | Before | Now |
| --- | --- | --- |
| wasm32 build | compiles | compiles, links, and renders in Chromium |
| Native build | not covered | compiles, links, runs, and paints — but needs `libxkbcommon-x11-devel` |
| Backend | read from source | confirmed at runtime: WebGPU unavailable → WebGL2 fallback, renders correctly |
| Bridge | "verified compiling" | verified **executing** in a browser, both directions |
| `single_threaded_web()` | "unreachable through the facade" | **reachable**; it compiles and is what upstream's gallery uses |
| Threading | "headers would bring threads back" | headers are necessary but **not sufficient**; `wasm-bindgen` rejects the shared-memory module |
| File picker | "panics on web" | returns an `Err`; does not panic |
| CI | "no `wasm32` job" | there is a `wasm32` **check** job; no job runs a browser |

The three implementation tickets that were blocked on this report are re-scoped by [WF-DEC-004](WF-DEC-004-one-rust-codebase-native-and-web.md), which replaces their assumed shape with one that covers all routes rather than chat and artifacts alone.

**New findings carried into WF-IMP-008**

- Install `libxkbcommon-x11-devel` before the first native build; without it the link fails on `-lxkbcommon-x11`.
- Build the web target single-threaded, as upstream does. The `multithreaded` default cannot be turned off, its threads cannot be enabled on this toolchain, and the fallback warning is noise in the console that the gallery does not produce.
- The `.SystemUIFont` alias resolves to the literal family name `IBM Plex Sans`; a font registered under another name will not satisfy it.
- Serve the bundle with `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp` regardless of the single-threaded decision — they are free, and they are the precondition for any future move to threads.
- The threading question itself is open and is tracked in [WF-IMP-028](WF-IMP-028-web-target-threading-mode.md).

**Artifacts** (scratch, outside the repository): `/var/home/swhite/.cache/gpui-probe/win-0x2600001.png` (native window screenshot), `web/shot.png` (browser screenshot), `web/pkg/` (wasm-bindgen output), `gallery-console.txt` (upstream gallery's browser console), `native-run.log`. The probe crate itself is `/tmp/gpui-kit-probe`.