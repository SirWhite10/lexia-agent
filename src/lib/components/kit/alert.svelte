<script lang="ts" module>
	import type { Snippet } from 'svelte';

	/** `alert.rs::AlertVariant` is Default/Info/Success/Warning/Error; the port
	 * folds Default into `neutral` and renames Error to the token name. */
	export type KitAlertVariant = 'neutral' | 'info' | 'success' | 'warning' | 'danger';

	export type KitAlertProps = {
		variant?: KitAlertVariant;
		title?: string;
		description?: string;
		/** Actions, rendered under the copy. */
		children?: Snippet;
		class?: string;
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let {
		variant = 'neutral',
		title,
		description,
		children,
		class: className,
	}: KitAlertProps = $props();

	// `alert.rs` mixes the status colour 4% toward transparent white for the
	// surface and 30% for the border. Mixing toward `--kit-background` instead
	// keeps the tint readable in both modes, since that token is what the
	// alert sits on.
	const TONE: Record<KitAlertVariant, { surface: string; border: string; text: string }> = {
		neutral: {
			surface: 'bg-(--kit-muted)',
			border: 'border-(--kit-border)',
			text: 'text-[color:var(--kit-foreground)]',
		},
		info: {
			surface: 'bg-[color-mix(in_oklab,var(--kit-info)_12%,var(--kit-background))]',
			border: 'border-(--kit-info)',
			text: 'text-[color:var(--kit-info)]',
		},
		success: {
			surface: 'bg-[color-mix(in_oklab,var(--kit-success)_12%,var(--kit-background))]',
			border: 'border-(--kit-success)',
			text: 'text-[color:var(--kit-success)]',
		},
		warning: {
			surface: 'bg-[color-mix(in_oklab,var(--kit-warning)_12%,var(--kit-background))]',
			border: 'border-(--kit-warning)',
			text: 'text-[color:var(--kit-warning)]',
		},
		danger: {
			surface: 'bg-[color-mix(in_oklab,var(--kit-danger)_12%,var(--kit-background))]',
			border: 'border-(--kit-danger)',
			text: 'text-[color:var(--kit-danger)]',
		},
	};
</script>

<div
	data-kit="alert"
	data-variant={variant}
	role="alert"
	class={cn(
		'w-full rounded-(--kit-radius-lg) border-s-4 px-(--kit-space-lg) py-(--kit-space-md)',
		TONE[variant].surface,
		TONE[variant].border,
		TONE[variant].text,
		className,
	)}
>
	<div class="flex min-w-0 flex-col gap-(--kit-space-sm)">
		{#if title}
			<p class="text-[length:var(--kit-text-sm)] leading-(--kit-leading-sm) font-semibold text-[color:var(--kit-foreground)]">
				{title}
			</p>
		{/if}

		{#if description}
			<p class="text-[length:var(--kit-text-sm)] leading-(--kit-leading-sm) text-[color:var(--kit-foreground)]">
				{description}
			</p>
		{/if}

		{#if children}
			<div class="mt-(--kit-space-xxs) flex flex-wrap items-center gap-(--kit-space-sm)">
				{@render children()}
			</div>
		{/if}
	</div>
</div>
