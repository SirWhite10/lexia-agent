<script lang="ts" module>
	import { cn } from '#lib/utils.js';
	import type { RunAction, RunView } from '../chat/model.js';
	import Spinner from '../spinner.svelte';

	export type RunCardProps = {
		run: RunView;
		/** Mirrors the real chat: cancelling a run is only offered while it can still change. */
		oncancel?: (runId: string) => void;
		class?: string | undefined;
	};

	const STATUS_LABEL: Record<RunAction['status'], string> = {
		queued: 'Queued',
		running: 'Running',
		succeeded: 'Done',
		failed: 'Failed',
		cancelled: 'Cancelled'
	};

	const STATUS_TONE: Record<RunAction['status'], string> = {
		queued: 'text-[color:var(--kit-muted-foreground)]',
		running: 'text-[color:var(--kit-foreground)]',
		succeeded: 'text-[color:var(--kit-success)]',
		failed: 'text-[color:var(--kit-danger)]',
		cancelled: 'text-[color:var(--kit-muted-foreground)]'
	};
</script>

<script lang="ts">
	let { run, oncancel, class: className }: RunCardProps = $props();

	let elapsed = $state(0);

	/**
	 * Elapsed time ticks only while the run can still move, so a finished run card does
	 * not keep a timer alive for the rest of the session.
	 */
	$effect(() => {
		if (run.status !== 'active' && run.status !== 'waiting') return;
		const timer = setInterval(() => (elapsed = Date.now() - run.startedAt), 200);
		elapsed = Date.now() - run.startedAt;
		return () => clearInterval(timer);
	});

	const terminal = $derived(run.status === 'succeeded' || run.status === 'failed' || run.status === 'cancelled');
	let collapsed = $state(false);

	/** A finished run starts collapsed; a live one is open so progress is visible. */
	$effect(() => {
		if (terminal && run.status !== 'active') collapsed = true;
	});

	const finished = $derived(run.actions.filter((action) => action.status === 'succeeded').length);
	const failed = $derived(run.actions.filter((action) => action.status === 'failed').length);

	function duration(action: RunAction): string | null {
		if (action.durationMs === undefined) return null;
		return action.durationMs >= 1000 ? `${(action.durationMs / 1000).toFixed(1)}s` : `${action.durationMs}ms`;
	}
</script>

<section
	data-kit="run-card"
	data-status={run.status}
	aria-label="Run {run.runId}: {run.title}"
	class={cn('w-full min-w-0 overflow-hidden rounded-(--kit-radius-lg) border border-(--kit-border) bg-(--kit-card)', className)}
