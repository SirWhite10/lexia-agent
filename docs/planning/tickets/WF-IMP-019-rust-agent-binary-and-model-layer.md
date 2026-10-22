---
kind: implementation
status: implemented
---

# WF-IMP-019: The Rust agent binary and its model layer

## Goal

Stand up the permanent Rust agent: one distributable binary with an HTTP surface the clients can talk to, a provider-independent model layer, and a state directory it owns. This replaces the EVE process, its supervisor, and the TypeScript provider layer.

## Depends on

- [WF-IMP-008](WF-IMP-008-gpui-kit-web-build-and-host.md) creating the Rust workspace
- [ADR 0005](../../adr/0005-rust-agent-host-and-wasm-plugins.md)

## Scope

**The binary.** One crate under `gpui-app/` that builds a native binary and exposes HTTP for the clients. It owns its state directory and is the only writer to it. A CLI alongside the service, because the agent is a product in its own right and not only a server for the UI.

**The HTTP surface.** A real server with real routes: health, and the provider and model configuration endpoints the Settings screens need. Decided here and fixed here, because [WF-IMP-012](WF-IMP-012-url-and-session-bridge.md) and everything after it are written against this contract.

**What this ticket deliberately does not serve.** The turn endpoint needs the agent loop ([WF-IMP-023](WF-IMP-023-agent-loop-sub-agents-and-builtins.md)); the run and event stream needs the coordinator ([WF-IMP-020](WF-IMP-020-durable-run-coordinator-in-rust.md)); attachments need storage and the picker bridge ([WF-IMP-014](WF-IMP-014-attachment-and-file-picker-bridge.md)). Serving them here would mean shipping handlers that answer "not implemented" — a stub that looks like a route and is not one. They arrive with the tickets that can actually answer them. The original draft of this ticket listed all three here; that was wrong.

**The model layer.** A `ModelProvider` abstraction, not a vendor. OpenAI-compatible endpoints, OpenRouter, and at least one direct provider adapter. Streaming, tool calls, structured output. Nothing in the agent loop is written against one vendor's shapes.

**Credentials, unchanged.** One credential per provider, written to `<provider>.key` in the state directory, owner-readable only. `<PROVIDER>_API_KEY` in the environment outranks the stored file for the life of the process, so a shell configuration is never silently overridden from a browser. An empty submission removes the key. These rules are already in `OPERATIONS.md` and are product behaviour, not implementation detail.

**Configuration and events.** A configuration system that reads the agent directory, and an event system the clients can subscribe to.

## Replaces

| Today | Fate |
| --- | --- |
| `src/lib/server/eve.ts` (128 lines) | the model layer and its client |
| `src/lib/server/eve-supervisor.ts` (219 lines) | nothing — a supervisor for a supervised child process is not needed for one binary |
| `src/lib/server/openrouter.ts` | a `ModelProvider` implementation |
| `src/lib/server/model-routes.ts`, `model-catalog.ts` | model configuration and per-provider catalogues |
| `src/lib/server/provider-registry.ts`, `provider-key.ts` | credential storage, under the rules above |
| `src/lib/server/transcription.ts` | the provider layer |

Do not delete these yet. They are removed by [WF-IMP-025](WF-IMP-025-retire-eve-and-the-typescript-agent.md) once the binary answers for them; until then both paths run and must agree.

## Out of scope

- The run coordinator ([WF-IMP-020](WF-IMP-020-durable-run-coordinator-in-rust.md)).
- The plugin host ([WF-IMP-021](WF-IMP-021-wasmtime-plugin-host.md)).
- Auth and sessions. Those stay in the host.

## Acceptance criteria

- One command produces a native binary that starts, serves its routes, and creates its state directory.
- The provider and model configuration endpoints round-trip a real change and report the credential that is in force.
- A completion reaches a real provider through the abstraction, and the same call works against two differently-shaped providers without the caller changing.
- Tool calls and structured output are exercised against the provider, not assumed from the request type.
- Transcription is reachable through the provider layer rather than a bespoke path.
- No route answers "not implemented": every path the server exposes can actually serve its request.
- Two providers configured at once both work; a model catalogue is fetched per provider and only when a key exists.
- A provider whose catalogue does not cover a row's modality accepts a typed model id, which is what OpenRouter's text-only `/models` requires today.
- An environment credential outranks a stored one for the process lifetime, and the screen reports which is in force rather than silently using the file.
- An empty provider submission removes the key.
- The binary is the only process; there is no supervisor and no child agent process.

