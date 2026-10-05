import { join, resolve } from 'node:path';
import { Database } from 'bun:sqlite';

// The thought tree is how Lexosa's routing reads: a question asked at the root,
// the answers it admits, and the branch each answer opens. It is editable by the
// operator in the UI and, once the runtime router exists, by the agent itself —
// which is why it is plain rows with stable ids rather than an opaque document.
const stateDir = resolve(process.env.LEXIA_STATE_DIR ?? join(process.cwd(), 'app-data/state'));
const database = new Database(join(stateDir, 'lexia.sqlite'));
database.run(`
	CREATE TABLE IF NOT EXISTS thought_nodes (
		id TEXT PRIMARY KEY,
		parent_id TEXT,
		title TEXT NOT NULL,
		kind TEXT NOT NULL CHECK (kind IN ('question', 'route', 'action')),
		question TEXT NOT NULL,
		options TEXT NOT NULL,
		notes TEXT NOT NULL,
		position_x REAL NOT NULL,
		position_y REAL NOT NULL,
		updated_at TEXT NOT NULL
	)
`);

export type ThoughtNodeKind = 'question' | 'route' | 'action';

export type ThoughtNode = {
	id: string;
	parentId: string | null;
	title: string;
	kind: ThoughtNodeKind;
	question: string;
	options: string[];
	notes: string;
	position: { x: number; y: number };
	updatedAt: string;
};

type ThoughtNodeRow = {
	id: string;
	parent_id: string | null;
	title: string;
	kind: ThoughtNodeKind;
	question: string;
	options: string;
	notes: string;
	position_x: number;
	position_y: number;
	updated_at: string;
};

/** Node columns are fixed-width on screen; the canvas snaps to this so a tree
 * stays readable after the operator has dragged things around. */
export const GRID_SIZE = 24;

function toNode(row: ThoughtNodeRow): ThoughtNode {
	return {
		id: row.id,
		parentId: row.parent_id,
		title: row.title,
		kind: row.kind,
		question: row.question,
		// A hand-edited row must not take the whole editor down.
		options: parseOptions(row.options),
		notes: row.notes,
		position: { x: row.position_x, y: row.position_y },
		updatedAt: row.updated_at
	};
}

function parseOptions(raw: string): string[] {
	try {
		const parsed: unknown = JSON.parse(raw);
		return Array.isArray(parsed) ? parsed.filter((entry): entry is string => typeof entry === 'string') : [];
	} catch {
		return [];
	}
}

/**
 * The pipeline the host runs today, as the default tree. Every entry names a
 * step that exists in code, not an aspiration: capture and retrieval come from
 * turn.ts, the slot order is prompt.ts's cache contract, the run and its actions
 * are built by the chat action, and reflection runs on the boundary defined in
 * turn.ts. A new workspace starts from this so the editor opens on how Lexosa
 * already thinks rather than on placeholders.
 */
const DEFAULT_PIPELINE: Omit<ThoughtNodeInput, 'parentId'>[] = [
	{
		title: 'Take in a turn',
		kind: 'question',
		question: 'What did the user just ask for?',
		options: [],
		notes: 'The root of every turn. Everything below runs inside the chat action.',
		position: { x: 48, y: 48 }
	},
	{
		title: 'Capture the turn',
		kind: 'action',
		question: 'Store the message and embed it for later retrieval.',
		options: [],
		notes: 'captureTurn: writes the verbatim log entry and embeds it, so the turn is retrievable on the very next request.',
		position: { x: 48, y: 192 }
	},
	{
		title: 'Retrieve memory',
		kind: 'action',
		question: 'Which remembered facts and past turns apply to this one?',
		options: [],
		notes: 'retrieveMemory: vector search over facts and the log, budget-capped. Without a key or on a provider failure it degrades to an empty memory block rather than failing the turn.',
		position: { x: 336, y: 192 }
	},
	{
		title: 'Assemble the prompt',
		kind: 'action',
		question: 'What does the model see before the request?',
		options: [],
		notes: 'assemblePrompt slots, in cache order: personality, then every skill in full, then retrieved memory, then the capabilities the router resolved. Everything above memory must stay byte-stable for the prefix to hit cache.',
		position: { x: 624, y: 192 }
	},
	{
		title: 'Decide what this needs',
		kind: 'route',
		question: 'Which builtin, workflow, tool or model route answers this turn?',
		options: ['Answer from the conversation', 'Write to memory or notes', 'Run a tool or workflow', 'Call a model route'],
		notes: 'Jev\u2019s classification. It is not implemented yet: the tools slot is assembled empty today, and this node is where its answer will land.',
		position: { x: 912, y: 192 }
	},
	{
		title: 'Run the work',
		kind: 'action',
		question: 'What actually happens while the turn runs?',
		options: [],
		notes: 'One run per turn. chat.reply runs immediately and hands the turn to EVE; note.write runs behind it in the background. Both are leased actions, so a crash is retried rather than lost.',
		position: { x: 1200, y: 192 }
	},
	{
		title: 'Reflect at the boundary',
		kind: 'action',
		question: 'Is this turn one where memory should be rewritten?',
		options: ['Yes, distil facts and traits', 'No, leave memory alone'],
		notes: 'maybeReflect: every tenth user turn, counted from the persisted log, distil new facts and personality traits. A failed reflection never breaks the turn that triggered it.',
		position: { x: 1488, y: 192 }
	}
];

