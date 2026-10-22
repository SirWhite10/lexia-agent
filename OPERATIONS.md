# Operations Router

This area describes how Lexosa is developed, updated, validated, and operated locally.

- `AGENTS.md` — repository instructions and workspace map.
- `RTK.md` — required command-output proxy instructions.
- `workflow/` — repeatable operational workflows and their Bun/TypeScript scripts.
- `docs/adr/` — accepted architectural decisions that constrain operations.
- `my-agent/` — **retiring.** The EVE agent source tree, run by `eve dev`. Removed by [WF-IMP-025](docs/planning/tickets/WF-IMP-025-retire-eve-and-the-typescript-agent.md).
- `app-data/state/` — **migrating.** Persistent Lexosa state: the SQLite run/action records and event log, plus agent memory and personality files. Moving to the Rust agent host's state directory under [ADR 0005](./docs/adr/0005-rust-agent-host-and-wasm-plugins.md); one writer, never two.
- `scripts/` — repository-level Bun automation. `agent.ts` manages agent directories; workflow-specific scripts belong under their workflow directory.
- `.eve/` — generated EVE runtime artifacts; never treat them as source of truth.
- `lexosa-agent/` — the agent data directory: manifest, instructions, memory, skills, sub-agents, plugins, config, workspace. Authored and tracked; what the host writes into it is gitignored.
- `templates/agent/` — the agent directory template, and `agent.zip` packed from it.
- `scripts/agent.ts` — creates an agent directory from the template (`new`), rebuilds the zip (`pack`), validates a directory (`check`).
- `gpui-app/` — the Rust crate and its README. `cargo`, pinned to a nightly by its `rust-toolchain.toml`. Builds the `lexosa` binary.

## The agent data directory

`lexosa-agent/` is data, not source. Nothing in it is compiled and nothing in it needs a rebuild to take effect — the host watches it, so an edit applies on the next event. See [lexosa-agent/README.md](lexosa-agent/README.md) for the structure and the watch contract.

Create an agent from the template rather than by hand:

```sh
bun scripts/agent.ts new code-reviewer      # lexosa-agent/subagents/code-reviewer/
bun scripts/agent.ts check <dir>            # structure parity, no unsubstituted tokens
bun scripts/agent.ts pack                   # rebuild agent.zip after editing templates/agent/
```

`pack` is deterministic, so an unchanged tree produces an unchanged zip. Commit both in the same change. `new` refuses to overwrite: a directory may hold a prompt version history, a memory index and plugins, and none of those are replaceable by accident.

## The agent host

`gpui-app/` builds one binary, `lexosa`. It is the agent host; the client arrives later.

```sh
cd gpui-app && cargo build

./target/debug/lexosa models      # use-cases, and what each resolves to
./target/debug/lexosa providers   # providers, and which credential is in force
./target/debug/lexosa serve       # the HTTP surface, on 127.0.0.1:8787
```

Run it from the repository root so `lexosa-agent/agent.toml` resolves, or pass `--agent-dir` / `--state-dir`, or set `LEXOSA_AGENT_DIR` / `LEXOSA_STATE_DIR`.

Two rules carry over from the retiring host and must keep working:

- **Its state directory is not the old one.** It defaults to `app-data/state/agent-host`, deliberately beside rather than inside `app-data/state`, because the TypeScript host still owns `lexia.sqlite` until [WF-IMP-025](docs/planning/tickets/WF-IMP-025-retire-eve-and-the-typescript-agent.md) removes it. Two writers on one database is the failure the migration exists to prevent.
- **`<PROVIDER>_API_KEY` outranks the stored key.** The stored key lives at `<state>/credentials/<provider>.key` at mode `0600`. `lexosa providers` says which one is in force and reports the shadow, so a masked key that is not actually being used is never shown as if it were.

**Toolchain.** Nightly, pinned to an exact date in `gpui-app/rust-toolchain.toml`, and the reason is in that file: the browser target needs `#![feature(stdarch_wasm_atomic_wait)]` through `wasm_thread`, and `gpui-kit` does not let it be turned off. Because the toolchain is per-directory, the agent host uses nightly too.


## Running the agent — retiring