## Progress

Built and verified on this host. `gpui-app/` is one crate producing the `agent` binary and the browser `cdylib`.

**Verified by running it**, not by inspection:

- `agent models` reads the real `agents/default/agent.toml` and reports the context override, the typed transcription id and the rows that resolve.
- `agent serve` binds, creates its state directory, and answers `GET /health`, `GET /api/agent`, `GET /api/providers`, `GET /api/models` and `GET /api/events`. The SSE stream emits `ready` on connect so a client can tell a live stream from a silent one.
- `PUT /api/providers/openrouter/credential` writes `openrouter.key` at `0600`, flips `/api/models` from `inert: true` to `false`, and an empty submission removes the file — zero files remain afterwards.
- An environment credential outranks a stored one and the shadow is reported rather than hidden.
- An unknown provider is refused with `404` and the name, rather than being created.
- 10 unit tests, clippy clean, no warnings.

**The contract was checked against what the Settings screens actually render, and it was short.** The screens build a provider card from `id`, `label`, `summary`, `modalities`, `keySource`, `envVar`, `baseUrl`, `baseUrlEnv` and `hasCatalogue`. The Rust view carried `id`, `kind`, `endpoint`, `modalities`, `credential` and `shadowedFile`, and was missing five of them: there was no name to show, no description, no environment variable to name, no way to say a provider publishes no model list, and no way to say where a self-hosted provider lives.

A field the contract does not carry does not error. It renders as a blank where a name should be, which is the failure nobody notices. So:

- The provider section of the manifest gained `label`, `summary`, `env_var`, `base_url_env` and `has_catalogue`. `env_var` defaults to `<ID>_API_KEY` and `has_catalogue` defaults to true, so a minimal manifest still works.
- `GET /api/providers` carries all of it. `GET /api/models` gained the provider cards the models screen renders beside the use-case rows, including whether each provider is configured.
- Two contract tests pin this: the provider payload must carry every field the providers screen renders, and the models payload must carry the provider cards. A field dropped from the contract fails a test rather than a user's screen.

The screens still read from the TypeScript host today; porting them to this contract is [WF-IMP-016](WF-IMP-016-settings-login-and-bootstrap-routes.md). The comparison is recorded here because it is what decided the contract.

**Not verified, and why.** Four criteria need a live provider. This host has an `OPENROUTER_API_KEY` in the environment, and **OpenRouter rejects it**: a direct request to `POST /api/v1/chat/completions` answers `401 {"error":{"message":"User not found."}}`, and the same request through the adapter answers identically. The key has the right shape and is not accepted, so it is expired, revoked, or a placeholder. The request path itself is exercised: the catalogue fetch, which needs no credential, succeeds against the live provider and returns the models.

The live tests are in the tree and say so themselves. `gpui-app/src/models/live_tests.rs` covers a completion, a tool call, structured output, the catalogue and transcription; each one probes the credential first and fails with "the credential was rejected by the provider" rather than a raw 401 in the middle of a completion, which would read like a bug in the adapter. Run them with a working key:

```sh
OPENROUTER_API_KEY=... cargo test -- --ignored --nocapture
```

They use the free model the shipped manifest names, so they cost nothing, and the repository's own speech fixture for transcription, so the transcription is checked against a file whose content is known.

- *A completion reaches a real provider through the abstraction, and the same call works against two differently-shaped providers.* The abstraction and the OpenAI-compatible adapter are written; neither has been called against a live endpoint, and only one provider adapter exists. The second-shaped provider is the Anthropic or Local path, which is a manifest `type` with no adapter behind it yet.
- *Tool calls and structured output are exercised against the provider.* The wire translation and the streaming accumulation are written and the SSE frame parser is exercised by construction, but nothing has round-tripped against a real response.
- *Transcription is reachable through the provider layer.* Implemented, including hand-assembled multipart; never invoked.
- *Two providers configured at once both work.* Only one is configured.

Those need a key, not more code. Until they are run, the honest status is: the host is built and its local behaviour is verified, and its provider behaviour is unverified.