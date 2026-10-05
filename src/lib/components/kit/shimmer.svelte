<script lang="ts" module>
	export type KitShimmerProps = {
		text: string;
		/** While tokens are still arriving: sweeping highlight and blinking caret. */
		pending?: boolean;
		class?: string;
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let { text, pending = true, class: className }: KitShimmerProps = $props();
</script>

<!--
	The crate composites a moving band over the glyphs themselves. In a browser
	that means clipping the text fill to a background, which the shared
	`.kit-shimmer` class cannot do: its gradient is built from `--kit-skeleton`,
	a surface colour that would render the glyphs invisible. The port keeps the
	same `kit-shimmer` keyframe and `200%` sweep, but runs the band between
	`--kit-muted-foreground` and `--kit-foreground` so the text stays readable at
	every point of the cycle, including when motion is reduced and the gradient
	freezes.
-->
<span data-kit="shimmer" class={cn('inline-flex items-baseline', className)}>
	<span
		class={cn(
			'text-[length:var(--kit-text-sm)] text-[color:var(--kit-muted-foreground)]',
			pending && 'kit-shimmer-text bg-clip-text text-transparent',
		)}
	>
		{text}
	</span>

	{#if pending}
		<span
			aria-hidden="true"
			class="kit-blink ml-px inline-block h-(--kit-leading-sm) w-px bg-(--kit-foreground) align-middle"
		></span>
	{/if}
</span>

<style>
	.kit-shimmer-text {
		background-image: linear-gradient(
			90deg,
			var(--kit-muted-foreground) 0%,
			var(--kit-foreground) 50%,
			var(--kit-muted-foreground) 100%
		);
		background-size: 200% 100%;
		animation: kit-shimmer 1.6s linear infinite;
	}

	@media (prefers-reduced-motion: reduce) {
		.kit-shimmer-text {
			animation: none;
			background-position: 50% 0;
		}
	}
</style>