The section below describes EVE, which [ADR 0005](./docs/adr/0005-rust-agent-host-and-wasm-plugins.md) supersedes and [WF-IMP-025](docs/planning/tickets/WF-IMP-025-retire-eve-and-the-typescript-agent.md) removes once the Rust agent host answers for it. EVE still runs today. Treat `eve dev`, the supervisor and `my-agent/` as pending deletion — do not build on them.

Two rules from the sections below **do** carry over unchanged and must survive the migration:

- **Password recovery.** The single-use code is minted server-side, printed to the terminal, written to `auth-recovery.txt`, and never reaches the browser; only a SHA-256 digest is stored. That property is preserved, not reimplemented.
- **Provider credentials.** One credential per provider, written owner-readable to `<provider>.key`; `<PROVIDER>_API_KEY` in the environment outranks the stored file for the life of the process; an empty submission removes the key. These move to the Rust host intact.

## Running EVE (retiring)

The original procedure, kept until [WF-IMP-025](docs/planning/tickets/WF-IMP-025-retire-eve-and-the-typescript-agent.md) deletes it.

`bun run dev` starts the host and the EVE agent together; the supervisor restarts EVE if it dies. Editing `my-agent/agent/` changes the agent's behaviour and EVE reloads it on the next turn. There is no build, stage, promote, or rollback step, and no second terminal to manage. See [ADR 0003](./docs/adr/0003-eve-development-runtime.md).

EVE runs as a Node program. The host strips Bun's Node-compatibility shim from
the child `PATH` and unsets `NODE` before spawning, because that shim cannot
evaluate one of EVE's internal modules; without the strip, `eve dev` exits on
every start and the supervisor loops on `Failed to evaluate authored module`
without EVE ever binding its port.

## Recovering a lost password

**Forgot password?** on the sign-in page mints a single-use recovery code; it never
resets anything on its own. The host prints the code to the terminal running
`bun run dev` and writes it to `auth-recovery.txt` in the state directory
(`app-data/state/` by default, or wherever `LEXIA_STATE_DIR` points). The browser never
receives the code, so minting one is the proof of shell access that separates the
operator from anyone who can merely load the app over the LAN. Codes expire after 30
minutes, are retired when a newer one is requested, and are deleted from disk once
spent; the plaintext never enters the database, only a SHA-256 digest does.

The username is readable in clear, unlike the password:
`bun -e 'import {Database} from "bun:sqlite"; console.log(new Database("app-data/state/lexia.sqlite", {readonly:true}).query("SELECT username FROM users").all())'`.

The session cookie is the user id in `lexia_session` and is not signed, so a session
also ends by deleting cookies for the host or running **Sign out**.

## Model provider credentials

Settings → Providers holds one credential per provider, each written to
`<provider>.key` in the state directory, owner-readable only. `<PROVIDER>_API_KEY` in
the host environment outranks the stored file for the life of the process, so a
shell configuration is never silently overridden from a browser; submitting a field
empty removes that provider's key.

Saving OpenRouter's key restarts the supervised EVE process: it reads
`process.env.OPENROUTER_API_KEY` once at module load and inherits the host
environment, so the restart is part of saving rather than something to remember.
No other provider's credential reaches EVE.

Settings → Models maps use-cases to a model and the provider that serves it.
Catalogues are fetched per provider and only when a key exists; a provider whose
list does not cover that row's modality — OpenRouter's `/models` is text-only, so
its transcription models are typed in — takes a typed model id instead of a picker.

Transcription runs in the background after a turn that carried audio, using the
Transcriptions use-case. It is the one row seeded with a model:
`openai/whisper-large-v3` through OpenRouter's `/api/v1/audio/transcriptions`.
`Local model (self-hosted)` points at an OpenAI-shaped transcription server —
whisper.cpp, Faster-Whisper, Parakeet — reached at `LOCAL_TRANSCRIPTION_URL`
(default `http://127.0.0.1:8080`), needing no credential. Handy
(https://github.com/cjpais/handy) is the reference for running such a model
offline. AssemblyAI's `universal-3-5-pro` is an AssemblyAI model, not an
OpenRouter one: reaching it means an AssemblyAI key and provider, which the
registry does not carry yet.
