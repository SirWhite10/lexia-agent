<script lang="ts" module>
	import type { ContextItem, Suggestion } from './composer-model.js';

	/** Handle the parent uses to insert at the caret and hand focus back. */
	export type ComposerApi = { focus: (position?: number) => void; caret: () => number };

	export type ComposerInlineProps = {
		/**
		 * The field reports every change rather than binding: the parent owns the
		 * document and has to record each change as an undo step.
		 */
		value?: string;
		catalog: ContextItem[];
		placeholder?: string;
		disabled?: boolean;
		/** Enter with no typeahead open sends the turn. */
		onsend?: () => void;
		/** Every keystroke, so the parent can record it as an undo step. */
		onchange?: (text: string) => void;
		/**
		 * A handle for the parent: the menu inserts at the caret and then returns focus
		 * there, so it never has to know how the field stores its selection.
		 */
		onapi?: (api: ComposerApi) => void;
		/** The parent owns the document, so it applies the removal and records the undo step. */
		onremove?: (token: string) => void;
		onaccept?: (suggestion: Suggestion, start: number, end: number) => void;
		onundo?: () => void;
		onredo?: () => void;
		class?: string | undefined;
	};
</script>

<script lang="ts">
	import { tick } from 'svelte';
	import { cn } from '#lib/utils.js';
	import { SUGGESTIONS } from './composer-library.js';
	import { KIND_GLYPH, KIND_LABEL, parseComposerText, suggestionsFor } from './composer-model.js';

	let {
		value = '',
		catalog,
		placeholder = 'Tell Lexia to do something…',
		disabled = false,
		onsend,
		onchange,
		onapi,
		onremove,
		onaccept,
		onundo,
		onredo,
		class: className
	}: ComposerInlineProps = $props();
	$effect(() => {
		onapi?.({
			caret: () => field?.selectionStart ?? 0,
			focus: (position?: number) => {
				void tick().then(() => {
					const node = field;
					if (!node) return;
					node.focus();
					const at = position ?? node.value.length;
					// `setSelectionRange` does not reliably emit a selection event, so the
					// tracked caret is updated here too; otherwise a menu insertion leaves
					// the typeahead closed on a token the user can plainly see.
					caret = at;
					node.setSelectionRange(at, at);
				});
			}
		});
	});

	// Local mirror of the prop: the parent is the only writer (typing, undo, redo,
	// menu insertion), so the field never has to be the source of truth itself.
	let text = $state('');
	$effect(() => {
		if (value !== text) text = value;
	});

	let field = $state<HTMLTextAreaElement | null>(null);
	let mirror = $state<HTMLDivElement | null>(null);
	let active = $state(0);
	let dismissed = $state<string | null>(null);
	// The caret has to live in state: `selectionStart` is a DOM property, so reading it
	// inside a derived value tracks nothing and the typeahead never opens.
	let caret = $state(0);

	/**
	 * The mirror and the textarea must lay text out identically, so both take the
	 * same box, padding and typography from one class string. The textarea owns the
	 * value, the caret and the selection and draws no glyphs; the mirror draws the
	 * glyphs and the badges in the same positions, and takes pointer events only for
	 * the remove buttons sitting above it.
	 */
	const FIELD_CLASS =
		'w-full resize-none bg-transparent px-(--kit-space-sm) py-(--kit-space-xs) text-[length:var(--kit-text-sm)] leading-[length:var(--kit-leading-sm)] break-words whitespace-pre-wrap outline-none';

	const segments = $derived(suggestionsFor(text, caret, catalog, SUGGESTIONS));
	const open = $derived(segments !== null && segments.token !== dismissed);
	const rendered = $derived(parseComposerText(text, catalog));

	/** Grow with the content to a ceiling, then scroll: a composer must never push the conversation off screen. */
	function sync() {
		const node = field;
		if (!node) return;
		node.style.height = 'auto';
		node.style.height = `${Math.min(node.scrollHeight, 160)}px`;
		if (mirror) mirror.scrollTop = node.scrollTop;
		caret = node.selectionStart ?? 0;
	}

	$effect(sync);

	async function accept(match: Suggestion) {
		if (!segments) return;
		const { start, end } = segments;
		onaccept?.(match, start, end);
		await tick();
		const node = field;
		if (!node) return;
		node.focus();
		const caretAt = start + match.token.length + 1;
		node.setSelectionRange(caretAt, caretAt);
		caret = caretAt;
	}

	function onKeydown(event: KeyboardEvent) {
		const list = open ? (segments?.matches ?? []) : [];
		const accel = event.metaKey || event.ctrlKey;

		if (accel && event.key.toLowerCase() === 'z') {
			event.preventDefault();
			event.shiftKey ? onredo?.() : onundo?.();
			return;
		}
		if (list.length > 0) {
			if (event.key === 'ArrowDown') {
				event.preventDefault();
				active = (active + 1) % list.length;
				return;
			}
			if (event.key === 'ArrowUp') {
				event.preventDefault();
				active = (active - 1 + list.length) % list.length;
				return;
			}
			// Enter accepts. Tab is left alone on purpose: trapping it would take the
			// typeahead out of reach of anyone who navigates by keyboard.
			if (event.key === 'Enter') {
				event.preventDefault();
				accept(list[active]);
				return;
			}
			if (event.key === 'Escape') {
				event.preventDefault();
				dismissed = segments?.token ?? null;
				return;
			}
		}
		if (event.key === 'Enter' && !event.shiftKey && !event.altKey && !accel) {
			event.preventDefault();
			onsend?.();
		}
	}
