import { createHash } from 'node:crypto';

/**
 * Prompt assembly (WF-IMP-005). The slot order is the cache contract from
 * WF-DEC-002: `[personality → all skills → retrieved memory → resolved tools]`.
 * Everything above the memory block is byte-stable across turns so provider
 * prompt caching hits; only the suffix moves as memory and capabilities change.
 * The prefix digest makes that property testable.
 *
 * This module is deliberately pure: it opens no database and reads no files, so
 * the runtime supplies the personality block and the retrieval layer supplies
 * memory. Keeping it side-effect free also keeps the cache contract provable
 * without standing up state.
 */

export type PromptSlots = {
	/** Lexia's personality block; rewritten only at reflection boundaries. */
	personality?: string | null;
	/** The complete skill set, always injected in full, never selected per turn. */
	skills: string[];
	/** Retrieved memory entries, already budget-capped by the retrieval layer. */
	memory: string[];
	/** Capabilities Jev's classification resolved for this request. */
	tools: string[];
};

export type AssembledPrompt = {
	/** The full system prompt in slot order. */
	system: string;
	/** Digest of everything above the memory block; equal across turns until personality or skills change. */
	prefixDigest: string;
	/** Digest of the volatile suffix; changes whenever memory or tools change. */
	suffixDigest: string;
	/** True when personality and skills are byte-identical to the previous assembly. */
	prefixStable: boolean;
};

const SECTION_HEADINGS = {
	personality: '# Personality',
	skills: '# Skills',
	memory: '# Memory',
	tools: '# Tools'
} as const;

function section(heading: string, body: string): string {
	return `${heading}\n\n${body.trim()}\n`;
}

function digest(parts: string[]): string {
	return createHash('sha256').update(parts.join(' ')).digest('hex');
}

// One module-level cursor, not per-caller state: the stability contract is a
// property of consecutive assemblies from the single runtime that owns them.
let previousPrefixDigest: string | null = null;

export function assemblePrompt(slots: PromptSlots): AssembledPrompt {
	const prefixParts: string[] = [];
	if (slots.personality?.trim()) prefixParts.push(section(SECTION_HEADINGS.personality, slots.personality));
	if (slots.skills.length > 0) prefixParts.push(section(SECTION_HEADINGS.skills, slots.skills.join('\n\n')));

	const suffixParts: string[] = [];
	if (slots.memory.length > 0) suffixParts.push(section(SECTION_HEADINGS.memory, slots.memory.join('\n\n')));
	if (slots.tools.length > 0) suffixParts.push(section(SECTION_HEADINGS.tools, slots.tools.join('\n\n')));

	const prefixDigest = digest(prefixParts);
	const suffixDigest = digest(suffixParts);
	const prefixStable = previousPrefixDigest === null || previousPrefixDigest === prefixDigest;
	previousPrefixDigest = prefixDigest;

	return {
		system: [...prefixParts, ...suffixParts].join('\n'),
		prefixDigest,
		suffixDigest,
		prefixStable
	};
}

/** Test seam: resets the consecutive-assembly cursor. */
export function resetPrefixTracking(): void {
	previousPrefixDigest = null;
}
