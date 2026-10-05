<script lang="ts" module>
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	/** `gpui-component/src/message.rs` → `MessageAlignment`. */
	export type MessageAlignment = 'start' | 'end';

	export type MessageProps = {
		alignment?: MessageAlignment;
		/** Identity slot. Upstream reserves a 32px circle at the bottom edge of the content. */
		avatar?: Snippet;
		/** Sender name and time. Upstream drops its horizontal inset when the content holds a ghost bubble. */
		header?: Snippet;
		children?: Snippet;
		/** Delivery state, reactions or actions. Sits outside the avatar row so it never moves the avatar. */
		footer?: Snippet;
		/** Set when the content holds a ghost bubble, which has no surface to inset against. */
		ghostContent?: boolean;
		class?: string | undefined;
	};
</script>

<script lang="ts">
	let { alignment = 'start', avatar, header, children, footer, ghostContent = false, class: className }: MessageProps = $props();

	/** Upstream `MessageHeader`/`MessageFooter`: gap 1, text_xs, line height 1.25, medium weight, muted. */
	const metaRow = 'flex max-w-full min-w-0 gap-(--kit-space-xs) text-[length:var(--kit-text-xs)] leading-[1.25] font-medium text-[color:var(--kit-muted-foreground)]';
</script>

<div
	data-kit="message"
	data-alignment={alignment}
	class={cn('relative flex w-full min-w-0 flex-col gap-[10px]', alignment === 'end' ? 'items-end' : 'items-start', className)}
>
	<!-- The inner row is bottom-anchored so the avatar stays flush with the last line of
	     the message; it reverses for an outgoing message, putting the avatar on the right. -->
	<div class="flex w-full min-w-0 items-end gap-(--kit-space-sm)" class:flex-row-reverse={alignment === 'end'}>
		{#if avatar}
			<div
				data-kit="message-avatar"
				class="flex min-w-[32px] shrink-0 items-center justify-center self-end overflow-hidden rounded-(--kit-radius-full) bg-(--kit-muted)"
			>
				{@render avatar()}
			</div>
		{/if}

		<div class="flex w-full min-w-0 flex-col gap-[10px]" class:items-end={alignment === 'end'} class:items-start={alignment === 'start'}>
			{#if header}
				<div class={cn(metaRow, ghostContent ? '' : 'px-(--kit-space-md)')}>{@render header()}</div>
			{/if}
			{@render children?.()}
		</div>
	</div>

	{#if footer}
		<!-- Offset by the avatar baseline plus the row gap so the footer aligns with the
		     content column rather than with the avatar. -->
		<div class={cn(metaRow, ghostContent ? '' : 'px-(--kit-space-md)', avatar && (alignment === 'end' ? 'mr-[40px]' : 'ml-[40px]'))}>
			{@render footer()}
		</div>
	{/if}
</div>