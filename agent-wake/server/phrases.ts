/**
 * Wake phrases for one agent, each mapped to the command it runs.
 *
 * The detector reports which keyword fired, so one detector serves every phrase
 * and a phrase only needs to name its command. Phrases are the agent's own data:
 * they live in a JSON file in the agent directory and are read on change, so a
 * user edits them without rebuilding the host.
 *
 * The default phrase is the platform default, not a hard-coded wake word. It is
 * used when the user skips the setup step that picks a name.
 */

export const DEFAULT_PHRASE = 'AGENT';

export type WakePhrase = {
	/** Uppercased display phrase, spelled the way the keyword file expects. */
	phrase: string;
	/** The command this phrase runs. A command name, not a free-form string. */
	command: string;
};

const PHRASE_PATTERN = /^[A-Z0-9][A-Z0-9 '\-]{0,63}$/;
const COMMAND_PATTERN = /^[a-z][a-z0-9_.-]{0,63}$/;

/** Validate a phrase list. Throws with the first reason it is unusable. */
export function validatePhrases(phrases: WakePhrase[]): WakePhrase[] {
	if (phrases.length === 0) throw new Error('an agent needs at least one wake phrase');
	const seen = new Set<string>();
	for (const entry of phrases) {
		const phrase = typeof entry.phrase === 'string' ? entry.phrase.trim().toUpperCase() : '';
		if (!PHRASE_PATTERN.test(phrase)) {
			throw new Error(`phrase "${entry.phrase}" must be 1-64 letters, digits, spaces, hyphens or apostrophes`);
		}
		if (!COMMAND_PATTERN.test(entry.command)) {
			throw new Error(`command "${entry.command}" must be lowercase letters, digits, dots, underscores or hyphens`);
		}
		if (seen.has(phrase)) throw new Error(`phrase "${phrase}" is listed more than once`);
		seen.add(phrase);
	}
	return phrases.map((entry) => ({ phrase: entry.phrase.trim().toUpperCase(), command: entry.command }));
}

/** The command a detected keyword runs, or null when the phrase is not registered. */
export function commandFor(phrases: WakePhrase[], keyword: string): string | null {
	const wanted = keyword.trim().toUpperCase();
	return phrases.find((entry) => entry.phrase === wanted)?.command ?? null;
}

/** Phrases an agent starts with: the default, mapped to the agent's default command. */
export function defaultPhrases(): WakePhrase[] {
	return [{ phrase: DEFAULT_PHRASE, command: 'chat.reply' }];
}
