import { resolve } from 'node:path';
import { keywordsFilePath, modelDirPath } from './keywords';

const MAX_FRAME_BYTES = 1024 * 1024;
const MAX_PCM_BYTES = 64 * 1024;
const PROTOCOL_VERSION = 1;
const STARTUP_TIMEOUT_MS = 30_000;
const SHUTDOWN_TIMEOUT_MS = 2_000;

type WorkerJson = Record<string, unknown> & { type?: string; requestId?: string };
type PendingRequest = {
	resolve: (value: WorkerJson) => void;
	reject: (error: Error) => void;
	timer: ReturnType<typeof setTimeout>;
};
type WorkerListener = (event: WorkerJson) => void;

type SessionState = {
	sequence: bigint;
	queuedBytes: number;
};

export type WorkerHealth = {
	ready: boolean;
	state: 'starting' | 'ready' | 'failed' | 'stopped';
	threshold: number;
	message?: string;
};

export class DetectorWorker {
	private process: Bun.Subprocess<'pipe', 'pipe', 'pipe'> | null = null;
	private receiveBuffer = new Uint8Array(0);
	private writeChain: Promise<void> = Promise.resolve();
	private readonly pending = new Map<string, PendingRequest>();
	private readonly listeners = new Set<WorkerListener>();
	private readonly sessions = new Map<string, SessionState>();
	private readyState: WorkerHealth = { ready: false, state: 'starting', threshold: 0.25 };
	private thresholdValue = 0.25;
	private startup: Promise<void> | null = null;
	private failure: Error | null = null;

	async start(): Promise<void> {
		if (this.startup) return this.startup;
		this.startup = this.startInternal();
		return this.startup;
	}

	isReady(): boolean {
		return this.readyState.ready;
	}

	health(): WorkerHealth {
		return { ...this.readyState, threshold: this.thresholdValue };
	}

	threshold(): number {
		return this.thresholdValue;
	}

	onEvent(listener: WorkerListener): () => void {
		this.listeners.add(listener);
		return () => this.listeners.delete(listener);
	}

	async request(type: string, payload: Record<string, unknown> = {}, timeoutMs = 15_000): Promise<WorkerJson> {
		await this.start();
		if (!this.process || !this.readyState.ready) throw new Error(this.readyState.message ?? 'Detector worker is not ready');
		const requestId = crypto.randomUUID();
		const message: WorkerJson = { protocolVersion: PROTOCOL_VERSION, type, requestId, ...payload };
		return new Promise<WorkerJson>((resolveRequest, rejectRequest) => {
			const timer = setTimeout(() => {
				this.pending.delete(requestId);
				rejectRequest(new Error(`Worker request timed out: ${type}`));
			}, timeoutMs);
			this.pending.set(requestId, { resolve: resolveRequest, reject: rejectRequest, timer });
			this.enqueueControl(message).catch((error: unknown) => {
				const pending = this.pending.get(requestId);
				if (!pending) return;
				clearTimeout(pending.timer);
				this.pending.delete(requestId);
				pending.reject(error instanceof Error ? error : new Error(String(error)));
			});
		});
	}

	async sendPcm(sessionId: string, samples: Float32Array): Promise<void> {
		await this.start();
		if (!this.process || !this.readyState.ready) throw new Error('Detector worker is not ready');
		if (samples.length === 0 || samples.length * 4 > MAX_PCM_BYTES) throw new Error('PCM frame is empty or too large');
		for (const sample of samples) {
			if (!Number.isFinite(sample)) throw new Error('PCM frame contains a non-finite sample');
		}
		const state = this.sessions.get(sessionId);
		if (!state) throw new Error('Unknown detector session');
		if (state.queuedBytes + samples.length * 4 > MAX_PCM_BYTES * 4) throw new Error('Detector worker audio queue is full');
		const payload = new Uint8Array(24 + samples.length * 4);
		payload.set(sessionIdBytes(sessionId), 0);
		// Layout must match PcmPayload::parse in src/worker.rs: session id at 0..16,
		// big-endian sequence at 16..24, samples from 24.
		new DataView(payload.buffer).setBigUint64(16, state.sequence, false);
		state.sequence += 1n;
		state.queuedBytes += samples.length * 4;
		payload.set(new Uint8Array(samples.buffer, samples.byteOffset, samples.byteLength), 24);
		try {
			await this.enqueueFrame(2, payload, sessionId);
		} finally {
			state.queuedBytes = Math.max(0, state.queuedBytes - samples.length * 4);
		}
	}

