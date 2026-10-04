import { mkdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { Database } from 'bun:sqlite';
import { MODALITY_ORDER, providerById, type Modality } from './provider-registry.js';

// Use-case → model assignments for the host's own model router. Same state
// database and module pattern as runs.ts, agents.ts, and memory.ts: this module
// owns its connection and creates its table at load, so there is one writer
// convention in the codebase rather than a second one.
//
// Every assignment ships unchosen (model_id = ''). The operator picks the model
// per use-case; seeding invented defaults would silently route work to a model
// nobody chose and cost real provider credits.
const stateDir = resolve(process.env.LEXIA_STATE_DIR ?? join(process.cwd(), 'app-data/state'));
const databasePath = join(stateDir, 'lexia.sqlite');
mkdirSync(dirname(databasePath), { recursive: true });

const database = new Database(databasePath);
database.run('PRAGMA journal_mode = WAL;');
database.run(`
	CREATE TABLE IF NOT EXISTS model_assignments (
		use_case TEXT PRIMARY KEY,
		label TEXT NOT NULL,
		model_id TEXT NOT NULL,
		builtin INTEGER NOT NULL,
		created_at TEXT NOT NULL,
		provider_id TEXT NOT NULL DEFAULT '',
		modality TEXT NOT NULL DEFAULT 'text'
	);
`);

// The table predates modality and provider_id, and `CREATE TABLE IF NOT EXISTS`
// cannot add columns to a table that already exists. Existing rows land on the
// defaults: text work, no provider chosen.
const existingColumns = new Set(
	(database.query('PRAGMA table_info(model_assignments)').all() as { name: string }[]).map((column) => column.name)
);
if (!existingColumns.has('provider_id')) {
	database.run("ALTER TABLE model_assignments ADD COLUMN provider_id TEXT NOT NULL DEFAULT ''");
}
if (!existingColumns.has('modality')) {
	database.run("ALTER TABLE model_assignments ADD COLUMN modality TEXT NOT NULL DEFAULT 'text'");
}

/** The fallback every unmatched request resolves to. Its row can be re-pointed
 * but never deleted: without it a request with no matching route has nowhere to
 * go, which is a worse failure than an unhelpful model. */
export const ALWAYS_USE_CASE = 'always';

const BUILTIN_USE_CASES: ReadonlyArray<{ useCase: string; label: string; modality: Modality }> = [
	{ useCase: ALWAYS_USE_CASE, label: 'Always', modality: 'text' },
	{ useCase: 'quick', label: 'Quick answers', modality: 'text' },
	{ useCase: 'research', label: 'Research and search', modality: 'text' },
	{ useCase: 'heavy', label: 'Heavy work', modality: 'text' },
	{ useCase: 'speech', label: 'Spoken replies', modality: 'speech' },
	{ useCase: 'transcription', label: 'Transcriptions', modality: 'transcription' },
	{ useCase: 'images', label: 'Images', modality: 'image' },
	{ useCase: 'video', label: 'Video', modality: 'video' }
];

const seedBuiltin = database.query(
	"INSERT OR IGNORE INTO model_assignments (use_case, label, model_id, builtin, created_at, provider_id, modality) VALUES (?, ?, '', 1, ?, '', ?)"
);
for (const builtin of BUILTIN_USE_CASES) {
	seedBuiltin.run(builtin.useCase, builtin.label, new Date().toISOString(), builtin.modality);
}

export type UseCaseAssignment = {
	useCase: string;
	label: string;
	modelId: string;
	/** Which provider serves the chosen model; empty until one is picked. */
	providerId: string;
	/** What kind of work this use-case is. Decides the providers on offer. */
	modality: Modality;
	builtin: boolean;
	createdAt: string;
};

type AssignmentRow = {
	use_case: string;
	label: string;
	model_id: string;
	provider_id: string;
	modality: Modality;
	builtin: number;
	created_at: string;
};

const SELECT_COLUMNS = 'use_case, label, model_id, provider_id, modality, builtin, created_at';

function toAssignment(row: AssignmentRow): UseCaseAssignment {
	return {
		useCase: row.use_case,
		label: row.label,
		modelId: row.model_id,
		providerId: row.provider_id,
		modality: row.modality,
		builtin: row.builtin === 1,
		createdAt: row.created_at
	};
}

/** One row by id, mapped here so every caller reads the same camelCase shape
 * listUseCases returns. */
function findUseCase(useCase: string): UseCaseAssignment | null {
	const row = database
		.query(`SELECT ${SELECT_COLUMNS} FROM model_assignments WHERE use_case = ?`)
		.get(useCase) as AssignmentRow | null;
	return row ? toAssignment(row) : null;
}

/** Custom ids are slugs of the label so a use-case reads the same in the store
 * as in the URL and the form, and so a duplicate id is obvious rather than a
 * silent second row under a random key. */
function slugify(label: string): string {
	return label
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-+|-+$/g, '')
		.slice(0, 40);
}

