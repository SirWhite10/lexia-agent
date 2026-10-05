<script lang="ts">
	import { cn } from '#lib/utils.js';
	import type { LinkPayload } from '../types.js';

	interface Props {
		data: LinkPayload;
		class?: string;
	}

	let { data, class: className }: Props = $props();
</script>

<a
	data-kit="link-card"
	href={data.url}
	target="_blank"
	rel="noopener noreferrer"
	class={cn(
		'flex min-w-0 flex-col gap-(--kit-space-sm) rounded-(--kit-radius-md) border border-(--kit-border) p-(--kit-space-md) no-underline transition-colors duration-(--kit-duration-fast) hover:bg-(--kit-list-hover) active:bg-(--kit-list-active)',
		className
	)}
>
	<span class="flex items-center gap-(--kit-space-xs) text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">
		<span class="truncate">{data.host}</span>
		<span class="text-[length:var(--kit-text-xs)]">{data.site}</span>
		<span aria-hidden="true">↗</span>
		<span class="sr-only">(opens in a new tab)</span>
	</span>

	<span class="flex flex-col gap-(--kit-space-xs) sm:flex-row sm:items-start sm:justify-between">
		<span class="flex min-w-0 flex-1 flex-col gap-(--kit-space-xxs)">
			<span class="break-words text-[length:var(--kit-text-md)] leading-(--kit-leading-md) font-medium text-[color:var(--kit-foreground)]">
				{data.title}
			</span>
			{#if data.description}
				<span class="text-[length:var(--kit-text-sm)] leading-(--kit-leading-sm) text-[color:var(--kit-muted-foreground)]">
					{data.description}
				</span>
			{/if}
		</span>
		{#if data.imageUrl}
			<img
				src={data.imageUrl}
				alt=""
				loading="lazy"
				class="w-full shrink-0 rounded-(--kit-radius-sm) bg-(--kit-muted) object-cover sm:w-40"
				style:aspect-ratio="16 / 9"
			/>
		{/if}
	</span>
</a>