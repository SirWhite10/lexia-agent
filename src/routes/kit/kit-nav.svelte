<script lang="ts" module>
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	export type KitSection = 'chat' | 'components' | 'artifacts';

	export type KitNavProps = {
		section?: KitSection;
		onSelect?: (section: KitSection) => void;
		/** Counts shown beside each entry, so the page advertises what it holds. */
		class?: string | undefined;
		footer?: Snippet;
	};

	const ITEMS: Array<{ id: KitSection; label: string; hint: string }> = [
		{ id: 'chat', label: 'Chat', hint: 'The production interface, ported' },
		{ id: 'components', label: 'Components', hint: 'Buttons to data tables, live' },
		{ id: 'artifacts', label: 'Artifacts', hint: 'Everything the agent can hand back' }
	];
</script>

<script lang="ts">
	let { section, onSelect, class: className, footer }: KitNavProps = $props();
</script>

<!-- Persistent navigation, the desktop convention: the rail never disappears, because a
     window that loses its structure is a window you have to re-find your place in. The
     phone reaches the same entries through a sheet opened from the header. -->
<nav data-kit="nav" aria-label="Kit sections" class={cn('flex h-full min-h-0 flex-col bg-(--kit-sidebar) text-[color:var(--kit-sidebar-foreground)]', className)}>
	<ul class="flex flex-col gap-(--kit-space-xxs) p-(--kit-space-md)">
		{#each ITEMS as item (item.id)}
			<li>
				<button
					type="button"
					data-kit="nav-item"
					aria-current={section === item.id ? 'page' : undefined}
					onclick={() => onSelect?.(item.id)}
					class={cn(
						'flex w-full flex-col items-start gap-(--kit-space-xxs) rounded-(--kit-radius-md) px-(--kit-space-md) py-(--kit-space-sm) text-left transition-colors duration-(--kit-duration-fast)',
						section === item.id
							? 'bg-(--kit-sidebar-accent) text-[color:var(--kit-sidebar-accent-foreground)]'
							: 'text-[color:var(--kit-muted-foreground)] hover:bg-(--kit-sidebar-accent)/60 hover:text-[color:var(--kit-sidebar-accent-foreground)]'
					)}
				>
					<span class="text-[length:var(--kit-text-sm)] leading-(--kit-leading-sm) font-medium">{item.label}</span>
					<span class="text-[length:var(--kit-text-xs)] leading-(--kit-leading-xs) opacity-80">{item.hint}</span>
				</button>
			</li>
		{/each}
	</ul>

	{#if footer}
		<div class="mt-auto border-t border-(--kit-sidebar-border) p-(--kit-space-md) text-[length:var(--kit-text-xs)] leading-(--kit-leading-xs) text-[color:var(--kit-muted-foreground)]">
			{@render footer()}
		</div>
	{/if}
</nav>