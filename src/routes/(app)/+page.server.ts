import { fail, redirect } from '@sveltejs/kit';
import {
	attachmentKind,
	linkAttachmentsToMessage,
	listAttachments,
	saveAttachment,
	type Attachment
} from '#lib/server/attachments.js';
import { appendMessage, createSubAgent, listMessages } from '#lib/server/agents.js';
import { bootstrapStatus } from '#lib/server/bootstrap.js';
import { dispatchQueuedActions } from '#lib/server/builtins.js';
import { EveError, eveReachable, sendToEve } from '#lib/server/eve.js';
import { planTurn } from '#lib/server/planner.js';
import {
	addAction,
	cancelRun,
	claimAction,
	commitPlan,
	completeAction,
	createRun,
	failAction,
	getRun,
	listActions,
	listRuns
} from '#lib/server/runs.js';
import { transcribePendingAudio } from '#lib/server/transcription.js';
import { captureTurn, maybeReflect, prepareTurn } from '#lib/server/turn.js';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async () => {
	const messages = listMessages();
	return {
		messages,
		// Attachments ride along with the messages they were sent with, so the
		// chat renders chips without a second request.
		attachments: listAttachments(messages.map((message) => message.id)),
		// Runs are durable (ADR 0002), so a refresh keeps the work the agent did
		// instead of a chat that has forgotten it. `createdAt` is what puts each
		// one back in the order it happened.
		runs: listRuns(20).map((run) => ({
			id: run.id,
			messageId: run.messageId,
			status: run.status,
			createdAt: run.createdAt,
			actions: listActions(run.id).map((action) => ({
				id: action.id,
				capability: action.capability,
				outcome: action.outcome,
				status: action.status,
				progress: action.progress,
				error: action.error
			}))
		})),
		bootstrapped: bootstrapStatus().bootstrapped,
		// Probed on load so the composer can say why nothing is answering instead
		// of letting every turn look like a silent no-op.
		eveUp: await eveReachable()
	};
};

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
		const attachmentIds = JSON.parse(String(form.get('attachmentIds') ?? '[]')) as string[];
		if (!body && attachmentIds.length === 0) return fail(400, { error: 'Say something, or attach something to look at.' });

		const message = appendMessage({
			authorKind: 'user',
			body: body || '(no text — see attachments)'
		});
		linkAttachmentsToMessage(attachmentIds, message.id);

		// Voice notes are transcribed in the background: a provider round trip must
		// not hold the reply open, and the transcript lands on the attachment row
	// for the next render to pick up.
		if ((listAttachments([message.id])[message.id] ?? []).some((attachment) => attachment.kind === 'audio')) {
			void transcribePendingAudio();
		}

		// Capture first so the turn is retrievable immediately, then build the
		// prompt from whatever memory already applies to it.
		await captureTurn(message);
		const turn = await prepareTurn({ query: message.body });

		const run = createRun({ title: message.body, messageId: message.id });
		// The reply is the point of the turn: hand the message to EVE, read the
		// answer off its stream, and record it as an assistant message. A failure
		// here is a real failure, not a silent no-op.
		// The planner decides the graph behind this turn. Today the only intent it
		// recognises is delegation, and it does so before the reply is claimed, so
		// the sub-agent exists by the time the user sees the run.
		const plan = planTurn({ body: message.body });

		const reply = addAction({
			runId: run.id,
			outcome: 'Reply to the request',
			capability: 'chat.reply',
			executionMode: 'immediate'
		});

		if (plan.subagent) {
			const subAgent = createSubAgent({
				name: plan.subagent.name,
				focus: plan.subagent.focus,
				systemPrompt: plan.subagent.systemPrompt,
				capabilities: 'chat.reply'
			});
			addAction({
				runId: run.id,
				outcome: `Delegate to the ${subAgent.name}`,
				capability: 'subagent.delegate',
				executionMode: 'immediate',
				dependsOn: [reply.id],
				// Seeded now so the capability knows which role card to run and what
				// for, without the run graph having to carry a second channel.
				input: JSON.stringify({ subAgentId: subAgent.id, task: plan.subagent.task })
			});
		}

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
				// The agent is told what came with the turn rather than handed the
				// bytes: reading attachment contents into the prompt is its own
				// decision, not something to smuggle in through the chat turn.
				const attached = listAttachments([message.id])[message.id] ?? [];
				const prompt =
					attached.length === 0
						? message.body
						: `${message.body}\n\nAttached to this message: ${attached
								.map((attachment) => `${attachment.name} (${attachment.kind}, ${attachment.size} bytes)`)
								.join(', ')}`;
				const answer = await sendToEve(prompt);
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

	/** One upload action for both picked files and recorded voice notes: bytes go
	 * to the state directory first, and the chat links them to a message when
	 * that message is sent. Transcription is a later pass over the stored audio. */
	attach: async ({ request }) => {
		const form = await request.formData();
		const saved: Attachment[] = [];

		for (const entry of form.getAll('files')) {
			if (!(entry instanceof File) || entry.size === 0) continue;
			try {
				saved.push(
					saveAttachment({
						name: entry.name,
						mimeType: entry.type,
						bytes: await entry.arrayBuffer(),
						kind: attachmentKind(entry.name, entry.type)
					})
				);
			} catch (error) {
				return fail(413, {
					error: error instanceof Error ? error.message : 'That file could not be attached.'
				});
			}
		}

		if (saved.length === 0) return fail(400, { error: 'No file arrived with that upload.' });
		return { attachments: saved };
	},

	cancel: async ({ request }) => {
		const form = await request.formData();
		const runId = String(form.get('runId') ?? '');
		if (!getRun(runId)) return fail(404, { error: 'That run no longer exists.' });
		cancelRun(runId, 'user-cancelled');
		return { cancelled: true };
	}
};
