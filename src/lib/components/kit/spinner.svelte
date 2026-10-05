<script lang="ts" module>
	import type { Snippet } from 'svelte';

	export type KitSpinnerSize = 'sm' | 'md' | 'lg' | 'xl';

	export type KitSpinnerProps = {
		size?: KitSpinnerSize;
		/** Announced to assistive tech; the ring itself stays decorative. */
		label?: Snippet;
		class?: string;
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let { size = 'md', label, class: className }: KitSpinnerProps = $props();

	const SIZE_CLASS: Record<KitSpinnerSize, string> = {
		sm: 'size-3 border-[1.5px]',
		md: 'size-4 border-2',
		lg: 'size-5 border-2',
		xl: 'size-6 border-2',
	};
</script>

<span data-kit="spinner" class={cn('inline-flex shrink-0', className)}>
	<span
		role="status"
		aria-live="polite"
		class={cn(
			// GPUI draws a rotating icon; the browser port uses the CSS ring the
			// `kit-spin` keyframe in kit.css already drives.
			'kit-spin inline-block rounded-(--kit-radius-full) border-(--kit-border) border-t-(--kit-primary)',
			SIZE_CLASS[size],
		)}
	>
		{#if label}
			<span class="sr-only">{@render label()}</span>
		{/if}
	</span>
</span>
