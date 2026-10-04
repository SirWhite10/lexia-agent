import { chmodSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

// The OpenRouter credential has two sources: OPENROUTER_API_KEY in the process
// environment (what .env and process supervision provide) and this file, which
// the Settings → Integrations form writes. The environment wins, so an
// operator's explicit shell configuration is never silently overridden by a key
// typed into a browser; the file is the fallback for a workspace that has none.
//
// Both consumers read the same variable but at different times: the host reads
// it per request (openrouter.ts), while EVE reads it once at module load
// (my-agent/agent/agent.ts) and inherits this process's environment. That is why
// saving a key also restarts the supervised EVE process.
const stateDir = resolve(process.env.LEXIA_STATE_DIR ?? join(process.cwd(), 'app-data/state'));
const keyPath = join(stateDir, 'openrouter.key');

type ProviderKeyState = { bootEnvKey: string | null };

const state = ((globalThis as typeof globalThis & { __lexosaProviderKey?: ProviderKeyState }).__lexosaProviderKey ??= {
	// Whatever the environment supplied when this host started is the operator's
	// deliberate configuration and outranks the store for the life of the
	// process. It is captured on globalThis because the dev server replaces this
	// module's instance on every edit while the process environment survives:
	// deciding precedence from the live value would let a key this module wrote
	// earlier masquerade as an environment key, permanently outranking the store.
	bootEnvKey: process.env.OPENROUTER_API_KEY ?? null
});

export type KeySource = 'environment' | 'stored' | 'none';

export function storedOpenRouterKey(): string | null {
	if (!existsSync(keyPath)) return null;
	const key = readFileSync(keyPath, 'utf8').trim();
	return key.length > 0 ? key : null;
}

/** Puts the stored key in the environment when the host started without one.
 * Must run before the EVE supervisor spawns: EVE inherits the host environment
 * and never re-reads it. */
export function applyOpenRouterKey(): void {
	if (state.bootEnvKey || process.env.OPENROUTER_API_KEY) return;
	const key = storedOpenRouterKey();
	if (key) process.env.OPENROUTER_API_KEY = key;
}

/** Which credential is in effect, for display. Never returns any part of the
 * key: the Settings pages promise the secret does not reach the client. */
export function openRouterKeySource(): KeySource {
	if (state.bootEnvKey) return 'environment';
	if (!process.env.OPENROUTER_API_KEY) return storedOpenRouterKey() ? 'stored' : 'none';
	return 'stored';
}

/** Stores the key, or removes it when the field is submitted empty, so a
 * credential typed into the browser is never a one-way door. Returns whether
 * the live credential changed: EVE captured the old one at module load, so the
 * caller restarts it only when this says true. */
export function saveOpenRouterKey(key: string): boolean {
	const trimmed = key.trim();

	if (!trimmed) {
		rmSync(keyPath, { force: true });
		if (state.bootEnvKey || !process.env.OPENROUTER_API_KEY) return false;
		delete process.env.OPENROUTER_API_KEY;
		return true;
	}

	mkdirSync(stateDir, { recursive: true });
	// Write a fresh file so the owner-only mode applies even if an older, looser
	// file was left behind by a previous install.
	rmSync(keyPath, { force: true });
	writeFileSync(keyPath, `${trimmed}\n`, { mode: 0o600 });
	chmodSync(keyPath, 0o600);

	// A key the host booted with keeps winning: saving from the browser must not
	// silently outrank an operator's shell configuration. The stored file is
	// still written, so removing the environment variable activates it later.
	if (state.bootEnvKey) return false;
	process.env.OPENROUTER_API_KEY = trimmed;
	return true;
}