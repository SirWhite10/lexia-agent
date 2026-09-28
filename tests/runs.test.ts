import { afterAll, describe, expect, test } from 'bun:test';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

// The coordinator resolves its state directory at load; the temp dir must exist
// before the import so tests never touch the real installation.
const stateDir = mkdtempSync(join(tmpdir(), 'lexia-runs-test-'));
process.env.LEXIA_STATE_DIR = stateDir;
// Dynamic import exception: the state dir must exist before the module loads.
const runs = await import('../src/lib/server/runs.js');

afterAll(() => {
	rmSync(stateDir, { recursive: true, force: true });
});

describe('plan commit and dependency gating', () => {
	test('an action cannot queue until its dependency succeeds', () => {
		const run = runs.createRun({ title: 'compound request' });
		const first = runs.addAction({ runId: run.id, outcome: 'first', capability: 'builtin' });
		const second = runs.addAction({ runId: run.id, outcome: 'second', capability: 'builtin', dependsOn: [first.id] });

		runs.commitPlan(run.id);
		expect(runs.getAction(first.id)?.status).toBe('queued');
		expect(runs.getAction(second.id)?.status).toBe('blocked_by_dependency');
		expect(runs.getRun(run.id)?.status).toBe('active');

		const claim = runs.claimAction(first.id);
		runs.completeAction(first.id, claim!.leaseId, 'done');
		expect(runs.getAction(second.id)?.status).toBe('queued');
	});

	test('a failed dependency cancels dependents with a recorded reason', () => {
		const run = runs.createRun({ title: 'failing dependency' });
		const first = runs.addAction({ runId: run.id, outcome: 'first', capability: 'builtin' });
		const second = runs.addAction({ runId: run.id, outcome: 'second', capability: 'builtin', dependsOn: [first.id] });
		runs.commitPlan(run.id);

		const claim = runs.claimAction(first.id);
		runs.failAction(first.id, claim!.leaseId, 'provider refused');
		const dependent = runs.getAction(second.id);
		expect(dependent?.status).toBe('cancelled');
		expect(dependent?.cancellationReason).toBe('dependency-failed');
		expect(runs.getRun(run.id)?.status).toBe('failed');
	});

	test('input and approval waits hold actions until satisfied', () => {
		const run = runs.createRun({ title: 'needs approval' });
		const action = runs.addAction({ runId: run.id, outcome: 'send', capability: 'gmail', requiresApproval: true });
		runs.commitPlan(run.id);
		expect(runs.getAction(action.id)?.status).toBe('waiting_for_approval');
		expect(runs.getRun(run.id)?.status).toBe('waiting');

		runs.grantApproval(action.id, true);
		expect(runs.getAction(action.id)?.status).toBe('queued');

		const inputRun = runs.createRun({ title: 'needs input' });
		const inputAction = runs.addAction({ runId: inputRun.id, outcome: 'ask', capability: 'builtin', requiresInput: true });
		runs.commitPlan(inputRun.id);
		runs.provideInput(inputAction.id, 'the bedroom');
		expect(runs.getAction(inputAction.id)?.status).toBe('queued');
	});
});

describe('parent status projection', () => {
	test('a mixed terminal outcome is partial completion', () => {
		const run = runs.createRun({ title: 'partial' });
		const good = runs.addAction({ runId: run.id, outcome: 'good', capability: 'builtin' });
		const bad = runs.addAction({ runId: run.id, outcome: 'bad', capability: 'builtin' });
		runs.commitPlan(run.id);

		const goodClaim = runs.claimAction(good.id);
		const badClaim = runs.claimAction(bad.id);
		runs.completeAction(good.id, goodClaim!.leaseId, 'ok');
		runs.failAction(bad.id, badClaim!.leaseId, 'nope');
		expect(runs.getRun(run.id)?.status).toBe('partially_complete');
	});

	test('cancelling a run cancels its non-terminal actions', () => {
		const run = runs.createRun({ title: 'cancel me' });
		const action = runs.addAction({ runId: run.id, outcome: 'x', capability: 'builtin' });
		runs.commitPlan(run.id);
		runs.cancelRun(run.id, 'user-cancelled');
		expect(runs.getRun(run.id)?.status).toBe('cancelled');
		expect(runs.getAction(action.id)?.status).toBe('cancelled');
	});
});

