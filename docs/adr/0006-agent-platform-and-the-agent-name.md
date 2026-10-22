---
status: accepted
---

# The product is `agent-platform`; the binary and the agent are `agent`

Lexosa, Lexia and Alexia were working names. The product is now **agent-platform**: one repository that hosts one or more agents, each of which a user creates, names, shapes and gives a personality. The binary is `agent`. The word "agent" names the product, the binary and the instance, and no other name is used for them.

## Decisions

| Thing | Name |
|---|---|
| Repository | `agent-platform` (the GitHub remote is renamed with it) |
| Product and binary | `agent` |
| Environment prefix | `AGENT_` (replaces `LEXOSA_`, `LEXIA_`) |
| Agent instance directory | `agents/default/` (replaces `lexosa-agent/`) |
| Agent template | `templates/agent/` (unchanged) |
| Wake-word sibling | `agent-wake/` (from `lexia-sherpa-wake-test/`) |

## Wake words and commands

One agent holds **several wake phrases**, each mapped to a command. The detector already reports which keyword fired, so a phrase maps to a command rather than to a separate detector.

The default wake word is **AGENT**. The user chooses a name during setup and may skip that step; the default is kept so the wake path can be tested without setup.

Changing phrases restarts the detector and is refused while a microphone stream is active, as the sibling already does.

## Ports from the wake prototype

- The Svelte UI is ported into the SvelteKit host as a route. SvelteKit remains the slim host that serves the web and desktop surfaces, per [ADR 0005](0005-rust-agent-host-and-wasm-plugins.md).
- The sherpa-onnx detector stays in its own worker process, supervised by the Rust host. The native library does not link into the host binary.
- The prototype's single env-credential login is replaced by the host's credential flow. The prototype is single-operator; the platform is not.
- Microphone input is a new capability. It is granted per agent through the consent ledger, never assumed.

## Dropped from the merge

- `lexia-sherpa-wake-test.zip`
- The prebuilt Linux binary
- `.venv-tools/`
- `target/`

`models/` is not committed. It is fetched by the prepare script.

## Historical text

Active code, configuration and current documentation use the new names. ADRs and tickets are history and keep the names they were written with. This ADR is the mapping from the old names to the new ones.

## Consequences

- Every reference to `lexosa-agent/` in tickets and docs now means `agents/default/`.
- No compatibility aliases for the old environment variables or paths. Cutover is clean, per the repository's rules.