>
	<header class="flex items-center gap-(--kit-space-sm) border-b border-(--kit-border) px-(--kit-space-md) py-(--kit-space-sm)">
		<div class="flex min-w-0 flex-1 flex-col">
			<p class="truncate text-[length:var(--kit-text-sm)] leading-(--kit-leading-sm) font-medium">{run.title}</p>
			<p class="flex items-center gap-(--kit-space-xs) text-[length:var(--kit-text-xs)] leading-(--kit-leading-xs) text-[color:var(--kit-muted-foreground)]">
				<svg viewBox="0 0 16 16" class="size-3" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.25">
					<circle cx="8" cy="8" r="5.25" />
					<path d="M8 5v3.2l2 1.4" stroke-linecap="round" />
				</svg>
				<span class="font-mono">{run.runId}</span>
				<span aria-hidden="true">·</span>
				<span class={run.status === 'failed' ? 'text-[color:var(--kit-danger)]' : run.status === 'succeeded' ? 'text-[color:var(--kit-success)]' : ''}>
					{run.status}
				</span>
				{#if !terminal}
					<span class="font-mono" aria-live="off">{(elapsed / 1000).toFixed(1)}s</span>
				{/if}
			</p>
		</div>

		{#if oncancel && !terminal}
			<button
				type="button"
				data-kit="run-cancel"
				class="grid size-[28px] shrink-0 place-items-center rounded-(--kit-radius-sm) text-[color:var(--kit-muted-foreground)] transition-colors duration-(--kit-duration-fast) hover:bg-(--kit-muted) hover:text-[color:var(--kit-danger)]"
				onclick={() => oncancel(run.runId)}
				aria-label="Cancel run {run.runId}"
			>
				<svg viewBox="0 0 16 16" class="size-4" aria-hidden="true">
					<circle cx="8" cy="8" r="5.5" stroke="currentColor" stroke-width="1.5" fill="none" />
					<rect x="6" y="6" width="4" height="4" rx="0.5" fill="currentColor" />
				</svg>
			</button>
		{/if}
	</header>

	<div class="h-1 w-full bg-(--kit-skeleton)" aria-hidden="true">
		<div
			class="h-full bg-(--kit-primary) transition-[width] duration-(--kit-duration-normal) ease-(--kit-ease-move)"
			style="width: {run.actions.length === 0 ? 0 : Math.round((finished + failed) / run.actions.length * 100)}%"
		></div>
	</div>

	<div class="flex items-center justify-between gap-(--kit-space-sm) px-(--kit-space-md) pt-(--kit-space-sm)">
		<p class="text-[length:var(--kit-text-xs)] leading-(--kit-leading-xs) text-[color:var(--kit-muted-foreground)]">
			{finished} of {run.actions.length} actions done{failed > 0 ? `, ${failed} failed` : ''}
		</p>
		<button
			type="button"
			data-kit="run-toggle"
			class="rounded-(--kit-radius-sm) px-(--kit-space-xs) text-[length:var(--kit-text-xs)] font-medium text-[color:var(--kit-muted-foreground)] transition-colors duration-(--kit-duration-fast) hover:text-[color:var(--kit-foreground)]"
			onclick={() => (collapsed = !collapsed)}
			aria-expanded={!collapsed}
		>
			{collapsed ? 'Show actions' : 'Hide actions'}
		</button>
	</div>

	{#if !collapsed}
		<ul class="kit-scroll mt-(--kit-space-xs) max-h-[320px] overflow-y-auto px-(--kit-space-md) pb-(--kit-space-md)">
			{#each run.actions as action (action.id)}
				<li class="border-t border-(--kit-border)/70 py-(--kit-space-sm) first:border-t-0">
					<div class="flex min-w-0 items-start gap-(--kit-space-sm)">
						<span class="mt-[3px] grid size-[16px] shrink-0 place-items-center" aria-hidden="true">
							{#if action.status === 'running'}
								<Spinner size="sm" />
							{:else if action.status === 'succeeded'}
								<svg viewBox="0 0 16 16" class={cn('size-3.5', STATUS_TONE.succeeded)} fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3.5 8.5l3 3 6-6.5" /></svg>
							{:else if action.status === 'failed'}
								<svg viewBox="0 0 16 16" class={cn('size-3.5', STATUS_TONE.failed)} fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M5 5l6 6M11 5l-6 6" /></svg>
							{:else if action.status === 'cancelled'}
								<svg viewBox="0 0 16 16" class={cn('size-3.5', STATUS_TONE.cancelled)} fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 8h8" /></svg>
							{:else}
								<span class="size-[6px] rounded-full border border-(--kit-muted-foreground)"></span>
							{/if}
						</span>

						<div class="flex min-w-0 flex-1 flex-col gap-(--kit-space-xxs)">
							<div class="flex min-w-0 flex-wrap items-baseline gap-x-(--kit-space-sm) gap-y-(--kit-space-xxs)">
								<span class="min-w-0 text-[length:var(--kit-text-sm)] leading-(--kit-leading-sm)">{action.outcome}</span>
								{#if action.agent}
									<span class="rounded-(--kit-radius-full) bg-(--kit-accent) px-(--kit-space-xs) text-[length:var(--kit-text-xs)] leading-(--kit-leading-xs) text-[color:var(--kit-muted-foreground)]">{action.agent}</span>
								{/if}
							</div>
							<p class="flex items-center gap-(--kit-space-xs) text-[length:var(--kit-text-xs)] leading-(--kit-leading-xs) text-[color:var(--kit-muted-foreground)]">
								<span class="font-mono">{action.capability}</span>
								<span aria-hidden="true">·</span>
								<span class={STATUS_TONE[action.status]}>{STATUS_LABEL[action.status]}</span>
								{#if action.progress}
									<span aria-hidden="true">·</span>
									<span class="font-mono">{action.progress}</span>
								{/if}
								{#if duration(action)}
									<span aria-hidden="true">·</span>
									<span class="font-mono">{duration(action)}</span>
								{/if}
							</p>
							{#if action.error}
								<p class="text-[length:var(--kit-text-xs)] leading-(--kit-leading-xs) text-[color:var(--kit-danger)]">{action.error}</p>
							{/if}
						</div>
					</div>
				</li>
			{/each}
		</ul>
	{/if}
</section>