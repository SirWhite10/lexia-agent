import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { Database } from 'bun:sqlite';

// The memory corpus (WF-IMP-004): verbatim user-message log entries and
// distilled facts, each fact mirrored as human-editable markdown, with a vector
// index in the shared state database. Fact file paths are stored per row, so
// the on-disk layout can follow EVE's authored structure (WF-INV-001) without a
// migration. Same state database and module pattern as auth.ts and agents.ts.
const stateDir = resolve(process.env.LEXIA_STATE_DIR ?? join(process.cwd(), 'app-data/state'));
const factsDir = join(stateDir, 'agent/memory/facts');
const personalityPath = join(stateDir, 'agent/personality.md');
const databasePath = join(stateDir, 'lexia.sqlite');
mkdirSync(dirname(databasePath), { recursive: true });

const database = new Database(databasePath);
database.run('PRAGMA journal_mode = WAL;');
database.run('PRAGMA foreign_keys = ON;');
database.run(`
	CREATE TABLE IF NOT EXISTS memory_entries (
		id TEXT PRIMARY KEY,
		kind TEXT NOT NULL CHECK (kind IN ('fact', 'log')),
		source_message_id TEXT,
		body TEXT NOT NULL,
		tags TEXT NOT NULL DEFAULT '',
		file_path TEXT,
		embedding BLOB,
		embedding_model TEXT,
		created_at TEXT NOT NULL
	);

	CREATE TABLE IF NOT EXISTS memory_revisions (
		entry_id TEXT NOT NULL REFERENCES memory_entries(id),
		revision INTEGER NOT NULL,
		body TEXT NOT NULL,
		created_at TEXT NOT NULL,
		PRIMARY KEY (entry_id, revision)
	);
`);

export const PERSONA_FILE = 'agent/personality.md';
const DEFAULT_RETRIEVAL_LIMIT = 10;
const DEFAULT_TOKEN_BUDGET = 1500;
/** Deterministic token estimate; a real tokenizer would only move the cap slightly. */
const CHARS_PER_TOKEN = 4;

export type MemoryKind = 'fact' | 'log';

export type MemoryEntry = {
	id: string;
	kind: MemoryKind;
	sourceMessageId: string | null;
	body: string;
	tags: string;
	filePath: string | null;
	embeddingModel: string | null;
	createdAt: string;
};

export type MemoryFactRevision = {
	entryId: string;
	revision: number;
	body: string;
	createdAt: string;
};

type MemoryRow = {
	id: string;
	kind: MemoryKind;
	source_message_id: string | null;
	body: string;
	tags: string;
	file_path: string | null;
	embedding_model: string | null;
	created_at: string;
};

type RevisionRow = { entry_id: string; revision: number; body: string; created_at: string };

type VectorRow = MemoryRow & { embedding: Uint8Array };

// One timestamp source for every write; callers never format their own.
function now(): string {
	return new Date().toISOString();
}

function toMemoryEntry(row: MemoryRow): MemoryEntry {
	return {
		id: row.id,
		kind: row.kind,
		sourceMessageId: row.source_message_id,
		body: row.body,
		tags: row.tags,
		filePath: row.file_path,
		embeddingModel: row.embedding_model,
		createdAt: row.created_at,
	};
}

/** One verbatim user-message log entry; pass the embedding once it is computed. */
export function recordLogEntry(input: {
	sourceMessageId?: string | null;
	body: string;
	tags?: string;
	embedding?: Uint8Array | null;
	embeddingModel?: string | null;
}): MemoryEntry {
	const entry: MemoryEntry = {
		id: crypto.randomUUID(),
		kind: 'log',
		sourceMessageId: input.sourceMessageId ?? null,
		body: input.body,
		tags: input.tags?.trim() ?? '',
		filePath: null,
		embeddingModel: input.embeddingModel ?? null,
		createdAt: now(),
	};
	database
		.query(
			'INSERT INTO memory_entries (id, kind, source_message_id, body, tags, file_path, embedding, embedding_model, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)',
		)
		.run(
			entry.id,
			entry.kind,
			entry.sourceMessageId,
			entry.body,
			entry.tags,
			entry.filePath,
			input.embedding ?? null,
			entry.embeddingModel,
			entry.createdAt,
		);
	return entry;
}

