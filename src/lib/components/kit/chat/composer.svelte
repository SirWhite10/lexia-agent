<script lang="ts" module>
	import type { ContextItem } from './composer-model.js';

	export type ComposerSend = { text: string; items: ContextItem[]; catalog: ContextItem[] };
	export type ComposerProps = {
		/** Blocks sending and says why, instead of letting a turn look like a silent no-op. */
		blockedReason?: string | null;
		/** A run is live, so the send control becomes stop. */
		running?: boolean;
		onsend?: (payload: ComposerSend) => void;
		onstop?: () => void;
		hint?: import('svelte').Snippet;
		class?: string | undefined;
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';
	import Attachment from '../attachment.svelte';
	import Button from '../button.svelte';
	import Tooltip from '../tooltip.svelte';
	import ComposerInline, { type ComposerApi } from './composer-inline.svelte';
	import ComposerMenu from './composer-menu.svelte';
	import { fileItem } from './composer-library.js';
	import {
		KIND_GLYPH,
		KIND_LABEL,
		addItem,
		applySuggestion,
		describeTransition,
		itemsInComposer,
		removeItem,
		type ComposerDocument,
		type Suggestion
	} from './composer-model.js';

	let { blockedReason = null, running = false, onsend, onstop, hint, class: className }: ComposerProps = $props();

	let doc = $state<ComposerDocument>({ text: '', catalog: [] });
	let past = $state<ComposerDocument[]>([]);
	let future = $state<ComposerDocument[]>([]);
	let undoLabel = $state<string | null>(null);
	let redoLabel = $state<string | null>(null);
	let lastTypingAt = 0;
	let fieldApi = $state<ComposerApi | null>(null);

	const items = $derived(itemsInComposer(doc.text, doc.catalog));
	const canSend = $derived(doc.text.trim().length > 0 && !blockedReason);
	const fileItems = $derived(items.filter((item) => item.kind === 'file'));
	const chipItems = $derived(items.filter((item) => item.kind !== 'file'));

	/**
	 * Every change is recorded, but consecutive typing coalesces into one step: undo
	 * should take back the sentence you just typed, not one character at a time, and
	 * it should never merge an item removal into the typing that preceded it.
	 */
	function record(next: ComposerDocument, kind: 'typing' | 'structural') {
		if (next.text === doc.text && next.catalog.length === doc.catalog.length) return;
		const now = Date.now();
		const coalesce = kind === 'typing' && now - lastTypingAt < 700 && past.length > 0;
		lastTypingAt = kind === 'typing' ? now : 0;
		undoLabel = describeTransition(doc, next, 'Undo');
		redoLabel = null;
		past = coalesce ? [...past.slice(0, -1), doc] : [...past, doc];
		future = [];
		doc = next;
	}

	function undo() {
		const previous = past[past.length - 1];
		if (!previous) return;
		redoLabel = describeTransition(previous, doc, 'Redo');
		undoLabel = past.length > 1 ? describeTransition(past[past.length - 2], previous, 'Undo') : null;
		future = [doc, ...future];
		past = past.slice(0, -1);
		doc = previous;
		fieldApi?.focus();
	}

	function redo() {
		const next = future[0];
		if (!next) return;
		undoLabel = describeTransition(doc, next, 'Undo');
		redoLabel = future.length > 1 ? describeTransition(next, future[1], 'Redo') : null;
		past = [...past, doc];
		future = future.slice(1);
		doc = next;
		fieldApi?.focus();
	}

	/**
	 * Menu choices drop their trigger into the text so the inline typeahead opens on it.
	 * The token is separated from the words around it, because a token only counts as
	 * one when it starts the text or follows whitespace.
	 */
	function insert(token: string) {
		const at = fieldApi?.caret() ?? doc.text.length;
		const head = doc.text.slice(0, at);
		const tail = doc.text.slice(at);
		const lead = head.length > 0 && !/\s$/.test(head) ? ' ' : '';
		const trail = tail.length > 0 && !/^\s/.test(tail) ? ' ' : '';
		record({ text: `${head}${lead}${token}${trail}${tail}`, catalog: doc.catalog }, 'structural');
		fieldApi?.focus(at + lead.length + token.length);
	}

	function accept(suggestion: Suggestion, start: number, end: number) {
		record(applySuggestion(doc, start, end, suggestion), 'structural');
	}

	function drop(token: string) {
		record(removeItem(doc, token), 'structural');
		fieldApi?.focus();
	}

	function attach(files: FileList) {
		let next = doc;
		for (const file of Array.from(files)) {
			next = addItem(next, fileItem(file.name, `${Math.max(1, Math.round(file.size / 1024))} kB`, 'ready'), fieldApi?.caret() ?? next.text.length);
		}
		record(next, 'structural');
		fieldApi?.focus();
	}

	/** Demo-only: exercises the uploading state without a real upload. */
	function sample() {
		record(addItem(doc, { token: '#brief-pdf', kind: 'file', label: 'brief.pdf', detail: '184 kB', state: 'uploading' }, fieldApi?.caret() ?? doc.text.length), 'structural');
		fieldApi?.focus();
	}

	function send() {
		if (!canSend) return;
		onsend?.({ text: doc.text.trim(), items, catalog: doc.catalog });
		doc = { text: '', catalog: [] };
		past = [];
		future = [];
		undoLabel = null;
		redoLabel = null;
		fieldApi?.focus(0);
	}
</script>

<div
	data-kit="composer"
	class={cn(
		'border-t border-(--kit-border) bg-(--kit-background) px-(--kit-space-md) pt-(--kit-space-sm) pb-[calc(var(--kit-space-md)+env(safe-area-inset-bottom))] md:px-(--kit-space-xl) md:pb-(--kit-space-lg)',
		className
	)}
>
	<!-- Everything currently in the turn, above the field, for quick view. Rendered from the
	     same parse as the inline badges, so the two can never disagree. -->
	{#if items.length > 0}
		<ul class="mb-(--kit-space-sm) flex flex-wrap items-start gap-(--kit-space-sm)" aria-label="Context added to this turn">
			{#each fileItems as item (item.token)}
				<li class="w-[220px]">
					<Attachment
						name={item.label}
						kind="file"
						size={item.detail}
						state={item.state ?? 'ready'}
						onremove={() => drop(item.token)}
					/>
				</li>
			{/each}
			{#each chipItems as item (item.token)}
				<li class="flex items-center gap-(--kit-space-xs) rounded-(--kit-radius-md) border border-(--kit-border) bg-(--kit-card) py-[3px] pr-[3px] pl-(--kit-space-sm) text-[length:var(--kit-text-xs)] leading-[length:var(--kit-leading-xs)]">
					<span class="text-[color:var(--kit-muted-foreground)]" aria-hidden="true">{KIND_GLYPH[item.kind]}</span>
					<span class="font-medium">{item.label}</span>
					{#if item.detail}
						<span class="max-w-[180px] truncate text-[color:var(--kit-muted-foreground)]">{item.detail}</span>
					{/if}
					<button
						type="button"
						data-kit="composer-remove"
						aria-label="Remove {KIND_LABEL[item.kind].replace(/s$/, '')} {item.label}"
						onclick={() => drop(item.token)}
						class="grid size-[18px] place-items-center rounded-full text-[color:var(--kit-muted-foreground)] transition-colors duration-(--kit-duration-fast) hover:bg-(--kit-muted) hover:text-[color:var(--kit-danger)]"
					>
						<span aria-hidden="true">×</span>
					</button>
				</li>
			{/each}
		</ul>
	{/if}

	<!-- Undo and redo sit above the field on the right: removing a context item or a
	     paragraph by accident should cost one keystroke, not a retype. -->
	<div class="mb-(--kit-space-xs) flex items-center justify-between gap-(--kit-space-sm)">
		<p class="truncate text-[length:var(--kit-text-xs)] leading-[length:var(--kit-leading-xs)] text-[color:var(--kit-muted-foreground)]">
			{items.length === 0
				? 'Type / $ # ^ @ to add context'
				: `${items.length} ${items.length === 1 ? 'item' : 'items'} in context`}
		</p>
		<div class="flex items-center gap-(--kit-space-xxs)">
			<Tooltip label={undoLabel ?? 'Nothing to undo'}>
				<Button variant="ghost" size="xsmall" iconOnly onclick={undo} disabled={past.length === 0} aria-label="Undo">
					<svg viewBox="0 0 16 16" class="size-3.5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
						<path d="M6 4L3 7l3 3M3 7h6.5A3.5 3.5 0 0 1 13 10.5A3.5 3.5 0 0 1 9.5 14" />
					</svg>
				</Button>
			</Tooltip>
			<Tooltip label={redoLabel ?? 'Nothing to redo'}>
				<Button variant="ghost" size="xsmall" iconOnly onclick={redo} disabled={future.length === 0} aria-label="Redo">
					<svg viewBox="0 0 16 16" class="size-3.5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
						<path d="M10 4l3 3-3 3M13 7H6.5A3.5 3.5 0 0 0 3 10.5A3.5 3.5 0 0 0 6.5 14" />
					</svg>
				</Button>
			</Tooltip>
		</div>
	</div>

	<form
		class="flex items-center gap-(--kit-space-xs) rounded-(--kit-radius-xl) border border-(--kit-border) bg-(--kit-card) p-(--kit-space-xs) transition-colors duration-(--kit-duration-fast) focus-within:border-(--kit-ring)"
		onsubmit={(event) => {
			event.preventDefault();
			send();
		}}
	>
		<ComposerMenu oninsert={insert} onfiles={attach} disabled={running} onsample={sample} />

		<ComposerInline
			value={doc.text}
			catalog={doc.catalog}
			placeholder={blockedReason ?? 'Tell Lexia to do something…'}
			onsend={send}
			onapi={(api) => (fieldApi = api)}
			onchange={(text) => record({ ...doc, text }, 'typing')}
			onaccept={accept}
			onremove={drop}
			onundo={undo}
			onredo={redo}
		/>

		{#if running}
			<Button variant="danger" size="medium" onclick={() => onstop?.()}>
				<svg viewBox="0 0 16 16" class="size-3.5" aria-hidden="true">
					<rect x="4" y="4" width="8" height="8" rx="1" fill="currentColor" />
				</svg>
				Stop
			</Button>
		{:else}
			<Button variant="primary" size="medium" type="submit" disabled={!canSend}>
				Send
				<svg viewBox="0 0 16 16" class="size-3.5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
					<path d="M3 8h9m0 0l-3.5-3.5M12 8l-3.5 3.5" />
				</svg>
			</Button>
		{/if}
	</form>

	{#if blockedReason}
		<p class="mt-(--kit-space-sm) text-[length:var(--kit-text-xs)] leading-[length:var(--kit-leading-xs)] text-[color:var(--kit-danger)]">{blockedReason}</p>
	{:else if hint}
		<div class="mt-(--kit-space-sm) flex flex-wrap items-center gap-(--kit-space-xs) text-[length:var(--kit-text-xs)] leading-[length:var(--kit-leading-xs)] text-[color:var(--kit-muted-foreground)]">
			{@render hint()}
		</div>
	{/if}
</div>