</script>

<div class={cn('relative min-w-0 flex-1', className)}>
	{#if open && segments}
		<div
			data-kit="composer-typeahead"
			role="listbox"
			aria-label={KIND_LABEL[segments.kind]}
			class="kit-rise absolute bottom-full left-0 z-30 mb-(--kit-space-xs) w-full min-w-[280px] max-w-[420px] overflow-hidden rounded-(--kit-radius-lg) border border-(--kit-border) bg-(--kit-popover) p-(--kit-space-xxs) shadow-xl"
		>
			<!-- Both handlers: mousedown keeps focus in the field, click covers touch and any
			     pointer path that never produces a mousedown. `accept` no-ops once the token
			     is gone, so the second one cannot double-apply. -->
			{#each segments.matches as match, index (match.token)}
				<button
					type="button"
					role="option"
					aria-selected={index === active}
					data-kit="composer-suggestion"
					onmousedown={(event) => {
						event.preventDefault();
						accept(match);
					}}
					onmouseenter={() => (active = index)}
					onclick={() => accept(match)}
					class="flex w-full items-center gap-(--kit-space-sm) rounded-(--kit-radius-sm) px-(--kit-space-sm) py-(--kit-space-xs) text-left"
					class:bg-(--kit-accent)={index === active}
				>
					<span
						class="grid size-[20px] shrink-0 place-items-center rounded-(--kit-radius-sm) bg-(--kit-muted) text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]"
						aria-hidden="true">{KIND_GLYPH[match.kind]}</span
					>
					<span class="min-w-0 flex-1">
						<span class="block truncate text-[length:var(--kit-text-sm)] leading-[length:var(--kit-leading-sm)]">{match.label}</span>
						{#if match.detail}
							<span class="block truncate text-[length:var(--kit-text-xs)] leading-[length:var(--kit-leading-xs)] text-[color:var(--kit-muted-foreground)]"
								>{match.detail}</span
							>
						{/if}
					</span>
					<span class="shrink-0 font-mono text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">{match.token}</span>
				</button>
			{/each}
		</div>
	{/if}

	<!-- aria-hidden: the textarea owns the accessible value. This layer draws glyphs and
	     badges only; it is pointer-transparent apart from the remove buttons. -->
	<div
		bind:this={mirror}
		aria-hidden="true"
		class="pointer-events-none absolute inset-0 z-10 max-h-40 overflow-hidden text-[color:var(--kit-foreground)] select-none"
	>
		<div class={FIELD_CLASS}>
			{#if text.length === 0}
				<span class="text-[color:var(--kit-muted-foreground)]">{placeholder}</span>
			{:else}
				{#each rendered as segment, index (index)}
					{#if segment.kind === 'text'}
						{segment.text}
					{:else if segment.kind === 'pending'}
						<span class="rounded-(--kit-radius-sm) bg-(--kit-muted) px-1">{segment.text}</span>
					{:else}
						<span
							data-kit="composer-badge"
							class="relative mx-px inline-flex translate-y-[1px] items-center gap-1 rounded-(--kit-radius-sm) bg-(--kit-accent) px-1.5 py-px align-baseline text-[length:var(--kit-text-xs)] leading-[length:var(--kit-leading-xs)] text-[color:var(--kit-accent-foreground)]"
						>
							<span class="opacity-70" aria-hidden="true">{KIND_GLYPH[segment.item.kind]}</span>
							{segment.item.label}
							<button
								type="button"
								tabindex="-1"
								aria-label="Remove {KIND_LABEL[segment.item.kind].replace(/s$/, '')} {segment.item.label}"
								onclick={() => onremove?.(segment.item.token)}
								class="pointer-events-auto absolute -top-[7px] -right-[7px] grid size-[14px] place-items-center rounded-full bg-[color:var(--kit-popover)] text-[length:var(--kit-text-xs)] leading-none text-[color:var(--kit-muted-foreground)] shadow-xs ring-1 ring-(--kit-border) transition-colors duration-(--kit-duration-fast) hover:text-[color:var(--kit-danger)]"
							>
								<span aria-hidden="true">×</span>
							</button>
						</span>
					{/if}
				{/each}
			{/if}
		</div>
	</div>

	<textarea
		bind:this={field}
		value={text}
		oninput={(event) => {
			text = event.currentTarget.value;
			onchange?.(text);
			sync();
		}}
		{placeholder}
		{disabled}
		rows="1"
		aria-label="Message Lexia. Type / for tools, $ for skills, # for files, ^ for memory, @ for agents."
		onscroll={sync}
		onclick={sync}
		onkeyup={sync}
		onselect={sync}
		onkeydown={onKeydown}
		class={cn(
			FIELD_CLASS,
			'relative block max-h-40 text-transparent caret-[color:var(--kit-foreground)] selection:bg-(--kit-selection) selection:text-transparent',
			'disabled:opacity-60'
		)}
	></textarea>
</div>