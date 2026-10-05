<script lang="ts" module>
	import type { Snippet } from 'svelte';

	export type KitProgressSize = 'sm' | 'md';

	export type KitProgressTone = 'primary' | 'success' | 'danger';

	export type KitProgressProps = {
		/** Percentage 0-100, or `null` while the amount is unknown. */
		value: number | null;
		size?: KitProgressSize;
		/** Visible caption above the bar. */
		label?: Snippet;
		/** Accessible name; also announced in place of a value while unknown. */
		ariaLabel?: string;
		tone?: KitProgressTone;
		class?: string;
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let {
		value,
		size = 'md',
		label,
		ariaLabel,
		tone = 'primary',
		class: className,
	}: KitProgressProps = $props();

	const indeterminate = $derived(value === null);
	const percent = $derived(
		indeterminate ? 0 : Math.min(100, Math.max(0, Math.round(value as number))),
	);

	// `progress.rs` heights are Small 6px and Medium 8px, each rounded to a pill
	// of half its own height; the track is the fill colour at 20%.
	const SIZE_CLASS: Record<KitProgressSize, string> = {
		sm: 'h-1.5',
		md: 'h-2',
	};

	const FILL_CLASS: Record<KitProgressTone, string> = {
		primary: 'bg-(--kit-primary)',
		success: 'bg-(--kit-success)',
		danger: 'bg-(--kit-danger)',
	};

	const TRACK_CLASS: Record<KitProgressTone, string> = {
		primary: 'bg-[color-mix(in_oklab,var(--kit-primary)_20%,var(--kit-background))]',
		success: 'bg-[color-mix(in_oklab,var(--kit-success)_20%,var(--kit-background))]',
		danger: 'bg-[color-mix(in_oklab,var(--kit-danger)_20%,var(--kit-background))]',
	};
</script>

{#if label}
	<p class="mb-(--kit-space-xs) text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">
		{@render label()}
	</p>
{/if}

<div
	data-kit="progress"
	role="progressbar"
	aria-label={ariaLabel}
	aria-valuemin={0}
	aria-valuemax={100}
	aria-valuenow={indeterminate ? undefined : percent}
	aria-valuetext={indeterminate ? (ariaLabel ?? 'Working…') : undefined}
	class={cn(
		'relative w-full overflow-hidden rounded-(--kit-radius-full)',
		SIZE_CLASS[size],
		TRACK_CLASS[tone],
		className,
	)}
>
	{#if indeterminate}
		<div
			class={cn(
				'kit-progress-slide absolute inset-y-0 left-0 w-1/3 rounded-(--kit-radius-full)',
				FILL_CLASS[tone],
			)}
		></div>
	{:else}
		<div
			class={cn(
				'h-full rounded-(--kit-radius-full)',
				'transition-[width] duration-(--kit-duration-normal) ease-(--kit-ease-move)',
				FILL_CLASS[tone],
			)}
			style:width={`${percent}%`}
		></div>
	{/if}
</div>

<style>
	/* The crate slides the indicator between a quarter inset on each side; a
	   CSS keyframe is the browser equivalent, and under reduced motion it comes
	   to rest on that same segment rather than vanishing. */
	.kit-progress-slide {
		animation: kit-progress-slide var(--kit-duration-slow) var(--kit-ease-move) infinite;
	}

	@keyframes kit-progress-slide {
		from {
			transform: translateX(-100%);
		}
		to {
			transform: translateX(400%);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.kit-progress-slide {
			animation: none;
			transform: translateX(125%);
		}
	}
</style>
