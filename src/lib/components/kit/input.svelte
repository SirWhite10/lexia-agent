<script lang="ts" module>
	import type { WithElementRef } from '#lib/utils.js';
	import type { HTMLInputAttributes } from 'svelte/elements';

	/** `sizing.rs::input_h` maps Small/Medium to `h_6`/`h_8`; the port rounds
	 * Large down to `h_10` so the three steps stay evenly spaced for touch. */
	export type KitInputSize = 'sm' | 'md' | 'lg';

	export type KitInputProps = Omit<HTMLInputAttributes, 'size' | 'value'> &
		WithElementRef<{
			value?: string;
			placeholder?: string;
			size?: KitInputSize;
			disabled?: boolean;
			invalid?: boolean;
			/** Rendered below the field in the danger colour and announced. */
			error?: string;
		}, HTMLInputElement>;
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let {
		class: className,
		value = $bindable(''),
		placeholder,
		size = 'md',
		disabled = false,
		invalid = false,
		error,
		id,
		name,
		ref = $bindable(null),
		...restProps
	}: KitInputProps = $props();

	const uid = $props.id();
	const fieldId = $derived(id ?? uid);
	// The message is the field's description, so a screen reader announces the
	// reason with the field rather than as an interruption of its own.
	const errorId = $derived(error ? `${fieldId}-error` : undefined);

	const SIZE_CLASS: Record<KitInputSize, string> = {
		sm: 'h-6 px-(--kit-space-sm) text-[length:var(--kit-text-xs)]',
		md: 'h-8 px-(--kit-space-md) text-[length:var(--kit-text-sm)]',
		lg: 'h-10 px-(--kit-space-lg) text-[length:var(--kit-text-md)]',
	};
</script>

<div data-kit="input" class={cn('flex w-full flex-col gap-(--kit-space-xs)', className)}>
	<input
		bind:this={ref}
		bind:value
		id={fieldId}
		{name}
		type="text"
		{placeholder}
		{disabled}
		aria-invalid={invalid || error ? true : undefined}
		aria-describedby={errorId}
		class={cn(
			'w-full rounded-(--kit-radius-md) border bg-(--kit-background) text-[color:var(--kit-foreground)] outline-none',
			'placeholder:text-[color:var(--kit-muted-foreground)]',
			'disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50',
			SIZE_CLASS[size],
			// The border carries the invalid state: a second signal (the message
			// below) is needed because colour alone is not readable.
			invalid || error ? 'border-(--kit-danger)' : 'border-(--kit-input-border)',
		)}
		{...restProps}
	/>

	{#if error}
		<p id={errorId} role="alert" class="text-[length:var(--kit-text-xs)] text-[color:var(--kit-danger)]">
			{error}
		</p>
	{/if}
</div>
