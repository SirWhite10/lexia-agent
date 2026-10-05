<script lang="ts" module>
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	export type SheetProps = {
		open?: boolean;
		title: string;
		/**
		 * `bottom` is the phone shape: a sheet that rises from the bottom edge with a
		 * drag handle affordance. `left` and `right` are the desktop side panels.
		 */
		side?: 'left' | 'right' | 'bottom';
		description?: string;
		children?: Snippet;
		footer?: Snippet;
		/** Width for the side variants; ignored for `bottom`, which is full-bleed. */
		width?: string;
		class?: string | undefined;
	};
</script>

<script lang="ts">
	/**
	 * Same decision as `dialog.svelte`: the platform `<dialog>` supplies the focus trap,
	 * Escape dismissal and focus restoration. Only the edge-anchored presentation and its
	 * motion are ours, because that is the part `gpui-component/src/sheet.rs` owns.
	 */
	let {
		open = $bindable(false),
		title,
		side = 'right',
		description,
		children,
		footer,
		width = '320px',
		class: className
	}: SheetProps = $props();

	let element = $state<HTMLDialogElement | null>(null);

	$effect(() => {
		const node = element;
		if (!node) return;
		if (open && !node.open) node.showModal();
		else if (!open && node.open) node.close();
	});

	function onCancel(event: Event) {
		event.preventDefault();
		open = false;
	}
</script>

<dialog
	bind:this={element}
	data-kit="sheet"
	data-side={side}
	class={cn(
		'kit-sheet m-0 max-h-none bg-(--kit-popover) p-0 text-[color:var(--kit-popover-foreground)]',
		side === 'bottom'
			? 'mt-auto max-w-none border-0 border-t border-(--kit-border)'
			: side === 'left'
				? 'h-full max-h-full border-0 border-r border-(--kit-border)'
				: 'mt-0 mr-0 h-full max-h-full border-0 border-l border-(--kit-border)',
		className
	)}
	style={side === 'bottom' ? undefined : `width: min(${width}, 100vw)`}
	aria-labelledby="kit-sheet-title"
	aria-describedby={description ? 'kit-sheet-description' : undefined}
	oncancel={onCancel}
	onclose={() => (open = false)}
>
	{#if side === 'bottom'}
		<div class="mx-auto mt-(--kit-space-sm) h-1 w-10 shrink-0 rounded-(--kit-radius-full) bg-(--kit-border)" aria-hidden="true"></div>
	{/if}

	<div class="flex items-center justify-between gap-(--kit-space-md) border-b border-(--kit-border) px-(--kit-space-lg) py-(--kit-space-md)">
		<div class="min-w-0">
			<h2 id="kit-sheet-title" class="truncate text-[length:var(--kit-text-md)] leading-(--kit-leading-md) font-semibold">{title}</h2>
			{#if description}
				<p id="kit-sheet-description" class="truncate text-[length:var(--kit-text-xs)] leading-(--kit-leading-xs) text-[color:var(--kit-muted-foreground)]">
					{description}
				</p>
			{/if}
		</div>
		<button
			type="button"
			data-kit="sheet-close"
			class="grid size-[28px] shrink-0 place-items-center rounded-(--kit-radius-sm) text-[color:var(--kit-muted-foreground)] transition-colors duration-(--kit-duration-fast) hover:bg-(--kit-muted) hover:text-[color:var(--kit-foreground)]"
			onclick={() => (open = false)}
			aria-label="Close {title}"
		>
			<svg viewBox="0 0 16 16" class="size-4" aria-hidden="true">
				<path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" fill="none" />
			</svg>
		</button>
	</div>

	{#if children}
		<div class="kit-scroll flex-1 overflow-y-auto px-(--kit-space-lg) py-(--kit-space-md)">{@render children()}</div>
	{/if}

	{#if footer}
		<div class="flex flex-wrap gap-(--kit-space-sm) border-t border-(--kit-border) bg-(--kit-muted)/40 p-(--kit-space-md)">
			{@render footer()}
		</div>
	{/if}
</dialog>