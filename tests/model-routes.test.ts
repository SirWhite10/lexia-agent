import { afterAll, beforeEach, describe, expect, test } from 'bun:test';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Database } from 'bun:sqlite';

// model-routes.ts resolves its database when the module loads, so the temp
// directory has to exist before the import.
const workspace = mkdtempSync(join(tmpdir(), 'lexosa-routes-test-'));
process.env.LEXIA_STATE_DIR = workspace;
const routes = await import('../src/lib/server/model-routes.js');

const MODULE = new URL('../src/lib/server/model-routes.ts', import.meta.url).pathname;

afterAll(() => {
	rmSync(workspace, { recursive: true, force: true });
});

// The built-ins are seeded once when the module loads, so a reset puts them
// back rather than deleting them: clearing the table would leave the next test
// with a workspace that has no use-cases at all.
beforeEach(() => {
	const db = new Database(join(workspace, 'lexia.sqlite'));
	db.run('DELETE FROM model_assignments WHERE builtin = 0');
	db.run("UPDATE model_assignments SET model_id = '', provider_id = '' WHERE builtin = 1");
	db.close();
});

describe('builtin use-cases', () => {
	test('cover text, speech, transcription, image and video, all unchosen', () => {
		const builtins = routes.listUseCases().filter((row) => row.builtin);

		expect(builtins.map((row) => row.modality)).toEqual([
			'text',
			'text',
			'text',
			'text',
			'speech',
			'transcription',
			'image',
			'video'
		]);
		expect(builtins.every((row) => row.modelId === '' && row.providerId === '')).toBe(true);
	});

	test('Always leads and cannot be removed', () => {
		expect(routes.listUseCases()[0].useCase).toBe(routes.ALWAYS_USE_CASE);

		expect(() => routes.removeUseCase(routes.ALWAYS_USE_CASE)).toThrow(/built-in/);
		expect(() => routes.removeUseCase('video')).toThrow(/built-in/);
	});
});

describe('assignments', () => {
	test('a chosen model reads back with the provider that serves it', () => {
		const saved = routes.assignModel('images', 'some-image-model', 'higgsfield');

		expect(saved.modelId).toBe('some-image-model');
		expect(saved.providerId).toBe('higgsfield');
		expect(routes.modelForUseCase('images')).toBe('some-image-model');
		expect(routes.providerForUseCase('images')).toBe('higgsfield');
		// Other rows are untouched.
		expect(routes.modelForUseCase('video')).toBeNull();
	});

	test('a model from a provider the app does not know is refused', () => {
		expect(() => routes.assignModel('images', 'some-model', 'not-a-provider')).toThrow(/provider/);
		expect(routes.modelForUseCase('images')).toBeNull();
	});

	test('clearing keeps the use-case and drops the provider too', () => {
		routes.assignModel('video', 'runway-model', 'runway');

		expect(routes.assignModel('video', '').providerId).toBe('');
		expect(routes.listUseCases().some((row) => row.useCase === 'video')).toBe(true);
	});

	test('a request with no match falls through to Always', () => {
		routes.assignModel(routes.ALWAYS_USE_CASE, 'fallback-model', 'openrouter');

		expect(routes.resolveModelId('quick')).toBe('fallback-model');
		routes.assignModel('quick', 'quick-model', 'openrouter');
		expect(routes.resolveModelId('quick')).toBe('quick-model');
	});
});

describe('custom use-cases', () => {
	test('keep the modality they were created with', () => {
		const created = routes.createUseCase('Meeting summaries', 'speech');

		expect(created.modality).toBe('speech');
		expect(routes.listUseCases().find((row) => row.useCase === created.useCase)?.modality).toBe('speech');
	});

	test('default to text work', () => {
		expect(routes.createUseCase('Invoice drafts').modality).toBe('text');
	});

	test('can be removed, and a duplicate label is refused', () => {
		const created = routes.createUseCase('Invoice drafts');

		expect(() => routes.createUseCase('invoice DRAFTS')).toThrow(/already exists/);
		routes.removeUseCase(created.useCase);
		expect(routes.listUseCases().some((row) => row.useCase === created.useCase)).toBe(false);
	});
});

describe('migration', () => {
	test('adds modality and provider_id to a table that predates them', async () => {
		const legacy = mkdtempSync(join(tmpdir(), 'lexosa-legacy-test-'));
		const db = new Database(join(legacy, 'lexia.sqlite'));
		db.run(`CREATE TABLE model_assignments (
			use_case TEXT PRIMARY KEY, label TEXT NOT NULL, model_id TEXT NOT NULL,
			builtin INTEGER NOT NULL, created_at TEXT NOT NULL
		)`);
		db.run("INSERT INTO model_assignments VALUES ('always', 'Always', 'older-model', 1, '2026-01-01T00:00:00.000Z')");
		db.close();

		const child = Bun.spawn(
			[
				'bun',
				'-e',
				`const routes = await import(${JSON.stringify(MODULE)});
				 const rows = routes.listUseCases();
				 console.log(JSON.stringify({
					 always: rows.find((row) => row.useCase === 'always'),
					 providerId: routes.providerForUseCase('always'),
					 count: rows.length
				 }));`
			],
			{ env: { ...process.env, LEXIA_STATE_DIR: legacy }, stdout: 'pipe', stderr: 'pipe' }
		);
		const output = (await new Response(child.stdout).text()).trim();
		if ((await child.exited) !== 0) {
			rmSync(legacy, { recursive: true, force: true });
			throw new Error(await new Response(child.stderr).text());
		}
		rmSync(legacy, { recursive: true, force: true });

		const migrated = JSON.parse(output);
		// The existing row survives, keeps its model, and lands on text with no
		// provider: that is what a row written before this change could have meant.
		expect(migrated.always.modelId).toBe('older-model');
		expect(migrated.always.modality).toBe('text');
		expect(migrated.providerId).toBeNull();
		expect(migrated.count).toBe(8);
	});
});