	async openSession(sessionId: string, sampleRate: number): Promise<void> {
		if (!/^[0-9a-f-]{36}$/i.test(sessionId)) throw new Error('Invalid detector session id');
		if (!Number.isInteger(sampleRate) || sampleRate < 8_000 || sampleRate > 192_000) throw new Error('Invalid detector sample rate');
		this.sessions.set(sessionId, { sequence: 0n, queuedBytes: 0 });
		try {
			await this.request('open_session', { sessionId, sampleRate });
		} catch (error) {
			this.sessions.delete(sessionId);
			throw error;
		}
	}

	async closeSession(sessionId: string): Promise<WorkerJson> {
		try {
			return await this.request('close_session', { sessionId });
		} finally {
			this.sessions.delete(sessionId);
		}
	}

	async runFixture(fixtureId: string, path: string): Promise<WorkerJson> {
		return this.request('run_fixture', { fixtureId, path }, 60_000);
	}

	async stop(): Promise<void> {
		if (!this.process) return;
		try {
			await this.request('shutdown', {}, SHUTDOWN_TIMEOUT_MS);
		} catch {
			this.process.kill();
		}
		await Promise.race([this.process.exited, new Promise<void>((resolveExit) => setTimeout(resolveExit, SHUTDOWN_TIMEOUT_MS))]);
		if (!this.process.killed) this.process.kill();
		this.process = null;
		this.readyState = { ready: false, state: 'stopped', threshold: this.thresholdValue };
	}

	/** Restart the worker so it reloads the keyword file. Sessions do not survive. */
	async restart(): Promise<void> {
		await this.stop();
		this.startup = null;
		this.failure = null;
		this.receiveBuffer = new Uint8Array(0);
		this.sessions.clear();
		await this.start();
	}

	hasActiveSessions(): boolean {
		return this.sessions.size > 0;
	}

	private async startInternal(): Promise<void> {
		const executable = await this.findExecutable();
		this.process = Bun.spawn([executable], {
			cwd: process.cwd(),
			stdin: 'pipe',
			stdout: 'pipe',
			stderr: 'pipe',
			env: {
				...Bun.env,
				AGENT_WAKE_MODEL_DIR: modelDirPath(),
				AGENT_WAKE_KEYWORDS_FILE: keywordsFilePath()
			}
		});
		void this.readStdout(this.process);
		void this.readStderr(this.process);
		this.process.exited.then((code) => this.handleProcessExit(code));
		const requestId = crypto.randomUUID();
		const ready = new Promise<void>((resolveReady, rejectReady) => {
			const timer = setTimeout(() => rejectReady(new Error('Detector worker startup timed out')), STARTUP_TIMEOUT_MS);
			this.pending.set(requestId, {
				resolve: (message) => {
					if (typeof message.threshold === 'number' && Number.isFinite(message.threshold)) this.thresholdValue = message.threshold;
					clearTimeout(timer);
					if (message.type === 'error') rejectReady(new Error(String(message.message ?? 'Detector worker failed to start')));
					else resolveReady();
				},
				reject: (error) => {
					clearTimeout(timer);
					rejectReady(error);
				},
				timer
			});
		});
		await this.enqueueControl({ protocolVersion: PROTOCOL_VERSION, type: 'hello', requestId });
		try {
			await ready;
			this.readyState = { ready: true, state: 'ready', threshold: this.thresholdValue };
		} catch (error) {
			this.readyState = { ready: false, state: 'failed', threshold: this.thresholdValue, message: error instanceof Error ? error.message : String(error) };
			this.process.kill();
			throw error;
		}
	}

	private async findExecutable(): Promise<string> {
		// Only the dedicated worker binary speaks this protocol. The sibling
		// `agent-wake` is a microphone CLI and has no worker mode.
		const candidates = Bun.env.AGENT_WAKE_WORKER_BIN
			? [resolve(process.cwd(), Bun.env.AGENT_WAKE_WORKER_BIN)]
			: [resolve(process.cwd(), 'target/release/agent-wake-worker')];
		for (const candidate of candidates) {
			if (await Bun.file(candidate).exists()) return candidate;
		}
		throw new Error('Detector worker binary is missing; build the Rust worker first');
	}

	private enqueueControl(message: WorkerJson): Promise<void> {
		return this.enqueueFrame(1, new TextEncoder().encode(JSON.stringify(message)));
	}

