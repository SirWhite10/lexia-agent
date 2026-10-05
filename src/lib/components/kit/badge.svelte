<script lang="ts" module>
	import type { Snippet } from 'svelte';

	export type KitBadgeVariant =
		| 'default'
		| 'secondary'
		| 'outline'
		| 'success'
		| 'warning'
		| 'danger'
		| 'info';

	/** `badge.rs` sizes the chip 10/16/24px by `Size`; the port keeps Large at
	 * the crate's 24px and moves the two smaller steps onto the same 4px
	 * rhythm (16/20px) so a badge can sit inside a 24px row without a
	 * half-pixel edge. */
	export type KitBadgeSize = 'sm' | 'md' | 'lg';

	export type KitBadgeProps = {
		variant?: KitBadgeVariant;
		size?: KitBadgeSize;
		class?: string;
		children?: Snippet;
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let {
		variant = 'default',
		size = 'md',
		class: className,
		children,
	}: KitBadgeProps = $props();

	const VARIANT_CLASS: Record<KitBadgeVariant, string> = {
		default: 'bg-(--kit-primary) text-[color:var(--kit-primary-foreground)]',
		secondary: 'bg-(--kit-secondary) text-[color:var(--kit-secondary-foreground)]',
		outline: 'border border-(--kit-border) text-[color:var(--kit-foreground)]',
		// Status chips take the status colour as the surface and its
		// `*_foreground` as the text, matching the crate's button rule.
		success: 'bg-(--kit-success) text-[color:var(--kit-success-foreground)]',
		warning: 'bg-(--kit-warning) text-[color:var(--kit-warning-foreground)]',
		danger: 'bg-(--kit-danger) text-[color:var(--kit-danger-foreground)]',
		info: 'bg-(--kit-info) text-[color:var(--kit-info-foreground)]',
	};

	const SIZE_CLASS: Record<KitBadgeSize, string> = {
		sm: 'h-4 px-(--kit-space-xs) text-[length:var(--kit-text-xs)]',
		md: 'h-5 px-(--kit-space-xs) text-[length:var(--kit-text-xs)]',
		lg: 'h-6 px-(--kit-space-sm) text-[length:var(--kit-text-sm)]',
	};
</script>

<span
	data-kit="badge"
	data-variant={variant}
	class={cn(
		'inline-flex shrink-0 items-center justify-center whitespace-nowrap rounded-(--kit-radius-full) font-medium',
		VARIANT_CLASS[variant],
		SIZE_CLASS[size],
		className,
	)}
>
	{@render children?.()}
</span>
