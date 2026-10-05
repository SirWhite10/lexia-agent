<script lang="ts" module>
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	export type DialogProps = {
		open?: boolean;
		title: string;
		description?: string;
		/** Body content. Kept inside the scroll region, never the page. */
		children?: Snippet;
		/** Footer actions, right-aligned. Cancel comes first, confirm last. */
		footer?: Snippet;
		size?: 'small' | 'medium' | 'large';
		class?: string | undefined;
	};
</script>

<script lang="ts">
	/**
	 * The dialog uses the platform `<dialog>` element rather than a hand-rolled
	 * overlay. The browser already supplies what GPUI Kit gets from `FocusTrapElement`
	 * and `window.open_dialog`: modal focus trapping, inertness of the page behind it,
	 * Escape dismissal, and focus restoration to the trigger on close. Re-implementing
	 * those in a div would be strictly worse, so the port uses the platform primitive
	 * and keeps only the presentation.
	 */
	let { open = $bindable(false), title, description, children, footer, size = 'medium', class: className }: DialogProps =
		$props();

	let element = $state<HTMLDialogElement | null>(null);

	$effect(() => {
		const node = element;
		if (!node) return;
		if (open && !node.open) node.showModal();
		else if (!open && node.open) node.close();
	});

	/** `cancel` is the browser's Escape handling; letting it through keeps our own close path single-sourced in `close`. */
	function onCancel(event: Event) {
		event.preventDefault();
		open = false;
	}
</script>

<dialog
	bind:this={element}
	data-kit="dialog"
	class={cn(
		'kit-dialog m-auto w-[calc(100vw-32px)] rounded-(--kit-radius-xl) border border-(--kit-border) bg-(--kit-popover) p-0 text-[color:var(--kit-popover-foreground)] shadow-xl',
		size === 'small' ? 'max-w-[380px]' : size === 'large' ? 'max-w-[760px]' : 'max-w-[520px]',
		className
	)}
	aria-labelledby="kit-dialog-title"
	aria-describedby={description ? 'kit-dialog-description' : undefined}
	oncancel={onCancel}
	onclose={() => (open = false)}
>
	<div class="flex items-start justify-between gap-(--kit-space-lg) p-(--kit-space-xl) pb-(--kit-space-md)">
		<div class="min-w-0">
			<h2 id="kit-dialog-title" class="text-[length:var(--kit-text-lg)] leading-(--kit-leading-lg) font-semibold">{title}</h2>
			{#if description}
				<p id="kit-dialog-description" class="mt-(--kit-space-xs) text-[length:var(--kit-text-sm)] leading-(--kit-leading-sm) text-[color:var(--kit-muted-foreground)]">
					{description}
				</p>
			{/if}
		</div>
		<button
			type="button"
			data-kit="dialog-close"
			class="-mt-(--kit-space-xs) -mr-(--kit-space-sm) grid size-[28px] shrink-0 place-items-center rounded-(--kit-radius-sm) text-[color:var(--kit-muted-foreground)] transition-colors duration-(--kit-duration-fast) hover:bg-(--kit-muted) hover:text-[color:var(--kit-foreground)]"
			onclick={() => (open = false)}
			aria-label="Close {title}"
		>
			<svg viewBox="0 0 16 16" class="size-4" aria-hidden="true">
				<path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" fill="none" />
			</svg>
		</button>
	</div>

	{#if children}
		<div class="kit-scroll max-h-[min(60vh,520px)] overflow-y-auto px-(--kit-space-xl) pb-(--kit-space-lg)">{@render children()}</div>
	{/if}

	{#if footer}
		<div class="flex flex-wrap justify-end gap-(--kit-space-sm) border-t border-(--kit-border) bg-(--kit-muted)/40 p-(--kit-space-md)">
			{@render footer()}
		</div>
	{/if}
</dialog>