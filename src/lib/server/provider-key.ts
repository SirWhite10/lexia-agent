import { chmodSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { PROVIDERS, providerById } from './provider-registry.js';

// One credential per provider, from two sources: the provider's environment
// variable (what .env and process supervision provide) and a file in the state
// directory that the Settings → Providers form writes. The environment wins, so
// an operator's deliberate configuration is never silently overridden by a key
// typed into a browser.
//
// Hydration has to happen before the EVE supervisor spawns: EVE inherits this
// process's environment once and never re-reads it, which is why saving
// OpenRouter's key restarts it.
const stateDir = resolve(process.env.LEXIA_STATE_DIR ?? join(process.cwd(), 'app-data/state'));

type ProviderKeyState = { bootEnv: Record<string, string | null> };

// Captured on globalThis because the dev server replaces this module's instance
// on every edit while the process environment survives. Deciding precedence from
// the live value instead would let a key this module wrote earlier masquerade as
// an environment key and permanently outrank the file.
const state = ((globalThis as typeof globalThis & { __lexosaProviderKeys?: ProviderKeyState }).__lexosaProviderKeys ??= {
	bootEnv: Object.fromEntries(PROVIDERS.map((provider) => [provider.id, process.env[provider.envVar] ?? null]))
});

export type KeySource = 'environment' | 'stored' | 'none';

function keyPath(providerId: string): string {
	return join(stateDir, `${providerId}.key`);
}

export function storedProviderKey(providerId: string): string | null {
	const path = keyPath(providerId);
	if (!existsSync(path)) return null;
	const key = readFileSync(path, 'utf8').trim();
	return key.length > 0 ? key : null;
}

/** Puts stored keys in the environment for every provider the host booted
 * without. Must run before the EVE supervisor spawns. */
export function applyProviderKeys(): void {
	for (const provider of PROVIDERS) {
		if (state.bootEnv[provider.id] || process.env[provider.envVar]) continue;
		const key = storedProviderKey(provider.id);
		if (key) process.env[provider.envVar] = key;
	}
}

/** Which credential is in effect for a provider, for display. Never returns any
 * part of it: the Settings pages promise the secret does not reach the client. */
export function providerKeySource(providerId: string): KeySource {
	const provider = providerById(providerId);
	if (!provider) return 'none';
	if (state.bootEnv[providerId]) return 'environment';
	if (!process.env[provider.envVar]) return storedProviderKey(providerId) ? 'stored' : 'none';
	return 'stored';
}

/** The live credential, from whichever source wins. Throws for an unknown
 * provider rather than reading a variable that may not belong to it. */
export function providerKey(providerId: string): string | null {
	const provider = providerById(providerId);
	if (!provider) throw new Error(`Unknown provider “${providerId}”.`);
	return process.env[provider.envVar] ?? storedProviderKey(providerId);
}

/** Stores the key, or removes it when the field is submitted empty, so a
 * credential typed into the browser is never a one-way door. Returns whether the
 * live credential changed: EVE captured OpenRouter's at load, so the caller
 * restarts it only when this says true. */
export function saveProviderKey(providerId: string, key: string): boolean {
	const provider = providerById(providerId);
	if (!provider) throw new Error(`Unknown provider “${providerId}”.`);

	const trimmed = key.trim();

	if (!trimmed) {
		rmSync(keyPath(providerId), { force: true });
		if (state.bootEnv[providerId] || !process.env[provider.envVar]) return false;
		delete process.env[provider.envVar];
		return true;
	}

	mkdirSync(stateDir, { recursive: true });
	// Write a fresh file so the owner-only mode applies even if an older, looser
	// file was left behind by a previous install.
	rmSync(keyPath(providerId), { force: true });
	writeFileSync(keyPath(providerId), `${trimmed}\n`, { mode: 0o600 });
	chmodSync(keyPath(providerId), 0o600);

	// A key the host booted with keeps winning: saving from the browser must not
	// silently outrank an operator's shell configuration. The file is still
	// written, so removing the environment variable activates it later.
	if (state.bootEnv[providerId]) return false;
	process.env[provider.envVar] = trimmed;
	return true;
}