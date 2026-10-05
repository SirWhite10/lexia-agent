import { afterAll, describe, expect, test } from 'bun:test';
import { existsSync, mkdtempSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { PROVIDERS } from '../src/lib/server/provider-registry.js';

// provider-key.ts resolves its state directory when the module loads and
// captures the boot environment once, on globalThis. The temp directory must
// exist before the import, and the boot-environment case has to run in its own
// process because that capture cannot be repeated.
const workspace = mkdtempSync(join(tmpdir(), 'lexosa-keys-test-'));
process.env.LEXIA_STATE_DIR = workspace;
// provider-key.ts captures the boot environment once, so every provider
// variable has to be absent before it loads — not only the ones this file
// names. A developer's .env or shell must not decide whether this suite
// passes: with OPENROUTER_API_KEY set, the key source is 'environment'
// rather than 'none' and saving reports no change.
for (const provider of PROVIDERS) delete process.env[provider.envVar];
const keys = await import('../src/lib/server/provider-key.js');

const MODULE = new URL('../src/lib/server/provider-key.ts', import.meta.url).pathname;

afterAll(() => {
	rmSync(workspace, { recursive: true, force: true });
});

/** Runs a snippet in a child process so boot-time environment capture is real. */
async function inChild(env: Record<string, string>, body: string): Promise<string> {
	const child = Bun.spawn(['bun', '-e', body], {
		env: { ...process.env, ...env },
		stdout: 'pipe',
		stderr: 'pipe'
	});
	const out = (await new Response(child.stdout).text()).trim();
	const code = await child.exited;
	if (code !== 0) throw new Error(await new Response(child.stderr).text());
	return out;
}

describe('provider keys', () => {
	test('a fresh workspace has none', () => {
		expect(keys.providerKeySource('openrouter')).toBe('none');
		expect(keys.storedProviderKey('openrouter')).toBeNull();
	});

	test('are written per provider, owner-readable only', () => {
		expect(keys.saveProviderKey('openrouter', 'or-key')).toBe(true);
		expect(keys.saveProviderKey('higgsfield', 'hf-key')).toBe(true);

		expect(keys.storedProviderKey('openrouter')).toBe('or-key');
		expect(keys.storedProviderKey('higgsfield')).toBe('hf-key');
		expect(statSync(join(workspace, 'openrouter.key')).mode & 0o777).toBe(0o600);
		expect(statSync(join(workspace, 'higgsfield.key')).mode & 0o777).toBe(0o600);
	});

	test('one provider never answers for another', () => {
		expect(keys.providerKey('openai')).toBeNull();
		expect(keys.providerKey('higgsfield')).toBe('hf-key');
		expect(() => keys.providerKey('nope')).toThrow(/Unknown provider/);
	});

	test('removing one key leaves the others alone', () => {
		expect(keys.saveProviderKey('openrouter', '')).toBe(true);

		expect(existsSync(join(workspace, 'openrouter.key'))).toBe(false);
		expect(keys.storedProviderKey('higgsfield')).toBe('hf-key');
		expect(keys.providerKeySource('openrouter')).toBe('none');
	});

	test('a stored key reaches the environment on boot', () => {
		keys.applyProviderKeys();

		expect(process.env.HIGGSFIELD_API_KEY).toBe('hf-key');
		expect(keys.providerKeySource('higgsfield')).toBe('stored');
	});

	test('a key the host booted with wins over a saved one', async () => {
		const output = await inChild(
			{ LEXIA_STATE_DIR: workspace, OPENAI_API_KEY: 'from-the-shell' },
			`const keys = await import(${JSON.stringify(MODULE)});
			 const changed = keys.saveProviderKey('openai', 'from-the-browser');
			 console.log(JSON.stringify({
				 changed,
				 source: keys.providerKeySource('openai'),
				 live: keys.providerKey('openai'),
				 stored: keys.storedProviderKey('openai')
			 }));`
		);

		expect(JSON.parse(output)).toEqual({
			changed: false,
			source: 'environment',
			live: 'from-the-shell',
			stored: 'from-the-browser'
		});
	});
});