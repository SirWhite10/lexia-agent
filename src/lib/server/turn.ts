import { listMessages } from './agents.js';
import { countUserTurns, loadPersonality, recordLogEntry, reflect, retrieveMemory, setEntryEmbedding } from './memory.js';
import type { MemoryEntry } from './memory.js';
import { completeOpenRouter, embedOpenRouterText } from './openrouter.js';
import { assemblePrompt } from './prompt.js';
import type { AssembledPrompt } from './prompt.js';

// The per-turn seam (WF-IMP-004 + WF-IMP-005). This is where the live provider
// is bound to the pure pieces: embed the turn, retrieve role-scoped memory,
// assemble the cache-stable prompt, and — at reflection boundaries — let the
// model distil new facts and traits. Without this module the pipeline exists
// but no request ever reaches it.

export const EMBEDDING_MODEL = process.env.OPENROUTER_EMBEDDING_MODEL ?? 'openai/text-embedding-3-small';
export const REFLECTION_MODEL = process.env.OPENROUTER_REFLECTION_MODEL ?? 'openai/gpt-4o-mini';
/** WF-DEC-002: reflect every ~10 turns, never on every turn. */
export const REFLECTION_INTERVAL = 10;

export type Turn = {
	prompt: AssembledPrompt;
	memory: MemoryEntry[];
	/** True when no memory was injected because the provider was unavailable. */
	degraded: boolean;
};

/**
 * Builds the system prompt for a user turn. The personality block is the only
 * slot that changes between reflection boundaries, so the cache prefix stays
 * byte-stable while memory varies per request.
 */
export async function prepareTurn(input: { query: string; scope?: string | null }): Promise<Turn> {
	if (!process.env.OPENROUTER_API_KEY) {
		return { prompt: assemblePrompt({ skills: [], memory: [], tools: [] }), memory: [], degraded: true };
	}
	try {
		const memory = await retrieveMemory({
			query: input.query,
			scope: input.scope ?? null,
			embed: (text) => embedOpenRouterText(EMBEDDING_MODEL, text),
			embeddingModel: EMBEDDING_MODEL
		});
		return {
			prompt: assemblePrompt({
				personality: loadPersonality(),
				skills: [],
				memory: memory.map((entry) => entry.body),
				tools: []
			}),
			memory,
			degraded: false
		};
	} catch {
		// Retrieval must never take a turn down: a provider failure degrades to
		// an empty memory block rather than a broken conversation.
		return { prompt: assemblePrompt({ skills: [], memory: [], tools: [] }), memory: [], degraded: true };
	}
}

/** Captures a user turn into the verbatim log and embeds it for later retrieval. */
export async function captureTurn(message: { id: string; body: string }): Promise<boolean> {
	const entry = recordLogEntry({ sourceMessageId: message.id, body: message.body });
	if (!process.env.OPENROUTER_API_KEY) return false;
	try {
		const vector = await embedOpenRouterText(EMBEDDING_MODEL, message.body);
		setEntryEmbedding(entry.id, new Uint8Array(new Float32Array(vector).buffer), EMBEDDING_MODEL);
		return true;
	} catch {
		// The row exists and is picked up by a later re-index; the turn proceeds.
		return false;
	}
}

/**
 * Reflection runs on a turn boundary, not per turn (WF-DEC-002). The trigger is
 * derived from the persisted log count, so it survives restarts and needs no
 * scheduler state. Returns null when this turn is not a boundary, and a failed
 * reflection never breaks the turn that triggered it.
 */
export async function maybeReflect(): Promise<{ facts: number; traits: number } | null> {
	if (!process.env.OPENROUTER_API_KEY) return null;
	const turns = countUserTurns();
	if (turns === 0 || turns % REFLECTION_INTERVAL !== 0) return null;
	try {
		const transcript = listMessages()
			.slice(-REFLECTION_INTERVAL * 2)
			.map((message) => `${message.authorKind}: ${message.body}`)
			.join('\n');
		const result = await reflect({
			transcript,
			knownFacts: '',
			callModel: (system, user) => completeOpenRouter(REFLECTION_MODEL, system, user)
		});
		return { facts: result.facts.length, traits: result.traits.length };
	} catch {
		return null;
	}
}
