import { afterAll, beforeAll, describe, expect, test } from 'bun:test';
import type { Server } from 'bun';
import {
	commandForKeyword,
	currentPhrases,
	fetchAgentPhrases,
	setCurrentPhrases,
	spellPhrases
} from './agent-phrases';

let server: Server | undefined;
let origin = '';
let mode: 'valid' | 'bad-phrase' | 'not-json' = 'valid';

beforeAll(() => {
	server = Bun.serve({
		port: 0,
		fetch(request) {
			if (!new URL(request.url).pathname.endsWith('/api/agent')) {
				return new Response('Not found', { status: 404 });
			}
			if (mode === 'not-json') {
				return new Response('this is not json', { status: 200 });
			}
			if (mode === 'bad-phrase') {
				return Response.json({ wake: [{ phrase: 'H!ENROY', command: 'chat.reply' }] });
			}
			return Response.json({
				name: 'agent',
				title: 'Agent',
				description: 'The main agent.',
				wake: [
					{ phrase: 'AGENT', command: 'chat.reply' },
					{ phrase: 'HEY AGENT', command: 'note.write' }
				]
			});
		}
	});
	origin = `http://127.0.0.1:${server.port}`;
	process.env.AGENT_HOST_URL = origin;
});

afterAll(() => {
	server?.stop(true);
	delete process.env.AGENT_HOST_URL;
});

describe('fetchAgentPhrases', () => {
	test('reads the phrases the agent declares', async () => {
		mode = 'valid';
		const result = await fetchAgentPhrases();
		expect(result.unavailable).toBeNull();
		expect(result.phrases.length).toBe(2);
		expect(result.phrases[0]).toEqual({ phrase: 'AGENT', command: 'chat.reply' });
		expect(result.phrases[1]).toEqual({ phrase: 'HEY AGENT', command: 'note.write' });
	});

	test('a phrase the host would not accept is reported, not used', async () => {
		mode = 'bad-phrase';
		const result = await fetchAgentPhrases();
		expect(result.phrases.length).toBe(0);
		expect(result.unavailable).toContain('not usable');
	});

	test('a host that answers with something else is reported', async () => {
		mode = 'not-json';
		const result = await fetchAgentPhrases();
		expect(result.phrases.length).toBe(0);
		expect(result.unavailable).toContain('not JSON');
	});

	test('a host that cannot be reached is reported, and the detector still runs', async () => {
		process.env.AGENT_HOST_URL = 'http://127.0.0.1:1';
		const result = await fetchAgentPhrases();
		expect(result.phrases.length).toBe(0);
		expect(result.unavailable).toContain('did not answer');
		process.env.AGENT_HOST_URL = origin;
	});
});

describe('commandForKeyword', () => {
	test('a detection maps to the command its phrase declares', () => {
		setCurrentPhrases([
			{ phrase: 'AGENT', command: 'chat.reply' },
			{ phrase: 'HEY AGENT', command: 'note.write' }
		]);
		expect(commandForKeyword('AGENT')).toBe('chat.reply');
		expect(commandForKeyword('HEY AGENT')).toBe('note.write');
		expect(commandForKeyword('agent')).toBe('chat.reply');
	});

	test('a keyword the agent does not answer to runs nothing, rather than a default', () => {
		expect(commandForKeyword('SOMETHING ELSE')).toBeNull();
		expect(commandForKeyword('LEXIA')).toBeNull();
	});

	test('the phrases in force can be read back', () => {
		expect(currentPhrases().length).toBe(2);
	});
});

const python = Bun.env.AGENT_WAKE_PYTHON ?? 'python3';
const sentencepiece = await Bun.spawn([python, '-c', 'import sentencepiece'], {
	stdout: 'ignore',
	stderr: 'ignore'
}).exited;

describe.skipIf(sentencepiece !== 0)('spellPhrases', () => {
	test('a phrase the model cannot spell is refused with the reason', async () => {
		const result = await spellPhrases([{ phrase: 'O\u2019BRIEN', command: 'chat.reply' }]);
		expect(result.added.length).toBe(0);
		expect(result.refused.length).toBe(1);
		expect(result.refused[0].reason).toContain('cannot spell');
	});
});
