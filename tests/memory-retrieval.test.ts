import { afterAll, describe, expect, test } from 'bun:test';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

// The memory module resolves its state directory at load; the temp dir must be
// in place before the import so tests never touch the real installation.
const stateDir = mkdtempSync(join(tmpdir(), 'lexia-memory-test-'));
process.env.LEXIA_STATE_DIR = stateDir;
// Dynamic import exception: the state dir must exist before the module loads.
const memory = await import('../src/lib/server/memory.js');

const MODEL = 'openai/text-embedding-3-small';

function toBlob(vector: number[]): Uint8Array {
	return new Uint8Array(new Float32Array(vector).buffer);
}

async function indexFact(body: string, tags: string, vector: number[], model = MODEL): Promise<string> {
	const entry = memory.recordFact({ body, tags });
	memory.setEntryEmbedding(entry.id, toBlob(vector), model);
	return entry.id;
}

afterAll(() => {
	rmSync(stateDir, { recursive: true, force: true });
});

describe('role-scoped retrieval', () => {
	test('ranks by cosine similarity within the role card scope', async () => {
		await indexFact('bedroom is room 2 with 3 lights', 'rank:home', [1, 0, 0]);
		await indexFact('user prefers oat milk', 'rank:grocery', [0, 1, 0]);
		await indexFact('desk lamp is on the left', 'rank:home', [0.8, 0.2, 0]);

		const hits = await memory.retrieveMemory({
			query: 'lights',
			scope: 'rank:home',
			embed: async () => [1, 0, 0],
			embeddingModel: MODEL,
		});
		expect(hits[0]?.body).toBe('bedroom is room 2 with 3 lights');
		expect(hits.map((hit) => hit.body).sort()).toEqual(['bedroom is room 2 with 3 lights', 'desk lamp is on the left']);
	});

	test('an unscoped caller sees every entry regardless of tags', async () => {
		const hits = await memory.retrieveMemory({
			query: 'lights',
			embed: async () => [1, 0, 0],
			embeddingModel: MODEL,
		});
		expect(hits.map((hit) => hit.body)).toContain('bedroom is room 2 with 3 lights');
		expect(hits.map((hit) => hit.body)).toContain('user prefers oat milk');
	});

	test('never mixes vector spaces: a different embedding model is invisible', async () => {
		await indexFact('stale vector from an old model', 'model:home', [1, 0, 0], 'old/model-v1');
		const hits = await memory.retrieveMemory({
			query: 'lights',
			scope: 'model:home',
			embed: async () => [1, 0, 0],
			embeddingModel: MODEL,
		});
		expect(hits).toEqual([]);
	});

	test('respects the entry count limit', async () => {
		const long = 'y'.repeat(200);
		for (let i = 0; i < 12; i++) await indexFact(`capped fact ${i} ${long}`, 'limit:bulk', [1, 0, 0]);

		const capped = await memory.retrieveMemory({
			query: 'bulk',
			scope: 'limit:bulk',
			embed: async () => [1, 0, 0],
			embeddingModel: MODEL,
			limit: 3,
		});
		expect(capped.length).toBe(3);
	});

	test('respects the token budget', async () => {
		const long = 'z'.repeat(400);
		for (let i = 0; i < 12; i++) await indexFact(`budget fact ${i} ${long}`, 'budget:bulk', [1, 0, 0]);

		const budgeted = await memory.retrieveMemory({
			query: 'bulk',
			scope: 'budget:bulk',
			embed: async () => [1, 0, 0],
			embeddingModel: MODEL,
			tokenBudget: 120,
		});
		const spent = budgeted.reduce((total, hit) => total + Math.ceil(hit.body.length / 4), 0);
		expect(budgeted.length).toBeGreaterThan(0);
		expect(spent).toBeLessThanOrEqual(120);
	});

	test('returns nothing when the query cannot be embedded', async () => {
		const hits = await memory.retrieveMemory({ query: 'anything', embed: async () => null });
		expect(hits).toEqual([]);
	});
});

