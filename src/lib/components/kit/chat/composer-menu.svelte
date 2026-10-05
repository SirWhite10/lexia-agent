<script lang="ts" module>
	import type { ContextKind } from './composer-model.js';

	export type ComposerMenuProps = {
		/**
		 * Insert text at the caret and hand focus back to the field, so the inline
		 * typeahead opens on the token that was just inserted.
		 */
		oninsert?: (token: string) => void;
		onfiles?: (files: FileList) => void;
		/** Demo-only: exercises the uploading state without a real upload. */
		onsample?: () => void;
		disabled?: boolean;
	};

	type Entry = {
		id: string;
		label: string;
		hint: string;
		glyph: string;
		/** Text placed in the field; `null` opens the file picker instead. */
		token: string | null;
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let { oninsert, onfiles, onsample, disabled = false }: ComposerMenuProps = $props();

	let open = $state(false);
	let trigger = $state<HTMLButtonElement | null>(null);
	let items = $state<HTMLButtonElement[]>([]);
	let active = $state(0);
	let picker = $state<HTMLInputElement | null>(null);

	const ENTRIES: Entry[] = [
		{ id: 'attachments', label: 'Attachments', hint: 'Add files to this turn', glyph: '📎', token: null },
		{ id: 'link', label: 'Link', hint: 'Reference a URL as context', glyph: '↗', token: 'https://' },
		{ id: 'tool', label: 'Tools', hint: 'Run a capability in this turn', glyph: '⚙', token: '/' },
		{ id: 'skill', label: 'Skills', hint: 'Apply a skill', glyph: '$', token: '$' },
		{ id: 'file', label: 'Files', hint: 'Bring a repository file in', glyph: '#', token: '#' },
		{ id: 'memory', label: 'Memory', hint: 'Recall something stored', glyph: '^', token: '^' },
		{ id: 'agent', label: 'Agents', hint: 'Address a sub-agent', glyph: '@', token: '@' }
	];

	function toggle() {
		open = !open;
		active = 0;
	}

	function choose(entry: Entry) {
		open = false;
		trigger?.focus();
		if (entry.token === null) picker?.click();
		else oninsert?.(entry.token);
	}

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			open = false;
			trigger?.focus();
			return;
		}
		if (event.key === 'ArrowDown') {
			event.preventDefault();
			active = (active + 1) % ENTRIES.length;
			items[active]?.focus();
			return;
		}
		if (event.key === 'ArrowUp') {
			event.preventDefault();
			active = (active - 1 + ENTRIES.length) % ENTRIES.length;
			items[active]?.focus();
		}
	}

	/** The trigger's own arrow keys open the menu, so it is reachable without a pointer. */
	function onTriggerKeydown(event: KeyboardEvent) {
		if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
			event.preventDefault();
			open = true;
			active = event.key === 'ArrowDown' ? 0 : ENTRIES.length - 1;
			queueMicrotask(() => items[active]?.focus());
		}
	}
</script>

<div class="relative shrink-0">
	<button
		bind:this={trigger}
		type="button"
		data-kit="composer-menu-trigger"
		class="grid size-[32px] place-items-center rounded-(--kit-radius-md) text-[color:var(--kit-muted-foreground)] transition-colors duration-(--kit-duration-fast) hover:bg-(--kit-muted) hover:text-[color:var(--kit-foreground)] disabled:opacity-50"
		{disabled}
		aria-haspopup="menu"
		aria-expanded={open}
		aria-label="Add to this turn's context"
		onclick={toggle}
		onkeydown={onTriggerKeydown}
	>
		<svg viewBox="0 0 16 16" class="size-4" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round">
			<path d="M8 3.5v9M3.5 8h9" />
		</svg>
	</button>

	{#if open}
		<!-- Click-away and Escape both close this surface; Escape returns focus to the trigger. -->
		<button
			type="button"
			class="fixed inset-0 z-20 cursor-default"
			aria-hidden="true"
			tabindex="-1"
			onclick={() => {
				open = false;
				trigger?.focus();
			}}
		></button>

		<div
			tabindex="-1"
			data-kit="composer-menu"
			role="menu"
			aria-label="Add to context"
			onkeydown={onKeydown}
			class="kit-rise absolute bottom-full left-0 z-30 mb-(--kit-space-xs) w-[300px] overflow-hidden rounded-(--kit-radius-lg) border border-(--kit-border) bg-(--kit-popover) p-(--kit-space-xxs) shadow-xl"
		>
			{#each ENTRIES as entry, index (entry.id)}
				<button
					bind:this={items[index]}
					type="button"
					role="menuitem"
					data-kit="composer-menu-item"
					onclick={() => choose(entry)}
					onmouseenter={() => (active = index)}
					class="flex w-full items-center gap-(--kit-space-sm) rounded-(--kit-radius-sm) px-(--kit-space-sm) py-(--kit-space-xs) text-left"
					class:bg-(--kit-accent)={index === active}
				>
					<span
						class="grid size-[24px] shrink-0 place-items-center rounded-(--kit-radius-sm) bg-(--kit-muted) text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]"
						aria-hidden="true">{entry.glyph}</span
					>
					<span class="min-w-0 flex-1">
						<span class="block truncate text-[length:var(--kit-text-sm)] leading-[length:var(--kit-leading-sm)]">{entry.label}</span>
						<span class="block truncate text-[length:var(--kit-text-xs)] leading-[length:var(--kit-leading-xs)] text-[color:var(--kit-muted-foreground)]"
							>{entry.hint}</span
						>
					</span>
					{#if entry.token}
						<span class="shrink-0 font-mono text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">{entry.token}</span>
					{/if}
				</button>
			{/each}

			{#if onsample}
				<div class="my-(--kit-space-xxs) border-t border-(--kit-border)"></div>
				<button
					type="button"
					role="menuitem"
					data-kit="composer-menu-sample"
					onclick={() => {
						open = false;
						trigger?.focus();
						onsample?.();
					}}
					class="flex w-full items-center gap-(--kit-space-sm) rounded-(--kit-radius-sm) px-(--kit-space-sm) py-(--kit-space-xs) text-left hover:bg-(--kit-accent)"
				>
					<span class="min-w-0 flex-1">
						<span class="block text-[length:var(--kit-text-xs)] leading-[length:var(--kit-leading-xs)]">Sample upload</span>
						<span class="block text-[length:var(--kit-text-xs)] leading-[length:var(--kit-leading-xs)] text-[color:var(--kit-muted-foreground)]"
							>Exercise the uploading state</span
						>
					</span>
				</button>
			{/if}
		</div>
	{/if}
</div>

<input
	bind:this={picker}
	type="file"
	multiple
	class="sr-only"
	aria-hidden="true"
	tabindex="-1"
	onchange={(event) => {
		const input = event.currentTarget;
		if (input.files && input.files.length > 0) onfiles?.(input.files);
		input.value = '';
	}}
/>