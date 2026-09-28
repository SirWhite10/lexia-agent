import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { completeAction, failAction, listActions, reportProgress, renewLease } from './runs.js';
import type { Action } from './runs.js';

// Deterministic builtin capabilities for the vertical slice (WF-IMP-001). Real
// integrations (Gmail, Calendar, Home Assistant, cards) arrive with their own
// slices; the coordinator, lease, and event machinery is identical for all of
// them, so the slice proves the execution contract without coupling to a
// provider. Every builtin is a real local side effect, not a stub: `note.write`
// appends a file under the state directory, and `chat.ack` records a reply.
const stateDir = resolve(process.env.LEXIA_STATE_DIR ?? join(process.cwd(), 'app-data/state'));
const notesDir = join(stateDir, 'agent/notes');

export type BuiltinContext = { action: Action; input: string | null };

export type Builtin = {
	capability: string;
	/** Human sentence shown in the chat while this action runs. */
	describe: (action: Action) => string;
	execute: (context: BuiltinContext) => Promise<{ progress?: string[]; result: string }>;
};

const BUILTINS: Record<string, Builtin> = {
	'note.write': {
		capability: 'note.write',
		describe: (action) => action.outcome,
		execute: async ({ action, input }) => {
			mkdirSync(notesDir, { recursive: true });
			const file = join(notesDir, `${action.id}.md`);
			writeFileSync(file, `# ${action.outcome}\n\n${input ?? ''}\n`, 'utf8');
			return { result: `note.write completed for action ${action.id}` };
		}
	},
	'chat.ack': {
		capability: 'chat.ack',
		describe: (action) => action.outcome,
		execute: async ({ action, input }) => ({
			progress: ['acknowledged', 'recorded'],
			result: `chat.ack recorded for action ${action.id}${input ? `: ${input}` : ''}`
		})
	}
};

export function builtinFor(capability: string): Builtin | undefined {
	return BUILTINS[capability];
}

export type DispatchReport = { executed: string[]; failed: string[]; skipped: string[] };

/**
 * Claims every queued action this process can execute and runs it under a
 * renewable lease. Failures are recorded on the action, never thrown into the
 * void: an unhandled worker error must not erase a run. Actions for
 * capabilities with no builtin are left queued for a real worker to claim.
 */
export async function dispatchQueuedActions(
	runId: string,
	claim: (actionId: string, ttl?: number) => { action: Action; leaseId: string } | null
): Promise<DispatchReport> {
	const report: DispatchReport = { executed: [], failed: [], skipped: [] };
	// Completing an action can unblock a dependent into `queued` after this pass
	// has already visited it, so keep sweeping until a pass executes nothing.
	// Each sweep is bounded by the run's remaining actions; a capability with no
	// builtin is left queued for a real worker and stops the loop naturally.
	for (;;) {
		let executedThisPass = 0;
		for (const action of listActions(runId)) {
			if (action.status !== 'queued') continue;
			const builtin = builtinFor(action.capability);
			if (!builtin) {
				report.skipped.push(action.id);
				continue;
			}
			const lease = claim(action.id);
			if (!lease) {
				report.skipped.push(action.id);
				continue;
			}
			try {
				// Renew once so a slow builtin never trips its own lease. Execute
				// exactly once: these builtins have real side effects.
				renewLease(action.id, lease.leaseId, 60_000);
				const outcome = await builtin.execute({ action: lease.action, input: action.result });
				for (const step of outcome.progress ?? []) {
					reportProgress(action.id, lease.leaseId, step);
				}
				completeAction(action.id, lease.leaseId, outcome.result);
				report.executed.push(action.id);
				executedThisPass++;
			} catch (error) {
				failAction(action.id, lease.leaseId, error instanceof Error ? error.message : 'builtin failed');
				report.failed.push(action.id);
			}
		}
		if (executedThisPass === 0) return report;
	}
}
