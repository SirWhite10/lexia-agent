/**
 * Composer model: the input is the source of truth.
 *
 * Everything the composer shows — the tray of context items above the field, the
 * inline badges inside it, the typeahead list — is derived by re-scanning `text`
 * against a catalog of items the user has actually chosen. Nothing stores a caret
 * range as state, so a paste, a selection delete, or an undo can never leave a
 * badge pointing at text that no longer exists: the tokens simply stop matching.
 *
 * A token is a trigger character plus a word, sitting at the start of the text or
 * after whitespace:
 *
 *   `/search`   tool      `$scout`     skill
 *   `#kit.css`  file      `^gate`      memory
 *   `@reviewer` agent     `https://…`  link, detected without a trigger
 *
 * An unresolved token stays plain text and drives the typeahead. Choosing a
 * suggestion registers it in the catalog, which is what turns the same text into
 * a badge. Tokens never contain spaces — a word is a word — so a file whose name
 * has spaces is registered under a hyphenated token while the badge keeps the real
 * name, which is the same trade every mention-style composer makes.
 */

export type ContextKind = 'tool' | 'skill' | 'file' | 'memory' | 'agent' | 'link';

export type ContextItem = {
	/** Token as it appears in the text, including its trigger. Unique per item. */
	token: string;
	kind: ContextKind;
	/** Text inside the badge. */
	label: string;
	/** Secondary text for the tray, e.g. a file size or a capability name. */
	detail?: string;
	state?: 'uploading' | 'ready' | 'failed';
};

export type ComposerDocument = {
	text: string;
	catalog: ContextItem[];
};

export type Segment =
	| { kind: 'text'; start: number; end: number; text: string }
	/** A token with no catalog entry yet: plain text that drives the typeahead. */
	| { kind: 'pending'; start: number; end: number; text: string; context: ContextKind }
	| { kind: 'item'; start: number; end: number; text: string; item: ContextItem };

export const TRIGGER_BY_KIND: Record<Exclude<ContextKind, 'link'>, string> = {
	tool: '/',
	skill: '$',
	file: '#',
	memory: '^',
	agent: '@'
};

const KIND_BY_TRIGGER: Record<string, ContextKind> = Object.fromEntries(
	Object.entries(TRIGGER_BY_KIND).map(([kind, trigger]) => [trigger, kind as ContextKind])
);

export const KIND_LABEL: Record<ContextKind, string> = {
	tool: 'Tools',
	skill: 'Skills',
	file: 'Files',
	memory: 'Memory',
	agent: 'Agents',
	link: 'Links'
};

/** Shown on a badge and in the typeahead, so a badge is never identified by colour alone. */
export const KIND_GLYPH: Record<ContextKind, string> = {
	tool: '⚙',
	skill: '$',
	file: '#',
	memory: '^',
	agent: '@',
	link: '↗'
};

const WORD = '[A-Za-z0-9_.-]*';
const TOKEN = new RegExp(`(?:^|\\s)([/$#^@])(${WORD})|(https?:\\/\\/[^\\s]+)`, 'g');

/**
 * Splits the text into plain runs, unresolved tokens and resolved items. This is
 * the only thing the composer renders from; keeping the tray and the inline
 * overlay on one parse is what stops them from disagreeing.
 */
export function parseComposerText(text: string, catalog: ContextItem[]): Segment[] {
	const byToken = new Map(catalog.map((item) => [item.token, item]));
	const segments: Segment[] = [];
	let cursor = 0;

	for (const match of text.matchAll(TOKEN)) {
		const [whole, trigger, word, url] = match;
		// The regex consumes the whitespace before a trigger so it cannot match mid-word;
		// the segment starts at the trigger, which keeps that space in the plain run.
		const matched = match.index ?? 0;
		const start = trigger ? matched + (whole[0] === ' ' ? 1 : 0) : matched;
		const end = start + (url ? url.length : word.length + 1);

		if (start > cursor) segments.push({ kind: 'text', start: cursor, end: start, text: text.slice(cursor, start) });

		if (url) {
			const item = byToken.get(url) ?? { token: url, kind: 'link' as const, label: url, detail: 'Link' };
			segments.push({ kind: 'item', start, end, text: url, item });
			cursor = end;
			continue;
		}

		const token = `${trigger}${word}`;
		const item = byToken.get(token);
		const atBoundary = end === text.length || /\s/.test(text[end]);
		if (item) segments.push({ kind: 'item', start, end, text: token, item });
		else if (atBoundary) segments.push({ kind: 'pending', start, end, text: token, context: KIND_BY_TRIGGER[trigger] });
		else segments.push({ kind: 'text', start, end, text: token });

		cursor = end;
	}

	if (cursor < text.length) segments.push({ kind: 'text', start: cursor, end: text.length, text: text.slice(cursor) });
	return segments;
}

