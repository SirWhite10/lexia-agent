<script lang="ts" module>
	import { cn } from '#lib/utils.js';
	import type { ChatMessage } from '../chat/model.js';
	import { KIND_GLYPH } from './composer-model.js';
	import ArtifactHost from '../artifacts/artifact-host.svelte';
	import Attachment from '../attachment.svelte';
	import Avatar from '../avatar.svelte';
	import Bubble from '../bubble.svelte';
	import Button from '../button.svelte';
	import Message from '../message.svelte';
	import Shimmer from '../shimmer.svelte';
	import Tooltip from '../tooltip.svelte';

	export type MessageItemProps = {
		message: ChatMessage;
		/** Set from `$effect` after mount so the server and client render the same clock. */
		hydrated?: boolean;
		class?: string | undefined;
	};
</script>

<script lang="ts">
	let { message, hydrated = false, class: className }: MessageItemProps = $props();

	let copied = $state(false);

	const isUser = $derived(message.authorKind === 'user');
	const alignment = $derived<'start' | 'end'>(isUser ? 'end' : 'start');

	/** Human-scale relative time; the seeded conversation is minutes old, a live run is seconds old. */
	function relativeTime(from: number, now: number): string {
		const seconds = Math.max(0, Math.round((now - from) / 1000));
		if (seconds < 45) return 'just now';
		const minutes = Math.round(seconds / 60);
		if (minutes < 60) return `${minutes}m ago`;
		const hours = Math.round(minutes / 60);
		if (hours < 24) return `${hours}h ago`;
		return `${Math.round(hours / 24)}d ago`;
	}

	async function copyBody() {
		await navigator.clipboard.writeText(message.body);
		copied = true;
		setTimeout(() => (copied = false), 1500);
	}
</script>

<article
	data-kit="message-item"
	data-author={message.authorKind}
	class={cn('group relative w-full min-w-0', className)}
	aria-label="{message.authorKind === 'user' ? 'You' : 'Lexia'} said"
>
	<Message {alignment} ghostContent={message.streaming === true}>
		{#snippet avatar()}
			{#if isUser}
				<Avatar name="You" size="md" />
			{:else}
				<Avatar name="Lexia" size="md" />
			{/if}
		{/snippet}

		{#snippet header()}
			<span class="font-medium text-[color:var(--kit-foreground)]">{isUser ? 'You' : 'Lexia'}</span>
			{#if hydrated}<span aria-hidden="true">·</span><span>{relativeTime(message.createdAt, Date.now())}</span>{/if}
			{#if message.runId}
				<span aria-hidden="true">·</span>
				<span class="font-mono">{message.runId}</span>
			{/if}
		{/snippet}

		<div class="flex w-full min-w-0 flex-col items-start gap-(--kit-space-sm)" class:items-end={alignment === 'end'}>
			<div class="flex max-w-full flex-col items-start gap-(--kit-space-xs)" class:items-end={alignment === 'end'}>
				{#if message.streaming}
					<Shimmer text={message.body} />
				{:else}
					<Bubble variant={isUser ? 'filled' : 'muted'}>
						<p class="whitespace-pre-wrap break-words">{message.body}</p>
					</Bubble>
				{/if}

				{#if message.attachments && message.attachments.length > 0}
					<ul class="flex w-full flex-wrap gap-(--kit-space-sm)">
						{#each message.attachments as attachment (attachment.name)}
							<li class="w-full max-w-[240px]">
								<Attachment name={attachment.name} kind={attachment.kind} size={attachment.size} state={attachment.state ?? 'ready'} />
							</li>
						{/each}
					</ul>
				{/if}

				{#if message.context && message.context.length > 0}
					<ul class="flex flex-wrap items-center gap-(--kit-space-xs)" aria-label="Context added to this turn">
						{#each message.context as item (item.token)}
							<li class="flex items-center gap-(--kit-space-xs) rounded-(--kit-radius-md) border border-(--kit-border) bg-(--kit-card) px-(--kit-space-sm) py-[2px] text-[length:var(--kit-text-xs)] leading-[length:var(--kit-leading-xs)]">
								<span class="text-[color:var(--kit-muted-foreground)]" aria-hidden="true">{KIND_GLYPH[item.kind as keyof typeof KIND_GLYPH] ?? '#'}</span>
								<span class="font-medium">{item.label}</span>
								{#if item.detail}
									<span class="max-w-[200px] truncate text-[color:var(--kit-muted-foreground)]">{item.detail}</span>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
			</div>

			<!-- Artifacts sit outside the bubble and span the message column: they are wide
			     documents, not something a chat bubble should squeeze. -->
			{#if message.artifacts && message.artifacts.length > 0}
				<div class="flex w-full min-w-0 flex-col gap-(--kit-space-md)">
					{#each message.artifacts as artifact (artifact.id)}
						<ArtifactHost {artifact} />
					{/each}
				</div>
			{/if}
		</div>

		{#snippet footer()}
			<div class="flex items-center gap-(--kit-space-xxs) opacity-0 transition-opacity duration-(--kit-duration-fast) group-hover:opacity-100 group-focus-within:opacity-100">
				<Tooltip label={copied ? 'Copied' : 'Copy this message'}>
					<Button variant="ghost" size="xsmall" iconOnly onclick={copyBody} aria-label="Copy this message">
						{#if copied}
							<svg viewBox="0 0 16 16" class="size-3.5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3.5 8.5l3 3 6-6.5" /></svg>
						{:else}
							<svg viewBox="0 0 16 16" class="size-3.5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.25"><rect x="5.5" y="5.5" width="7" height="7" rx="1.5" /><path d="M10.5 3.5H4a1.5 1.5 0 0 0-1.5 1.5v6.5" /></svg>
						{/if}
					</Button>
				</Tooltip>
				{#if message.runId}
					<span class="font-mono text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">from {message.runId}</span>
				{/if}
			</div>
		{/snippet}
	</Message>
</article>