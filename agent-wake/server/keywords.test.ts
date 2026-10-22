import { afterAll, beforeAll, describe, expect, test } from 'bun:test';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { appendKeywordLine, normalizeName, readKeywordLines, tokenizeName } from './keywords';

let directory = '';

beforeAll(async () => {
	directory = await mkdtemp(join(tmpdir(), 'agent-wake-keywords-'));
});

afterAll(async () => {
	if (directory) await rm(directory, { recursive: true, force: true });
});

describe('normalizeName', () => {
	test('uppercases and trims the wake word', () => {
		expect(normalizeName('  henroy ')).toBe('HENROY');
		expect(normalizeName("o'brien-smith")).toBe("O'BRIEN-SMITH");
	});

	test('rejects names the detector could not be told about', () => {
		expect(() => normalizeName('')).toThrow('must not be empty');
		expect(() => normalizeName('   ')).toThrow('must not be empty');
		expect(() => normalizeName('H!ENROY')).toThrow('1-64 characters');
		expect(() => normalizeName('H'.repeat(65))).toThrow('1-64 characters');
		expect(() => normalizeName('MARÍA')).toThrow('1-64 characters');
		expect(() => normalizeName(42)).toThrow('must be a string');
		expect(() => normalizeName(null)).toThrow('must be a string');
	});
});

describe('keyword file', () => {
	test('appends a keyword once and leaves no temporary file', async () => {
		const path = join(directory, 'keywords.txt');
		expect(await appendKeywordLine('▁LE X IA', path)).toBe(true);
		expect(await appendKeywordLine('▁HE N RO Y', path)).toBe(true);
		expect(await appendKeywordLine('▁HE N RO Y', path)).toBe(false);
		expect(await readKeywordLines(path)).toEqual(['▁LE X IA', '▁HE N RO Y']);
		expect(await Bun.file(`${path}.tmp`).exists()).toBe(false);
	});

	test('ignores blank lines and surrounding whitespace', async () => {
		const path = join(directory, 'messy.txt');
		await Bun.write(path, '\n  ▁LE X IA  \n\n');
		expect(await readKeywordLines(path)).toEqual(['▁LE X IA']);
		expect(await appendKeywordLine('▁LE X IA', path)).toBe(false);
	});

	test('reads a missing file as an empty list', async () => {
		expect(await readKeywordLines(join(directory, 'absent.txt'))).toEqual([]);
	});
});

const python = Bun.env.AGENT_WAKE_PYTHON ?? 'python3';
const sentencepiece = await Bun.spawn([python, '-c', 'import sentencepiece'], {
	stdout: 'ignore',
	stderr: 'ignore'
}).exited;

describe.skipIf(sentencepiece !== 0)('tokenizeName', () => {
	test('spells a wake word the way the detector expects', async () => {
		expect(await tokenizeName('LEXIA')).toBe('▁LE X IA');
		expect(await tokenizeName('HENROY')).toBe('▁HE N RO Y');
	});

	test('refuses names the model cannot spell', async () => {
		await expect(tokenizeName('O\u2019BRIEN')).rejects.toThrow('cannot spell');
	});
});