describe('re-index after an embedding model change', () => {
	// Re-index is corpus-global, so these assert per-row outcomes rather than
	// global counts: a fact is retrievable on the new model only if it was
	// re-embedded, and a fact that failed keeps its old (hidden) vector.
	test('re-embeds foreign-model and missing rows; a failed embed stays unindexed', async () => {
		const stale = memory.recordFact({ body: 'reindex stale foreign model row', tags: 'reindex:foreign' });
		memory.setEntryEmbedding(stale.id, toBlob([0, 0, 1]), 'old/model-v1');
		const missing = memory.recordFact({ body: 'reindex row with no vector', tags: 'reindex:missing' });
		const broken = memory.recordFact({ body: 'reindex row whose embed always fails', tags: 'reindex:broken' });
		const current = memory.recordFact({ body: 'reindex row already on new model', tags: 'reindex:current' });
		memory.setEntryEmbedding(current.id, toBlob([0, 1, 0]), MODEL);

		const result = await memory.reindexMemory({
			model: MODEL,
			embed: async (text) => {
				if (text.includes('always fails')) throw new Error('provider down');
				return [1, 0, 0];
			},
		});

		expect(result.failed).toContain(broken.id);
		expect(result.reindexed).toBeGreaterThanOrEqual(3);

		const hits = await memory.retrieveMemory({
			query: 'reindex',
			scope: 'reindex:foreign',
			embed: async () => [1, 0, 0],
			embeddingModel: MODEL,
		});
		expect(hits.map((hit) => hit.id)).toContain(stale.id);

		const missingHits = await memory.retrieveMemory({
			query: 'reindex',
			scope: 'reindex:missing',
			embed: async () => [1, 0, 0],
			embeddingModel: MODEL,
		});
		expect(missingHits.map((hit) => hit.id)).toContain(missing.id);

		const brokenHits = await memory.retrieveMemory({
			query: 'reindex',
			scope: 'reindex:broken',
			embed: async () => [1, 0, 0],
			embeddingModel: MODEL,
		});
		expect(brokenHits.map((hit) => hit.id)).not.toContain(broken.id);

		const currentHits = await memory.retrieveMemory({
			query: 'reindex',
			scope: 'reindex:current',
			embed: async () => [0, 1, 0],
			embeddingModel: MODEL,
		});
		expect(currentHits.map((hit) => hit.id)).toContain(current.id);
	});

	test('a null embed skips a row and never clears its existing vector', async () => {
		const entry = memory.recordFact({ body: 'reindex null embed survivor', tags: 'reindex:null' });
		memory.setEntryEmbedding(entry.id, toBlob([0, 0, 1]), 'old/model-v1');

		const result = await memory.reindexMemory({
			model: MODEL,
			embed: async (text) => (text.includes('null embed survivor') ? null : [1, 0, 0]),
		});
		expect(result.skipped).toBe(1);

		// Still indexed, but under the old model: invisible to new-model retrieval.
		const hidden = await memory.retrieveMemory({
			query: 'null embed survivor',
			scope: 'reindex:null',
			embed: async () => [1, 0, 0],
			embeddingModel: MODEL,
		});
		expect(hidden.map((hit) => hit.id)).not.toContain(entry.id);

		// Visible again to a caller still on the old model: the vector survived.
		const visibleToOld = await memory.retrieveMemory({
			query: 'null embed survivor',
			scope: 'reindex:null',
			embed: async () => [0, 0, 1],
			embeddingModel: 'old/model-v1',
		});
		expect(visibleToOld.map((hit) => hit.id)).toContain(entry.id);
	});
});

describe('reflection', () => {
	test('distills a fact into a versioned row plus markdown and rewrites personality traits', async () => {
		const result = await memory.reflect({
			transcript: 'user: turn off the bedroom lights\nlexia: done',
			knownFacts: '',
			callModel: async () =>
				JSON.stringify({
					facts: [{ body: 'bedroom lights are on the north wall switch', tags: ['reflect:home'] }],
					traits: ['answers concisely', 'prefers short confirmations'],
				}),
		});

		expect(result.facts).toHaveLength(1);
		const fact = result.facts[0];
		expect(fact.body).toBe('bedroom lights are on the north wall switch');
		const file = readFileSync(join(stateDir, fact.filePath as string), 'utf8');
		expect(file).toContain(fact.body);
		expect(file).toContain('tags: reflect:home');
		expect(memory.listFactRevisions(fact.id)).toHaveLength(1);

		const personality = memory.loadPersonality();
		expect(personality).toContain('answers concisely');
		expect(personality).toContain('prefers short confirmations');
		expect(personality).toMatch(/- \[\d{4}-\d{2}-\d{2}T[^\]]+\] answers concisely/);
	});

	test('preserves the first-observed date of a trait that survives a rewrite', async () => {
		await memory.reflect({
			transcript: 'user: more\nlexia: ok',
			knownFacts: '',
			callModel: async () =>
				JSON.stringify({ facts: [], traits: ['answers concisely', 'asks before acting'] }),
		});
		const personality = memory.loadPersonality();
		const dates = [...personality.matchAll(/^- \[([^\]]+)\] answers concisely$/gm)].map((match) => match[1]);
		expect(dates).toHaveLength(1);
		expect(personality).toContain('asks before acting');
	});

	test('revises an existing fact instead of duplicating it', async () => {
		const original = memory.recordFact({ body: 'kitchen has a blue kettle', tags: 'reflect:kitchen' });
		await memory.reflect({
			transcript: 'user: the kettle is green now',
			knownFacts: '',
			callModel: async () =>
				JSON.stringify({
					facts: [{ body: 'kitchen has a blue kettle', tags: ['reflect:kitchen', 'reflect:home'] }],
					traits: [],
				}),
		});
		const matches = memory.listFacts().filter((entry) => entry.body === 'kitchen has a blue kettle');
		expect(matches).toHaveLength(1);
		expect(memory.listFactRevisions(original.id)).toHaveLength(2);
	});
});