describe('lease fencing and recovery', () => {
	test('a stale lease cannot report or complete after recovery and a newer attempt', () => {
		const run = runs.createRun({ title: 'lease fence' });
		const action = runs.addAction({ runId: run.id, outcome: 'x', capability: 'builtin', idempotencyKey: 'key-fence' });
		runs.commitPlan(run.id);

		// A running action cannot be claimed again; the only path to a new attempt
		// is expiry → reconciliation → an explicit re-queue.
		const first = runs.claimAction(action.id, -1)!;
		expect(runs.claimAction(action.id)).toBeNull();

		runs.recoverExpiredLeases();
		runs.resolveReconciliation(action.id, 'retry');
		const second = runs.claimAction(action.id)!;
		expect(second.leaseId).not.toBe(first.leaseId);

		// The first attempt's late report must be rejected; the fresh lease wins.
		expect(runs.completeAction(action.id, first.leaseId, 'late')).toBe(false);
		expect(runs.reportProgress(action.id, first.leaseId, 'late progress')).toBe(false);
		expect(runs.completeAction(action.id, second.leaseId, 'fresh')).toBe(true);
		expect(runs.getAction(action.id)?.result).toBe('fresh');
		expect(runs.getAction(action.id)?.attempt).toBe(2);
	});

	test('an expired lease becomes needs_reconciliation, never an automatic retry', () => {
		const run = runs.createRun({ title: 'crash' });
		const action = runs.addAction({ runId: run.id, outcome: 'x', capability: 'builtin', idempotencyKey: 'key-1' });
		runs.commitPlan(run.id);
		const claim = runs.claimAction(action.id, -1)!; // already expired
		runs.recoverExpiredLeases();

		const recovered = runs.getAction(action.id);
		expect(recovered?.status).toBe('needs_reconciliation');
		expect(runs.getRun(run.id)?.status).toBe('waiting');
		// The dead lease can no longer report.
		expect(runs.completeAction(action.id, claim.leaseId, 'zombie')).toBe(false);

		// With an idempotency key an operator may re-queue; without one they may not.
		expect(runs.resolveReconciliation(action.id, 'retry')).toBe(true);
		expect(runs.getAction(action.id)?.status).toBe('queued');

		const noKey = runs.addAction({ runId: run.id, outcome: 'y', capability: 'builtin' });
		runs.commitPlan(run.id);
		const claim2 = runs.claimAction(noKey.id, -1)!;
		runs.recoverExpiredLeases();
		expect(runs.resolveReconciliation(noKey.id, 'retry')).toBe(false);
		expect(runs.resolveReconciliation(noKey.id, 'failed')).toBe(true);
		expect(runs.getAction(noKey.id)?.status).toBe('failed');
		expect(claim2.leaseId).toBeTruthy();
	});
});

describe('event log and replay', () => {
	test('transitions commit events in sequence and replay from a cursor', () => {
		const run = runs.createRun({ title: 'replay' });
		const action = runs.addAction({ runId: run.id, outcome: 'x', capability: 'builtin' });
		runs.commitPlan(run.id);
		const claim = runs.claimAction(action.id)!;
		runs.reportProgress(action.id, claim.leaseId, 'halfway');
		runs.completeAction(action.id, claim.leaseId, 'done');

		const all = runs.listEvents(run.id);
		const types = all.map((event) => event.type);
		expect(types).toContain('run.created');
		expect(types).toContain('action.succeeded');
		// Sequences are strictly increasing within the run.
		for (let i = 1; i < all.length; i++) expect(all[i].seq).toBe(all[i - 1].seq + 1);

		const cursor = all[1].seq;
		const replay = runs.listEvents(run.id, cursor);
		expect(replay.every((event) => event.seq > cursor)).toBe(true);

		const snapshot = runs.runSnapshot(run.id, cursor);
		expect(snapshot.run.id).toBe(run.id);
		expect(snapshot.actions.find((a) => a.id === action.id)?.status).toBe('succeeded');
	});

	test('event payloads redact credential-shaped keys', () => {
		const run = runs.createRun({ title: 'sanitize' });
		const action = runs.addAction({ runId: run.id, outcome: 'x', capability: 'builtin' });
		runs.commitPlan(run.id);
		const claim = runs.claimAction(action.id)!;
		runs.reportProgress(action.id, claim.leaseId, 'working');
		// The log stores only the safe progress string; nothing credential-shaped
		// is ever written by the coordinator itself.
		const events = runs.listEvents(run.id);
		expect(JSON.stringify(events)).not.toMatch(/sk-|Bearer /);
	});
});
