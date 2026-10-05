/**
 * What the composer can offer, and how a chosen thing becomes a token.
 *
 * The vocabulary is the repository's own: capabilities the run stream actually
 * emits, the standing sub-agents EVE knows, the files this page touched. A
 * typeahead full of invented names would be a demo of nothing.
 */

import type { ContextItem, Suggestion } from './composer-model.js';

export type SuggestionLibrary = {
	tool: Suggestion[];
	skill: Suggestion[];
	file: Suggestion[];
	memory: Suggestion[];
	agent: Suggestion[];
};

export const SUGGESTIONS: SuggestionLibrary = {
	tool: [
		{ kind: 'tool', token: '/chat.reply', label: 'chat.reply', detail: 'Stream a reply off EVE' },
		{ kind: 'tool', token: '/note.write', label: 'note.write', detail: 'Record the turn as a note' },
		{ kind: 'tool', token: '/subagent.spawn', label: 'subagent.spawn', detail: 'Fan out to a sub-agent' },
		{ kind: 'tool', token: '/file.read', label: 'file.read', detail: 'Read a file into the turn' },
		{ kind: 'tool', token: '/artifact.write', label: 'artifact.write', detail: 'Emit an inline artifact' },
		{ kind: 'tool', token: '/memory.write', label: 'memory.write', detail: 'Distil a fact into memory' }
	],
	skill: [
		{ kind: 'skill', token: '$scout', label: 'scout', detail: 'Read-only investigation' },
		{ kind: 'skill', token: '$reviewer', label: 'reviewer', detail: 'Quality and security review' },
		{ kind: 'skill', token: '$sonic', label: 'sonic', detail: 'Mechanical updates only' },
		{ kind: 'skill', token: '$before-and-after', label: 'before-and-after', detail: 'Visual comparison of a change' },
		{ kind: 'skill', token: '$browser', label: 'browser', detail: 'Drive a real browser tab' }
	],
	file: [
		{ kind: 'file', token: '#kit.css', label: 'kit.css', detail: '11.6 kB · tokens' },
		{ kind: 'file', token: '#composer-model.ts', label: 'composer-model.ts', detail: 'Composer model' },
		{ kind: 'file', token: '#PRODUCT.md', label: 'PRODUCT.md', detail: 'Product router' },
		{ kind: 'file', token: '#AGENTS.md', label: 'AGENTS.md', detail: 'Repository rules' },
		{ kind: 'file', token: '#package.json', label: 'package.json', detail: 'Manifest' }
	],
	memory: [
		{ kind: 'memory', token: '^release-gate', label: 'release-gate', detail: 'Promote only after build and health check' },
		{ kind: 'memory', token: '^workspace-map', label: 'workspace-map', detail: 'Router files to read first' },
		{ kind: 'memory', token: '^no-host-credentials', label: 'no-host-credentials', detail: 'Secrets resolve at runtime, never in context' },
		{ kind: 'memory', token: '^rtk-prefix', label: 'rtk-prefix', detail: 'Prefix shell commands with rtk' }
	],
	agent: [
		{ kind: 'agent', token: '@EVE', label: 'EVE', detail: 'The standing agent' },
		{ kind: 'agent', token: '@scout', label: 'scout', detail: 'Sub-agent · read-only' },
		{ kind: 'agent', token: '@reviewer', label: 'reviewer', detail: 'Sub-agent · read-only' },
		{ kind: 'agent', token: '@sonic', label: 'sonic', detail: 'Sub-agent · mechanical' }
	]
};

/**
 * Files arrive from a picker, so their names are whatever the machine says. A
 * token cannot contain a space, so the text gets a hyphenated form while the badge
 * keeps the real name.
 */
export function fileItem(name: string, size: string, state: ContextItem['state'] = 'ready'): ContextItem {
	const slug = name.replace(/\.[A-Za-z0-9]+$/, '').replace(/[^A-Za-z0-9_.-]+/g, '-').replace(/-+/g, '-');
	return { token: `#${slug}`, kind: 'file', label: name, detail: size, state };
}