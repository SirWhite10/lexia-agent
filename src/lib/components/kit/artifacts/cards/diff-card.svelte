<script lang="ts">
	import { cn } from '#lib/utils.js';
	import type { DiffPayload } from '../types.js';

	interface Props {
		data: DiffPayload;
		class?: string;
	}

	let { data, class: className }: Props = $props();

	/**
	 * The marker column repeats the sign as a glyph so the diff still reads when
	 * the added and removed tints are indistinguishable.
	 */
	const LINE: Record<DiffPayload['lines'][number]['kind'], { mark: string; class: string }> = {
		context: { mark: ' ', class: 'text-[color:var(--kit-code-foreground)]' },
		add: { mark: '+', class: 'bg-(--kit-code-added) text-[color:var(--kit-code-added-foreground)]' },
		remove: { mark: '−', class: 'bg-(--kit-code-removed) text-[color:var(--kit-code-removed-foreground)]' }
	};
</script>

<div
	data-kit="diff-card"
	class={cn('overflow-hidden rounded-(--kit-radius-md) border border-(--kit-border)', className)}
>
	<div
		class="flex flex-wrap items-center justify-between gap-(--kit-space-sm) border-b border-(--kit-border) px-(--kit-space-sm) py-(--kit-space-xs)"
	>
		<span class="truncate font-mono text-[length:var(--kit-text-xs)] text-[color:var(--kit-foreground)]">{data.path}</span>
		<span class="flex items-center gap-(--kit-space-sm) text-[length:var(--kit-text-xs)] tabular-nums">
			<span class="text-[color:var(--kit-code-added-foreground)]"><span aria-hidden="true">+</span>{data.added} added</span>
			<span class="text-[color:var(--kit-code-removed-foreground)]"><span aria-hidden="true">−</span>{data.removed} removed</span>
		</span>
	</div>

	<!-- A scrollable region has to be focusable, or a keyboard reader cannot scroll it. -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div role="region" aria-label="Diff, scrollable sideways" tabindex="0" class="kit-scroll overflow-x-auto">
		<pre class="m-0 w-max min-w-full font-mono text-[length:var(--kit-text-mono)] leading-(--kit-leading-mono)"><code
				>{#each data.lines as line, index (index)}{@const style = LINE[line.kind]}<span class="flex {style.class}"
					><span aria-hidden="true" class="w-(--kit-space-md) shrink-0 select-none text-center">{style.mark}</span
					><span class="whitespace-pre pr-(--kit-space-md)">{line.text}</span
					></span
				>{/each}</code
			></pre
		>
	</div>
</div>