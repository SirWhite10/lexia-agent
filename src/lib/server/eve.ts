// Server-side client for the EVE dev server's HTTP API. The host never runs a
// model itself: it hands a turn to EVE, reads the reply off the session
// stream, and records it as a chat message. Credentials stay inside EVE.
//
// The base URL is configuration because the EVE process is external to the
// stable host (ADR 0001); in development `eve dev` serves it on 2000.
const EVE_BASE_URL = (process.env.LEXIA_EVE_URL ?? 'http://127.0.0.1:2000').replace(/\/$/, '');
const SESSION_TTL_MS = 60_000;

export type EveReply = {
	reply: string;
	reasoning: string | null;
	/** True when EVE answered with an error rather than text. */
	failed: boolean;
};

export class EveError extends Error {
	constructor(message: string) {
		super(message);
		this.name = 'EveError';
	}
}

async function eveFetch(path: string, init: RequestInit = {}): Promise<Response> {
	try {
		return await fetch(`${EVE_BASE_URL}${path}`, { signal: AbortSignal.timeout(SESSION_TTL_MS), ...init });
	} catch (error) {
		if (error instanceof DOMException && error.name === 'TimeoutError') {
			throw new EveError('EVE did not respond within 60 seconds.');
		}
		throw new EveError(`Could not reach EVE at ${EVE_BASE_URL}. Is \`eve dev\` running?`);
	}
}

/** True when an EVE server answers, so the UI can say so before the user types. */
export async function eveReachable(): Promise<boolean> {
	try {
		const response = await fetch(`${EVE_BASE_URL}/`, { signal: AbortSignal.timeout(2_000) });
		return response.status > 0;
	} catch {
		return false;
	}
}

async function createSession(): Promise<string> {
	const response = await eveFetch('/eve/v1/session', {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: '{}'
	});
	const payload = (await response.json()) as { sessionId?: string; ok?: boolean };
	if (!response.ok || !payload.sessionId) {
		throw new EveError('EVE did not return a session id.');
	}
	return payload.sessionId;
}

type EveEvent = { type?: string; data?: Record<string, unknown> };

/**
 * Sends one user turn to EVE and resolves with its reply. The stream is
 * attached before the message, exactly as the EVE API expects: the reply
 * arrives as `message.completed` on that stream, not in the POST response.
 */
export async function sendToEve(message: string, existingSessionId?: string | null): Promise<EveReply & { sessionId: string }> {
	const sessionId = existingSessionId ?? (await createSession());

	const streamResponse = await eveFetch(`/eve/v1/session/${sessionId}/stream`);
	if (!streamResponse.body) throw new EveError('EVE returned no event stream.');

	const reader = streamResponse.body.getReader();
	const decoder = new TextDecoder();
	let buffer = '';
	let reply = '';
	let reasoning: string | null = null;
	let failure: string | null = null;

	// Read until the turn reaches a terminal event, then stop reading.
	const readUntilTerminal = (async () => {
		try {
			for (;;) {
				const { done, value } = await reader.read();
				if (done) break;
				buffer += decoder.decode(value, { stream: true });
				const lines = buffer.split('\n');
				buffer = lines.pop() ?? '';
				for (const line of lines) {
					if (!line.trim()) continue;
					let event: EveEvent;
					try {
						event = JSON.parse(line) as EveEvent;
					} catch {
						continue;
					}
					const data = event.data ?? {};
					if (event.type === 'message.completed' && typeof data.message === 'string') {
						reply = data.message;
					}
					if (event.type === 'reasoning.completed' && typeof data.text === 'string') {
						reasoning = data.text;
					}
					if (event.type === 'turn.completed' || event.type === 'turn.failed' || event.type === 'turn.cancelled') {
						if (event.type === 'turn.failed') {
							const details = data.details as { message?: string } | undefined;
							failure = details?.message ?? 'EVE reported a failed turn.';
						}
						return;
					}
				}
			}
		} catch {
			// Stream closed without a terminal event; the reply check below decides.
		}
	})();

	await eveFetch(`/eve/v1/session/${sessionId}`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ message })
	});

	await readUntilTerminal;
	// Release the stream so EVE is not left holding an open reader.
	await reader.cancel().catch(() => {});

	if (failure) return { reply, reasoning, failed: true, sessionId };
	return { reply: reply.trim(), reasoning, failed: false, sessionId };
}