/**
 * Adds any default node the tree is missing and leaves the rest alone, so the
 * operator's own edits and re-parenting survive. Matched by title: the pipeline
 * is identified by what it is called, not by an id that would not survive a move
 * between installations.
 *
 * Reads the table directly rather than through `listThoughtNodes`, which seeds
 * an empty tree — going through it here would recurse into the seed forever.
 */
export function addDefaultPipeline(): ThoughtNode[] {
	const existing = readAllNodes();
	let root = existing.find((node) => node.parentId === null);
	// A tree that has nodes but no root is damaged; adding to it would hang the
	// new branches off nothing.
	if (!root && existing.length > 0) return [];

	const added: ThoughtNode[] = [];
	for (const template of DEFAULT_PIPELINE) {
		const isRoot = template.title === 'Take in a turn';
		const parentId = isRoot ? null : (root?.id ?? null);
		if (existing.some((node) => node.title === template.title && node.parentId === parentId)) continue;

		const saved = saveThoughtNode({ ...template, parentId });
		if (isRoot) root = saved;
		added.push(saved);
	}
	return added;
}

/** A fresh workspace gets the pipeline above rather than an empty canvas: a
 * blank tree teaches nothing about what a node is for. */
function seedStarterTree(): void {
	addDefaultPipeline();
}

function readAllNodes(): ThoughtNode[] {
	const rows = database.query('SELECT * FROM thought_nodes ORDER BY position_y, position_x').all() as ThoughtNodeRow[];
	return rows.map(toNode);
}

export function listThoughtNodes(): ThoughtNode[] {
	const nodes = readAllNodes();
	if (nodes.length === 0) {
		seedStarterTree();
		return readAllNodes();
	}
	return nodes;
}


export type ThoughtNodeInput = Omit<ThoughtNode, 'id' | 'updatedAt'> & { id?: string };

export function saveThoughtNode(input: ThoughtNodeInput): ThoughtNode {
	const id = input.id ?? crypto.randomUUID();
	const updatedAt = new Date().toISOString();

	// A parent that is not in the tree is refused rather than stored: a stale tab
	// re-parenting onto a node someone else deleted would otherwise leave a
	// branch nothing can reach from the root.
	if (input.parentId !== null && !database.query('SELECT 1 FROM thought_nodes WHERE id = ?').get(input.parentId)) {
		throw new Error('That node no longer exists, so it cannot hold a branch.');
	}
	database
		.query(
			`INSERT INTO thought_nodes (id, parent_id, title, kind, question, options, notes, position_x, position_y, updated_at)
			 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			 ON CONFLICT(id) DO UPDATE SET
				parent_id = excluded.parent_id,
				title = excluded.title,
				kind = excluded.kind,
				question = excluded.question,
				options = excluded.options,
				notes = excluded.notes,
				position_x = excluded.position_x,
				position_y = excluded.position_y,
				updated_at = excluded.updated_at`
		)
		.run(
			id,
			input.parentId,
			input.title.trim() || 'Untitled node',
			input.kind,
			input.question.trim(),
			JSON.stringify(input.options.map((option) => option.trim()).filter(Boolean)),
			input.notes.trim(),
			input.position.x,
			input.position.y,
			updatedAt
		);

	const row = database.query('SELECT * FROM thought_nodes WHERE id = ?').get(id) as ThoughtNodeRow;
	return toNode(row);
}

/** Removes a node and everything below it. Routing is a tree, so a half-deleted
 * branch would leave questions whose parent no longer exists. */
export function deleteThoughtNode(id: string): void {
	const ids = [id];
	for (let index = 0; index < ids.length; index += 1) {
		const children = database.query('SELECT id FROM thought_nodes WHERE parent_id = ?').all(ids[index]) as { id: string }[];
		ids.push(...children.map((child) => child.id));
	}

	const statement = database.query('DELETE FROM thought_nodes WHERE id = ?');
	for (const nodeId of ids) statement.run(nodeId);
}