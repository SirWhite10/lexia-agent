import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { appendMessage, getSubAgent } from './agents.js';
import { sendToEve } from './eve.js';
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
	},

	/**
	 * Delegation: the planner already decided this request belongs to a
	 * sub-agent and created its role card, so this capability only has to run the
	 * job and get the answer back into the conversation.
	 */
	'subagent.delegate': {
		capability: 'subagent.delegate',
		describe: (action) => action.outcome,
		execute: async ({ action, input }) => {
			const plan = parseDelegation(input);
			const subAgent = getSubAgent(plan.subAgentId);
			if (!subAgent) throw new Error('That sub-agent no longer exists.');

			const finding = await sendToEve([subAgent.currentPrompt ?? '', plan.task].filter(Boolean).join('\n\n'));
			if (finding.failed || !finding.reply) throw new Error(finding.reply || 'The sub-agent returned nothing.');

			// The finding is a message of its own, not just action output: the main
			// agent reads the conversation, so that is where it has to live.
			appendMessage({ authorKind: 'sub_agent', authorId: subAgent.id, runId: action.runId, body: finding.reply });

			// Hand it straight back so the user gets an answer in this turn instead
			// of having to come back and ask whether it finished.
			const report = await sendToEve(
				[
					`A sub-agent you delegated (${subAgent.name}) reported back with:\n\n${finding.reply}`,
					'Reply to the user now, in your own voice, using what it found. Say plainly if it is inconclusive.'
				].join('\n\n')
			);
			if (!report.failed && report.reply) {
				appendMessage({ authorKind: 'lexia', runId: action.runId, body: report.reply });
			}

			return { progress: ['delegated', 'working', 'reported back'], result: `${subAgent.name}: ${finding.reply}` };
		}
	}
};

/** The delegation payload the planner seeded onto the action. A malformed one
 * is a planner bug, and saying so beats failing the run silently. */
function parseDelegation(input: string | null): { subAgentId: string; task: string } {
	if (!input) throw new Error('This delegation carries no plan.');
	let parsed: unknown;
	try {
		parsed = JSON.parse(input);
	} catch {
		throw new Error('This delegation carries an unreadable plan.');
	}
	const plan = parsed as { subAgentId?: unknown; task?: unknown };
	if (typeof plan.subAgentId !== 'string' || typeof plan.task !== 'string') {
		throw new Error('This delegation carries an incomplete plan.');
	}
	return { subAgentId: plan.subAgentId, task: plan.task };
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
