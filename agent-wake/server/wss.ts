import { verifyHostToken } from './host-token';
import { commandForKeyword } from './agent-phrases';
import { getSession } from './auth';
import { DetectorWorker } from './worker';
import {
	AUDIO_IDLE_TIMEOUT_MS,
	AUDIO_PROTOCOL_VERSION,
	MAX_AUDIO_FRAME_BYTES,
	MAX_AUDIO_QUEUE_MS,
	audioError,
	configuredOrigins,
	isAllowedOrigin,
	jsonMessage,
	newSocketData,
	parseAudioControl,
	validateAudioFrame,
	type AudioSocketData
} from './audio';


export type WSServerOptions = {
	worker: DetectorWorker;
};

export function createWebSocketHandler({ worker }: WSServerOptions) {
	let activeSession: string | null = null;
	const sockets = new Map<string, Bun.ServerWebSocket<AudioSocketData>>();
	const idleTimers = new Map<string, ReturnType<typeof setInterval>>();

	worker.onEvent((event) => {
		const sessionId = typeof event.sessionId === 'string' ? event.sessionId : null;
		if (!sessionId) return;
		const ws = sockets.get(sessionId);
		if (!ws || ws.data.state === 'closed') return;
		if (event.type === 'stats') {
			ws.send(jsonMessage(toStatsEvent(event)));
			return;
		}
		if (event.type === 'wake') {
			const keyword = typeof event.keyword === 'string' ? event.keyword : 'wake word';
			ws.send(jsonMessage({
				type: 'wake',
				sessionId,
				keyword,
				// The command the agent's phrase declares. Null when the keyword
				// is not one the agent answers to: the detection is still
				// reported, because a detection nobody mapped is information.
				command: commandForKeyword(keyword),
				timestamp: typeof event.timestamp === 'number' ? event.timestamp : Date.now(),
				threshold: worker.threshold()
			}));
			return;
		}
		if (event.type === 'status') {
			ws.send(jsonMessage({
				type: 'status',
				sessionId,
				state: typeof event.state === 'string' ? event.state : undefined,
				model: 'sherpa-onnx',
				threshold: worker.threshold(),
				stats: event.stats
			}));
			return;
		}
		if (event.type === 'error') {
			const details = event.error && typeof event.error === 'object' ? event.error as Record<string, unknown> : {};
			const code = typeof details.code === 'string' ? details.code : 'worker_error';
			const message = typeof details.message === 'string' ? details.message : 'The detector worker reported an error.';
			ws.send(jsonMessage({ type: 'error', code, message, retryable: details.retryable === true }));
			if (code === 'sequence_gap' || code === 'invalid_pcm' || code === 'session_not_found') {
				ws.data.state = 'closed';
				ws.close(1008, code);
			}
		}
	});

	async function closeWithError(ws: Bun.ServerWebSocket<AudioSocketData>, code: string, message: string, status = 1008): Promise<void> {
		if (ws.data.state !== 'closed') {
			if (ws.data.state === 'streaming' || ws.data.state === 'ready') void worker.closeSession(ws.data.sessionId).catch(() => undefined);
			if (activeSession === ws.data.sessionId) activeSession = null;
			ws.data.state = 'closed';
		}
		ws.send(jsonMessage(audioError(code, message)));
		ws.close(status, code);
	}

	async function handleMessage(ws: Bun.ServerWebSocket<AudioSocketData>, message: string | Buffer<ArrayBuffer>): Promise<void> {
		const data = ws.data;
		if (data.state === 'closed') return;
		if (typeof message !== 'string') {
			if (data.state !== 'streaming' || data.sampleRate === undefined) {
				await closeWithError(ws, 'invalid_state', 'Start metadata must be the first audio message.');
				return;
			}
			if (message.byteLength > MAX_AUDIO_FRAME_BYTES) {
				await closeWithError(ws, 'frame_too_large', 'Audio frame exceeds the 64 KiB limit.', 1009);
				return;
			}
			const samples = validateAudioFrame(new Uint8Array(message.buffer, message.byteOffset, message.byteLength), data.sampleRate);
			if (!samples) {
				await closeWithError(ws, 'invalid_frame', 'Audio frames must be finite mono float32 PCM of 20–40 ms.');
				return;
			}
			try {
				await worker.sendPcm(data.sessionId, samples);
				data.lastFrameAt = Date.now();
			} catch (error) {
				await closeWithError(ws, 'backpressure', error instanceof Error ? error.message : 'The detector is not accepting audio.');
			}
			return;
		}

		const control = parseAudioControl(message);
		if (!control) {
			await closeWithError(ws, 'invalid_control', 'Expected a valid start or stop control message.');
			return;
		}
		if (control.type === 'start') {
			if (data.state !== 'connected') {
				await closeWithError(ws, 'invalid_state', 'This audio session has already been started.');
				return;
			}
			if (activeSession !== null) {
				await closeWithError(ws, 'session_busy', 'Another microphone stream is already active.', 1013);
				return;
			}
			if (!worker.isReady()) {
				await closeWithError(ws, 'model_unavailable', 'The detector model is not ready.', 1013);
				return;
			}
			activeSession = data.sessionId;
			try {
				await worker.openSession(data.sessionId, control.sampleRate);
				if (data.state === 'closed') {
					// The socket failed or closed mid-handshake. closeWithError already released
					// the claim; release the worker session so the supervisor does not retain it.
					void worker.closeSession(data.sessionId).catch(() => undefined);
					return;
				}
				data.sampleRate = control.sampleRate;
				data.state = 'streaming';
				ws.send(jsonMessage({
					type: 'ready',
					protocolVersion: AUDIO_PROTOCOL_VERSION,
					sessionId: data.sessionId,
					sampleRate: control.sampleRate,
					channels: 1,
					encoding: 'pcm_f32le',
					threshold: worker.threshold(),
					maxFrameBytes: MAX_AUDIO_FRAME_BYTES,
					maxQueueMs: MAX_AUDIO_QUEUE_MS
				}));
			} catch (error) {
				if (activeSession === data.sessionId) activeSession = null;
				await closeWithError(ws, 'session_start_failed', error instanceof Error ? error.message : 'Could not start the detector session.');
			}
			return;
		}
		if (data.state !== 'streaming') {
			await closeWithError(ws, 'invalid_state', 'No active audio session can be stopped.');
			return;
		}
		data.state = 'stopping';
		try {
			const result = await worker.closeSession(data.sessionId);
			ws.send(jsonMessage({ type: 'stopped', sessionId: data.sessionId, stats: result.stats ?? null }));
		} catch (error) {
			ws.send(jsonMessage(audioError('session_close_failed', error instanceof Error ? error.message : 'Could not close the detector session.')));
		} finally {
			if (activeSession === data.sessionId) activeSession = null;
			data.state = 'closed';
			ws.close(1000, 'stopped');
		}
	}

	function handleOpen(ws: Bun.ServerWebSocket<AudioSocketData>): void {
		ws.data.state = 'connected';
		ws.data.lastFrameAt = Date.now();
		sockets.set(ws.data.sessionId, ws);
		const timer = setInterval(() => {
			if (ws.data.state !== 'streaming' || Date.now() - ws.data.lastFrameAt < AUDIO_IDLE_TIMEOUT_MS) return;
			void closeWithError(ws, 'idle_timeout', 'No microphone frame arrived before the idle timeout.');
		}, 1_000);
		idleTimers.set(ws.data.sessionId, timer);
		ws.send(jsonMessage({ type: 'status', state: 'connected', protocolVersion: AUDIO_PROTOCOL_VERSION }));
	}

	function handleClose(ws: Bun.ServerWebSocket<AudioSocketData>): void {
		clearInterval(idleTimers.get(ws.data.sessionId));
		idleTimers.delete(ws.data.sessionId);
		sockets.delete(ws.data.sessionId);
		if (activeSession === ws.data.sessionId) {
			activeSession = null;
			void worker.closeSession(ws.data.sessionId).catch(() => undefined);
		}
		ws.data.state = 'closed';
	}

	return {
		data: {} as AudioSocketData,
		open: handleOpen,
		message: handleMessage,
		close: handleClose,
		maxPayloadLength: MAX_AUDIO_FRAME_BYTES,
		backpressureLimit: MAX_AUDIO_FRAME_BYTES,
		closeOnBackpressureLimit: true,
		idleTimeout: 0,
		perMessageDeflate: false,
		async fetch(request: Request, server: Bun.Server<AudioSocketData>): Promise<Response | undefined> {
			const url = new URL(request.url);
			if (url.pathname !== '/ws/audio') return undefined;
			// Two ways in. The service's own session cookie, for a client that
			// logged in here; or a token the agent host signed, for the host's
			// own page. The host's cookie is scoped to the host's origin and
			// never reaches this process, so the token is how the page connects.
			const hostUser = verifyHostToken(url.searchParams.get('token'));
			// The origin allowlist guards the cookie path: a cookie is ambient,
			// so any page the user visits could present it. A host token is
			// not: it is signed for one user, worth a minute, and only the
			// host can mint one. A cross-site page cannot obtain it, so
			// refusing a token-carrying request on its origin would keep the
			// host's own page from connecting without closing anything.
			if (!hostUser && !isAllowedOrigin(request.headers.get('origin'), configuredOrigins())) {
				return new Response(JSON.stringify(audioError('origin_forbidden', 'Origin is not allowed.')), { status: 403, headers: { 'content-type': 'application/json' } });
			}
			const session = hostUser ? null : getSession(request);
			if (!session && !hostUser) return new Response(JSON.stringify(audioError('unauthorized', 'Sign in before opening an audio stream.')), { status: 401, headers: { 'content-type': 'application/json' } });
			const identity = session?.username ?? hostUser!.id;
			const upgraded = server.upgrade(request, { data: newSocketData(identity) });
			return upgraded ? undefined : new Response(JSON.stringify(audioError('upgrade_failed', 'WebSocket upgrade failed.')), { status: 400, headers: { 'content-type': 'application/json' } });
		}
	};
}

function toStatsEvent(event: Record<string, unknown>): Record<string, unknown> {
	const stats = event.stats && typeof event.stats === 'object' ? event.stats as Record<string, unknown> : {};
	const sampleCount = typeof stats.sample_count === 'number' ? stats.sample_count : 0;
	const sampleRate = typeof stats.sample_rate === 'number' ? stats.sample_rate : 0;
	return {
		type: 'stats',
		sessionId: event.sessionId,
		framesReceived: typeof stats.frame_count === 'number' ? stats.frame_count : 0,
		bytesReceived: sampleCount * Float32Array.BYTES_PER_ELEMENT,
		durationMs: typeof stats.duration_ms === 'number' ? stats.duration_ms : 0,
		queueDepth: 0,
		queueCapacity: Math.floor(sampleRate * Float32Array.BYTES_PER_ELEMENT * (MAX_AUDIO_QUEUE_MS / 1000)),
		droppedFrames: 0,
		rms: typeof stats.rms === 'number' ? stats.rms : 0,
		peak: typeof stats.peak === 'number' ? stats.peak : 0
	};
}
