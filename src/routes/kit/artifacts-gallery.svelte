<script lang="ts">
	import { seededArtifacts } from '#lib/components/kit/demo/transcript.js';
	import ArtifactHost from '#lib/components/kit/artifacts/artifact-host.svelte';

	/**
	 * Every artifact the agent can emit, in one place. The chat shows them in context;
	 * this view exists so the whole set can be compared side by side on one screen size,
	 * which is the only practical way to notice two cards disagreeing about padding.
	 */
	const gallery = Object.values(seededArtifacts);
</script>

<section aria-label="Artifacts" class="flex min-h-0 flex-1 flex-col">
	<header class="border-b border-(--kit-border) px-(--kit-space-md) py-(--kit-space-md) md:px-(--kit-space-xl)">
		<h2 class="text-[length:var(--kit-text-lg)] leading-(--kit-leading-lg) font-semibold">Artifacts</h2>
		<p class="mt-(--kit-space-xs) max-w-[70ch] text-[length:var(--kit-text-sm)] leading-(--kit-leading-sm) text-[color:var(--kit-muted-foreground)]">
			{gallery.length} shapes. Four carry source the agent wrote — HTML, Svelte and React run in a
			sandboxed frame, plain code does not run at all — and the rest are typed data the host
			draws itself.
		</p>
	</header>

	<div class="kit-scroll min-h-0 flex-1 overflow-y-auto px-(--kit-space-md) py-(--kit-space-lg) md:px-(--kit-space-xl)">
		<div class="grid grid-cols-1 items-start gap-(--kit-space-lg) lg:grid-cols-2 2xl:grid-cols-3">
			{#each gallery as artifact (artifact.id)}
				<ArtifactHost {artifact} />
			{/each}
		</div>
	</div>
</section>