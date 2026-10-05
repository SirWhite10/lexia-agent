<script lang="ts" module>
	export type KitSeparatorProps = {
		orientation?: 'horizontal' | 'vertical';
		/** Text shown between the two halves of a horizontal rule. */
		label?: string;
		class?: string;
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let { orientation = 'horizontal', label, class: className }: KitSeparatorProps = $props();

	// `separator.rs` paints a solid `border`-coloured line; a labelled rule is
	// a centred `text_xs` chip in `muted_foreground` on the background colour,
	// so the line appears to run through it.
	const LINE = 'bg-(--kit-border)';
</script>

{#if orientation === 'vertical'}
	<div
		data-kit="separator"
		role="separator"
		aria-orientation="vertical"
		class={cn('w-px shrink-0 self-stretch', LINE, className)}
	></div>
{:else if label}
	<div data-kit="separator" role="separator" aria-orientation="horizontal" class={cn('flex w-full items-center', className)}>
		<span class={cn('h-px flex-1', LINE)}></span>
		<span
			class="mx-auto shrink-0 bg-(--kit-background) px-(--kit-space-md) py-(--kit-space-xs) text-center text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]"
		>
			{label}
		</span>
		<span class={cn('h-px flex-1', LINE)}></span>
	</div>
{:else}
	<div
		data-kit="separator"
		role="separator"
		aria-orientation="horizontal"
		class={cn('h-px w-full shrink-0', LINE, className)}
	></div>
{/if}
