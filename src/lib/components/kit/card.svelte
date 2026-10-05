<script lang="ts" module>
	/**
	 * Port of the card surface GPUI Kit builds from `tokens.card` and
	 * `tokens.border`.
	 *
	 * The crate has no standalone `Card` component — a card is `tokens.card` with a
	 * 1px `border` at `radius_lg` (8px), which is the same recipe
	 * `DataTable::render` uses for its bordered frame (table/data_table.rs:168).
	 * The header is separated from the body by a hairline rather than a second
	 * nested surface, and the footer by the same hairline, so a card reads as one
	 * object instead of three stacked boxes.
	 */

	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';

	export type KitCardPadding = 'none' | 'sm' | 'md' | 'lg';

	export type KitCardProps = Omit<HTMLAttributes<HTMLDivElement>, 'children'> & {
		title?: string;
		description?: string;
		/** The card body. Omit it for a card that is only a header. */
		children?: Snippet;
		/** Pinned under the body, behind its own hairline. */
		footer?: Snippet;
		padding?: KitCardPadding;
		/** A card that is itself a link target: it lifts on hover and focus-within. */
		interactive?: boolean;
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let {
		title,
		description,
		children,
		footer,
		padding = 'md',
		interactive = false,
		class: className,
		...restProps
	}: KitCardProps = $props();

	const BODY_PADDING: Record<KitCardPadding, string> = {
		none: '',
		sm: 'p-(--kit-space-sm)',
		md: 'p-(--kit-space-md)',
		lg: 'p-(--kit-space-lg)',
	};

	const hasHeader = $derived(title !== undefined || description !== undefined);
	const hasFooter = $derived(footer !== undefined);
</script>

<div
	data-kit="card"
	class={cn(
		'flex w-full min-w-0 flex-col rounded-(--kit-radius-lg) border border-(--kit-border) bg-(--kit-card)',
		'text-[length:var(--kit-text-sm)] text-[color:var(--kit-card-foreground)]',
		interactive && [
			'transition-[border-color,background-color] duration-(--kit-duration-fast)',
			'hover:border-(--kit-ring) hover:bg-(--kit-list-hover)',
			'focus-within:border-(--kit-ring) focus-within:bg-(--kit-list-hover)',
		],
		className,
	)}
	{...restProps}
>
	{#if hasHeader}
		<div class="flex flex-col gap-(--kit-space-xxs) px-(--kit-space-md) py-(--kit-space-sm)">
			{#if title}
				<h3 class="m-0 text-[length:var(--kit-text-md)] font-medium leading-(--kit-leading-md)">
					{title}
				</h3>
			{/if}
			{#if description}
				<p class="m-0 text-[length:var(--kit-text-sm)] text-[color:var(--kit-muted-foreground)]">
					{description}
				</p>
			{/if}
		</div>
		{#if children}
			<div class="h-px shrink-0 bg-(--kit-border)" aria-hidden="true"></div>
		{/if}
	{/if}

	{#if children}
		<div class={cn('min-w-0', BODY_PADDING[padding])}>
			{@render children()}
		</div>
	{/if}

	{#if hasFooter}
		<div class="mt-auto">
			<div class="h-px bg-(--kit-border)" aria-hidden="true"></div>
			<div class="px-(--kit-space-md) py-(--kit-space-sm)">
				{@render footer?.()}
			</div>
		</div>
	{/if}
</div>
