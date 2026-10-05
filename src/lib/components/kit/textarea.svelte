<script lang="ts" module>
	import type { WithElementRef } from '#lib/utils.js';
	import type { HTMLTextareaAttributes } from 'svelte/elements';

	/** Same height scale as `input.svelte`; the crate's `Textarea` is an
	 * `Input` in multi-line mode, so it borrows `sizing.rs::input_h`. */
	export type KitTextareaSize = 'sm' | 'md' | 'lg';

	export type KitTextareaProps = Omit<HTMLTextareaAttributes, 'value'> &
		WithElementRef<{
			value?: string;
			placeholder?: string;
			size?: KitTextareaSize;
			disabled?: boolean;
			invalid?: boolean;
			rows?: number;
			/** Rendered below the field in the danger colour and announced. */
			error?: string;
		}, HTMLTextAreaElement>;
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
		rows = 3,
		id,
		name,
		ref = $bindable(null),
		...restProps
	}: KitTextareaProps = $props();

	const uid = $props.id();
	const fieldId = $derived(id ?? uid);
	const errorId = $derived(error ? `${fieldId}-error` : undefined);

	const SIZE_CLASS: Record<KitTextareaSize, string> = {
		sm: 'px-(--kit-space-sm) py-(--kit-space-xs) text-[length:var(--kit-text-xs)]',
		md: 'px-(--kit-space-md) py-(--kit-space-sm) text-[length:var(--kit-text-sm)]',
		lg: 'px-(--kit-space-lg) py-(--kit-space-md) text-[length:var(--kit-text-md)]',
	};
</script>

<div data-kit="textarea" class={cn('flex w-full flex-col gap-(--kit-space-xs)', className)}>
	<textarea
		bind:this={ref}
		bind:value
		id={fieldId}
		{name}
		{placeholder}
		{disabled}
		{rows}
		aria-invalid={invalid || error ? true : undefined}
		aria-describedby={errorId}
		class={cn(
			'w-full resize-y rounded-(--kit-radius-md) border bg-(--kit-background) text-[color:var(--kit-foreground)] outline-none',
			'placeholder:text-[color:var(--kit-muted-foreground)]',
			'disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50',
			SIZE_CLASS[size],
			invalid || error ? 'border-(--kit-danger)' : 'border-(--kit-input-border)',
		)}
		{...restProps}
	></textarea>

	{#if error}
		<p id={errorId} role="alert" class="text-[length:var(--kit-text-xs)] text-[color:var(--kit-danger)]">
			{error}
		</p>
	{/if}
</div>
