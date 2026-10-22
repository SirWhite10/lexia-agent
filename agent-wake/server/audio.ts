import { randomUUID } from 'node:crypto';

export const AUDIO_PROTOCOL_VERSION = 1;
export const MAX_AUDIO_FRAME_BYTES = 64 * 1024;
export const MAX_AUDIO_QUEUE_MS = 250;
export const AUDIO_IDLE_TIMEOUT_MS = 10_000;
export const DEFAULT_ALLOWED_ORIGINS = ['https://lexia.dlx.studio', 'http://localhost:8787', 'http://127.0.0.1:8787'];

export type AudioStart = {
	protocolVersion: number;
	sampleRate: number;
	channels: 1;
	encoding: 'pcm_f32le';
};

export type AudioSocketData = {
	sessionId: string;
	userId: string;
	state: 'connected' | 'ready' | 'streaming' | 'stopping' | 'closed';
	sampleRate?: number;
	lastFrameAt: number;
};

export type AudioControlMessage =
	| ({ type: 'start' } & AudioStart)
	| { type: 'stop' };

export function parseAudioStart(message: string): AudioStart | null {
	try {
		const value = JSON.parse(message) as Record<string, unknown>;
		if (value.type !== 'start' || value.protocolVersion !== AUDIO_PROTOCOL_VERSION || value.channels !== 1 || value.encoding !== 'pcm_f32le') return null;
		if (typeof value.sampleRate !== 'number' || !Number.isInteger(value.sampleRate) || value.sampleRate < 8_000 || value.sampleRate > 192_000) return null;
		return {
			protocolVersion: AUDIO_PROTOCOL_VERSION,
			sampleRate: value.sampleRate,
			channels: 1,
			encoding: 'pcm_f32le'
		};
	} catch {
		return null;
	}
}

export function parseAudioControl(message: string): AudioControlMessage | null {
	try {
		const value = JSON.parse(message) as Record<string, unknown>;
		if (value.type === 'stop') return { type: 'stop' };
		if (value.type === 'start') return parseAudioStart(message) ? (value as unknown as AudioControlMessage) : null;
	} catch {
		return null;
	}
	return null;
}

export function isAllowedOrigin(origin: string | null, allowedOrigins: string[] = configuredOrigins()): boolean {
	return origin !== null && allowedOrigins.includes(origin);
}

export function configuredOrigins(): string[] {
	const configured = Bun.env.ALLOWED_ORIGINS?.split(',').map((origin) => origin.trim()).filter(Boolean) ?? [];
	return [...new Set([...DEFAULT_ALLOWED_ORIGINS, ...configured])];
}

export function validateAudioFrame(data: Uint8Array, sampleRate: number): Float32Array | null {
	if (data.byteLength === 0 || data.byteLength > MAX_AUDIO_FRAME_BYTES || data.byteLength % 4 !== 0) return null;
	const minimumSamples = Math.floor((sampleRate * 20) / 1000);
	const maximumSamples = Math.ceil((sampleRate * 40) / 1000);
	const sampleCount = data.byteLength / 4;
	if (sampleCount < minimumSamples || sampleCount > maximumSamples) return null;
	const copy = data.slice().buffer;
	const samples = new Float32Array(copy);
	for (const sample of samples) if (!Number.isFinite(sample)) return null;
	return samples;
}

export function audioError(code: string, message: string, retryable = false): Record<string, unknown> {
	return { type: 'error', error: { code, message, retryable } };
}

export function newSocketData(userId: string): AudioSocketData {
	return { sessionId: randomUUID(), userId, state: 'connected', lastFrameAt: Date.now() };
}

export function jsonMessage(value: Record<string, unknown>): string {
	return JSON.stringify(value);
}