/** Context items currently in the composer, in document order. One token is one item. */
export function itemsInComposer(text: string, catalog: ContextItem[]): ContextItem[] {
	const seen = new Set<string>();
	const items: ContextItem[] = [];
	for (const segment of parseComposerText(text, catalog)) {
		if (segment.kind !== 'item' || seen.has(segment.item.token)) continue;
		seen.add(segment.item.token);
		items.push(segment.item);
	}
	return items;
}

/**
 * The sentence a turn actually says, with resolved tokens lifted out. Context travels
 * as items and badges, never as punctuation inside the message body.
 */
export function textWithoutContext(text: string, catalog: ContextItem[]): string {
	return parseComposerText(text, catalog)
		.filter((segment) => segment.kind === 'text')
		.map((segment) => segment.text)
		.join('')
		.replace(/\s+/g, ' ')
		.trim();
}

export type Suggestion = {
	kind: ContextKind;
	token: string;
	label: string;
	detail?: string;
};

/**
 * What the typeahead should offer for the token under the caret. A caret inside a
 * resolved token offers nothing: the item is already chosen, and reopening its
 * list would fight the badge sitting in the text.
 */
export function suggestionsFor(
	text: string,
	caret: number,
	catalog: ContextItem[],
	library: Record<Exclude<ContextKind, 'link'>, Suggestion[]>
): { token: string; start: number; end: number; kind: ContextKind; matches: Suggestion[] } | null {
	const segment = parseComposerText(text, catalog).find(
		(candidate) => candidate.kind === 'pending' && caret >= candidate.start && caret <= candidate.end
	);
	if (!segment || segment.kind !== 'pending' || segment.context === 'link') return null;

	const kind = segment.context;
	const query = segment.text.slice(1).toLowerCase();
	const matches = library[kind].filter((suggestion) => suggestion.token.slice(1).toLowerCase().startsWith(query));
	return matches.length === 0 ? null : { token: segment.text, start: segment.start, end: segment.end, kind, matches };
}

/**
 * Replaces the token under the caret with the chosen suggestion. The trailing
 * space is part of the insertion: without it the next word would fuse into the
 * token and the badge would swallow it.
 */
export function applySuggestion(document: ComposerDocument, start: number, end: number, suggestion: Suggestion): ComposerDocument {
	const item: ContextItem = { token: suggestion.token, kind: suggestion.kind, label: suggestion.label, detail: suggestion.detail };
	const tail = document.text.slice(end);
	// One space after the token so the next word cannot fuse into it, and none when the
	// text already carries whitespace there.
	const separator = tail.length > 0 && /^\s/.test(tail) ? '' : ' ';
	return {
		text: `${document.text.slice(0, start)}${suggestion.token}${separator}${tail}`,
		catalog: document.catalog.some((existing) => existing.token === item.token) ? document.catalog : [...document.catalog, item]
	};
}

/** Inserts an item at the caret and registers it, for menu choices that own their own token. */
export function addItem(document: ComposerDocument, item: ContextItem, at: number): ComposerDocument {
	const tail = document.text.slice(at);
	const separator = tail.length > 0 && /^\s/.test(tail) ? '' : ' ';
	return {
		text: `${document.text.slice(0, at)}${item.token}${separator}${tail}`,
		catalog: document.catalog.some((existing) => existing.token === item.token) ? document.catalog : [...document.catalog, item]
	};
}

/**
 * Deletes a token's own range, plus enough whitespace that the words either side
 * do not fuse. The caret belongs where the token started, which the caller applies
 * after the next tick.
 */
export function removeItem(document: ComposerDocument, token: string): ComposerDocument {
	const segment = parseComposerText(document.text, document.catalog).find(
		(candidate) => candidate.kind === 'item' && candidate.item.token === token
	);
	if (!segment) return document;

	const before = document.text.slice(0, segment.start);
	const after = document.text.slice(segment.end);
	const spacer = `${before.length > 0 && !/\s$/.test(before) ? ' ' : ''}${after.length > 0 && !/^\s/.test(after) ? ' ' : ''}`;

	return {
		text: `${before}${spacer}${after}`,
		catalog: document.catalog.filter((item) => item.token !== token)
	};
}

/** A short human description of a transition, used as the undo and redo tooltip. */
export function describeTransition(before: ComposerDocument, after: ComposerDocument, verb: 'Undo' | 'Redo'): string {
	const beforeItems = itemsInComposer(before.text, before.catalog);
	const afterItems = itemsInComposer(after.text, after.catalog);
	const lost = beforeItems.filter((item) => !afterItems.some((other) => other.token === item.token));
	const gained = afterItems.filter((item) => !beforeItems.some((other) => other.token === item.token));

	if (lost.length === 1) return `${verb} removing ${KIND_LABEL[lost[0].kind].replace(/s$/, '')} ${lost[0].label}`;
	if (lost.length > 1) return `${verb} removing ${lost.length} context items`;
	if (gained.length === 1) return `${verb} adding ${KIND_LABEL[gained[0].kind].replace(/s$/, '')} ${gained[0].label}`;
	if (gained.length > 1) return `${verb} adding ${gained.length} context items`;
	return `${verb} the text edit`;
}