/** Grouped by modality in the order the settings page shows them, built-ins
 * before custom rows inside each group, so the page only has to cut the list. */
export function listUseCases(): UseCaseAssignment[] {
	const rows = database.query(`SELECT ${SELECT_COLUMNS} FROM model_assignments`).all() as AssignmentRow[];

	return rows
		.map(toAssignment)
		.sort((left, right) => {
			const byModality =
				MODALITY_ORDER.indexOf(left.modality) - MODALITY_ORDER.indexOf(right.modality);
			if (byModality !== 0) return byModality;
			const leftIndex = BUILTIN_USE_CASES.findIndex((entry) => entry.useCase === left.useCase);
			const rightIndex = BUILTIN_USE_CASES.findIndex((entry) => entry.useCase === right.useCase);
			if (leftIndex !== -1 || rightIndex !== -1) {
				return (leftIndex === -1 ? BUILTIN_USE_CASES.length : leftIndex) - (rightIndex === -1 ? BUILTIN_USE_CASES.length : rightIndex);
			}
			return left.createdAt.localeCompare(right.createdAt) || left.label.localeCompare(right.label);
		});
}

/** Points a use-case at a model on a provider. An empty id clears the assignment
 * back to "not chosen" rather than storing a blank that resolves to a blank
 * request. A model is only accepted from a provider the app knows: an id from a
 * service that is not configured can never be called. */
export function assignModel(useCase: string, modelId: string, providerId = ''): UseCaseAssignment {
	const existing = findUseCase(useCase);
	if (!existing) throw new Error(`No use-case named “${useCase}”.`);

	const trimmed = modelId.trim();
	if (trimmed.length > 200) throw new Error('That model ID is too long to be a real model id.');
	if (trimmed && !providerById(providerId)) throw new Error('Choose the provider that serves this model.');

	database
		.query('UPDATE model_assignments SET model_id = ?, provider_id = ? WHERE use_case = ?')
		.run(trimmed, trimmed ? providerId : '', useCase);
	return { ...existing, modelId: trimmed, providerId: trimmed ? providerId : '' };
}

/** Rejects every label that would be ambiguous in the picker: blank names, names
 * that normalise onto an existing row's id, and names that duplicate an existing
 * label in any casing. */
export function createUseCase(label: string, modality: Modality = 'text'): UseCaseAssignment {
	const trimmed = label.trim();
	if (!trimmed) throw new Error('Enter a name for the use-case.');
	if (trimmed.length > 60) throw new Error('Keep the use-case name under 60 characters.');

	const useCase = slugify(trimmed);
	if (!useCase) throw new Error('That name has no letters or numbers to use as an id.');
	if (BUILTIN_USE_CASES.some((builtin) => builtin.useCase === useCase)) {
		throw new Error(`“${trimmed}” resolves to the built-in use-case id “${useCase}”. Pick another name.`);
	}
	if (findUseCase(useCase)) throw new Error(`A use-case with the id “${useCase}” already exists.`);
	if (listUseCases().some((row) => row.label.toLowerCase() === trimmed.toLowerCase())) {
		throw new Error(`A use-case named “${trimmed}” already exists.`);
	}

	const createdAt = new Date().toISOString();
	database
		.query(
			"INSERT INTO model_assignments (use_case, label, model_id, builtin, created_at, provider_id, modality) VALUES (?, ?, '', 0, ?, '', ?)"
		)
		.run(useCase, trimmed, createdAt, modality);
	return { useCase, label: trimmed, modelId: '', providerId: '', modality, builtin: false, createdAt };
}

export function removeUseCase(useCase: string): void {
	const existing = findUseCase(useCase);
	if (!existing) throw new Error(`No use-case named “${useCase}”.`);
	if (existing.builtin) throw new Error(`“${existing.label}” is a built-in use-case and cannot be removed.`);

	database.query('DELETE FROM model_assignments WHERE use_case = ?').run(useCase);
}

/** The assigned model id, or null when the use-case has not been pointed at one. */
export function modelForUseCase(useCase: string): string | null {
	return findUseCase(useCase)?.modelId || null;
}

/** Which provider serves the assigned model, for display and for the router. */
export function providerForUseCase(useCase: string): string | null {
	return findUseCase(useCase)?.providerId || null;
}

/** The model the router should run for a request: the requested use-case's own
 * assignment when it has one, otherwise the Always fallback, otherwise nothing
 * configured — the caller decides what an unconfigured router does. */
export function resolveModelId(requested?: string | null): string | null {
	if (requested) {
		const assigned = modelForUseCase(requested);
		if (assigned) return assigned;
	}

	return modelForUseCase(ALWAYS_USE_CASE);
}