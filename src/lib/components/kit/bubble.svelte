<script lang="ts" module>
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	/** `gpui-component/src/bubble.rs` → `BubbleVariant`. */
	export type BubbleVariant = 'filled' | 'secondary' | 'muted' | 'tinted' | 'outline' | 'ghost' | 'destructive';

	export type BubbleProps = {
		variant?: BubbleVariant;
		alignment?: 'start' | 'end';
		children?: Snippet;
		class?: string | undefined;
	};

	const SURFACE: Record<BubbleVariant, string> = {
		filled: 'bg-(--kit-primary) text-[color:var(--kit-primary-foreground)]',
		// Upstream deliberately uses `muted` rather than `secondary` here: the secondary
		// role is tuned for buttons and sits a tier too dark for conversation.
		secondary: 'bg-(--kit-muted) text-[color:var(--kit-secondary-foreground)]',
		muted: 'bg-(--kit-muted) text-[color:var(--kit-foreground)]',
		tinted: 'bg-[color-mix(in_oklab,var(--kit-primary)_var(--kit-tinted-mix),var(--kit-background))] text-[color:var(--kit-foreground)]',
		outline: 'border-(--kit-border) bg-(--kit-background) text-[color:var(--kit-foreground)]',
		ghost: 'bg-transparent text-[color:var(--kit-foreground)]',
		destructive: 'bg-[color-mix(in_oklab,var(--kit-danger)_var(--kit-destructive-mix),transparent)] text-[color:var(--kit-danger)]'
	};
</script>

<script lang="ts">
	let { variant = 'secondary', alignment, children, class: className }: BubbleProps = $props();
</script>

<!-- Radius is `radius * 2.5` (20px) upstream, padding `px_3 py_2`, and the line
     height is a relative 1.625 rather than the theme's absolute step, because a
     bubble holds wrapped paragraphs rather than a single line of UI copy. -->
<div
	data-kit="bubble"
	data-variant={variant}
	class={cn(
		'min-w-0 max-w-full border border-transparent px-(--kit-space-md) py-(--kit-space-sm) text-[length:var(--kit-text-sm)] leading-[1.625]',
		// A ghost bubble has no surface to clip against, so overflow clipping would
		// only cut the shadows and overhanging controls of rich content.
		variant === 'ghost' ? 'rounded-(--kit-radius-none) p-0' : 'rounded-(--kit-radius-2xl)',
		variant === 'ghost' ? '' : 'overflow-hidden',
		alignment === 'end' ? 'self-end' : 'self-start',
		SURFACE[variant],
		className
	)}
>
	{@render children?.()}
</div>