export function setEntryEmbedding(id: string, embedding: Uint8Array, model: string): void {
	database
		.query('UPDATE memory_entries SET embedding = ?, embedding_model = ? WHERE id = ?')
		.run(embedding, model, id);
}

/**
 * Number of captured user turns, the reflection trigger (WF-DEC-002). Derived
 * from the persisted log so the cadence survives restarts without scheduler
 * state.
 */
export function countUserTurns(): number {
	return (
		database
			.query<{ count: number }>("SELECT COUNT(*) AS count FROM memory_entries WHERE kind = 'log'")
			.get()?.count ?? 0
	);
}

export type ReindexResult = { reindexed: number; skipped: number; failed: string[] };

/**
 * Re-embeds every row whose vector is missing or was produced by a different
 * model. Retrieval already hides foreign-model rows, so a changed embedding
 * model degrades to "no memory" rather than to confident nonsense; this is the
 * recovery path that fills the gap again. Bodies are re-embedded one at a time
 * so a single provider failure costs one row, not the whole corpus.
 */
export async function reindexMemory(input: {
	embed: (text: string) => Promise<number[] | null>;
	model: string;
}): Promise<ReindexResult> {
	const rows = database
		.query<{ id: string; body: string }>(
			'SELECT id, body FROM memory_entries WHERE embedding IS NULL OR embedding_model IS NULL OR embedding_model <> ?',
		)
		.all(input.model);
	const result: ReindexResult = { reindexed: 0, skipped: 0, failed: [] };
	for (const row of rows) {
		try {
			const vector = await input.embed(row.body);
			if (!vector) {
				result.skipped++;
				continue;
			}
			setEntryEmbedding(row.id, new Uint8Array(new Float32Array(vector).buffer), input.model);
			result.reindexed++;
		} catch {
			result.failed.push(row.id);
		}
	}
	return result;
}

/** The markdown mirror is the human source; the row is the index. */
function writeFactFile(id: string, body: string, tags: string, createdAt: string): string {
	mkdirSync(factsDir, { recursive: true });
	const relativePath = `agent/memory/facts/${id}.md`;
	const file = [`---`, `tags: ${tags}`, `updated: ${createdAt}`, `---`, '', body, ''].join('\n');
	writeFileSync(join(stateDir, relativePath), file, 'utf8');
	return relativePath;
}

/** Distilled facts start at revision 1 and gain revisions instead of overwrites. */
export function recordFact(input: { body: string; tags?: string }): MemoryEntry {
	const entry: MemoryEntry = {
		id: crypto.randomUUID(),
		kind: 'fact',
		sourceMessageId: null,
		body: input.body,
		tags: input.tags?.trim() ?? '',
		filePath: null,
		embeddingModel: null,
		createdAt: now(),
	};
	entry.filePath = writeFactFile(entry.id, entry.body, entry.tags, entry.createdAt);
	database.run('BEGIN');
	try {
		database
			.query(
				'INSERT INTO memory_entries (id, kind, source_message_id, body, tags, file_path, embedding, embedding_model, created_at) VALUES (?, ?, NULL, ?, ?, ?, NULL, ?, ?)',
			)
			.run(
				entry.id,
				entry.kind,
				entry.body,
				entry.tags,
				entry.filePath,
				entry.embeddingModel,
				entry.createdAt,
			);
		database
			.query('INSERT INTO memory_revisions (entry_id, revision, body, created_at) VALUES (?, 1, ?, ?)')
			.run(entry.id, entry.body, entry.createdAt);
		database.run('COMMIT');
	} catch (error) {
		database.run('ROLLBACK');
		throw error;
	}
	return entry;
}

