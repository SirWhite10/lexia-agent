import { fail, redirect } from '@sveltejs/kit';
import { appendMessage, listMessages } from '#lib/server/agents.js';
import { bootstrapStatus } from '#lib/server/bootstrap.js';
import { dispatchQueuedActions } from '#lib/server/builtins.js';
import { EveError, eveReachable, sendToEve } from '#lib/server/eve.js';
import { addAction, cancelRun, claimAction, commitPlan, completeAction, createRun, failAction, getRun } from '#lib/server/runs.js';
import { captureTurn, maybeReflect, prepareTurn } from '#lib/server/turn.js';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async () => ({
	messages: listMessages(),
	bootstrapped: bootstrapStatus().bootstrapped,
	// Probed on load so the composer can say why nothing is answering instead
	// of letting every turn look like a silent no-op.
	eveUp: await eveReachable()
});

/**
 * One user turn becomes one run (ADR 0002). The vertical slice plans a single
 * immediate action plus a background action so a run can demonstrate progress
 * and partial completion; the planner that derives this graph from intent
 * arrives with its own slice. The turn is captured into memory and its prompt
 * assembled on the way in, which is the capture → retrieve → inject loop.
 */
export const actions: Actions = {
	send: async ({ request }) => {
		const form = await request.formData();
		const body = String(form.get('body') ?? '').trim();
		if (!body) return fail(400, { error: 'Say something first.' });

		const message = appendMessage({ authorKind: 'user', body });
		// Capture first so the turn is retrievable immediately, then build the
		// prompt from whatever memory already applies to it.
		await captureTurn(message);
		const turn = await prepareTurn({ query: message.body });

		const run = createRun({ title: message.body, messageId: message.id });
		// The reply is the point of the turn: hand the message to EVE, read the
		// answer off its stream, and record it as an assistant message. A failure
		// here is a real failure, not a silent no-op.
		const reply = addAction({
			runId: run.id,
			outcome: 'Reply to the request',
			capability: 'chat.reply',
			executionMode: 'immediate'
		});
		addAction({
			runId: run.id,
			outcome: 'Record the request as a note',
			capability: 'note.write',
			executionMode: 'background',
			dependsOn: [reply.id]
		});
		commitPlan(run.id);

		const lease = claimAction(reply.id, 120_000);
		if (lease) {
			try {
				const answer = await sendToEve(message.body);
				if (answer.failed || !answer.reply) {
					failAction(reply.id, lease.leaseId, answer.reply || 'EVE returned no reply.');
				} else {
					appendMessage({ authorKind: 'lexia', body: answer.reply, runId: run.id });
					completeAction(reply.id, lease.leaseId, answer.reply);
				}
			} catch (error) {
				failAction(reply.id, lease.leaseId, error instanceof Error ? error.message : 'EVE is unreachable.');
			}
		}
		await dispatchQueuedActions(run.id, claimAction);
		// Reflection is a boundary effect: it may distil facts and traits, but it
		// never decides whether this turn succeeded.
		await maybeReflect();
		return { runId: run.id, messageId: message.id, memoryHits: turn.memory.length, degraded: turn.degraded };
	},

	cancel: async ({ request }) => {
		const form = await request.formData();
		const runId = String(form.get('runId') ?? '');
		if (!getRun(runId)) return fail(404, { error: 'That run no longer exists.' });
		cancelRun(runId, 'user-cancelled');
		return { cancelled: true };
	}
};
