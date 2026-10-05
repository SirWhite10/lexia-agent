<script lang="ts">
	import { cn } from '#lib/utils.js';
	import type { RecordPayload } from '../types.js';

	interface Props {
		data: RecordPayload;
		class?: string;
	}

	let { data, class: className }: Props = $props();
</script>

<div data-kit="record-card" class={cn('min-w-0', className)}>
	{#if data.caption}
		<p class="mb-(--kit-space-xs) text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">{data.caption}</p>
	{/if}
	<dl class="divide-y divide-(--kit-border)">
		{#each data.items as item (item.label)}
			<div class="grid grid-cols-[minmax(0,2fr)_minmax(0,3fr)] gap-(--kit-space-sm) py-(--kit-space-xs)">
				<dt class="text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">{item.label}</dt>
				<dd class="min-w-0 break-words text-[length:var(--kit-text-sm)] text-[color:var(--kit-foreground)]">
					{item.value}
					{#if item.hint}
						<span class="mt-(--kit-space-xxs) block text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">
							{item.hint}
						</span>
					{/if}
				</dd>
			</div>
		{/each}
	</dl>
</div>