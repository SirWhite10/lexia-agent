<script lang="ts">
	import { cn } from '#lib/utils.js';
	import type { MediaPayload } from '../types.js';

	interface Props {
		data: MediaPayload;
		class?: string;
	}

	let { data, class: className }: Props = $props();

	// The frame is reserved before the bytes arrive, so nothing below it moves
	// when the image loads, and a broken URL leaves the alt text visible.
	let broken = $state(false);
</script>

<figure data-kit="media-card" class={cn('m-0 min-w-0', className)}>
	<img
		src={data.imageUrl}
		alt={data.alt}
		loading="lazy"
		onerror={() => (broken = true)}
		onload={() => (broken = false)}
		class={cn(
			'w-full rounded-(--kit-radius-md) border border-(--kit-border) bg-(--kit-muted) object-cover',
			broken && 'min-h-(--kit-space-xxl)'
		)}
		style:aspect-ratio={broken ? undefined : data.ratio}
	/>

	<figcaption class="mt-(--kit-space-sm) flex flex-wrap items-start justify-between gap-(--kit-space-sm)">
		<p class="m-0 min-w-0 flex-1 text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">
			{#if broken}
				<span class="text-[color:var(--kit-danger)]">The image did not load.</span>
				{data.alt}
			{:else if data.caption}
				{data.caption}
			{:else}
				{data.alt}
			{/if}
		</p>
		<div class="flex shrink-0 items-center gap-(--kit-space-sm) text-[length:var(--kit-text-xs)]">
			<a
				href={data.imageUrl}
				download
				target="_blank"
				rel="noopener noreferrer"
				class="rounded-(--kit-radius-sm) border border-(--kit-border) px-(--kit-space-sm) py-(--kit-space-xxs) text-[color:var(--kit-foreground)] transition-colors duration-(--kit-duration-fast) hover:bg-(--kit-secondary) active:bg-(--kit-secondary-active)"
			>
				Download
			</a>
			<a
				href={data.imageUrl}
				target="_blank"
				rel="noopener noreferrer"
				class="text-[color:var(--kit-link)] underline-offset-4 hover:underline"
			>
				Open<span aria-hidden="true" class="ml-(--kit-space-xxs)">↗</span>
				<span class="sr-only"> image in a new tab</span>
			</a>
		</div>
	</figcaption>
</figure>