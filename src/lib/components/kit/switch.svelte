<script lang="ts" module>
	import type { Snippet } from 'svelte';

	export type KitSwitchProps = {
		checked?: boolean;
		disabled?: boolean;
		/** Visible text beside the track. */
		label?: Snippet;
		/** Accessible name for the switch; label text already names it. */
		ariaLabel?: string;
		class?: string;
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let {
		checked = $bindable(false),
		disabled = false,
		label,
		ariaLabel,
		class: className,
	}: KitSwitchProps = $props();

	// `switch.rs` medium geometry: a 36x20 track, a 16px thumb on a 2px inset,
	// radius clamped to the track height so it reads as a pill. The thumb is
	// `switch_thumb`, whose fallback is `background`.
	const TRACK_WIDTH = 36;
	const THUMB_TRAVEL = TRACK_WIDTH - 16 - 4;
</script>

<button
	type="button"
	role="switch"
	aria-checked={checked}
	aria-label={ariaLabel}
	{disabled}
	data-kit="switch"
	data-state={checked ? 'checked' : 'unchecked'}
	onclick={() => (checked = !checked)}
	class={cn(
		'inline-flex items-start gap-(--kit-space-sm) rounded-(--kit-radius-md) text-left',
		'disabled:cursor-not-allowed disabled:opacity-50',
		className,
	)}
>
	<span
		aria-hidden="true"
		// The transparent 1px border is load-bearing: the focus ring tints the
		// border, and that line is what keeps the ring visible on the unchecked
		// track (`switch.rs`).
		class={cn(
			'relative flex shrink-0 items-center border border-transparent p-px',
			'transition-colors duration-(--kit-duration-fast) ease-(--kit-ease-move)',
			checked ? 'bg-(--kit-primary)' : 'bg-(--kit-switch)',
		)}
		style:width={`${TRACK_WIDTH}px`}
		style:height="20px"
	>
		<span
			class={cn(
				'size-4 rounded-(--kit-radius-full) bg-(--kit-switch-thumb)',
				'transition-transform duration-(--kit-duration-fast) ease-(--kit-ease-move)',
			)}
			style:transform={`translateX(${checked ? THUMB_TRAVEL : 0}px)`}
		></span>
	</span>

	{#if label}
		<span
			class={cn(
				'min-w-0 text-[length:var(--kit-text-sm)] text-[color:var(--kit-foreground)]',
				disabled && 'text-[color:var(--kit-muted-foreground)]',
			)}
		>
			{@render label()}
		</span>
	{/if}
</button>
