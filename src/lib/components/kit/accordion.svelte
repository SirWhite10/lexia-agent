<script lang="ts" module>
	/**
	 * Port of `gpui-component`'s `Accordion` / `AccordionItem`.
	 *
	 * Metrics from the crate (accordion.rs:279):
	 *   - the trigger is `h_flex` + `justify_between` + `gap_3`, medium weight,
	 *     padded `py_2 px_3` at the default size;
	 *   - the panel body carries the matching `pb_2 px_3`, so content lines up
	 *     under the title rather than under the chevron;
	 *   - items are joined by a 1px `border_b` in `border`, never by a nested card;
	 *   - the chevron is `ChevronDown` rotated a half turn when open.
	 *
	 * The crate springs the panel open. A browser has no spring here, so the reveal
	 * animates `grid-template-rows` from `0fr` to `1fr` over
	 * `--kit-duration-normal`: the height follows the content instead of jumping
	 * to it, and nothing has to be measured in JavaScript.
	 */

	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';

	export type KitAccordionItem = {
		id: string;
		title: string;
		content?: Snippet;
		disabled?: boolean;
	};

	export type KitAccordionProps = Omit<HTMLAttributes<HTMLDivElement>, 'children'> & {
		items: KitAccordionItem[];
		/** The open item's id, bindable so the caller can drive the accordion. */
		value?: string | null;
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let {
		items,
		value = $bindable(null),
		class: className,
		...restProps
	}: KitAccordionProps = $props();

	/** Unique per instance, so two accordions on one page never share ids. */
	const uid = $props.id();

	/** Trigger buttons in document order, so arrow keys never leave this accordion. */
	let triggers: HTMLButtonElement[] = $state([]);

	function toggle(id: string) {
		// Single expand: opening an item closes whichever one was open.
		value = value === id ? null : id;
	}

	function triggerId(id: string) {
		return `${uid}-trigger-${id}`;
	}

	function panelId(id: string) {
		return `${uid}-panel-${id}`;
	}

	/**
	 * The crate moves between list rows with the up/down keys; here the triggers
	 * are buttons in a column, so ArrowDown/ArrowUp walk them (skipping disabled
	 * ones) and Home/End jump to the ends. Left/Right are left alone — they belong
	 * to whatever the panel content itself is doing.
	 */
	function onKeydown(event: KeyboardEvent) {
		if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;

		const focusable = items
			.map((item, index) => ({ item, index }))
			.filter((entry) => !entry.item.disabled)
			.map((entry) => entry.index);
		if (focusable.length === 0) return;

		const current = triggers.indexOf(event.target as HTMLButtonElement);
		const position = focusable.indexOf(current);
		const next = (() => {
			if (event.key === 'Home') return focusable[0];
			if (event.key === 'End') return focusable[focusable.length - 1];
			// A trigger that is somehow not in the list still moves somewhere sane.
			if (position < 0) return focusable[0];
			const step = event.key === 'ArrowDown' ? 1 : -1;
			return focusable[(position + step + focusable.length) % focusable.length];
		})();

		event.preventDefault();
		triggers[next]?.focus();
	}
</script>

<div
	data-kit="accordion"
	class={cn(
		'flex w-full min-w-0 flex-col overflow-hidden rounded-(--kit-radius-lg) border border-(--kit-border)',
		className,
	)}
	{...restProps}
>
	{#each items as item, index (item.id)}
		{@const open = value === item.id}
		<div class={cn(index < items.length - 1 && 'border-b border-(--kit-border)')}>
			<h3 class="m-0">
				<button
					id={triggerId(item.id)}
					bind:this={triggers[index]}
					type="button"
					class={cn(
						'flex w-full items-center justify-between gap-(--kit-space-md) px-(--kit-space-md) py-(--kit-space-sm)',
						'text-left text-[length:var(--kit-text-sm)] font-medium text-[color:var(--kit-foreground)]',
						'transition-colors duration-(--kit-duration-fast)',
						'hover:bg-(--kit-list-hover)',
						'active:bg-(--kit-secondary-active)',
						'disabled:pointer-events-none disabled:text-[color:var(--kit-muted-foreground)] disabled:opacity-50',
					)}
					aria-expanded={open}
					aria-controls={panelId(item.id)}
					onclick={() => toggle(item.id)}
					onkeydown={onKeydown}
					disabled={item.disabled}
				>
					<span class="min-w-0 truncate">{item.title}</span>
					<svg
						class={cn(
							'size-4 shrink-0 text-[color:var(--kit-muted-foreground)]',
							'transition-transform duration-(--kit-duration-normal) ease-(--kit-ease-move)',
							'motion-reduce:transition-none',
							open && 'rotate-180',
						)}
						viewBox="0 0 16 16"
						fill="none"
						stroke="currentColor"
						stroke-width="1.5"
						stroke-linecap="round"
						stroke-linejoin="round"
						aria-hidden="true"
					>
						<path d="m4 6 4 4 4-4" />
					</svg>
				</button>
			</h3>

			<!-- The inner `min-h-0 overflow-hidden` wrapper is what lets a `0fr` grid
			     row collapse to nothing while the content stays laid out, so the
			     open/close transition has a height to interpolate. `inert` keeps a
			     closed panel out of the tab order and the accessibility tree without
			     unmounting it mid-transition. -->
			<div
				id={panelId(item.id)}
				role="region"
				aria-labelledby={triggerId(item.id)}
				inert={!open}
				class={cn(
					'grid transition-[grid-template-rows] duration-(--kit-duration-normal) ease-(--kit-ease-move)',
					'motion-reduce:transition-none',
					open ? 'grid-rows-[1fr]' : 'grid-rows-[0fr]',
				)}
			>
				<div class="min-h-0 overflow-hidden">
					<div class="px-(--kit-space-md) pb-(--kit-space-md) text-[length:var(--kit-text-sm)] text-[color:var(--kit-muted-foreground)]">
						{@render item.content?.()}
					</div>
				</div>
			</div>
		</div>
	{/each}
</div>
