<script lang="ts">
	import { untrack } from 'svelte';
	import { cn } from '#lib/utils.js';
	import type { ChecklistPayload } from '../types.js';

	interface Props {
		data: ChecklistPayload;
		class?: string;
	}

	let { data, class: className }: Props = $props();

	// The artifact reports the plan as it was emitted; toggling is local to this
	// card, so a demo run can be walked through without mutating the artifact.
	let done = $state(untrack(() => data.items.map((item) => item.done)));

	const completed = $derived(done.filter(Boolean).length);
	const total = $derived(data.items.length || data.total);
	const percent = $derived(total === 0 ? 0 : Math.round((completed / total) * 100));
</script>

<div data-kit="checklist-card" class={cn('min-w-0', className)}>
	<div class="mb-(--kit-space-sm) flex items-center justify-between gap-(--kit-space-sm) text-[length:var(--kit-text-xs)]">
		<span class="text-[color:var(--kit-muted-foreground)]">Plan progress</span>
		<span class="font-medium tabular-nums">{completed} / {total}</span>
	</div>

	<div
		role="progressbar"
		aria-valuemin={0}
		aria-valuemax={total}
		aria-valuenow={completed}
		aria-label="Checklist completion"
		class="mb-(--kit-space-sm) h-1 w-full overflow-hidden rounded-(--kit-radius-full) bg-(--kit-muted)"
	>
		<div
			class="h-full rounded-(--kit-radius-full) bg-(--kit-primary) transition-[width] duration-(--kit-duration-normal)"
			style:width={`${percent}%`}
		></div>
	</div>

	<ul class="m-0 list-none divide-y divide-(--kit-border) p-0">
		{#each data.items as item, index (index)}
			<li class="py-(--kit-space-xs)">
				<label class="flex cursor-pointer items-start gap-(--kit-space-sm) text-[length:var(--kit-text-sm)]">
					<input
						type="checkbox"
						bind:checked={done[index]}
						class="mt-0.5 h-4 w-4 shrink-0 accent-(--kit-primary)"
					/>
					<span class="min-w-0 flex-1">
						<span class="block break-words" class:text-[color:var(--kit-muted-foreground)]={done[index]} class:line-through={done[index]}>
							{item.label}
						</span>
						{#if item.detail}
							<span class="mt-(--kit-space-xxs) block text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">
								{item.detail}
							</span>
						{/if}
					</span>
				</label>
			</li>
		{/each}
	</ul>
</div>