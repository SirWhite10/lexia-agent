import { mkdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { Database } from 'bun:sqlite';

// Conversation and sub-agent state (WF-IMP-003): messages in the massive chat
// and standing sub-agents with versioned Lexosa-authored prompts. The memory
// corpus lives in memory.ts against the same state database. Same module
// pattern as auth.ts (own connection per module).
const stateDir = resolve(process.env.LEXIA_STATE_DIR ?? join(process.cwd(), 'app-data/state'));
const databasePath = join(stateDir, 'lexia.sqlite');
mkdirSync(dirname(databasePath), { recursive: true });

const database = new Database(databasePath);
database.run('PRAGMA journal_mode = WAL;');
database.run('PRAGMA foreign_keys = ON;');
database.run(`
	CREATE TABLE IF NOT EXISTS messages (
		id TEXT PRIMARY KEY,
		author_kind TEXT NOT NULL CHECK (author_kind IN ('user', 'lexia', 'sub_agent')),
		author_id TEXT,
		addressee_id TEXT,
		run_id TEXT,
		body TEXT NOT NULL,
		created_at TEXT NOT NULL
	);

	CREATE TABLE IF NOT EXISTS sub_agents (
		id TEXT PRIMARY KEY,
		name TEXT NOT NULL,
		focus TEXT NOT NULL DEFAULT '',
		memory_scope TEXT NOT NULL DEFAULT '',
		capabilities TEXT NOT NULL DEFAULT '',
		status TEXT NOT NULL DEFAULT 'standing' CHECK (status IN ('standing', 'retired')),
		created_at TEXT NOT NULL,
		archived_at TEXT
	);

	CREATE TABLE IF NOT EXISTS sub_agent_prompt_versions (
		sub_agent_id TEXT NOT NULL REFERENCES sub_agents(id),
		version INTEGER NOT NULL,
		body TEXT NOT NULL,
		created_at TEXT NOT NULL,
		PRIMARY KEY (sub_agent_id, version)
	);

	-- Clean cutover (WF-IMP-003): the old chat/agent node model is discarded,
	-- scratch rows included. Nothing carries forward from it.
	DROP TABLE IF EXISTS agents;
`);

export type MessageAuthorKind = 'user' | 'lexia' | 'sub_agent';
export type SubAgentStatus = 'standing' | 'retired';

export type Message = {
	id: string;
	authorKind: MessageAuthorKind;
	authorId: string | null;
	addresseeId: string | null;
	runId: string | null;
	body: string;
	createdAt: string;
};

export type SubAgent = {
	id: string;
	name: string;
	focus: string;
	memoryScope: string;
	capabilities: string;
	status: SubAgentStatus;
	createdAt: string;
	archivedAt: string | null;
};

/** A sub-agent plus the prompt version it currently runs with. */
export type SubAgentDetail = SubAgent & {
	currentPrompt: string | null;
	promptVersion: number | null;
};

export type SubAgentPromptVersion = {
	subAgentId: string;
	version: number;
	body: string;
	createdAt: string;
};

type MessageRow = {
	id: string;
	author_kind: MessageAuthorKind;
	author_id: string | null;
	addressee_id: string | null;
	run_id: string | null;
	body: string;
	created_at: string;
};

type SubAgentRow = {
	id: string;
	name: string;
	focus: string;
	memory_scope: string;
	capabilities: string;
	status: SubAgentStatus;
	created_at: string;
	archived_at: string | null;
};

type PromptRow = { sub_agent_id: string; version: number; body: string; created_at: string };

// One timestamp source for every write; callers never format their own.
function now(): string {
	return new Date().toISOString();
}

export function appendMessage(input: {
	authorKind: MessageAuthorKind;
	authorId?: string | null;
	addresseeId?: string | null;
	runId?: string | null;
	body: string;
}): Message {
	const message: Message = {
		id: crypto.randomUUID(),
		authorKind: input.authorKind,
		authorId: input.authorId ?? null,
		addresseeId: input.addresseeId ?? null,
		runId: input.runId ?? null,
		body: input.body,
		createdAt: now(),
	};
	database
		.query(
			'INSERT INTO messages (id, author_kind, author_id, addressee_id, run_id, body, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)',
		)
		.run(
			message.id,
			message.authorKind,
			message.authorId,
			message.addresseeId,
			message.runId,
			message.body,
			message.createdAt,
		);
	return message;
}

/** Oldest first: the massive chat reads as one conversation. */
export function listMessages(): Message[] {
	return database
		.query<MessageRow>(
			'SELECT id, author_kind, author_id, addressee_id, run_id, body, created_at FROM messages ORDER BY created_at ASC',
		)
		.all()
		.map((row) => ({
			id: row.id,
			authorKind: row.author_kind,
			authorId: row.author_id,
			addresseeId: row.addressee_id,
			runId: row.run_id,
			body: row.body,
			createdAt: row.created_at,
		}));
}

/** Active standing sub-agents, newest first. */
export function listStandingSubAgents(): SubAgent[] {
	return database
		.query<SubAgentRow>(
			"SELECT id, name, focus, memory_scope, capabilities, status, created_at, archived_at FROM sub_agents WHERE status = 'standing' AND archived_at IS NULL ORDER BY created_at DESC",
		)
		.all()
		.map((row) => ({
			id: row.id,
			name: row.name,
			focus: row.focus,
			memoryScope: row.memory_scope,
			capabilities: row.capabilities,
			status: row.status,
			createdAt: row.created_at,
			archivedAt: row.archived_at,
		}));
}

export function getSubAgent(id: string): SubAgentDetail | undefined {
	const row = database
		.query<SubAgentRow>(
			'SELECT id, name, focus, memory_scope, capabilities, status, created_at, archived_at FROM sub_agents WHERE id = ?',
		)
		.get(id);
	if (!row) return undefined;
	const prompt = database
		.query<PromptRow>(
			'SELECT sub_agent_id, version, body, created_at FROM sub_agent_prompt_versions WHERE sub_agent_id = ? ORDER BY version DESC LIMIT 1',
		)
		.get(id);
	return {
		id: row.id,
		name: row.name,
		focus: row.focus,
		memoryScope: row.memory_scope,
		capabilities: row.capabilities,
		status: row.status,
		createdAt: row.created_at,
		archivedAt: row.archived_at,
		currentPrompt: prompt?.body ?? null,
		promptVersion: prompt?.version ?? null,
	};
}

export function listSubAgentPromptVersions(id: string): SubAgentPromptVersion[] {
	return database
		.query<PromptRow>(
			'SELECT sub_agent_id, version, body, created_at FROM sub_agent_prompt_versions WHERE sub_agent_id = ? ORDER BY version ASC',
		)
		.all(id)
		.map((row) => ({
			subAgentId: row.sub_agent_id,
			version: row.version,
			body: row.body,
			createdAt: row.created_at,
		}));
}

/** Lexosa authors the system prompt; the host persists it as version 1. */
export function createSubAgent(input: {
	name: string;
	systemPrompt: string;
	focus?: string;
	memoryScope?: string;
	capabilities?: string;
}): SubAgentDetail {
	const subAgent: SubAgent = {
		id: crypto.randomUUID(),
		name: input.name.trim() || 'Untitled',
		focus: input.focus?.trim() ?? '',
		memoryScope: input.memoryScope?.trim() ?? '',
		capabilities: input.capabilities?.trim() ?? '',
		status: 'standing',
		createdAt: now(),
		archivedAt: null,
	};
	database.run('BEGIN');
	try {
		database
			.query(
				'INSERT INTO sub_agents (id, name, focus, memory_scope, capabilities, status, created_at, archived_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)',
			)
			.run(
				subAgent.id,
				subAgent.name,
				subAgent.focus,
				subAgent.memoryScope,
				subAgent.capabilities,
				subAgent.status,
				subAgent.createdAt,
				subAgent.archivedAt,
			);
		database
			.query(
				'INSERT INTO sub_agent_prompt_versions (sub_agent_id, version, body, created_at) VALUES (?, 1, ?, ?)',
			)
			.run(subAgent.id, input.systemPrompt, now());
		database.run('COMMIT');
	} catch (error) {
		database.run('ROLLBACK');
		throw error;
	}
	return { ...subAgent, currentPrompt: input.systemPrompt, promptVersion: 1 };
}

export function renameSubAgent(id: string, name: string): void {
	database
		.query('UPDATE sub_agents SET name = ? WHERE id = ?')
		.run(name.trim() || 'Untitled', id);
}

export function updateSubAgentRoleCard(
	id: string,
	roleCard: { focus?: string; memoryScope?: string; capabilities?: string },
): void {
	const sets: string[] = [];
	const values: string[] = [];
	if (roleCard.focus !== undefined) {
		sets.push('focus = ?');
		values.push(roleCard.focus.trim());
	}
	if (roleCard.memoryScope !== undefined) {
		sets.push('memory_scope = ?');
		values.push(roleCard.memoryScope.trim());
	}
	if (roleCard.capabilities !== undefined) {
		sets.push('capabilities = ?');
		values.push(roleCard.capabilities.trim());
	}
	if (sets.length === 0) return;
	values.push(id);
	database.query(`UPDATE sub_agents SET ${sets.join(', ')} WHERE id = ?`).run(...values);
}

export function retireSubAgent(id: string): void {
	database.query("UPDATE sub_agents SET status = 'retired' WHERE id = ?").run(id);
}

/** Archive, never delete: retired sub-agents stay queryable. */
export function archiveSubAgent(id: string): void {
	database
		.query('UPDATE sub_agents SET archived_at = ? WHERE id = ? AND archived_at IS NULL')
		.run(now(), id);
}

/** Revisions never overwrite: "what did it run with?" must stay answerable. */
export function reviseSubAgentPrompt(id: string, body: string): number {
	const next =
		database
			.query<{ next: number }>(
				'SELECT COALESCE(MAX(version), 0) + 1 AS next FROM sub_agent_prompt_versions WHERE sub_agent_id = ?',
			)
			.get(id)?.next ?? 1;
	database
		.query(
			'INSERT INTO sub_agent_prompt_versions (sub_agent_id, version, body, created_at) VALUES (?, ?, ?, ?)',
		)
		.run(id, next, body, now());
	return next;
}
