<script lang="ts">
	import { cn } from '#lib/utils.js';
	import type { TerminalPayload } from '../types.js';

	interface Props {
		data: TerminalPayload;
		class?: string;
	}

	let { data, class: className }: Props = $props();

	const LINE: Record<TerminalPayload['lines'][number]['kind'], string> = {
		command: 'text-[color:var(--kit-foreground)]',
		stdout: 'text-[color:var(--kit-code-foreground)]',
		stderr: 'text-[color:var(--kit-danger)]',
		dim: 'text-[color:var(--kit-muted-foreground)]'
	};
</script>

<div
	data-kit="terminal-card"
	class={cn('overflow-hidden rounded-(--kit-radius-md) border border-(--kit-border) bg-(--kit-code-background)', className)}
>
	<div
		class="flex flex-wrap items-center justify-between gap-(--kit-space-sm) border-b border-(--kit-border) px-(--kit-space-sm) py-(--kit-space-xs)"
	>
		<div class="min-w-0">
			<p class="m-0 truncate font-mono text-[length:var(--kit-text-xs)] text-[color:var(--kit-foreground)]">
				<span class="text-[color:var(--kit-muted-foreground)]">$</span>
				<span class="ml-(--kit-space-xs)">{data.command}</span>
			</p>
			{#if data.cwd}
				<p class="m-0 truncate font-mono text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">{data.cwd}</p>
			{/if}
		</div>
		{#if data.exitCode !== 0}
			<span
				class="shrink-0 rounded-(--kit-radius-sm) border border-(--kit-danger) px-(--kit-space-sm) py-(--kit-space-xxs) text-[length:var(--kit-text-xs)] font-medium text-[color:var(--kit-danger)]"
			>
				Failed · exit {data.exitCode}
			</span>
		{:else}
			<span class="shrink-0 text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)] tabular-nums">
				exit 0
			</span>
		{/if}
	</div>

	<!-- A scrollable region has to be focusable, or a keyboard reader cannot scroll it. -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div role="region" aria-label="Command output, scrollable" tabindex="0" class="kit-scroll max-h-80 overflow-auto">
		<pre class="m-0 w-max min-w-full p-(--kit-space-sm) font-mono text-[length:var(--kit-text-mono)] leading-(--kit-leading-mono)"><code
				>{#each data.lines as line, index (index)}<span class="flex {LINE[line.kind]}"
					><span class="whitespace-pre pr-(--kit-space-lg)">{line.text}</span
					></span
				>{/each}</code
			></pre
		>
	</div>
</div>