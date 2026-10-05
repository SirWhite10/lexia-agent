<script lang="ts" module>
	export type KitSkeletonProps = {
		width?: string;
		height?: string;
		radius?: string;
		/** Renders a stacked text block instead of a single bar. */
		lines?: number;
		/** `skeleton.rs::secondary` halves the opacity for nested placeholders. */
		secondary?: boolean;
		class?: string;
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let {
		width,
		height,
		radius,
		lines,
		secondary = false,
		class: className,
	}: KitSkeletonProps = $props();

	const count = $derived(Math.max(0, Math.floor(lines ?? 0)));
</script>

{#if count > 0}
	<div data-kit="skeleton" class={cn('flex w-full flex-col gap-(--kit-space-sm)', className)}>
		{#each Array.from({ length: count }, (_, i) => i) as line (line)}
			<div
				class={cn(
					'kit-pulse bg-(--kit-skeleton)',
					// The last line of a paragraph is short; matching that keeps the
					// block reading as text rather than as a stack of rules.
					line === count - 1 ? 'w-3/5' : 'w-full',
					secondary && 'opacity-50',
				)}
				style:height={height ?? 'var(--kit-text-lg)'}
				style:border-radius={radius ?? 'var(--kit-radius-sm)'}
				aria-hidden="true"
			></div>
		{/each}
	</div>
{:else}
	<div
		data-kit="skeleton"
		class={cn('kit-pulse bg-(--kit-skeleton)', secondary && 'opacity-50', className)}
		style:width={width ?? '100%'}
		style:height={height ?? 'var(--kit-text-lg)'}
		style:border-radius={radius ?? 'var(--kit-radius-sm)'}
		aria-hidden="true"
	></div>
{/if}
