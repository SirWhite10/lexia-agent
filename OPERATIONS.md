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

Settings → Integrations writes the OpenRouter key to `openrouter.key` in the state
directory, owner-readable only, and restarts the supervised EVE process so the new
credential reaches the agent. EVE reads `process.env.OPENROUTER_API_KEY` once at
module load and inherits the host environment; that inheritance is why the restart is
part of saving rather than something to remember. Submitting the field empty removes
the stored key and restarts EVE again.

`OPENROUTER_API_KEY` in the environment (see `.env.example`) takes precedence over the
stored key, and the Integrations page says so when both are present. With no key from
either source, the host reports `OpenRouter is not configured on the Lexosa host.` and
Settings → Models disables its test panel.