export function reviseFact(id: string, body: string, tags?: string): number {
	const next =
		database
			.query<{ next: number }>(
				'SELECT COALESCE(MAX(revision), 0) + 1 AS next FROM memory_revisions WHERE entry_id = ?',
			)
			.get(id)?.next ?? 1;
	const stamp = now();
	database.run('BEGIN');
	try {
		database
			.query('UPDATE memory_entries SET body = ?, tags = COALESCE(?, tags), file_path = COALESCE(file_path, ?) WHERE id = ?')
			.run(body, tags?.trim() ?? null, `agent/memory/facts/${id}.md`, id);
		database
			.query('INSERT INTO memory_revisions (entry_id, revision, body, created_at) VALUES (?, ?, ?, ?)')
			.run(id, next, body, stamp);
		database.run('COMMIT');
	} catch (error) {
		database.run('ROLLBACK');
		throw error;
	}
	const row = database
		.query<MemoryRow>(
			'SELECT id, kind, source_message_id, body, tags, file_path, embedding_model, created_at FROM memory_entries WHERE id = ?',
		)
		.get(id);
	if (row) writeFactFile(id, row.body, row.tags, row.created_at);
	return next;
}

export function listFacts(): MemoryEntry[] {
	return database
		.query<MemoryRow>(
			"SELECT id, kind, source_message_id, body, tags, file_path, embedding_model, created_at FROM memory_entries WHERE kind = 'fact' ORDER BY created_at DESC",
		)
		.all()
		.map(toMemoryEntry);
}

export function listFactRevisions(id: string): MemoryFactRevision[] {
	return database
		.query<RevisionRow>(
			'SELECT entry_id, revision, body, created_at FROM memory_revisions WHERE entry_id = ? ORDER BY revision ASC',
		)
		.all(id)
		.map((row) => ({
			entryId: row.entry_id,
			revision: row.revision,
			body: row.body,
			createdAt: row.created_at,
		}));
}

/** The injected personality block; empty string before the first reflection. */
export function loadPersonality(): string {
	return existsSync(personalityPath) ? readFileSync(personalityPath, 'utf8') : '';
}

/**
 * Rewrites the trait file, keeping the original date on traits that survive so
 * the file records when each trait was first observed, not when it was touched.
 */
export function savePersonality(traits: string[]): string {
	mkdirSync(dirname(personalityPath), { recursive: true });
	// Trait text identifies a line; the record holds when it was first observed.
	const previous: Record<string, string> = {};
	for (const line of loadPersonality().split('\n')) {
		const match = line.match(/^- \[([^\]]+)\] (.+)$/);
		if (match) previous[match[2]] = match[1];
	}
	const stamp = now();
	const lines = traits
		.map((trait) => trait.trim())
		.filter((trait) => trait.length > 0)
		.map((trait) => `- [${previous[trait] ?? stamp}] ${trait}`);
	const file = ['# Lexosa personality', '', ...lines, ''].join('\n');
	writeFileSync(personalityPath, file, 'utf8');
	return file;
}

const REFLECTION_SYSTEM = `You maintain Lexosa's long-term memory. Reply with JSON only, no prose and no code fences:
{"facts": [{"body": "one concise durable fact about the user or their world", "tags": ["comma","separated"]}], "traits": ["one personality trait Lexosa has observed"]}
Rules: facts are durable (preferences, facts about the user's home, work, people), never transient chit-chat, never secrets; reuse the wording of an existing fact when you revise it; traits are short lowercase phrases describing how Lexosa should behave for this user.`;

export type ReflectionResult = { facts: MemoryEntry[]; traits: string[]; file: string };

/**
 * One reflection step: the model proposes distilled facts and the full trait
 * set; facts land as versioned rows plus markdown, traits rewrite the
 * personality file. The caller supplies the model call so the pipeline never
 * holds a provider credential of its own.
 */