	private enqueueFrame(kind: number, payload: Uint8Array, sessionId?: string): Promise<void> {
		const operation = this.writeChain.then(async () => {
			if (!this.process) throw new Error('Detector worker is not running');
			const frame = new Uint8Array(4 + 1 + payload.length);
			new DataView(frame.buffer).setUint32(0, payload.length, false);
			frame[4] = kind;
			frame.set(payload, 5);
			this.process.stdin.write(frame);
			this.process.stdin.flush();
			if (sessionId) {
				const state = this.sessions.get(sessionId);
				if (state) state.queuedBytes = Math.max(0, state.queuedBytes - 0);
			}
		});
		this.writeChain = operation.catch(() => undefined);
		return operation;
	}

	private async readStdout(process: Bun.Subprocess<'pipe', 'pipe', 'pipe'>): Promise<void> {
		const reader = process.stdout.getReader();
		try {
			while (true) {
				const { done, value } = await reader.read();
				if (done) break;
				this.appendBytes(value);
			}
		} catch {
			this.handleProcessExit(null);
		}
	}

	private appendBytes(chunk: Uint8Array): void {
		const combined = new Uint8Array(this.receiveBuffer.length + chunk.length);
		combined.set(this.receiveBuffer);
		combined.set(chunk, this.receiveBuffer.length);
		this.receiveBuffer = combined;
		while (this.receiveBuffer.length >= 4) {
			const length = new DataView(this.receiveBuffer.buffer, this.receiveBuffer.byteOffset).getUint32(0, false);
			if (length > MAX_FRAME_BYTES) {
				this.fail(new Error('Detector worker sent an oversized frame'));
				return;
			}
			if (this.receiveBuffer.length < 5 + length) return;
			const kind = this.receiveBuffer[4];
			const payload = this.receiveBuffer.slice(5, 5 + length);
			this.receiveBuffer = this.receiveBuffer.slice(5 + length);
			if (kind === 1) {
				try {
					this.handleJson(JSON.parse(new TextDecoder().decode(payload)) as WorkerJson);
				} catch {
					this.fail(new Error('Detector worker sent invalid JSON'));
				}
			} else if (kind !== 2) {
				this.fail(new Error('Detector worker sent an unknown frame kind'));
			}
		}
	}

	private handleJson(message: WorkerJson): void {
		const requestId = typeof message.requestId === 'string' ? message.requestId : undefined;
		if (requestId) {
			const pending = this.pending.get(requestId);
			if (pending) {
				clearTimeout(pending.timer);
				const details = message.error && typeof message.error === 'object' ? message.error as Record<string, unknown> : {};
				const detail = typeof details.message === 'string' ? details.message : typeof message.message === 'string' ? message.message : 'Detector worker error';
				this.pending.delete(requestId);
				if (message.type === 'error') pending.reject(new Error(detail));
				else pending.resolve(message);
			}
		}
		for (const listener of this.listeners) listener(message);
	}

	private async readStderr(process: Bun.Subprocess<'pipe', 'pipe', 'pipe'>): Promise<void> {
		const reader = process.stderr.getReader();
		const decoder = new TextDecoder();
		let pendingLine = '';
		try {
			while (true) {
				const { done, value } = await reader.read();
				if (done) break;
				pendingLine += decoder.decode(value, { stream: true });
				const lines = pendingLine.split('\n');
				pendingLine = lines.pop() ?? '';
				for (const line of lines) {
					if (line.trim()) console.error(`[detector-worker] ${line.slice(0, 500)}`);
				}
			}
		} catch {
			return;
		}
	}

	private handleProcessExit(code: number | null): void {
		if (this.readyState.state === 'stopped') return;
		const error = new Error(`Detector worker exited${code === null ? '' : ` with code ${code}`}`);
		this.fail(error);
	}

	private fail(error: Error): void {
		if (this.failure) return;
		this.failure = error;
		this.readyState = { ready: false, state: 'failed', threshold: this.thresholdValue, message: error.message };
		for (const [requestId, pending] of this.pending) {
			clearTimeout(pending.timer);
			this.pending.delete(requestId);
			pending.reject(error);
		}
	}
}

function sessionIdBytes(sessionId: string): Uint8Array {
	const bytes = new Uint8Array(16);
	const hex = sessionId.replaceAll('-', '');
	for (let index = 0; index < 16; index += 1) bytes[index] = Number.parseInt(hex.slice(index * 2, index * 2 + 2), 16);
	return bytes;
}
