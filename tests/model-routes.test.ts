import { afterAll, describe, expect, test } from 'bun:test';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

// The store resolves its state directory at load, and SQLite creates the file on
// first query, so the temp dir must exist before the import: these tests must
// never touch the real installation's assignments.
const stateDir = mkdtempSync(join(tmpdir(), 'lexia-model-routes-test-'));
process.env.LEXIA_STATE_DIR = stateDir;
// Dynamic import exception: the state dir must exist before the module loads.
const routes = await import('../src/lib/server/model-routes.js');

afterAll(() => {
	rmSync(stateDir, { recursive: true, force: true });
});

describe('builtin use-cases', () => {
	test('a fresh workspace seeds the four builtins with no model chosen', () => {
		const rows = routes.listUseCases();

		expect(rows.map((row) => row.useCase)).toEqual(['always', 'quick', 'research', 'heavy']);
		expect(rows.map((row) => row.label)).toEqual(['Always', 'Quick answers', 'Research and search', 'Heavy work']);
		expect(rows.every((row) => row.builtin)).toBe(true);
		// Nothing is assigned until the operator picks: an invented default would
		// silently route work to a model nobody chose.
		expect(rows.every((row) => row.modelId === '')).toBe(true);
		expect(routes.modelForUseCase('always')).toBeNull();
	});

	test('the Always fallback cannot be removed', () => {
		expect(() => routes.removeUseCase('always')).toThrow(/built-in/i);
		expect(routes.listUseCases().map((row) => row.useCase)).toContain('always');
	});

	test('another builtin is protected too', () => {
		expect(() => routes.removeUseCase('heavy')).toThrow(/built-in/i);
		expect(routes.listUseCases().find((row) => row.useCase === 'heavy')).toBeDefined();
	});
});

describe('assignments', () => {
	test('a chosen model reads back for its own use-case and nothing else', () => {
		const assigned = routes.assignModel('quick', 'anthropic/claude-sonnet-4');

		expect(assigned.modelId).toBe('anthropic/claude-sonnet-4');
		expect(routes.modelForUseCase('quick')).toBe('anthropic/claude-sonnet-4');
		expect(routes.modelForUseCase('research')).toBeNull();
		expect(routes.listUseCases().find((row) => row.useCase === 'quick')?.modelId).toBe('anthropic/claude-sonnet-4');
	});

	test('an assignment survives re-reading the store', async () => {
		routes.assignModel('research', 'openai/gpt-4.1-mini');

		// Dynamic import exception: a static import would return the cached
		// instance above, and the point of this test is a second connection to
		// the same file. The specifier is runtime-unique only to defeat that cache.
		const reread = await import(`../src/lib/server/model-routes.js?reload=${Date.now()}`);

		// A fresh module instance reads what the page's load function would read
		// on the next request, so this is the value a later visit would see.
		expect(reread.modelForUseCase('research')).toBe('openai/gpt-4.1-mini');
		expect(reread.listUseCases().find((row) => row.useCase === 'research')?.modelId).toBe('openai/gpt-4.1-mini');
	});

	test('an unknown use-case is refused rather than created by assignment', () => {
		expect(() => routes.assignModel('nope', 'some/model')).toThrow(/No use-case/);
		expect(routes.listUseCases().some((row) => row.useCase === 'nope')).toBe(false);
	});
});

describe('resolveModelId', () => {
	test('the requested use-case wins when it has a model', () => {
		routes.assignModel('always', 'fallback/model');
		routes.assignModel('heavy', 'deep/model');

		expect(routes.resolveModelId('heavy')).toBe('deep/model');
	});

	test('an unassigned use-case falls through to Always', () => {
		// Cleared because an earlier test assigned it: the fallthrough only
		// happens for a use-case with no model of its own.
		routes.assignModel('quick', '');
		routes.assignModel('always', 'fallback/model');

		expect(routes.resolveModelId('quick')).toBe('fallback/model');
		expect(routes.resolveModelId('unknown-use-case')).toBe('fallback/model');
		expect(routes.resolveModelId()).toBe('fallback/model');
	});

	test('with nothing assigned anywhere the router resolves to null', () => {
		// Cleared through the public API rather than a direct delete, so this
		// covers what the settings page can actually produce, and every use-case
		// is cleared because an earlier test left some assigned.
		for (const row of routes.listUseCases()) routes.assignModel(row.useCase, '');

		expect(routes.resolveModelId('heavy')).toBeNull();
		expect(routes.resolveModelId()).toBeNull();
		expect(routes.resolveModelId('quick')).toBeNull();
	});
});

describe('custom use-cases', () => {
	test('a name becomes a slugged custom row that can be assigned and removed', () => {
		const created = routes.createUseCase('  Meeting Summaries  ');

		expect(created.useCase).toBe('meeting-summaries');
		expect(created.label).toBe('Meeting Summaries');
		expect(created.builtin).toBe(false);
		expect(created.modelId).toBe('');

		routes.assignModel('meeting-summaries', 'google/gemini-2.5-flash');
		expect(routes.modelForUseCase('meeting-summaries')).toBe('google/gemini-2.5-flash');

		routes.removeUseCase('meeting-summaries');
		expect(routes.listUseCases().some((row) => row.useCase === 'meeting-summaries')).toBe(false);
		expect(routes.modelForUseCase('meeting-summaries')).toBeNull();
	});

	test('an empty name is refused', () => {
		expect(() => routes.createUseCase('   ')).toThrow(/Enter a name/);
	});

	test('a duplicate name is refused in any casing', () => {
		routes.createUseCase('Release Notes');

		expect(() => routes.createUseCase('release notes')).toThrow(/already exists/);
		expect(routes.listUseCases().filter((row) => row.label.toLowerCase() === 'release notes')).toHaveLength(1);
		routes.removeUseCase('release-notes');
	});

	test('a name that slugifies onto a builtin is refused', () => {
		expect(() => routes.createUseCase('Always')).toThrow(/built-in use-case id/);
		expect(routes.listUseCases().find((row) => row.useCase === 'always')?.label).toBe('Always');
	});

	test('a name with no letters or digits cannot become an id', () => {
		expect(() => routes.createUseCase('!!!')).toThrow(/no letters or numbers/);
	});

	test('custom rows list after the builtins', () => {
		routes.createUseCase('Voice Notes');

		expect(routes.listUseCases().map((row) => row.useCase)).toEqual([
			'always',
			'quick',
			'research',
			'heavy',
			'voice-notes'
		]);
		routes.removeUseCase('voice-notes');
	});
});