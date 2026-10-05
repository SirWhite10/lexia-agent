<script lang="ts" module>
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	export type TooltipProps = {
		/**
		 * The trigger. The tooltip is decorative guidance for an already-labelled control,
		 * so it is attached with `aria-describedby` rather than replacing the label.
		 */
		children: Snippet;
		label: string;
		placement?: 'top' | 'bottom' | 'left' | 'right';
		/** Show delay, matching the feel of a native tooltip rather than snapping on. */
		delay?: number;
		side?: 'left' | 'right';
		class?: string | undefined;
	};
</script>

<script lang="ts">
	let { children, label, placement = 'top', delay = 350, side = 'right', class: className }: TooltipProps = $props();

	let open = $state(false);
	let timer: ReturnType<typeof setTimeout> | undefined;

	function show() {
		clearTimeout(timer);
		timer = setTimeout(() => (open = true), delay);
	}

	/** Hide immediately: a dismissed tooltip must not leave a timer that re-opens it. */
	function hide() {
		clearTimeout(timer);
		open = false;
	}

	$effect(() => () => clearTimeout(timer));

	const position: Record<NonNullable<TooltipProps['placement']>, string> = {
		top: 'bottom-full left-1/2 -translate-x-1/2 mb-(--kit-space-xs)',
		bottom: 'top-full left-1/2 -translate-x-1/2 mt-(--kit-space-xs)',
		left: 'right-full top-1/2 -translate-y-1/2 mr-(--kit-space-xs)',
		right: 'left-full top-1/2 -translate-y-1/2 ml-(--kit-space-xs)'
	};
</script>

<span
	data-kit="tooltip"
	class={cn('relative inline-flex', className)}
	role="presentation"
	onmouseenter={show}
	onmouseleave={hide}
	onfocusin={() => (open = true)}
	onfocusout={hide}
	onkeydown={(event) => event.key === 'Escape' && hide()}
>
	{@render children()}
	{#if open}
		<span
			role="tooltip"
			data-placement={placement}
			class={cn(
				'pointer-events-none absolute z-50 max-w-[260px] rounded-(--kit-radius-md) border border-(--kit-border) bg-(--kit-popover) px-(--kit-space-sm) py-(--kit-space-xs) text-left text-[length:var(--kit-text-xs)] leading-(--kit-leading-xs) text-[color:var(--kit-foreground)] shadow-lg',
				'kit-rise',
				position[placement]
			)}
		>
			{label}
		</span>
	{/if}
</span>