/**
 * The agent's wake phrases, as the host declares them.
 *
 * The host owns the manifest and the rules about what a phrase may be, so the
 * wake service asks the host rather than parsing `agent.toml` itself. One
 * authority means a phrase that the host would refuse is never spelled into
 * the detector here.
 *
 * The detector matches pre-spelled keywords, so a phrase has to be written the
 * way the model's own vocabulary spells it before it can fire. That spelling is
 * the wake service's job, and it is why a phrase the model cannot say is
 * reported rather than silently ignored.
 */

import { appendKeywordLine, tokenizeName } from './keywords';
import { validatePhrases, type WakePhrase } from './phrases';

/** Where the agent host runs. The wake service is a separate process. */
const DEFAULT_HOST_URL = 'http://127.0.0.1:8787';

export function hostUrl(): string {
	return (Bun.env.AGENT_HOST_URL ?? DEFAULT_HOST_URL).replace(/\/+$/, '');
}

export type PhraseSync = {
	/** The phrases the agent declares, in the order it declares them. */
	phrases: WakePhrase[];
	/** Phrases added to the detector's keyword file by this sync. */
	added: string[];
	/** Phrases the detector's model cannot spell, with the reason. */
	refused: { phrase: string; reason: string }[];
	/** Why the phrases are missing, when they are. */
	unavailable: string | null;
};

function message(error: unknown): string {
	return error instanceof Error ? error.message : String(error);
}

/**
 * Read the agent's phrases from the host.
 *
 * A host that cannot be reached is not a crash: the detector still runs and
 * still fires, the detections simply carry no command. The reason is reported
 * so the operator knows the difference between "nothing woke it" and "it woke
 * but nothing was listening".
 */
export async function fetchAgentPhrases(): Promise<PhraseSync> {
	let response: Response;
	try {
		response = await fetch(`${hostUrl()}/api/agent`, {
			signal: AbortSignal.timeout(5_000),
			headers: { accept: 'application/json' }
		});
	} catch (error) {
		return {
			phrases: [],
			added: [],
			refused: [],
			unavailable: `the agent host at ${hostUrl()} did not answer: ${message(error)}`
		};
	}
	if (!response.ok) {
		return {
			phrases: [],
			added: [],
			refused: [],
			unavailable: `the agent host answered ${response.status} for /api/agent`
		};
	}
	let body: { wake?: unknown };
	try {
		body = (await response.json()) as { wake?: unknown };
	} catch (error) {
		return {
			phrases: [],
			added: [],
			refused: [],
			unavailable: `the agent host sent something that is not JSON: ${message(error)}`
		};
	}
	const declared = Array.isArray(body.wake) ? (body.wake as WakePhrase[]) : [];
	try {
		return { phrases: validatePhrases(declared), added: [], refused: [], unavailable: null };
	} catch (error) {
		return {
			phrases: [],
			added: [],
			refused: [],
			unavailable: `the agent host declared a wake phrase that is not usable: ${message(error)}`
		};
	}
}

/**
 * Spell each phrase into the detector's keyword file.
 *
 * A phrase the model cannot spell is refused with the reason and the others
 * are still added: one bad phrase should not cost the agent every other one.
 */
export async function spellPhrases(phrases: WakePhrase[]): Promise<{
	added: string[];
	refused: { phrase: string; reason: string }[];
}> {
	const added: string[] = [];
	const refused: { phrase: string; reason: string }[] = [];
	for (const { phrase } of phrases) {
		try {
			const tokens = await tokenizeName(phrase);
			if (await appendKeywordLine(tokens)) added.push(phrase);
		} catch (error) {
			refused.push({ phrase, reason: message(error) });
		}
	}
	return { added, refused };
}

/** The phrases the service is currently serving, and the command each runs. */
let current: WakePhrase[] = [];

export function setCurrentPhrases(phrases: WakePhrase[]): void {
	current = phrases;
}

export function currentPhrases(): WakePhrase[] {
	return current;
}

/**
 * The command a detected keyword runs, or null when the keyword is not one the
 * agent answers to. A detection without a command is reported as such rather
 * than mapped to a default: a default would run the wrong thing.
 */
export function commandForKeyword(keyword: string): string | null {
	const wanted = keyword.trim().toUpperCase();
	return current.find((entry) => entry.phrase === wanted)?.command ?? null;
}
