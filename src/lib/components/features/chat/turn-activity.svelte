<script lang="ts">
	import BotIcon from '@lucide/svelte/icons/bot';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import type { Snippet } from 'svelte';

	export type ActivityDetail = {
		label: string;
		value: string | null;
		/** True when the detail is a failure worth reading before expanding. */
		failed?: boolean;
	};

	let {
		title,
		status,
		details,
		cancel
	}: {
		/** One line, closed state: what the agent did. */
		title: string;
		/** Shown as a quiet suffix so the line reads as progress, not decoration. */
		status: string;
		details: ActivityDetail[];
		/** Rendered only while the work is still live. */
		cancel?: Snippet;
	} = $props();

	let expanded = $state(false);

	// Long output opens itself: two lines is the point where a summary stops
	// being honest about what happened.
	$effect(() => {
		expanded = details.some((detail) => (detail.value?.split('\n').length ?? 0) > 2);
	});
</script>

<!-- Bot side, always: work the agent did belongs to its side of the
     conversation, never filed under the request that triggered it. -->
<div class="flex items-start gap-2" data-activity-status={status}>
	<BotIcon class="mt-1 size-4 shrink-0 text-muted-foreground" />
	<div class="min-w-0 max-w-[80%]">
		<div class="flex flex-wrap items-center gap-2">
			<button
				type="button"
				class="inline-flex items-center gap-1 rounded-md text-left text-sm text-muted-foreground transition-colors hover:text-foreground"
				aria-expanded={expanded}
				onclick={() => (expanded = !expanded)}
			>
				<ChevronRightIcon class="size-3.5 shrink-0 transition-transform {expanded ? 'rotate-90' : ''}" />
				<span>{title}</span>
			</button>
			<span class="font-mono text-xs text-muted-foreground/80">{status}</span>
			{#if cancel}{@render cancel()}{/if}
		</div>

		{#if expanded}
			<div class="mt-1 space-y-1.5 border-l border-border pl-3">
				{#each details as detail (detail.label)}
					<div class="text-xs">
						<p class="font-mono text-muted-foreground">{detail.label}</p>
						{#if detail.value}
							<p class="mt-0.5 whitespace-pre-wrap text-muted-foreground {detail.failed ? 'text-destructive' : ''}">
								{detail.value}
							</p>
						{/if}
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>