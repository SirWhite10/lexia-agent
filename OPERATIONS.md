# Operations Router

This area describes how Lexosa is developed, updated, validated, and operated locally.

- `AGENTS.md` — repository instructions and workspace map.
- `RTK.md` — required command-output proxy instructions.
- `workflow/` — repeatable operational workflows and their Bun/TypeScript scripts.
- `docs/adr/` — accepted architectural decisions that constrain operations.
- `my-agent/` — the EVE agent source tree, edited in place; `eve dev` runs it.
- `app-data/state/` — persistent Lexosa state: the SQLite run/action records and event log, plus agent memory and personality files.
- `scripts/` — planned repository-level Bun automation; workflow-specific scripts belong under their workflow directory.
- `.eve/` — generated EVE runtime artifacts; never treat them as source of truth.

## Running the agent

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
