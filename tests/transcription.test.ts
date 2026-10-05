import { afterAll, describe, expect, test } from 'bun:test';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

// transcription.ts reads assignments, the key store and the registry, all of
// which resolve their state directory when they load, so the temp directory has
// to exist before the imports below.
const workspace = mkdtempSync(join(tmpdir(), 'lexosa-transcribe-test-'));
process.env.LEXIA_STATE_DIR = workspace;
const routes = await import('../src/lib/server/model-routes.js');
const keys = await import('../src/lib/server/provider-key.js');
const { extractTranscript, transcriptionTarget } = await import('../src/lib/server/transcription.js');

afterAll(() => {
	rmSync(workspace, { recursive: true, force: true });
});

describe('choosing a transcription provider', () => {
	test('is nothing until a model and a provider are assigned', () => {
		expect(transcriptionTarget()).toBeNull();

		routes.assignModel('transcription', 'whisper-1', 'openai');
		expect(transcriptionTarget()).toBeNull(); // assigned, but no key yet

		keys.saveProviderKey('openai', 'sk-test');
		const target = transcriptionTarget();
		expect(target?.url).toBe('https://api.openai.com/v1/audio/transcriptions');
		expect(target?.encoding).toBe('multipart');
		expect(target?.headers.Authorization).toBe('Bearer sk-test');
	});

	test('uses each provider\'s own call shape', () => {
		keys.saveProviderKey('deepgram', 'dg-test');
		routes.assignModel('transcription', 'nova-3', 'deepgram');
		const deepgram = transcriptionTarget();
		expect(deepgram?.encoding).toBe('raw');
		expect(deepgram?.url).toContain('api.deepgram.com/v1/listen?model=nova-3');
		expect(deepgram?.headers.Authorization).toBe('Token dg-test');

		keys.saveProviderKey('elevenlabs', 'el-test');
		routes.assignModel('transcription', 'scribe_v1', 'elevenlabs');
		const eleven = transcriptionTarget();
		expect(eleven?.headers['xi-api-key']).toBe('el-test');
		expect(eleven?.url).toBe('https://api.elevenlabs.io/v1/speech-to-text');
	});

	test('talks to OpenRouter\'s own transcription endpoint', () => {
		keys.saveProviderKey('openrouter', 'or-test');
		routes.assignModel('transcription', 'openai/whisper-large-v3', 'openrouter');
		const target = transcriptionTarget();

		expect(target?.url).toBe('https://openrouter.ai/api/v1/audio/transcriptions');
		expect(target?.headers.Authorization).toBe('Bearer or-test');
		expect(target?.encoding).toBe('multipart');
		keys.saveProviderKey('openrouter', '');
	});

	test('reaches a self-hosted server with no credential at all', () => {
		delete process.env.LOCAL_API_KEY;
		delete process.env.LOCAL_TRANSCRIPTION_URL;
		// A local server was started with its model already: an empty id is the
		// right answer there, not an unconfigured one.
		routes.assignModel('transcription', '', 'local');
		const target = transcriptionTarget();

		expect(target?.url).toBe('http://127.0.0.1:8080/v1/audio/transcriptions');
		expect(target?.headers).toEqual({});

		process.env.LOCAL_TRANSCRIPTION_URL = 'http://192.168.1.9:9000/';
		expect(transcriptionTarget()?.url).toBe('http://192.168.1.9:9000/v1/audio/transcriptions');
		delete process.env.LOCAL_TRANSCRIPTION_URL;
	});

	test('refuses a provider with no verified transcription endpoint', () => {
		// Runway is in the registry, but nothing has verified an endpoint for it:
		// guessing one would spend the operator's request to return a 404.
		keys.saveProviderKey('runway', 'rw-test');
		routes.assignModel('transcription', 'gen4', 'runway');

		expect(transcriptionTarget()).toBeNull();
	});

	test('falls back to the stored key, and stops when there is none', () => {
		routes.assignModel('transcription', 'nova-3', 'deepgram');
		delete process.env.DEEPGRAM_API_KEY;
		expect(transcriptionTarget()?.headers.Authorization).toBe('Token dg-test');

		keys.saveProviderKey('deepgram', '');
		expect(transcriptionTarget()).toBeNull();
	});
});

describe('reading a transcript out of a provider response', () => {
	test('handles the flat shape OpenAI and ElevenLabs return', () => {
		expect(extractTranscript('openai', { text: 'hello there' })).toBe('hello there');
		expect(extractTranscript('elevenlabs', { text: 'hello there' })).toBe('hello there');
	});

	test('handles Deepgram\'s nested shape', () => {
		expect(
			extractTranscript('deepgram', { results: { channels: [{ alternatives: [{ transcript: 'deep words' }] }] } })
		).toBe('deep words');
	});

	test('returns null rather than a blank note', () => {
		expect(extractTranscript('openai', {})).toBeNull();
		expect(extractTranscript('openai', { text: '   ' })).toBeNull();
		expect(extractTranscript('deepgram', { results: { channels: [] } })).toBeNull();
		expect(extractTranscript('openai', null)).toBeNull();
	});
});