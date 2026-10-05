import { mkdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { Database } from 'bun:sqlite';

// Durable run/action state and the append-only per-run event log (WF-IMP-001,
// ADR 0002). The stable Lexosa host owns lifecycle, scheduling, recovery, and
// progress publication; EVE executes assigned actions under renewable leases.
// Every state transition commits atomically with its replay event, and parent
// run status is a derived projection over child action states, never a second
// independently mutable truth. Same state database and module pattern as
// auth.ts, agents.ts, and memory.ts (own connection per module).
const stateDir = resolve(process.env.LEXIA_STATE_DIR ?? join(process.cwd(), 'app-data/state'));
const databasePath = join(stateDir, 'lexia.sqlite');
mkdirSync(dirname(databasePath), { recursive: true });

const database = new Database(databasePath);
database.run('PRAGMA journal_mode = WAL;');
database.run('PRAGMA foreign_keys = ON;');
database.run(`
	CREATE TABLE IF NOT EXISTS runs (
		id TEXT PRIMARY KEY,
		message_id TEXT,
		title TEXT NOT NULL,
		status TEXT NOT NULL CHECK (status IN ('planning', 'active', 'waiting', 'succeeded', 'failed', 'cancelled', 'partially_complete')),
		cancelled_reason TEXT,
		created_at TEXT NOT NULL,
		updated_at TEXT NOT NULL
	);

	CREATE TABLE IF NOT EXISTS actions (
		id TEXT PRIMARY KEY,
		run_id TEXT NOT NULL REFERENCES runs(id),
		outcome TEXT NOT NULL,
		capability TEXT NOT NULL,
		execution_mode TEXT NOT NULL CHECK (execution_mode IN ('immediate', 'background', 'scheduled', 'dependent')),
		status TEXT NOT NULL CHECK (status IN ('planned', 'waiting_for_input', 'waiting_for_approval', 'blocked_by_dependency', 'queued', 'running', 'needs_reconciliation', 'succeeded', 'failed', 'cancelled')),
		result TEXT,
		error TEXT,
		cancellation_reason TEXT,
		idempotency_key TEXT,
		requires_input INTEGER NOT NULL DEFAULT 0,
		requires_approval INTEGER NOT NULL DEFAULT 0,
		progress TEXT,
		attempt INTEGER NOT NULL DEFAULT 0,
		lease_id TEXT,
		lease_expires_at TEXT,
		created_at TEXT NOT NULL,
		updated_at TEXT NOT NULL
	);

	CREATE TABLE IF NOT EXISTS action_dependencies (
		action_id TEXT NOT NULL REFERENCES actions(id),
		depends_on_action_id TEXT NOT NULL REFERENCES actions(id),
		PRIMARY KEY (action_id, depends_on_action_id)
	);

	CREATE TABLE IF NOT EXISTS run_events (
		run_id TEXT NOT NULL REFERENCES runs(id),
		seq INTEGER NOT NULL,
		type TEXT NOT NULL,
		payload TEXT NOT NULL,
		created_at TEXT NOT NULL,
		PRIMARY KEY (run_id, seq)
	);

	CREATE INDEX IF NOT EXISTS idx_actions_run ON actions(run_id);
	CREATE INDEX IF NOT EXISTS idx_actions_lease ON actions(status, lease_expires_at);
`);

export type RunStatus =
	| 'planning'
	| 'active'
	| 'waiting'
	| 'succeeded'
	| 'failed'
	| 'cancelled'
	| 'partially_complete';

export type ActionStatus =
	| 'planned'
	| 'waiting_for_input'
	| 'waiting_for_approval'
	| 'blocked_by_dependency'
	| 'queued'
	| 'running'
	| 'needs_reconciliation'
	| 'succeeded'
	| 'failed'
	| 'cancelled';

export type ExecutionMode = 'immediate' | 'background' | 'scheduled' | 'dependent';

export type Run = {
	id: string;
	messageId: string | null;
	title: string;
	status: RunStatus;
	cancelledReason: string | null;
	createdAt: string;
	updatedAt: string;
};

export type Action = {
	id: string;
	runId: string;
	outcome: string;
	capability: string;
	executionMode: ExecutionMode;
	status: ActionStatus;
	result: string | null;
	error: string | null;
	cancellationReason: string | null;
	idempotencyKey: string | null;
	requiresInput: boolean;
	requiresApproval: boolean;
	progress: string | null;
	attempt: number;
	leaseId: string | null;
	leaseExpiresAt: string | null;
	dependsOn: string[];
	createdAt: string;
	updatedAt: string;
};

export type RunEvent = {
	runId: string;
	seq: number;
	type: string;
	payload: Record<string, unknown>;
	createdAt: string;
};

type RunRow = {
	id: string;
	message_id: string | null;
	title: string;
	status: RunStatus;
	cancelled_reason: string | null;
	created_at: string;
	updated_at: string;
};

type ActionRow = {
	id: string;
	run_id: string;
	outcome: string;
	capability: string;
	execution_mode: ExecutionMode;
	status: ActionStatus;
	result: string | null;
	error: string | null;
	cancellation_reason: string | null;
	idempotency_key: string | null;
	requires_input: number;
	requires_approval: number;
	progress: string | null;
	attempt: number;
	lease_id: string | null;
	lease_expires_at: string | null;
	created_at: string;
	updated_at: string;
};

type EventRow = { run_id: string; seq: number; type: string; payload: string; created_at: string };

const TERMINAL: Record<ActionStatus, boolean> = {
	planned: false,
	waiting_for_input: false,
	waiting_for_approval: false,
	blocked_by_dependency: false,
	queued: false,
	running: false,
	needs_reconciliation: false,
	succeeded: true,
	failed: true,
	cancelled: true,
};

// Keys whose values must never reach the event log or a run row. The adapter
// boundary sanitizes provider responses; this is the second, cheap net.
const REDACTED_KEYS = ['key', 'secret', 'token', 'password', 'authorization', 'cookie', 'cvv'];

function now(): string {
	return new Date().toISOString();
}

function sanitize(value: unknown): unknown {
	if (Array.isArray(value)) return value.map(sanitize);
	if (value !== null && typeof value === 'object') {
		const safe: Record<string, unknown> = {};
		for (const [key, entry] of Object.entries(value)) {
			safe[key] = REDACTED_KEYS.some((needle) => key.toLowerCase().includes(needle))
				? '[redacted]'
				: sanitize(entry);
		}
		return safe;
	}
	return value;
}

function toRun(row: RunRow): Run {
	return {
		id: row.id,
		messageId: row.message_id,
		title: row.title,
		status: row.status,
		cancelledReason: row.cancelled_reason,
		createdAt: row.created_at,
		updatedAt: row.updated_at,
	};
}

function dependenciesOf(actionId: string): string[] {
	return database
		.query<{ depends_on_action_id: string }>(
			'SELECT depends_on_action_id FROM action_dependencies WHERE action_id = ? ORDER BY depends_on_action_id',
		)
		.all(actionId)
		.map((row) => row.depends_on_action_id);
}

function toAction(row: ActionRow): Action {
	return {
		id: row.id,
		runId: row.run_id,
		outcome: row.outcome,
		capability: row.capability,
		executionMode: row.execution_mode,
		status: row.status,
		result: row.result,
		error: row.error,
		cancellationReason: row.cancellation_reason,
		idempotencyKey: row.idempotency_key,
		requiresInput: row.requires_input === 1,
		requiresApproval: row.requires_approval === 1,
		progress: row.progress,
		attempt: row.attempt,
		leaseId: row.lease_id,
		leaseExpiresAt: row.lease_expires_at,
		dependsOn: dependenciesOf(row.id),
		createdAt: row.created_at,
		updatedAt: row.updated_at,
	};
}

function appendEvent(runId: string, type: string, payload: Record<string, unknown>): void {
	const next =
		database
			.query<{ next: number }>('SELECT COALESCE(MAX(seq), 0) + 1 AS next FROM run_events WHERE run_id = ?')
			.get(runId)?.next ?? 1;
	database
		.query('INSERT INTO run_events (run_id, seq, type, payload, created_at) VALUES (?, ?, ?, ?, ?)')
		.run(runId, next, type, JSON.stringify(sanitize(payload)), now());
}

/**
 * Parent status is a projection over child states (ADR 0002): waiting whenever
 * progress needs input, approval, or reconciliation; active while any child can
 * still progress; otherwise the terminal mix decides success, failure,
 * cancellation, or partial completion.
 */
function projectRunStatus(runId: string): RunStatus {
	const rows = database
		.query<{ status: ActionStatus }>('SELECT status FROM actions WHERE run_id = ?')
		.all(runId)
		.map((row) => row.status);
	if (rows.length === 0) return 'succeeded';
	if (rows.some((status) => status === 'waiting_for_input' || status === 'waiting_for_approval' || status === 'needs_reconciliation')) {
		return 'waiting';
	}
	if (rows.some((status) => !TERMINAL[status])) return 'active';
	if (rows.every((status) => status === 'succeeded')) return 'succeeded';
	if (rows.every((status) => status === 'cancelled')) return 'cancelled';
	// No success anywhere plus at least one failure is failure, even when the
	// rest were cancelled; any other terminal mix is partial completion.
	if (!rows.some((status) => status === 'succeeded') && rows.some((status) => status === 'failed')) return 'failed';
	return 'partially_complete';
}

function refreshRunStatus(runId: string): void {
	database.query('UPDATE runs SET status = ?, updated_at = ? WHERE id = ?').run(projectRunStatus(runId), now(), runId);
}

export function createRun(input: { title: string; messageId?: string | null }): Run {
	const run: Run = {
		id: crypto.randomUUID(),
		messageId: input.messageId ?? null,
		title: input.title.trim() || 'Untitled request',
		status: 'planning',
		cancelledReason: null,
		createdAt: now(),
		updatedAt: now(),
	};
	database.run('BEGIN');
	try {
		database
			.query('INSERT INTO runs (id, message_id, title, status, cancelled_reason, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)')
			.run(run.id, run.messageId, run.title, run.status, run.cancelledReason, run.createdAt, run.updatedAt);
		appendEvent(run.id, 'run.created', { title: run.title, messageId: run.messageId });
		database.run('COMMIT');
	} catch (error) {
		database.run('ROLLBACK');
		throw error;
	}
	return run;
}

export function addAction(input: {
	runId: string;
	outcome: string;
	capability: string;
	executionMode?: ExecutionMode;
	dependsOn?: string[];
	/** Seeds `result` at plan time: the payload the action runs on, before the
	 * action has produced anything of its own. The dispatcher hands this to the
	 * capability as its input. */
	input?: string | null;
	requiresInput?: boolean;
	requiresApproval?: boolean;
	idempotencyKey?: string | null;
}): Action {
	const action: Action = {
		id: crypto.randomUUID(),
		runId: input.runId,
		outcome: input.outcome,
		capability: input.capability,
		executionMode: input.executionMode ?? 'immediate',
		status: 'planned',
		result: input.input ?? null,
		error: null,
		cancellationReason: null,
		idempotencyKey: input.idempotencyKey ?? null,
		requiresInput: input.requiresInput ?? false,
		requiresApproval: input.requiresApproval ?? false,
		progress: null,
		attempt: 0,
		leaseId: null,
		leaseExpiresAt: null,
		dependsOn: input.dependsOn ?? [],
		createdAt: now(),
		updatedAt: now(),
	};
	database.run('BEGIN');
	try {
		database
			.query(
				'INSERT INTO actions (id, run_id, outcome, capability, execution_mode, status, result, error, cancellation_reason, idempotency_key, requires_input, requires_approval, progress, attempt, lease_id, lease_expires_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, NULL, NULL, ?, ?, ?, NULL, 0, NULL, NULL, ?, ?)',
			)
			.run(
				action.id,
				action.runId,
				action.outcome,
				action.capability,
				action.executionMode,
				action.status,
				action.result,
				action.idempotencyKey,
				action.requiresInput ? 1 : 0,
				action.requiresApproval ? 1 : 0,
				action.createdAt,
				action.updatedAt,
			);
		for (const dependency of action.dependsOn) {
			database
				.query('INSERT INTO action_dependencies (action_id, depends_on_action_id) VALUES (?, ?)')
				.run(action.id, dependency);
		}
		appendEvent(action.runId, 'action.planned', { actionId: action.id, outcome: action.outcome, capability: action.capability });
		database.run('COMMIT');
	} catch (error) {
		database.run('ROLLBACK');
		throw error;
	}
	return action;
}

function setActionStatus(actionId: string, status: ActionStatus, event: string, payload: Record<string, unknown> = {}): void {
	const runId = database.query<{ run_id: string }>('SELECT run_id FROM actions WHERE id = ?').get(actionId)?.run_id;
	if (!runId) throw new Error(`Unknown action ${actionId}`);
	database.query('UPDATE actions SET status = ?, updated_at = ? WHERE id = ?').run(status, now(), actionId);
	appendEvent(runId, event, { actionId, ...payload });
	refreshRunStatus(runId);
}

/**
 * Moves the plan out of planning: eligible actions queue, actions with unmet
 * prerequisites wait, and an action whose dependency failed or was cancelled is
 * cancelled with a reason rather than silently dropped (ADR 0002).
 */
export function commitPlan(runId: string): Run {
	const actions = database
		.query<{ id: string; status: ActionStatus }>('SELECT id, status FROM actions WHERE run_id = ?')
		.all(runId);
	database.run('BEGIN');
	try {
		for (const { id } of actions) reconcileActionState(id);
		refreshRunStatus(runId);
		database.run('COMMIT');
	} catch (error) {
		database.run('ROLLBACK');
		throw error;
	}
	return getRun(runId) as Run;
}

function reconcileActionState(actionId: string): void {
	const action = database.query<ActionRow>('SELECT * FROM actions WHERE id = ?').get(actionId);
	if (!action || TERMINAL[action.status] || action.status === 'running') return;
	const dependencies = dependenciesOf(actionId)
		.map((id) => database.query<{ status: ActionStatus }>('SELECT status FROM actions WHERE id = ?').get(id))
		.filter((row): row is { status: ActionStatus } => row !== undefined);
	if (dependencies.some((dep) => dep.status === 'failed' || dep.status === 'cancelled')) {
		const reason = 'dependency-failed';
		database
			.query('UPDATE actions SET status = ?, cancellation_reason = ?, updated_at = ? WHERE id = ?')
			.run('cancelled', reason, now(), actionId);
		appendEvent(action.run_id, 'action.cancelled', { actionId, reason });
		return;
	}
	// Stay blocked only while a dependency is still in flight. Once every
	// dependency reaches a terminal state this falls through, which is what
	// re-queues a dependent after its prerequisite succeeds.
	if (dependencies.some((dep) => !TERMINAL[dep.status])) {
		if (action.status !== 'blocked_by_dependency') {
			database.query('UPDATE actions SET status = ?, updated_at = ? WHERE id = ?').run('blocked_by_dependency', now(), actionId);
			appendEvent(action.run_id, 'action.blocked', { actionId });
		}
		return;
	}
	if (action.requires_input === 1) {
		database.query('UPDATE actions SET status = ?, updated_at = ? WHERE id = ?').run('waiting_for_input', now(), actionId);
		appendEvent(action.run_id, 'action.waiting_input', { actionId });
		return;
	}
	if (action.requires_approval === 1) {
		database.query('UPDATE actions SET status = ?, updated_at = ? WHERE id = ?').run('waiting_for_approval', now(), actionId);
		appendEvent(action.run_id, 'action.waiting_approval', { actionId });
		return;
	}
	database.query('UPDATE actions SET status = ?, updated_at = ? WHERE id = ?').run('queued', now(), actionId);
	appendEvent(action.run_id, 'action.queued', { actionId });
}

export function provideInput(actionId: string, input: string): void {
	const action = database.query<ActionRow>('SELECT * FROM actions WHERE id = ?').get(actionId);
	if (!action || action.status !== 'waiting_for_input') return;
	database.run('BEGIN');
	try {
		database.query('UPDATE actions SET requires_input = 0, result = ?, updated_at = ? WHERE id = ?').run(input, now(), actionId);
		reconcileActionState(actionId);
		refreshRunStatus(action.run_id);
		database.run('COMMIT');
	} catch (error) {
		database.run('ROLLBACK');
		throw error;
	}
}

export function grantApproval(actionId: string, approved: boolean): void {
	const action = database.query<ActionRow>('SELECT * FROM actions WHERE id = ?').get(actionId);
	if (!action || action.status !== 'waiting_for_approval') return;
	database.run('BEGIN');
	try {
		if (approved) {
			database.query('UPDATE actions SET requires_approval = 0, updated_at = ? WHERE id = ?').run(now(), actionId);
			reconcileActionState(actionId);
		} else {
			database
				.query('UPDATE actions SET status = ?, cancellation_reason = ?, updated_at = ? WHERE id = ?')
				.run('cancelled', 'approval-denied', now(), actionId);
			appendEvent(action.run_id, 'action.cancelled', { actionId, reason: 'approval-denied' });
		}
		refreshRunStatus(action.run_id);
		database.run('COMMIT');
	} catch (error) {
		database.run('ROLLBACK');
		throw error;
	}
}

/** Claims a queued action under a fresh lease; the lease id fences later reports. */
export function claimAction(actionId: string, leaseTtlMs = 30_000): { action: Action; leaseId: string } | null {
	const action = database.query<ActionRow>('SELECT * FROM actions WHERE id = ?').get(actionId);
	if (!action || action.status !== 'queued') return null;
	const leaseId = crypto.randomUUID();
	const expires = new Date(Date.now() + leaseTtlMs).toISOString();
	database.run('BEGIN');
	try {
		database
			.query('UPDATE actions SET status = ?, attempt = attempt + 1, lease_id = ?, lease_expires_at = ?, updated_at = ? WHERE id = ?')
			.run('running', leaseId, expires, now(), actionId);
		appendEvent(action.run_id, 'action.running', { actionId, attempt: action.attempt + 1 });
		refreshRunStatus(action.run_id);
		database.run('COMMIT');
	} catch (error) {
		database.run('ROLLBACK');
		throw error;
	}
	return { action: getAction(actionId) as Action, leaseId };
}

export function renewLease(actionId: string, leaseId: string, leaseTtlMs = 30_000): boolean {
	const action = database.query<ActionRow>('SELECT * FROM actions WHERE id = ?').get(actionId);
	if (!action || action.status !== 'running' || action.lease_id !== leaseId) return false;
	database
		.query('UPDATE actions SET lease_expires_at = ?, updated_at = ? WHERE id = ?')
		.run(new Date(Date.now() + leaseTtlMs).toISOString(), now(), actionId);
	return true;
}

export function reportProgress(actionId: string, leaseId: string, progress: string): boolean {
	const action = database.query<ActionRow>('SELECT * FROM actions WHERE id = ?').get(actionId);
	if (!action || action.status !== 'running' || action.lease_id !== leaseId) return false;
	database.run('BEGIN');
	try {
		database.query('UPDATE actions SET progress = ?, updated_at = ? WHERE id = ?').run(progress, now(), actionId);
		appendEvent(action.run_id, 'action.progress', { actionId, progress });
		database.run('COMMIT');
	} catch (error) {
		database.run('ROLLBACK');
		throw error;
	}
	return true;
}

export function completeAction(actionId: string, leaseId: string, result: string): boolean {
	const action = database.query<ActionRow>('SELECT * FROM actions WHERE id = ?').get(actionId);
	if (!action || action.status !== 'running' || action.lease_id !== leaseId) return false;
	database.run('BEGIN');
	try {
		database
			.query('UPDATE actions SET status = ?, result = ?, lease_id = NULL, lease_expires_at = NULL, updated_at = ? WHERE id = ?')
			.run('succeeded', result, now(), actionId);
		appendEvent(action.run_id, 'action.succeeded', { actionId, result });
		unblockDependents(actionId);
		refreshRunStatus(action.run_id);
		database.run('COMMIT');
	} catch (error) {
		database.run('ROLLBACK');
		throw error;
	}
	return true;
}

export function failAction(actionId: string, leaseId: string, error: string): boolean {
	const action = database.query<ActionRow>('SELECT * FROM actions WHERE id = ?').get(actionId);
	if (!action || action.status !== 'running' || action.lease_id !== leaseId) return false;
	database.run('BEGIN');
	try {
		database
			.query('UPDATE actions SET status = ?, error = ?, lease_id = NULL, lease_expires_at = NULL, updated_at = ? WHERE id = ?')
			.run('failed', error, now(), actionId);
		appendEvent(action.run_id, 'action.failed', { actionId, error });
		unblockDependents(actionId);
		refreshRunStatus(action.run_id);
		database.run('COMMIT');
	} catch (error) {
		database.run('ROLLBACK');
		throw error;
	}
	return true;
}

function unblockDependents(completedActionId: string): void {
	const dependents = database
		.query<{ action_id: string }>('SELECT action_id FROM action_dependencies WHERE depends_on_action_id = ?')
		.all(completedActionId);
	for (const row of dependents) reconcileActionState(row.action_id);
}

export function cancelRun(runId: string, reason: string): void {
	const run = database.query<RunRow>('SELECT * FROM runs WHERE id = ?').get(runId);
	if (!run || run.status === 'succeeded' || run.status === 'failed' || run.status === 'cancelled') return;
	database.run('BEGIN');
	try {
		const active = database
			.query<{ id: string; status: ActionStatus }>('SELECT id, status FROM actions WHERE run_id = ?')
			.all(runId)
			.filter((row) => !TERMINAL[row.status]);
		for (const row of active) {
			database
				.query('UPDATE actions SET status = ?, cancellation_reason = ?, lease_id = NULL, lease_expires_at = NULL, updated_at = ? WHERE id = ?')
				.run('cancelled', reason, now(), row.id);
			appendEvent(runId, 'action.cancelled', { actionId: row.id, reason });
		}
		database
			.query('UPDATE runs SET status = ?, cancelled_reason = ?, updated_at = ? WHERE id = ?')
			.run('cancelled', reason, now(), runId);
		appendEvent(runId, 'run.cancelled', { reason });
		database.run('COMMIT');
	} catch (error) {
		database.run('ROLLBACK');
		throw error;
	}
}

/**
 * Startup recovery (ADR 0002): a running action whose lease expired moves to
 * needs_reconciliation; it is never blindly retried, because the side effect
 * may already have happened. Resolving that state is an explicit decision.
 */
export function recoverExpiredLeases(): string[] {
	const expired = database
		.query<{ id: string; run_id: string }>(
			"SELECT id, run_id FROM actions WHERE status = 'running' AND lease_expires_at IS NOT NULL AND lease_expires_at < ?",
		)
		.all(now());
	if (expired.length === 0) return [];
	database.run('BEGIN');
	try {
		for (const row of expired) {
			database
				.query('UPDATE actions SET status = ?, lease_id = NULL, lease_expires_at = NULL, updated_at = ? WHERE id = ?')
				.run('needs_reconciliation', now(), row.id);
			appendEvent(row.run_id, 'action.needs_reconciliation', { actionId: row.id });
			refreshRunStatus(row.run_id);
		}
		database.run('COMMIT');
	} catch (error) {
		database.run('ROLLBACK');
		throw error;
	}
	return expired.map((row) => row.id);
}

export function resolveReconciliation(actionId: string, resolution: 'retry' | 'succeeded' | 'failed'): boolean {
	const action = database.query<ActionRow>('SELECT * FROM actions WHERE id = ?').get(actionId);
	if (!action || action.status !== 'needs_reconciliation') return false;
	if (resolution === 'retry' && !action.idempotency_key) {
		// Automatic retry requires a provider-backed idempotency key; without one
		// the uncertain side effect needs a human decision.
		return false;
	}
	database.run('BEGIN');
	try {
		if (resolution === 'retry') {
			database.query('UPDATE actions SET status = ?, updated_at = ? WHERE id = ?').run('queued', now(), actionId);
			appendEvent(action.run_id, 'action.requeued', { actionId, reason: 'reconciled' });
		} else {
			database
				.query('UPDATE actions SET status = ?, error = ?, updated_at = ? WHERE id = ?')
				.run(resolution, resolution === 'failed' ? 'reconciled-as-failed' : null, now(), actionId);
			appendEvent(action.run_id, `action.${resolution}`, { actionId });
			unblockDependents(actionId);
		}
		refreshRunStatus(action.run_id);
		database.run('COMMIT');
	} catch (error) {
		database.run('ROLLBACK');
		throw error;
	}
	return true;
}

export function getRun(id: string): Run | undefined {
	const row = database.query<RunRow>('SELECT * FROM runs WHERE id = ?').get(id);
	return row ? toRun(row) : undefined;
}

export function listRuns(limit = 20): Run[] {
	return database
		.query<RunRow>('SELECT * FROM runs ORDER BY created_at DESC LIMIT ?')
		.all(limit)
		.map(toRun);
}

export function getAction(id: string): Action | undefined {
	const row = database.query<ActionRow>('SELECT * FROM actions WHERE id = ?').get(id);
	return row ? toAction(row) : undefined;
}

export function listActions(runId: string): Action[] {
	return database
		.query<ActionRow>('SELECT * FROM actions WHERE run_id = ? ORDER BY created_at ASC')
		.all(runId)
		.map(toAction);
}

export function listEvents(runId: string, afterSeq = 0): RunEvent[] {
	return database
		.query<EventRow>('SELECT * FROM run_events WHERE run_id = ? AND seq > ? ORDER BY seq ASC')
		.all(runId, afterSeq)
		.map((row) => ({ runId: row.run_id, seq: row.seq, type: row.type, payload: JSON.parse(row.payload) as Record<string, unknown>, createdAt: row.created_at }));
}

/** Fresh-connection snapshot: latest state plus every event after the cursor. */
export function runSnapshot(runId: string, afterSeq = 0): { run: Run; actions: Action[]; events: RunEvent[]; latestSeq: number } {
	const run = getRun(runId);
	if (!run) throw new Error(`Unknown run ${runId}`);
	const events = listEvents(runId, afterSeq);
	return { run, actions: listActions(runId), events, latestSeq: events.length === 0 ? afterSeq : events[events.length - 1].seq };
}

export type AgentTaskSummary = {
	actionId: string;
	runId: string;
	outcome: string;
	capability: string;
	status: ActionStatus;
	result: string | null;
	error: string | null;
	createdAt: string;
	updatedAt: string;
};

/**
 * One-shot delegations, newest first (WF-IMP-006): every non-immediate action
 * dispatched under a run, with its lifecycle and outcome. Standing sub-agents
 * are managed as role cards, so this list is their task history.
 */
export function listAgentTasks(limit = 20): AgentTaskSummary[] {
	return database
		.query<{
			id: string;
			run_id: string;
			outcome: string;
			capability: string;
			status: ActionStatus;
			result: string | null;
			error: string | null;
			created_at: string;
			updated_at: string;
		}>(
			"SELECT id, run_id, outcome, capability, status, result, error, created_at, updated_at FROM actions WHERE execution_mode != 'immediate' ORDER BY created_at DESC LIMIT ?",
		)
		.all(limit)
		.map((row) => ({
			actionId: row.id,
			runId: row.run_id,
			outcome: row.outcome,
			capability: row.capability,
			status: row.status,
			result: row.result,
			error: row.error,
			createdAt: row.created_at,
			updatedAt: row.updated_at
		}));
}

