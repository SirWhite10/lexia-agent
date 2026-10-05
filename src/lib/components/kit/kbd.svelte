<script lang="ts" module>
	import type { Snippet } from 'svelte';

	export type KitKbdProps = {
		/**
		 * One chip per entry, e.g. `['mod', 'k']`. Wins over `children` when both
		 * are given.
		 */
		keys?: string[];
		/** Free-form content, for callers that already hold a formatted string. */
		children?: Snippet;
		class?: string;
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let { keys = [], children, class: className }: KitKbdProps = $props();

	// `kbd.rs` renders `min_w_5`, `py_0p5`, `px_1`, `text_xs` and
	// `radius.half()` on a muted chip; `radius.half()` of the 6px theme radius
	// is the `--kit-radius-sm` step.
	const CHIP = 'inline-flex min-w-5 items-center justify-center rounded-(--kit-radius-sm) bg-(--kit-secondary) px-(--kit-space-sm) py-(--kit-space-xxs) text-center text-[length:var(--kit-text-xs)] leading-none text-[color:var(--kit-muted-foreground)]';
</script>

{#if keys.length > 0}
	<span data-kit="kbd" class={cn('inline-flex items-center gap-(--kit-space-xxs)', className)}>
		{#each keys as key (key)}
			<kbd class={CHIP}>{key}</kbd>
		{/each}
	</span>
{:else}
	<span data-kit="kbd" class={cn('inline-flex', className)}>
		{#if children}
			{@render children()}
		{:else}
			<kbd class={CHIP}>—</kbd>
		{/if}
	</span>
{/if}
