import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { loadPersonality, savePersonality } from './memory.js';

// First-run bootstrap (WF-IMP-007). Writes the two files a new install needs
// before the agent is useful: EVE's always-on system prompt and the personality
// block the prompt assembler injects. Memory is deliberately absent — the agent
// builds that itself as it works, and seeding it here would be fiction.
//
// instructions.md is authored EVE content and lives in the agent project, so the
// project root is configuration rather than an assumption. personality.md is
// mutable per-installation state and is written through savePersonality so the
// file keeps the format reflection already maintains.

const EVE_ROOT = process.env.LEXIA_EVE_ROOT ?? join(process.cwd(), 'my-agent');

export type BootstrapInput = {
	/** What the agent is called; becomes the instruction heading and chat identity. */
	name: string;
	/** One sentence on what the agent is for. */
	purpose: string;
	/** How the agent should come across, as free text. */
	tone: string;
	/** Short behaviour lines seeded as the first personality traits. */
	traits: string[];
};

export type BootstrapStatus = {
	bootstrapped: boolean;
	instructionsPath: string;
	personalityPath: string;
	/** Traits already present, so the flow can avoid overwriting real history. */
	existingTraits: number;
};

function instructionsFile(): string {
	return join(resolve(EVE_ROOT), 'agent/instructions.md');
}

export function bootstrapStatus(): BootstrapStatus {
	const instructions = instructionsFile();
	const traits = loadPersonality()
		.split('\n')
		.map((line) => line.match(/^- \[[^\]]+\] (.+)$/)?.[1])
		.filter((trait): trait is string => Boolean(trait));
	return {
		bootstrapped: existsSync(instructions) && traits.length > 0,
		instructionsPath: instructions,
		personalityPath: join(resolve(process.env.LEXIA_STATE_DIR ?? join(process.cwd(), 'app-data/state')), 'agent/personality.md'),
		existingTraits: traits.length
	};
}

/**
 * Composes the system prompt from what the user actually said. The structure
 * mirrors the scaffold's headings (identity, then customization) so the file
 * stays recognisable to anyone who has seen an EVE agent, and so a later EVE
 * scaffold update lands in a familiar place.
 */
export function renderInstructions(input: BootstrapInput): string {
	const traits = input.traits.map((trait) => trait.trim()).filter((trait) => trait.length > 0);
	const sections = [
		'# Identity',
		'',
		`You are ${input.name.trim() || 'Lexosa'}, a personal agent for this installation.`,
		'',
		input.purpose.trim() || 'You help with whatever the user asks.',
		'',
		'# Voice',
		'',
		input.tone.trim() || 'Direct and plain.',
		'',
		'# Working style',
		'',
		...(traits.length > 0
			? traits.map((trait) => `- ${trait}`)
			: ['- Ask before acting on anything the user did not clearly request.', '- Say what you did and what you skipped.']),
		'',
		'# Memory',
		'',
		'Memory is not yours to maintain by hand. Durable facts and personality traits are updated at reflection boundaries as you learn the user; do not invent memories, and do not ask the user to edit them.'
	];
	return sections.join('\n') + '\n';
}

export type BootstrapResult = { instructionsWritten: boolean; traitsAdded: number };

/**
 * Writes both files. instructions.md is replaced outright because the user is
 * describing the agent right now; personality is appended to rather than
 * replaced, because a working install may already have traits that reflection
 * earned. Re-running with the same traits is a no-op rather than a duplicate.
 */
export function bootstrapAgent(input: BootstrapInput): BootstrapResult {
	const instructions = renderInstructions(input);
	const path = instructionsFile();
	mkdirSync(dirname(path), { recursive: true });
	const previous = existsSync(path) ? readFileSync(path, 'utf8') : '';
	writeFileSync(path, instructions, 'utf8');

	const existing = loadPersonality()
		.split('\n')
		.map((line) => line.match(/^- \[[^\]]+\] (.+)$/)?.[1])
		.filter((trait): trait is string => Boolean(trait));
	// A bootstrap with no traits still has to leave a non-empty personality
	// block, otherwise the install never reads as bootstrapped and the flow
	// loops. Seed a starter trait when the user supplied none.
	const seeded = input.traits.map((trait) => trait.trim()).filter((trait) => trait.length > 0 && !existing.includes(trait));
	const additions = seeded.length > 0 ? seeded : existing.length === 0 ? ['is still learning how you like things done'] : [];
	if (additions.length > 0) savePersonality([...existing, ...additions]);

	return { instructionsWritten: previous !== instructions, traitsAdded: additions.length };
}
