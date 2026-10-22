import { rename } from 'node:fs/promises';
import { resolve } from 'node:path';

/**
 * Wake-word bookkeeping for the host.
 *
 * The detector matches pre-tokenized keywords, so a new name is spelled with the
 * model's own SentencePiece model, appended to the keyword file, and picked up
 * when the detector worker restarts.
 */

const DEFAULT_MODEL_DIR = 'models/sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01';
const TOKENIZER_SCRIPT = 'scripts/tokenize-keywords.py';
const NAME_PATTERN = /^[A-Z0-9][A-Z0-9 '\-]{0,63}$/;
const UNKNOWN_TOKEN = '<unk>';

export function modelDirPath(): string {
	return resolve(process.cwd(), Bun.env.AGENT_WAKE_MODEL_DIR ?? DEFAULT_MODEL_DIR);
}

export function keywordsFilePath(): string {
	return resolve(process.cwd(), Bun.env.AGENT_WAKE_KEYWORDS_FILE ?? 'assets/keywords.txt');
}

/** Uppercase and shape-check a display name, or explain why it cannot be a wake word. */
export function normalizeName(value: unknown): string {
	if (typeof value !== 'string') throw new Error('name must be a string');
	const name = value.trim().toUpperCase();
	if (name.length === 0) throw new Error('name must not be empty');
	if (!NAME_PATTERN.test(name)) {
		throw new Error('name must be 1-64 characters of letters, digits, spaces, hyphens or apostrophes');
	}
	return name;
}

/** Spell `name` with the model's SentencePiece vocabulary. */
export async function tokenizeName(name: string): Promise<string> {
	const modelPath = resolve(modelDirPath(), 'bpe.model');
	if (!(await Bun.file(modelPath).exists())) throw new Error(`sentencepiece model is missing: ${modelPath}`);
	const python = Bun.env.AGENT_WAKE_PYTHON ?? 'python3';
	const script = resolve(process.cwd(), TOKENIZER_SCRIPT);
	const child = Bun.spawn([python, script, modelPath, name], { stdout: 'pipe', stderr: 'pipe' });
	const [stdout, stderr] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text()]);
	const exitCode = await child.exited;
	if (exitCode !== 0) throw new Error(stderr.trim() || `${python} exited with code ${exitCode}`);
	const line = stdout.trim();
	if (line.length === 0) throw new Error('the tokenizer returned no tokens');
	const tokens = line.split(' ');
	const vocabulary = await modelVocabulary();
	const outside = tokens.filter((token) => !vocabulary.has(token));
	if (outside.length > 0 || tokens.includes(UNKNOWN_TOKEN)) {
		throw new Error(`this model cannot spell "${name}"${outside.length > 0 ? ` (${outside.join(' ')})` : ''}`);
	}
	return line;
}

let vocabulary: Promise<Set<string>> | null = null;

/** The pieces the detector's own model can recognise, read from its token list. */
function modelVocabulary(): Promise<Set<string>> {
	vocabulary ??= (async () => {
		const path = resolve(modelDirPath(), 'tokens.txt');
		if (!(await Bun.file(path).exists())) throw new Error(`model token list is missing: ${path}`);
		const text = await Bun.file(path).text();
		return new Set(text.split('\n').map((entry) => entry.slice(0, entry.lastIndexOf(' '))).filter(Boolean));
	})();
	return vocabulary;
}

export async function readKeywordLines(path = keywordsFilePath()): Promise<string[]> {
	const file = Bun.file(path);
	if (!(await file.exists())) return [];
	return (await file.text()).split('\n').map((line) => line.trim()).filter((line) => line.length > 0);
}

/** Append `line` unless it is already listed. Returns whether the file changed. */
export async function appendKeywordLine(line: string, path = keywordsFilePath()): Promise<boolean> {
	const lines = await readKeywordLines(path);
	if (lines.includes(line)) return false;
	const next = `${[...lines, line].join('\n')}\n`;
	const temporary = `${path}.tmp`;
	await Bun.write(temporary, next);
	await rename(temporary, path);
	return true;
}

async function rename(from: string, to: string): Promise<void> {
	const { rename } = await import('node:fs/promises');
	await rename(from, to);
}
