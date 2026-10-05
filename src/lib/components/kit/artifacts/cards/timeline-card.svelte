<script lang="ts">
	import { cn } from '#lib/utils.js';
	import { formatDuration } from '../format.js';
	import StatusMarker from './status-marker.svelte';
	import type { TimelinePayload } from '../types.js';

	interface Props {
		data: TimelinePayload;
		class?: string;
	}

	let { data, class: className }: Props = $props();

	const KIND_LABEL: Record<TimelinePayload['steps'][number]['kind'], string> = {
		request: 'request',
		subagent: 'sub-agent',
		tool: 'tool'
	};
</script>

<div data-kit="timeline-card" class={cn('min-w-0', className)}>
	<div
		class="mb-(--kit-space-sm) flex flex-wrap items-baseline justify-between gap-(--kit-space-xs) border-b border-(--kit-border) pb-(--kit-space-xs)"
	>
		<p class="m-0 min-w-0 break-words text-[length:var(--kit-text-sm)] font-medium">{data.title}</p>
		<p class="m-0 flex shrink-0 items-center gap-(--kit-space-sm) text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)] tabular-nums">
			<span>started {data.startedAt}</span>
			<span class="text-[color:var(--kit-foreground)]">{formatDuration(data.durationMs)}</span>
		</p>
	</div>

	<ol class="m-0 list-none p-0">
		{#each data.steps as step (step.id)}
			<li class="border-b border-(--kit-border) py-(--kit-space-xs) last:border-b-0">
				<div class="flex items-start gap-(--kit-space-sm)">
					<StatusMarker status={step.status} class="w-32 shrink-0 justify-start" />
					<div class="min-w-0 flex-1">
						<p class="m-0 flex flex-wrap items-baseline gap-x-(--kit-space-sm) text-[length:var(--kit-text-sm)]">
							<span class="break-words">{step.label}</span>
							<span class="text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">{KIND_LABEL[step.kind]}</span>
							<span class="ml-auto shrink-0 text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)] tabular-nums">
								{formatDuration(step.durationMs)}
							</span>
						</p>
						{#if step.detail}
							<p class="m-0 mt-(--kit-space-xxs) break-words text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">
								{step.detail}
							</p>
						{/if}
					</div>
				</div>

				{#if step.children?.length}
					<ul class="m-0 mt-(--kit-space-xs) ml-2 list-none border-l border-(--kit-border) pl-(--kit-space-md)">
						{#each step.children as child, index (index)}
							<li class="flex items-start gap-(--kit-space-sm) py-(--kit-space-xxs)">
								<StatusMarker status={child.status} class="w-32 shrink-0 justify-start" />
								<span class="min-w-0 flex-1 text-[length:var(--kit-text-xs)]">
									<span class="break-words">{child.label}</span>
									{#if child.detail}
										<span class="block text-[color:var(--kit-muted-foreground)]">{child.detail}</span>
									{/if}
								</span>
							</li>
						{/each}
					</ul>
				{/if}
			</li>
		{/each}
	</ol>
</div>