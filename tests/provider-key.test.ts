import { afterAll, describe, expect, test } from 'bun:test';
import { existsSync, mkdtempSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

// provider-key.ts reads LEXIA_STATE_DIR when the module loads and captures the
// boot-time OPENROUTER_API_KEY once, on globalThis. Dynamic import exception:
// both must be in place before the module is evaluated.
const workspace = mkdtempSync(join(tmpdir(), 'lexosa-key-test-'));
process.env.LEXIA_STATE_DIR = workspace;
delete process.env.OPENROUTER_API_KEY;
const keys = await import('../src/lib/server/provider-key.js');

const keyFile = join(workspace, 'openrouter.key');

afterAll(() => {
	delete process.env.OPENROUTER_API_KEY;
	rmSync(workspace, { recursive: true, force: true });
});

// The precedence decision is captured once per process, so the cases that need
// a host which booted with an environment key run in a child process with the
// variable set. Everything else shares this process's store-owned environment.
describe('a store-owned OpenRouter key', () => {
	test('is reported as nothing on a fresh workspace', () => {
		expect(keys.openRouterKeySource()).toBe('none');

		keys.applyOpenRouterKey();

		expect(process.env.OPENROUTER_API_KEY).toBeUndefined();
	});

	test('reaches the environment on boot and is reported as stored', () => {
		expect(keys.saveOpenRouterKey('from-browser')).toBe(true);

		keys.applyOpenRouterKey();

		expect(process.env.OPENROUTER_API_KEY).toBe('from-browser');
		expect(keys.storedOpenRouterKey()).toBe('from-browser');
		expect(keys.openRouterKeySource()).toBe('stored');
	});

	test('is written readable only by its owner', () => {
		expect(statSync(keyFile).mode & 0o777).toBe(0o600);
	});

	test('is replaced in place, and the change is reported as live', () => {
		expect(keys.saveOpenRouterKey('replacement-key')).toBe(true);

		expect(keys.storedOpenRouterKey()).toBe('replacement-key');
		expect(process.env.OPENROUTER_API_KEY).toBe('replacement-key');
		expect(keys.openRouterKeySource()).toBe('stored');
		expect(statSync(keyFile).mode & 0o777).toBe(0o600);
	});

	test('is removed when the field is submitted empty', () => {
		expect(keys.saveOpenRouterKey('   ')).toBe(true);

		expect(existsSync(keyFile)).toBe(false);
		expect(process.env.OPENROUTER_API_KEY).toBeUndefined();
		expect(keys.openRouterKeySource()).toBe('none');
	});
});

describe('a host that booted with OPENROUTER_API_KEY', () => {
	test('keeps the environment key and ignores a saved one', async () => {
		const script = `
			import { existsSync, writeFileSync } from 'node:fs';
			import { join } from 'node:path';
			writeFileSync(join(process.env.LEXIA_STATE_DIR, 'openrouter.key'), 'stored-key\\n');
			const keys = await import(${JSON.stringify(new URL('../src/lib/server/provider-key.ts', import.meta.url).pathname)});
			const before = keys.openRouterKeySource();
			keys.applyOpenRouterKey();
			const changed = keys.saveOpenRouterKey('browser-key');
			console.log(JSON.stringify({
				source: before,
				live: process.env.OPENROUTER_API_KEY,
				changed,
				file: existsSync(join(process.env.LEXIA_STATE_DIR, 'openrouter.key'))
			}));
		`;

		const child = Bun.spawn(['bun', '-e', script], {
			env: { ...process.env, LEXIA_STATE_DIR: workspace, OPENROUTER_API_KEY: 'operator-key' },
			stdout: 'pipe'
		});
		const output = JSON.parse((await new Response(child.stdout).text()).trim());

		expect(await child.exited).toBe(0);
		expect(output.source).toBe('environment');
		expect(output.live).toBe('operator-key');
		expect(output.changed).toBe(false);
		expect(output.file).toBe(true);
	});
});