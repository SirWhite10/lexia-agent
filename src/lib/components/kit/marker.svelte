<script lang="ts" module>
	import type { Snippet } from 'svelte';

	export type KitMarkerVariant = 'neutral' | 'success' | 'warning' | 'danger' | 'info';

	export type KitMarkerSize = 'sm' | 'md';

	export type KitMarkerProps = {
		variant?: KitMarkerVariant;
		size?: KitMarkerSize;
		/** Visible text beside the dot. Without it the dot carries the name. */
		label?: Snippet;
		/** Overrides the name announced when there is no visible label. */
		ariaLabel?: string;
		class?: string;
	};

	/** The dot is the state, so a colour-blind or greyscale reader still needs
	 * the name; this is what the wrapper announces when no text is supplied. */
	const VARIANT_NAME: Record<KitMarkerVariant, string> = {
		neutral: 'Neutral',
		success: 'Success',
		warning: 'Warning',
		danger: 'Error',
		info: 'Information',
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let {
		variant = 'neutral',
		size = 'md',
		label,
		ariaLabel,
		class: className,
	}: KitMarkerProps = $props();

	const DOT_CLASS: Record<KitMarkerVariant, string> = {
		neutral: 'bg-(--kit-muted-foreground)',
		success: 'bg-(--kit-success)',
		warning: 'bg-(--kit-warning)',
		danger: 'bg-(--kit-danger)',
		info: 'bg-(--kit-info)',
	};

	// `marker.rs` lays the row out as `h_flex gap_2 text_sm` in
	// `muted_foreground`; the dot itself is its own compact slot.
	const SIZE_CLASS: Record<KitMarkerSize, string> = {
		sm: 'size-1.5',
		md: 'size-2',
	};
</script>

<span
	data-kit="marker"
	data-variant={variant}
	role={label ? undefined : 'img'}
	aria-label={label ? undefined : (ariaLabel ?? `${VARIANT_NAME[variant]} status`)}
	class={cn(
		'inline-flex min-w-0 items-center gap-(--kit-space-sm) text-[length:var(--kit-text-sm)] text-[color:var(--kit-muted-foreground)]',
		className,
	)}
>
	<span
		aria-hidden="true"
		class={cn('shrink-0 rounded-(--kit-radius-full)', SIZE_CLASS[size], DOT_CLASS[variant])}
	></span>

	{#if label}
		<span class="min-w-0 truncate">{@render label()}</span>
	{/if}
</span>
