<script lang="ts" module>
	import type { Snippet } from 'svelte';

	export type KitCheckboxProps = {
		checked?: boolean;
		/** The mixed state: neither on nor off, drawn as a dash. */
		indeterminate?: boolean;
		disabled?: boolean;
		/** Visible text beside the box. */
		label?: Snippet;
		/** Accessible name; label text already names it. */
		ariaLabel?: string;
		class?: string;
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let {
		checked = $bindable(false),
		indeterminate = $bindable(false),
		disabled = false,
		label,
		ariaLabel,
		class: className,
	}: KitCheckboxProps = $props();

	// `checkbox.rs` medium: a 16px (`rems(1)`) indicator with
	// `radius.min(px(4))` corners, the `input` border when empty and a
	// `primary` fill when checked.
	function toggle() {
		checked = !checked;
		// A click resolves the mixed state to a definite one: activating an
		// indeterminate control means "yes", which is `checked = true`.
		indeterminate = false;
	}
</script>

<button
	type="button"
	role="checkbox"
	aria-checked={indeterminate ? 'mixed' : checked}
	aria-label={ariaLabel}
	{disabled}
	data-kit="checkbox"
	data-state={indeterminate ? 'indeterminate' : checked ? 'checked' : 'unchecked'}
	onclick={toggle}
	class={cn(
		'inline-flex items-start gap-(--kit-space-sm) rounded-(--kit-radius-sm) text-left',
		'disabled:cursor-not-allowed disabled:opacity-50',
		className,
	)}
>
	<span
		aria-hidden="true"
		class={cn(
			'relative flex size-4 shrink-0 items-center justify-center rounded-(--kit-radius-sm) border',
			'transition-colors duration-(--kit-duration-fast) ease-(--kit-ease-move)',
			indeterminate || checked
				? 'border-(--kit-primary) bg-(--kit-primary)'
				: 'border-(--kit-input-border) bg-(--kit-background)',
		)}
	>
		{#if indeterminate}
			<span class="h-0.5 w-2.5 rounded-(--kit-radius-full) bg-(--kit-primary-foreground)"></span>
		{:else if checked}
			<svg
				viewBox="0 0 12 12"
				class="size-3 text-[color:var(--kit-primary-foreground)]"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<path d="M2.5 6.2 4.8 8.5 9.5 3.8" />
			</svg>
		{/if}
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
