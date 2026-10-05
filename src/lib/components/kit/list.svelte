<script lang="ts" module>
	/**
	 * Port of `gpui-component`'s `List` / `ListItem` (list/list_item.rs:187).
	 *
	 * From the crate: the row is a full-width flex item, `items_center` +
	 * `justify_between`, padded `py_1 px_3` at `text_base`; hover fills with
	 * `tokens.list_hover`; a selected row fills with `list_active` and, when the
	 * theme does not highlight actively, is drawn with a 1px `selection` outline.
	 * Rows are separated by nothing at all — the surface change carries the
	 * structure — and even rows pick up `list_even`.
	 *
	 * Two ports are deliberate. The row is a real `<button>`, because the crate
	 * gets interactivity from GPUI's click handler and a browser needs the
	 * element itself. And the selection marker is the crate's `list_active_border`
	 * inset 2px from the row edge, which is what `active_highlight` themes show
	 * on touch, where a colour-only selection is not enough on its own.
	 */

	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';

	export type KitListItem = {
		id: string;
		primary: string;
		secondary?: string;
		/** Trailing text, right-aligned, for a timestamp or a count. */
		meta?: string;
		/** Leading glyph or badge, supplied by the caller. */
		icon?: Snippet;
		/** Forces the row selected regardless of `selectedId`. */
		selected?: boolean;
		disabled?: boolean;
	};

	export type KitListProps = Omit<HTMLAttributes<HTMLUListElement>, 'children'> & {
		items: KitListItem[];
		/** The selected row's id, bindable so the caller owns the selection. */
		selectedId?: string | null;
		emptyText?: string;
		/**
		 * The height of the scrolling region. GPUI lists are virtualised and always
		 * bounded by the window; in a page a list must be bounded explicitly or it
		 * scrolls the document instead of its own box.
		 */
		maxHeight?: string;
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let {
		items,
		selectedId = $bindable(null),
		emptyText,
		maxHeight = '20rem',
		class: className,
		...restProps
	}: KitListProps = $props();

	function isSelected(item: KitListItem): boolean {
		return item.selected ?? selectedId === item.id;
	}

	function select(id: string) {
		selectedId = selectedId === id ? null : id;
	}
</script>

<ul
	data-kit="list"
	class={cn('kit-scroll w-full min-w-0 list-none overflow-y-auto p-0', className)}
	style:max-height={maxHeight}
	{...restProps}
>
	{#if items.length === 0}
		<li class="px-(--kit-space-md) py-(--kit-space-lg) text-center text-[color:var(--kit-muted-foreground)]">
			{emptyText ?? 'Nothing to show'}
		</li>
	{:else}
		{#each items as item (item.id)}
			{@const selected = isSelected(item)}
			<li class="even:bg-(--kit-list-even)">
				<button
					type="button"
					class={cn(
						'flex w-full items-center gap-(--kit-space-md) px-(--kit-space-md) py-(--kit-space-md)',
						'text-left text-[length:var(--kit-text-sm)] text-[color:var(--kit-foreground)]',
						'transition-colors duration-(--kit-duration-fast)',
						'hover:bg-(--kit-list-hover)',
						'active:bg-(--kit-secondary-active)',
						'disabled:pointer-events-none disabled:text-[color:var(--kit-muted-foreground)] disabled:opacity-50',
						selected && [
							'bg-(--kit-list-active)',
							'shadow-[inset_2px_0_0_var(--kit-list-active-border)]',
						],
					)}
					aria-pressed={selected}
					onclick={() => select(item.id)}
					disabled={item.disabled}
				>
					{#if item.icon}
						<span class="flex size-6 shrink-0 items-center justify-center text-[color:var(--kit-muted-foreground)]">
							{@render item.icon()}
						</span>
					{/if}

					<span class="flex min-w-0 flex-1 flex-col">
						<span class="truncate font-medium">{item.primary}</span>
						{#if item.secondary}
							<span class="truncate text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">
								{item.secondary}
							</span>
						{/if}
					</span>

					{#if item.meta}
						<span class="shrink-0 text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)] [font-family:var(--kit-font-mono)]">
							{item.meta}
						</span>
					{/if}
				</button>
			</li>
		{/each}
	{/if}
</ul>
