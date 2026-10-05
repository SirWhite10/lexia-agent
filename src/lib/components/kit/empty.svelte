<script lang="ts" module>
	import type { Snippet } from 'svelte';

	export type KitEmptyProps = {
		title: string;
		description?: string;
		/** A 32px muted frame for an icon, matching `EmptyMediaVariant::Icon`. */
		icon?: Snippet;
		/** Actions, rendered under the copy. */
		children?: Snippet;
		class?: string;
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let {
		title,
		description,
		icon,
		children,
		class: className,
	}: KitEmptyProps = $props();
</script>

<!--
	`empty.rs` is a dashed, centred frame (`p_6`, `radius_xl`) holding an
	optional media slot, a medium-weight title and a muted description; the
	port drops the dashed border so the state reads as a region of the page
	rather than an object.
-->
<div
	data-kit="empty"
	class={cn(
		'flex w-full min-w-0 flex-col items-center justify-center gap-(--kit-space-md)',
		'rounded-(--kit-radius-xl) px-(--kit-space-xl) py-(--kit-space-xl) text-center',
		className,
	)}
>
	<div class="flex w-full max-w-xl min-w-0 flex-col items-center gap-(--kit-space-sm)">
		{#if icon}
			<div
				class="mb-(--kit-space-xs) flex size-8 shrink-0 items-center justify-center rounded-(--kit-radius-md) bg-(--kit-muted) text-[length:var(--kit-text-md)] text-[color:var(--kit-foreground)]"
			>
				{@render icon()}
			</div>
		{/if}

		<p class="max-w-full min-w-0 text-[length:var(--kit-text-sm)] leading-(--kit-leading-sm) font-medium text-[color:var(--kit-foreground)]">
			{title}
		</p>

		{#if description}
			<p class="w-full min-w-0 text-[length:var(--kit-text-sm)] text-[color:var(--kit-muted-foreground)]">
				{description}
			</p>
		{/if}

		{#if children}
			<div class="mt-(--kit-space-xs) flex flex-wrap items-center justify-center gap-(--kit-space-sm)">
				{@render children()}
			</div>
		{/if}
	</div>
</div>
