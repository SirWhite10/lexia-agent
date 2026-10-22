---
kind: implementation
status: in_progress
---

# WF-IMP-027: Rename to agent-platform and merge the wake prototype

## Goal

Make the repository `agent-platform`, the binary `agent`, and the instance `agents/default/`, and bring the wake-word prototype in as `agent-wake/`. Decisions are recorded in [ADR 0006](../../adr/0006-agent-platform-and-the-agent-name.md).

## Depends on

- The 39 uncommitted changes in this repository are committed or snapshotted first, so the rename diff can be reviewed on its own.
- [WF-IMP-019](WF-IMP-019-rust-agent-binary-and-model-layer.md) is complete or paused at a commit. Its code uses the old names.

## Scope

### Phase 1: snapshot and worktree

- Commit or snapshot the pending changes.
- Create a git worktree for the rename, per AGENTS.md.

### Phase 2: the mechanical rename

- `lexosa-agent/` becomes `agents/default/`.
- The Rust package and binary become `agent`. `gpui-app/` keeps its name until WF-IMP-008 decides its layout.
- `LEXOSA_*` and `LEXIA_*` environment variables become `AGENT_*`.
- Active docs and code use the new names. ADRs and tickets keep their history.
- Exclude `node_modules`, `target`, `.svelte-kit`, and generated artifacts from the search.

### Phase 3: import the wake prototype

- Create `agent-wake/` from `lexia-sherpa-wake-test/`.
- Do not carry `lexia-sherpa-wake-test.zip`, the prebuilt binary, `.venv-tools/`, or `target/`.
- Do not commit `models/`. Keep `scripts/prepare-model.sh` and fetch the model from it.
- Review `.scratch/` before importing it. It holds planning notes and is not carried by default.
- Rename the package `lexia-sherpa-wake-test` to `agent-wake`.
- Rename the wake keyword env vars from `LEXIA_*` to `AGENT_WAKE_*`.

### Phase 4: wake phrases per agent

- One agent holds several wake phrases. Each phrase maps to a command.
- The default phrase is `AGENT`. The setup wizard lets the user pick a name or skip it.
- Phrase changes restart the detector. They are refused while a microphone stream is active.

### Phase 5: web surface

- Port the sibling's Svelte UI into the SvelteKit host as a route.
- Replace the single env-credential login with the host's credential flow.
- Delete `web/` once the route is in place.

### Phase 6: capability

- Add `audio.capture` to the closed capability enum, granted per agent through the consent ledger.

## Out of scope

- Desktop app packaging. It follows the same surface.
- Choosing the command vocabulary. That belongs to WF-IMP-023.

## Acceptance criteria

- `grep` over active code and docs finds no `lexosa`, `lexia` or `alexia` outside ADRs and tickets.
- `cargo build` produces `agent`. `bun run check` and `bun test` pass.
- `agents/default/` passes `agent check`, and a fresh `agent new` produces a directory that passes it.
- The wake prototype runs from `agent-wake/` with the default phrase `AGENT`, and reports a detection on the bundled fixture.
- A phrase that the model cannot spell is refused with the reason, and the detector is not restarted.
- A second phrase can be added to one agent and maps to a different command.
- No credential, model or audio file is committed.

## Progress

Phases 1 and 2 are done. The rename is committed as `4ba5270`, the snapshot as `2371258`, and the template check fix as `325daf5`. Phases 3 to 6 are partly done.

**Done and verified**

- The sibling is imported as `agent-wake/` (52 files, no zip, binary, `.venv-tools`, `target`, `models` or `.scratch`). The model is copied locally to test and is gitignored.
- Wake identifiers renamed to `agent-wake`: env vars `AGENT_WAKE_*`, the worker `agent-wake-worker`, the systemd unit `agent-wake.service`, the fixture `agent-wake.wav`. Three double-rename faults found and fixed.
- **The phrases an agent answers to live in the manifest**, in a `[[wake]]` section: one agent holds several phrases, each mapped to a command. The host validates the section where the manifest is read, so a manifest that cannot work is refused at startup, and exposes it at `GET /api/agent`. Absent means the platform default, which is the path when setup is skipped.
- **The wake service asks the host for them** rather than parsing the manifest itself. One authority means a phrase the host would refuse is never spelled into the detector. The service spells each phrase into the detector's keyword file, because the detector matches pre-spelled keywords, and reports a phrase the model cannot spell without costing the agent the others.
- **A detection carries the command its phrase declares.** A keyword the agent does not answer to runs nothing rather than a default: a default would run the wrong thing.
- **Verified by running it:** the host served the agent's phrases, the service spelled `AGENT` into the keyword file and restarted the detector, and the worker came up ready. 19 tests in the service, 76 in the host, 73 in the TypeScript host.
- **The microphone is a capability now.** `audio_capture` is a variant in the closed enum, granted per agent and revocable through the consent ledger. The host owns the device: one capture session, shared by every granted plugin, opened lazily. Samples are mono f32 at 16 kHz whatever the device does natively. A device held by another process is reported as busy rather than answered with silence.
- **Proven against a real guest** (`guests/audio`) and **against the real microphone on this host**: 18,422 samples in 1.22 s, about 15,100/s against the 16 kHz rate.
- The ZIP route is removed. It served a file that is deliberately not carried, which is a route that looks like it works and does not.

- **The wake lab is a route in the host now**, at `/wake`, beside the other signed-in surfaces. The sibling's Svelte app is gone: it was a second app inside the repository, and its presence broke the host's tooling.
- **The host's session is what gates it.** The sibling's env-credential login is not ported: the host already has users, setup and recovery, and the route lives in the group that requires a session.
- **The two services talk by a signed token.** The host's session cookie is scoped to the host's origin, so the browser cannot present it to the wake service on another port. The host mints a short-lived token for the signed-in user and signs it; the wake service verifies it with the same secret; it is worth nothing after a minute. Without `AGENT_WAKE_TOKEN_SECRET` on both sides the feature is off and says so — the host refuses to mint and the service refuses the connection. An unsigned token is never accepted.
- **Verified by running it:** a signed-in user loads `/wake` and it renders; the token route refuses an unauthenticated request with 401; with the secret set, the wake service accepts the host's token and refuses a forged one.
- The wake service no longer serves a browser interface, so its static route and its web app are gone.

**Not done**

- **Coverage gap.** The sibling has no tests for its audio transport or capture path, and the port carries that gap with it. The transport is exercised against a real service in a browser, not by a test.

**Deliberately unchanged**

- **The wake phrase in the fixture.** The committed WAV says "Lexia", and the fixture checks the detection path rather than the phrase. The platform default is `AGENT`.
- **The production origin.** `lexia.dlx.studio` is a live domain. Renaming it means changing DNS and the tunnel, which is the operator's action. It appears in `deploy/`, `server/audio.ts`, `serve.ts` and the README.

**One finding worth keeping**

`AGENT` alone does not fire the detector, at thresholds 0.25, 0.10 or 0.05. "Hey agent" fires at all three, and the detector reports the keyword `AGENT` either way, so the mapping is unaffected. What failed is an *isolated* utterance, while the phrase in context fires. The model wants context around the keyword, which is what a person saying "Agent, do this" naturally provides. A recording of a real voice is still the honest test; the synthetic clips only prove the pipeline.

The bundled fixture now says "Hey agent" rather than the old product's name, so the committed WAV fires the platform's default phrase: the service's own fixture check answers `detected: true, keyword: "AGENT"`. It was generated with the same eSpeak NG 1.52.0 the previous fixture used, and its provenance is documented in `agent-wake/assets/fixtures/README.md` the same way.
