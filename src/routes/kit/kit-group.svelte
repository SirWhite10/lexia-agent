<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * One labelled group of live components. The shell is shared so every group in the
	 * showcase reads the same way and a reviewer can compare them row to row.
	 */
	let { title, note, children }: { title: string; note?: string; children: Snippet } = $props();
</script>

<section aria-label={title} class="flex flex-col gap-(--kit-space-md) rounded-(--kit-radius-lg) border border-(--kit-border) bg-(--kit-card) p-(--kit-space-lg)">
	<div class="flex flex-col gap-(--kit-space-xxs)">
		<h3 class="text-[length:var(--kit-text-md)] leading-(--kit-leading-md) font-semibold">{title}</h3>
		{#if note}
			<p class="max-w-[70ch] text-[length:var(--kit-text-xs)] leading-(--kit-leading-xs) text-[color:var(--kit-muted-foreground)]">{note}</p>
		{/if}
	</div>
	{@render children()}
</section>