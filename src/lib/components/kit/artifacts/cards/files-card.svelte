<script lang="ts">
	import { cn } from '#lib/utils.js';
	import type { FilesPayload } from '../types.js';

	interface Props {
		data: FilesPayload;
		class?: string;
	}

	let { data, class: className }: Props = $props();
</script>

<!--
	The state marker is a word with a glyph, never a tint alone: "created" and
	"modified" read the same in a monochrome screenshot.
-->
<div
	data-kit="files-card"
	class={cn('overflow-hidden rounded-(--kit-radius-md) border border-(--kit-border)', className)}
>
	<p
		class="m-0 truncate border-b border-(--kit-border) px-(--kit-space-sm) py-(--kit-space-xs) font-mono text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]"
	>
		{data.root}
	</p>

	<ul class="m-0 list-none divide-y divide-(--kit-border) p-0">
		{#each data.files as file (file.path)}
			<li
				class="flex min-w-0 items-center gap-(--kit-space-sm) px-(--kit-space-sm) py-(--kit-space-xs) text-[length:var(--kit-text-sm)]"
			>
				<span
					class="w-20 shrink-0 text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]"
					class:text-[color:var(--kit-success)]={file.created}
					class:text-[color:var(--kit-warning)]={file.modified}
				>
					{#if file.created}
						<span aria-hidden="true" class="font-mono">+</span> created{:else if file.modified}
						<span aria-hidden="true" class="font-mono">~</span> modified{:else}
						<span aria-hidden="true" class="font-mono">·</span> unchanged{/if}
				</span>
				<span class="min-w-0 flex-1 truncate font-mono text-[length:var(--kit-text-xs)]" title={file.path}>
					{file.path}
				</span>
				<span class="shrink-0 text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)] tabular-nums">{file.size}</span>
			</li>
		{/each}
	</ul>
</div>