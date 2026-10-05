<script lang="ts" module>
	/**
	 * Port of `gpui-component`'s `DescriptionList`.
	 *
	 * The crate (description_list.rs:250) lays each item out as a label column
	 * beside a value column, fills the label cell with
	 * `description_list_label` and draws 1px `border` rules between rows and
	 * columns. In a browser a bordered table of boxes is heavier than the crate's
	 * flat reading, so the rules here are hairlines on the row only — the label
	 * cell is not a filled box — and the label takes `muted_foreground` at
	 * `text_xs`, which is how the crate weights a label against its value.
	 *
	 * `columns` only takes effect above the medium breakpoint: on a phone two
	 * columns of label/value pairs are unreadable at the width available.
	 */

	import type { HTMLAttributes } from 'svelte/elements';

	export type KitDescriptionItem = {
		label: string;
		value: string;
		/** A second line under the value, for the qualifier the value needs. */
		hint?: string;
	};

	export type KitDescriptionListProps = Omit<HTMLAttributes<HTMLDListElement>, 'children'> & {
		items: KitDescriptionItem[];
		columns?: 1 | 2;
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let {
		items,
		columns = 1,
		class: className,
		...restProps
	}: KitDescriptionListProps = $props();
</script>

<dl
	data-kit="description-list"
	class={cn(
		'grid w-full min-w-0',
		// Two columns need a real width; below `md` they collapse to one and the
		// label sits above its value rather than beside it.
		columns === 2 ? 'grid-cols-1 md:grid-cols-2' : 'grid-cols-1',
		className,
	)}
	{...restProps}
>
	{#each items as item, index (index)}
		<div
			class={cn(
				'flex min-w-0 flex-col gap-(--kit-space-xxs) px-(--kit-space-sm) py-(--kit-space-sm)',
				index < items.length - 1 && 'border-b border-(--kit-border)',
				// In two columns the last row is the last one or two items, and a rule
				// under it would hang below the list.
				columns === 2 && 'md:[&:nth-last-child(-n+2)]:border-b-0',
			)}
		>
			<dt class="text-[length:var(--kit-text-xs)] leading-(--kit-leading-xs) text-[color:var(--kit-muted-foreground)]">
				{item.label}
			</dt>
			<dd
				class="m-0 flex min-w-0 flex-col gap-(--kit-space-xxs) text-[length:var(--kit-text-sm)] leading-(--kit-leading-sm) text-[color:var(--kit-foreground)]"
			>
				<span class="min-w-0">{item.value}</span>
				{#if item.hint}
					<span class="text-[length:var(--kit-text-xs)] leading-(--kit-leading-xs) text-[color:var(--kit-muted-foreground)]">
						{item.hint}
					</span>
				{/if}
			</dd>
		</div>
	{/each}
</dl>