export async function reflect(input: {
	transcript: string;
	knownFacts: string;
	callModel: (system: string, user: string) => Promise<string>;
}): Promise<ReflectionResult> {
	const reply = await input.callModel(
		REFLECTION_SYSTEM,
		`Known memory facts:\n${input.knownFacts || '(none)'}\n\nRecent conversation:\n${input.transcript || '(none)'}`,
	);
	const match = reply.match(/\{[\s\S]*\}/);
	if (!match) throw new Error('Reflection returned no JSON object.');
	const parsed = JSON.parse(match[0]) as {
		facts?: Array<{ body?: unknown; tags?: unknown }>;
		traits?: unknown;
	};

	const known = listFacts();
	const existing: Record<string, MemoryEntry> = {};
	for (const fact of known) existing[fact.body] = fact;
	const facts: MemoryEntry[] = [];
	for (const proposed of parsed.facts ?? []) {
		const body = typeof proposed.body === 'string' ? proposed.body.trim() : '';
		if (!body) continue;
		const tags = Array.isArray(proposed.tags) ? proposed.tags.filter((tag): tag is string => typeof tag === 'string').join(',') : '';
		const prior = existing[body];
		if (prior) {
			reviseFact(prior.id, body, tags);
			facts.push({ ...prior, body, tags, filePath: prior.filePath ?? `agent/memory/facts/${prior.id}.md` });
		} else {
			facts.push(recordFact({ body, tags }));
		}
	}

	const traits = Array.isArray(parsed.traits) ? parsed.traits.filter((trait): trait is string => typeof trait === 'string') : [];
	const file = savePersonality(traits);
	return { facts, traits, file };
}

export type RetrievalOptions = {
	query: string;
	/** Comma-separated role card scope; null or empty is the main agent (unscoped). */
	scope?: string | null;
	embed: (text: string) => Promise<number[] | null>;
	embeddingModel?: string | null;
	limit?: number;
	tokenBudget?: number;
};

const SELECT_VECTORS = 'SELECT id, kind, source_message_id, body, tags, file_path, embedding_model, created_at, embedding FROM memory_entries WHERE embedding IS NOT NULL';

/**
 * Role-scoped vector retrieval ranked by cosine similarity. Rows embedded with a
 * different model are invisible: vector spaces are not comparable, and mixing
 * them returns confident nonsense. Budget-capped by a deterministic token
 * estimate; a single oversized entry is truncated rather than dropped.
 */
export async function retrieveMemory(options: RetrievalOptions): Promise<MemoryEntry[]> {
	const queryVector = await options.embed(options.query);
	if (!queryVector) return [];

	const scopeTags = (options.scope ?? '')
		.split(',')
		.map((tag) => tag.trim())
		.filter((tag) => tag.length > 0);
	const rows = database.query<VectorRow>(SELECT_VECTORS).all();
	const ranked: Array<{ row: VectorRow; score: number }> = [];
	for (const row of rows) {
		if (options.embeddingModel && row.embedding_model !== options.embeddingModel) continue;
		if (scopeTags.length > 0) {
			const entryTags = row.tags
				.split(',')
				.map((tag) => tag.trim())
				.filter((tag) => tag.length > 0);
			if (!scopeTags.some((tag) => entryTags.includes(tag))) continue;
		}
		const vector = new Float32Array(row.embedding.byteLength / 4);
		new Uint8Array(vector.buffer).set(row.embedding);
		if (vector.length !== queryVector.length) continue;
		let dot = 0;
		let queryNorm = 0;
		let entryNorm = 0;
		for (let i = 0; i < vector.length; i++) {
			dot += vector[i] * queryVector[i];
			queryNorm += queryVector[i] * queryVector[i];
			entryNorm += vector[i] * vector[i];
		}
		if (queryNorm === 0 || entryNorm === 0) continue;
		ranked.push({ row, score: dot / (Math.sqrt(queryNorm) * Math.sqrt(entryNorm)) });
	}
	ranked.sort((a, b) => b.score - a.score);

	const limit = options.limit ?? DEFAULT_RETRIEVAL_LIMIT;
	const budget = options.tokenBudget ?? DEFAULT_TOKEN_BUDGET;
	const results: MemoryEntry[] = [];
	let spent = 0;
	for (const { row } of ranked.slice(0, limit)) {
		const entry = toMemoryEntry(row);
		const tokens = Math.ceil(entry.body.length / CHARS_PER_TOKEN);
		if (spent + tokens > budget) {
			const remaining = budget - spent;
			if (remaining > 0) results.push({ ...entry, body: entry.body.slice(0, remaining * CHARS_PER_TOKEN) });
			break;
		}
		spent += tokens;
		results.push(entry);
	}
	return results;
}
