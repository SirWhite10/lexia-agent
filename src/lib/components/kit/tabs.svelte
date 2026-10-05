<script lang="ts" module>
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	export type TabItem = {
		id: string;
		label: string;
		/** Optional leading glyph; keep it decorative, the label carries meaning. */
		icon?: Snippet;
		/** Small trailing count or status word, e.g. "3" or "live". */
		badge?: string;
		disabled?: boolean;
	};

	export type TabsProps = {
		items: TabItem[];
		value?: string;
		variant?: 'segmented' | 'underlined';
		size?: 'small' | 'medium';
		class?: string | undefined;
		children?: Snippet;
	};
</script>

<script lang="ts">
	/** `gpui-component/src/tab`: the segmented variant lives on `tab_bar.segmented.background`, the underlined one draws an indicator under the active tab. */
	let {
		items,
		value = $bindable(items[0]?.id ?? ''),
		variant = 'segmented',
		size = 'medium',
		class: className,
		children
	}: TabsProps = $props();

	let tabs = $state<HTMLButtonElement[]>([]);

	/** Roving focus: arrow keys move between tabs, Home/End jump to the ends, and a disabled tab is skipped. */
	function onKeydown(event: KeyboardEvent) {
		const enabled = items.filter((item) => !item.disabled);
		const current = enabled.findIndex((item) => item.id === value);
		let next = current;
		if (event.key === 'ArrowRight' || event.key === 'ArrowDown') next = (current + 1) % enabled.length;
		else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') next = (current - 1 + enabled.length) % enabled.length;
		else if (event.key === 'Home') next = 0;
		else if (event.key === 'End') next = enabled.length - 1;
		else return;
		event.preventDefault();
		value = enabled[next].id;
		tabs[items.indexOf(enabled[next])]?.focus();
	}
</script>

<div
	data-kit="tabs"
	role="tablist"
	class={cn(
		'inline-flex items-center',
		variant === 'segmented' ? 'gap-(--kit-space-xxs) rounded-(--kit-radius-md) bg-(--kit-tab-bar) p-(--kit-space-xxs)' : 'gap-(--kit-space-lg)',
		size === 'small' ? 'text-[length:var(--kit-text-xs)]' : 'text-[length:var(--kit-text-sm)]',
		className
	)}
>
	{#each items as item, index (item.id)}
		{@const active = item.id === value}
		<button
			bind:this={tabs[index]}
			data-kit="tab"
			type="button"
			role="tab"
			id={`kit-tab-${item.id}`}
			aria-selected={active}
			aria-controls={`kit-tabpanel-${item.id}`}
			disabled={item.disabled}
			tabindex={active ? 0 : -1}
			onclick={() => (value = item.id)}
			onkeydown={onKeydown}
			class={cn(
				'inline-flex items-center justify-center gap-(--kit-space-xs) rounded-(--kit-radius-sm) font-medium whitespace-nowrap transition-colors duration-(--kit-duration-fast) disabled:pointer-events-none disabled:opacity-50',
				size === 'small' ? 'h-[24px] px-(--kit-space-sm)' : 'h-[30px] px-(--kit-space-md)',
				variant === 'segmented' && active ? 'bg-(--kit-tab-active) text-[color:var(--kit-tab-active-foreground)] shadow-xs' : '',
				variant === 'segmented' && !active ? 'text-[color:var(--kit-tab-foreground)] hover:text-[color:var(--kit-foreground)]' : '',
				variant === 'underlined' &&
					(active
						? 'text-[color:var(--kit-tab-active-foreground)] shadow-[inset_0_-2px_0_0_var(--kit-primary)]'
						: 'text-[color:var(--kit-tab-foreground)] hover:text-[color:var(--kit-foreground)]')
			)}
		>
			{#if item.icon}<span class="flex shrink-0 items-center" aria-hidden="true">{@render item.icon()}</span>{/if}
			{item.label}
			{#if item.badge}
				<span class="rounded-(--kit-radius-full) bg-(--kit-accent) px-(--kit-space-xs) text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">{item.badge}</span>
			{/if}
		</button>
	{/each}
</div>

{#if children}
	<div id={`kit-tabpanel-${value}`} role="tabpanel" aria-labelledby={`kit-tab-${value}`} tabindex="0">
		{@render children()}
	</div>
{/if}