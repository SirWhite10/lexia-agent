import { error } from '@sveltejs/kit';
import { getRun, listEvents, runSnapshot } from '#lib/server/runs.js';
import type { RequestHandler } from './$types';

// SSE replay for one run (WF-IMP-001, ADR 0002). SQLite is authoritative: the
// client reconnects with Last-Event-ID and the stream replays everything after
// that per-run sequence, so a dropped browser never loses or duplicates run
// history. The database remains the source of truth; this transport is only a
// delivery optimization.
const TERMINAL_RUN = new Set(['succeeded', 'failed', 'cancelled', 'partially_complete']);
const POLL_INTERVAL_MS = 400;

export const GET: RequestHandler = ({ params, request, locals }) => {
	if (!locals.user) error(401, 'Sign in to stream runs.');
	if (!getRun(params.id)) error(404, 'Not found');

	// A reconnecting client presents the last sequence it saw and must receive
	// every event after it. A fresh client sends no cursor, starts at the
	// current head, and takes the snapshot as its baseline instead.
	const requested = Number(request.headers.get('last-event-id') ?? new URL(request.url).searchParams.get('after') ?? 0);
	const afterSeq = Number.isFinite(requested) ? requested : 0;
	const snapshot = runSnapshot(params.id, afterSeq);
	const replayFrom = afterSeq > 0 ? afterSeq : snapshot.latestSeq;

	const encoder = new TextEncoder();
	let closed = false;
	let poll: ReturnType<typeof globalThis.setTimeout> | undefined;

	const stream = new ReadableStream<Uint8Array>({
		start(controller) {
			let cursor = replayFrom;

			const send = (type: string, data: unknown, seq?: number) => {
				if (closed) return;
				const frame = (seq === undefined ? '' : `id: ${seq}\n`) + `event: ${type}\ndata: ${JSON.stringify(data)}\n\n`;
				controller.enqueue(encoder.encode(frame));
			};

			send('snapshot', snapshot);

			const drain = () => {
				if (closed) return;
				for (const event of listEvents(params.id, cursor)) {
					cursor = event.seq;
					send(event.type, event, event.seq);
				}
				if (TERMINAL_RUN.has(runSnapshot(params.id, cursor).run.status)) {
					send('run.terminal', { latestSeq: cursor });
					closed = true;
					controller.close();
					return;
				}
				poll = setTimeout(drain, POLL_INTERVAL_MS);
			};
			poll = setTimeout(drain, POLL_INTERVAL_MS);

			request.signal.addEventListener('abort', () => {
				closed = true;
				if (poll) clearTimeout(poll);
			});
		},
		cancel() {
			closed = true;
			if (poll) clearTimeout(poll);
		}
	});

	return new Response(stream, {
		headers: {
			'Content-Type': 'text/event-stream',
			'Cache-Control': 'no-store',
			Connection: 'keep-alive'
		}
	});
};
