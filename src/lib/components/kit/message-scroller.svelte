<script lang="ts" module>
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	export type MessageScrollerProps = {
		children?: Snippet;
		/** Distance from the bottom, in px, past which the scroller stops following new content. */
		stickThreshold?: number;
		/** Rendered above the bottom edge, outside the scroll region: a "jump to latest" control. */
		footer?: Snippet;
		class?: string | undefined;
		contentClass?: string | undefined;
	};
</script>

<script lang="ts">
	/**
	 * Owns its own scroll region, the way `MessageScroller` owns the message list in GPUI
	 * Kit rather than letting the window scroll the conversation. Auto-follow is the
	 * behaviour people expect from a chat and also the behaviour that loses work when it
	 * fights the reader, so following stops the moment the reader scrolls up and a control
	 * brings them back.
	 */
	let { children, stickThreshold = 64, footer, class: className, contentClass }: MessageScrollerProps = $props();

	let viewport = $state<HTMLDivElement | null>(null);
	let content = $state<HTMLDivElement | null>(null);
	let following = $state(true);

	function distanceFromBottom() {
		const node = viewport;
		if (!node) return 0;
		return node.scrollHeight - node.scrollTop - node.clientHeight;
	}

	function scrollToLatest(behavior: ScrollBehavior = 'smooth') {
		const node = viewport;
		if (!node) return;
		node.scrollTo({ top: node.scrollHeight, behavior });
		following = true;
	}

	function onScroll() {
		following = distanceFromBottom() <= stickThreshold;
	}

	// A resize observer rather than a content prop: messages, artifacts and streamed
	// tokens all change height without the scroller being told anything changed.
	$effect(() => {
		const node = content;
		if (!node) return;
		const observer = new ResizeObserver(() => {
			if (following) scrollToLatest('auto');
		});
		observer.observe(node);
		return () => observer.disconnect();
	});

	$effect(() => {
		scrollToLatest('auto');
	});
</script>

<div data-kit="message-scroller" class={cn('relative flex min-h-0 flex-1 flex-col', className)}>
		<!-- The viewport is a focusable scroll region on purpose: a conversation is
		     scrollable with the keyboard, and a region you can scroll should be
		     reachable with Tab. -->
		<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
		<div
			bind:this={viewport}
			onscroll={onScroll}
		class="kit-scroll min-h-0 flex-1 overflow-y-auto overscroll-contain"
		role="log"
		aria-live="polite"
		aria-relevant="additions text"
		tabindex="0"
	>
		<!-- The conversation keeps a readable measure on a wide window instead of
		     stretching to the viewport: a chat line has no natural length, and a full
		     width line is as hard to read as a narrow one. -->
		<div bind:this={content} class={cn('mx-auto flex w-full max-w-[880px] flex-col gap-[10px] px-(--kit-space-md) py-(--kit-space-lg) md:px-(--kit-space-xl)', contentClass)}>
			{@render children?.()}
		</div>
	</div>

	{#if !following}
		<button
			type="button"
			data-kit="jump-to-latest"
			class="absolute bottom-(--kit-space-md) left-1/2 flex -translate-x-1/2 items-center gap-(--kit-space-xs) rounded-(--kit-radius-full) border border-(--kit-border) bg-(--kit-popover) px-(--kit-space-md) py-(--kit-space-xs) text-[length:var(--kit-text-xs)] font-medium text-[color:var(--kit-foreground)] shadow-lg transition-colors duration-(--kit-duration-fast) hover:bg-(--kit-muted)"
			onclick={() => scrollToLatest()}
		>
			<svg viewBox="0 0 16 16" class="size-3.5" aria-hidden="true">
				<path d="M8 3v9m0 0l3.5-3.5M8 12L4.5 8.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" fill="none" />
			</svg>
			Jump to latest
		</button>
	{/if}

	{#if footer}
		{@render footer()}
	{/if}
</div>