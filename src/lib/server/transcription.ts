import { readFileSync } from 'node:fs';
import {
	listUntranscribedAudio,
	recordTranscript,
	recordTranscriptError,
	type UntranscribedAudio
} from './attachments.js';
import { modelForUseCase, providerForUseCase } from './model-routes.js';
import { providerBaseUrl, providerById } from './provider-registry.js';
import { providerKey } from './provider-key.js';

// Voice notes the composer records are stored on send and left alone. This is
// the pass that turns them into text, using the provider and model the operator
// assigned to the Transcriptions use-case in Settings → Models.
//
// The call shapes differ per provider and are the part most likely to be wrong,
// so each one is spelled out rather than assumed from a shared base URL.

const TRANSCRIPTION_USE_CASE = 'transcription';
const REQUEST_TIMEOUT_MS = 60_000;

export type TranscriptionTarget = {
	providerId: string;
	providerLabel: string;
	model: string;
	url: string;
	headers: Record<string, string>;
	/** Deepgram wants the raw bytes; the rest take a multipart form. */
	encoding: 'multipart' | 'raw';
};

export type TranscriptionReport = { transcribed: number; failed: number; skipped: boolean };

/** What the current configuration asks for, or null when it asks for nothing:
 * no provider, no model where one is needed, no credential on this host, or a
 * provider we have no verified transcription endpoint for. Each of those is a
 * state the operator can fix, so the sweep skips quietly rather than failing
 * the turn.
 */
export function transcriptionTarget(): TranscriptionTarget | null {
	const providerId = providerForUseCase(TRANSCRIPTION_USE_CASE);
	const provider = providerId ? providerById(providerId) : undefined;
	if (!provider || !providerId) return null;

	// A self-hosted server was started with its model already, so an empty id is a
	// valid answer there; everywhere else a model is required.
	const model = modelForUseCase(TRANSCRIPTION_USE_CASE) ?? '';
	const selfHosted = providerBaseUrl(provider);
	const key = providerKey(providerId);
	if (!key && !selfHosted) return null;
	if (!selfHosted && !model) return null;
	// Every hosted branch below needs a credential, and the guard above has
	// already established one is present for those.
	const authKey = key ?? '';

	if (providerId === 'deepgram') {
		return {
			providerId,
			providerLabel: provider.label,
			model,
			url: `https://api.deepgram.com/v1/listen?model=${encodeURIComponent(model)}&smart_format=true`,
			headers: { Authorization: `Token ${authKey}` },
			encoding: 'raw'
		};
	}

	if (providerId === 'elevenlabs') {
		return {
			providerId,
			providerLabel: provider.label,
			model,
			url: 'https://api.elevenlabs.io/v1/speech-to-text',
			headers: { 'xi-api-key': authKey },
			encoding: 'multipart'
		};
	}

	if (providerId === 'openrouter') {
		return {
			providerId,
			providerLabel: provider.label,
			model,
			url: 'https://openrouter.ai/api/v1/audio/transcriptions',
			headers: { Authorization: `Bearer ${authKey}` },
			encoding: 'multipart'
		};
	}

	if (providerId === 'openai') {
		return {
			providerId,
			providerLabel: provider.label,
			model,
			url: 'https://api.openai.com/v1/audio/transcriptions',
			headers: { Authorization: `Bearer ${authKey}` },
			encoding: 'multipart'
		};
	}

	if (providerId === 'local' && selfHosted) {
		return {
			providerId,
			providerLabel: provider.label,
			model,
			url: `${selfHosted}/v1/audio/transcriptions`,
			headers: authKey ? { Authorization: `Bearer ${authKey}` } : {},
			encoding: 'multipart'
		};
	}

	// A provider with no verified endpoint gets no target: a guessed URL would
	// turn a missing integration into a confusing 404 on the operator's key.
	return null;
}

/** Pulls the text out of whichever shape the provider answered with. Returns
 * null rather than a half-built string when the payload has no transcript, so a
 * changed response is recorded as a failure instead of a blank note. */
export function extractTranscript(providerId: string, payload: unknown): string | null {
	if (!payload || typeof payload !== 'object') return null;
	const body = payload as Record<string, unknown>;

	if (providerId === 'deepgram') {
		const results = body.results as { channels?: { alternatives?: { transcript?: unknown }[] }[] } | undefined;
		const transcript = results?.channels?.[0]?.alternatives?.[0]?.transcript;
		return typeof transcript === 'string' && transcript.trim().length > 0 ? transcript.trim() : null;
	}

	const text = body.text;
	return typeof text === 'string' && text.trim().length > 0 ? text.trim() : null;
}

async function transcribeOne(audio: UntranscribedAudio, target: TranscriptionTarget): Promise<void> {
	const bytes = readFileSync(audio.path);

	const response = await fetch(target.url, {
		method: 'POST',
		signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
		headers:
			target.encoding === 'raw'
				? { ...target.headers, 'Content-Type': audio.mimeType }
				: target.headers,
		body:
			target.encoding === 'raw'
				? new Uint8Array(bytes)
				: (() => {
						const form = new FormData();
						form.append('file', new Blob([bytes], { type: audio.mimeType }), audio.id);
						// A local server was started with its model already; sending an
						// empty `model` there would be asking it for something unnamed.
						if (target.model) form.append('model', target.model);
						return form;
					})()
	});

	if (!response.ok) {
		// The provider's own wording is the useful part here: a rejected key and a
		// wrong model fail the same way from the host's point of view.
		const detail = (await response.text()).replace(/\s+/g, ' ').trim().slice(0, 200);
		recordTranscriptError(
			audio.id,
			`${target.providerLabel} refused the audio (${response.status})${detail ? `: ${detail}` : ''}`
		);
		return;
	}

	const transcript = extractTranscript(target.providerId, await response.json());
	if (!transcript) {
		recordTranscriptError(audio.id, `${target.providerLabel} answered with no transcript.`);
		return;
	}

	recordTranscript(audio.id, transcript);
}

/**
 * Runs every voice note that has never been transcribed. Called after a turn
 * that carried audio; deliberately not awaited by the caller, because a provider
 * round trip has no business holding a reply open.
 */
export async function transcribePendingAudio(): Promise<TranscriptionReport> {
	const target = transcriptionTarget();
	if (!target) return { transcribed: 0, failed: 0, skipped: true };

	const pending = listUntranscribedAudio();
	const report: TranscriptionReport = { transcribed: 0, failed: 0, skipped: false };

	for (const audio of pending) {
		try {
			await transcribeOne(audio, target);
			report.transcribed += 1;
		} catch (error) {
			report.failed += 1;
			const reason =
				error instanceof DOMException && error.name === 'TimeoutError' ? 'timed out' : 'could not be reached';
			recordTranscriptError(audio.id, `${target.providerLabel} ${reason}.`);
		}
	}

	return report;
}