---
kind: implementation
status: planned
---

# WF-IMP-016: Port `/settings/*`, `/login` and `/bootstrap`

## Goal

Convert the form-and-action routes. These are the last ones and the highest risk: they are the only surfaces that carry authentication and credential writes, so a bridge bug here is a security bug rather than a display bug.

## Depends on

- [WF-IMP-013](WF-IMP-013-port-app-shell-to-gpui-kit.md)
- [WF-IMP-012](WF-IMP-012-url-and-session-bridge.md) for the guard and `returnTo` contract

## Reference behaviour

| Route | Reference | Notes |
| --- | --- | --- |
| `/login` | `src/routes/(auth)/login/+page.svelte` + actions | Sign in, **Forgot password?** |
| `/settings/general` | `src/routes/(app)/settings/general/+page.svelte` | |
| `/settings/accounts` | `src/routes/(app)/settings/accounts/+page.svelte` | Sign out |
| `/settings/providers` | `src/routes/(app)/settings/providers/+page.svelte` | One credential per provider, owner-readable files |
| `/settings/models` | `src/routes/(app)/settings/models/+page.svelte` | Use-case → model mapping; catalogues fetched per provider |
| `/bootstrap` | `src/routes/(app)/bootstrap/+page.svelte` | First-run setup |

## Why these are the risky ones

- **Password recovery.** Per `OPERATIONS.md`, the recovery code is minted server-side, printed to the terminal, written to `auth-recovery.txt`, and **never reaches the browser**. Only a SHA-256 digest is stored. The ported screen must not change that: no code path may fetch or display the plaintext.
- **Credential writes.** A provider key is written to `<provider>.key`, owner-readable only, and an empty submission removes the key. `<PROVIDER>_API_KEY` in the host environment outranks the stored file for the life of the process. Saving the OpenRouter key **restarts the supervised EVE process** — that is part of saving, and the screen must say so.
- **Guard ordering.** An unauthenticated cold load must never paint the authenticated shell before the server redirects.
- **Session cookie.** `lexia_session` holds the user id and is **unsigned**. Sign-out deletes the cookie. Nothing in this ticket may treat it as a bearer token the client can mint.

## Translation notes

- Forms are `+page.server.ts` actions today. On web the wasm app must submit over `FetchHttpClient` with same-origin credentials and render the server's field errors in place. Progressive enhancement — a form that works without the wasm app — is the safer shape and should be decided here rather than discovered late.
- `gpui-component` supplies `form`, `input`, `select`, `switch`, `checkbox`, `radio`, `tabs`, `setting` and `dialog`. `setting` is the natural container for a preferences page.

## Out of scope

- A second credential store. Provider keys stay on the host.
- New provider integrations.
- Changing what transcription is wired to.

## Acceptance criteria

- Sign in, sign out and `returnTo` all work, including a deep link that was guarded.
- **Forgot password?** mints a code that appears nowhere in the browser: no response body, no network payload, no DOM, no console.
- A saved provider key is written owner-readable and is never returned to the client; saving OpenRouter reports that EVE restarted.
- An empty provider-key submission removes the key.
- Saving the OpenRouter key restarts EVE, and the screen tells the operator rather than appearing to hang.
- Field errors from the server render against the right fields, with the values preserved.
- Bootstrap is reachable only when setup is not finished, and cannot be re-entered afterwards.

## Progress

Not started.