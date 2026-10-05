<script lang="ts">
	import { cn } from '#lib/utils.js';
	import ArtifactBody from './artifact-body.svelte';
	import { CODE_ARTIFACT_KIND, type Artifact } from './types.js';

	interface Props {
		artifact: Artifact;
		class?: string;
	}

	let { artifact, class: className }: Props = $props();

	let view = $state<'preview' | 'code'>('preview');

	const isCode = $derived(CODE_ARTIFACT_KIND[artifact.kind] === true);

	/**
	 * The segmented pair is two buttons with `aria-pressed`, not radio inputs:
	 * both views are always reachable and the pressed one is the current view.
	 */
	function tabClass(tab: 'preview' | 'code') {
		return cn(
			'rounded-(--kit-radius-sm) px-(--kit-space-sm) py-(--kit-space-xxs) text-[length:var(--kit-text-xs)] transition-colors duration-(--kit-duration-fast)',
			view === tab
				? 'bg-(--kit-muted) font-medium text-[color:var(--kit-foreground)]'
				: 'text-[color:var(--kit-muted-foreground)] hover:bg-(--kit-secondary) hover:text-[color:var(--kit-foreground)]'
		);
	}

	const regionClass = $derived(
		artifact.height === 'tall'
			? 'max-h-105 overflow-auto'
			: artifact.height === 'full'
				? 'max-h-140 overflow-auto'
				: ''
	);
</script>

<div
	data-kit="artifact-host"
	class={cn(
		'min-w-0 overflow-hidden rounded-(--kit-radius-lg) border border-(--kit-border) bg-(--kit-card) text-[color:var(--kit-card-foreground)]',
		className
	)}
>
	<div
		class="flex items-start gap-(--kit-space-sm) border-b border-(--kit-border) px-(--kit-space-md) py-(--kit-space-sm)"
	>
		<div class="flex min-w-0 flex-1 flex-col gap-(--kit-space-xxs)">
			<div class="flex min-w-0 flex-wrap items-center gap-(--kit-space-sm)">
				<span
					class="shrink-0 rounded-(--kit-radius-sm) border border-(--kit-border) bg-(--kit-muted) px-(--kit-space-sm) py-(--kit-space-xxs) font-mono text-[length:var(--kit-text-xs)] text-[color:var(--kit-secondary-foreground)]"
				>
					{artifact.kind}
				</span>
				<h3 class="m-0 min-w-0 break-words text-[length:var(--kit-text-sm)] leading-(--kit-leading-sm) font-medium">
					{artifact.title}
				</h3>
			</div>
			{#if artifact.summary}
				<p class="m-0 text-[length:var(--kit-text-xs)] leading-(--kit-leading-xs) text-[color:var(--kit-muted-foreground)]">
					{artifact.summary}
				</p>
			{/if}
		</div>

		{#if isCode}
			<div
				role="group"
				aria-label="Artifact view"
				class="flex shrink-0 items-center gap-(--kit-space-xxs) rounded-(--kit-radius-md) border border-(--kit-border) p-(--kit-space-xxs)"
			>
				<button
					type="button"
					aria-pressed={view === 'preview'}
					onclick={() => (view = 'preview')}
					class={tabClass('preview')}
				>
					Preview
				</button>
				<button
					type="button"
					aria-pressed={view === 'code'}
					onclick={() => (view = 'code')}
					class={tabClass('code')}
				>
					Code
				</button>
			</div>
		{/if}
	</div>

	<div class={cn('p-(--kit-space-md)', isCode && regionClass)}>
		<ArtifactBody {artifact} {view} />
	</div>
</div>