---
kind: implementation
status: implemented
---

# WF-IMP-002: Add an OpenRouter model test panel

## Goal

Allow an authenticated Lexosa user to verify host-side OpenRouter credentials and a selected model using one small request, before implementing the durable intent-run vertical slice.

## Scope

- Keep the OpenRouter API key in the Lexosa host environment and make provider calls server-side only.
- Accept a manually selected OpenRouter model ID; optionally prefill `OPENROUTER_MODEL`.
- Send a fixed test prompt with a low output-token cap and show the reply, resolved model, and provider token usage.
- Return safe, actionable errors without exposing the key or raw provider error payloads.
- Do not persist test prompts/results or wire the logical model tiers into EVE runtime routing.

## Acceptance

- Settings → Models reports whether the host key is configured and can make a user-triggered test request.
- The browser receives no API key, and model test data is not persisted.
- Provider errors are surfaced without leaking credentials or arbitrary upstream response bodies.
- `bun run check` passes.
