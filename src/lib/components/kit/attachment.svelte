<script lang="ts" module>
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	export type AttachmentKind = 'image' | 'file' | 'code';

	export type AttachmentProps = {
		name: string;
		kind?: AttachmentKind;
		/** Pre-formatted by the caller: an attachment card never re-derives a file size. */
		size?: string;
		/** While uploading, the card is inert; while failed, the retry control appears. */
		state?: 'uploading' | 'ready' | 'failed';
		/** 0-100, only meaningful while uploading. */
		progress?: number;
		/** Optional media preview. Images render above the text instead of beside it. */
		preview?: Snippet;
		onremove?: () => void;
		onretry?: () => void;
		class?: string | undefined;
	};
</script>

<script lang="ts">
	/** Sizes and control geometry from `gpui-component/src/attachment.rs`: the remove control is 20px overhanging the card by 6px, the retry control 24px, and the card radius comes from the size. */
	let { name, kind = 'file', size, state = 'ready', progress = 0, preview, onremove, onretry, class: className }: AttachmentProps = $props();

	const GLYPH: Record<AttachmentKind, string> = {
		image: 'M2 4.5A1.5 1.5 0 0 1 3.5 3h9A1.5 1.5 0 0 1 14 4.5v7a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 2 11.5zM5.5 9.5l2-2 1.5 1.5L11 7l2 2.5',
		code: 'M5.75 5.5L3.5 8l2.25 2.5M10.25 5.5L12.5 8l-2.25 2.5M9.5 3.5l-3 9',
		file: 'M4 2.5h5l3 3v8H4zM9 2.5V6h3'
	};
</script>

<div
	data-kit="attachment"
	data-state={state}
	class={cn(
		'relative flex min-w-0 flex-col gap-(--kit-space-sm) overflow-hidden rounded-(--kit-radius-lg) border border-(--kit-border) bg-(--kit-card) p-(--kit-space-sm) pr-[calc(var(--kit-space-md)+14px)] text-[color:var(--kit-foreground)]',
		// An inert card must read as inert, not merely be unclickable.
		state === 'uploading' && 'opacity-70',
		state === 'failed' && 'border-(--kit-danger)',
		className
	)}
>
	{#if preview}
		<div class="-m-(--kit-space-sm) -mr-[calc(var(--kit-space-md)+14px)] mb-(--kit-space-xs) overflow-hidden border-b border-(--kit-border)">
			{@render preview()}
		</div>
	{/if}

	<div class="flex min-w-0 items-center gap-(--kit-space-sm)">
		<span class="grid size-[28px] shrink-0 place-items-center rounded-(--kit-radius-sm) bg-(--kit-muted) text-[color:var(--kit-muted-foreground)]" aria-hidden="true">
			<svg viewBox="0 0 16 16" class="size-4" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
				<path d={GLYPH[kind]} />
			</svg>
		</span>
		<span class="flex min-w-0 flex-col">
			<span class="truncate text-[length:var(--kit-text-sm)] leading-(--kit-leading-sm) font-medium">{name}</span>
			<span class="truncate text-[length:var(--kit-text-xs)] leading-(--kit-leading-xs) text-[color:var(--kit-muted-foreground)]">
				{#if state === 'failed'}
					Upload failed
				{:else if size}
					{size}
				{:else}
					{kind}
				{/if}
			</span>
		</span>
	</div>

	{#if state === 'uploading'}
		<div
			class="h-1 w-full overflow-hidden rounded-(--kit-radius-full) bg-(--kit-skeleton)"
			role="progressbar"
			aria-valuenow={Math.round(progress)}
			aria-valuemin="0"
			aria-valuemax="100"
			aria-label="Uploading {name}"
		>
			<div class="h-full rounded-(--kit-radius-full) bg-(--kit-primary) transition-[width] duration-(--kit-duration-normal) ease-(--kit-ease-move)" style="width: {Math.min(100, Math.max(0, progress))}%"></div>
		</div>
	{/if}

	{#if state === 'failed' && onretry}
		<button
			type="button"
			data-kit="attachment-retry"
			class="flex size-[24px] items-center justify-center self-start rounded-(--kit-radius-full) text-[color:var(--kit-danger)] transition-colors duration-(--kit-duration-fast) hover:bg-(--kit-danger)/12"
			onclick={onretry}
			aria-label="Retry uploading {name}"
		>
			<svg viewBox="0 0 16 16" class="size-3.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
				<path d="M13 8a5 5 0 1 1-1.6-3.7M13 2.5V5h-2.5" />
			</svg>
		</button>
	{/if}

	{#if onremove}
		<button
			type="button"
			data-kit="attachment-remove"
			class="absolute top-0 right-0 grid size-[20px] place-items-center rounded-full bg-(--kit-popover) text-[color:var(--kit-muted-foreground)] shadow-xs ring-1 ring-(--kit-border) transition-colors duration-(--kit-duration-fast) hover:text-[color:var(--kit-danger)]"
			onclick={onremove}
			aria-label="Remove {name}"
		>
			<svg viewBox="0 0 16 16" class="size-3" aria-hidden="true">
				<path d="M4.5 4.5l7 7M11.5 4.5l-7 7" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" fill="none" />
			</svg>
		</button>
	{/if}
</div>