<script lang="ts" module>
	/**
	 * Port of `gpui-component`'s `StatusBar` (status_bar.rs:77).
	 *
	 * From the crate: a full-width `h_flex` + `items_center` + `gap_2` strip on
	 * `tokens.status_bar`, `text_xs` in `muted_foreground`, with a 1px top border
	 * in `status_bar_border` and `py_1 px_2`. Values are the thing being read, so
	 * they take the mono face and the foreground colour while the labels stay
	 * muted — the same weight split the crate makes between a status bar and a
	 * code readout.
	 *
	 * The crate lays items in one row and pins regions to either end. A browser
	 * phone cannot: at that width the items are unreadable side by side, so below
	 * `md` they wrap onto a second row, and the strip scrolls sideways inside its
	 * own region rather than widening the page.
	 */

	import type { HTMLAttributes } from 'svelte/elements';

	export type KitStatusTone = 'neutral' | 'success' | 'warning' | 'danger';

	export type KitStatusItem = {
		label: string;
		value: string;
		hint?: string;
		tone?: KitStatusTone;
	};

	export type KitStatusBarProps = Omit<HTMLAttributes<HTMLDivElement>, 'children'> & {
		items: KitStatusItem[];
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let {
		items,
		class: className,
		...restProps
	}: KitStatusBarProps = $props();

	/**
	 * The status colours are the crate's own status tokens. `neutral` deliberately
	 * takes no token: it is the default text colour, so it needs none.
	 */
	const TONE_VALUE: Record<KitStatusTone, string> = {
		neutral: 'text-[color:var(--kit-foreground)]',
		success: 'text-[color:var(--kit-success)]',
		warning: 'text-[color:var(--kit-warning)]',
		danger: 'text-[color:var(--kit-danger)]',
	};

	/**
	 * A tone must not rest on hue alone, or it is unreadable to a colour-blind
	 * reader and invisible in a greyscale screenshot. Every non-neutral tone
	 * therefore carries a glyph as well as its colour, and every tone is named in
	 * text for a screen reader. `neutral` has neither: it is the plain reading.
	 */
	const TONE_MARK: Record<KitStatusTone, string> = {
		neutral: '',
		success: '✓',
		warning: '!',
		danger: '✕',
	};

	const TONE_NAME: Record<KitStatusTone, string> = {
		neutral: 'Status',
		success: 'OK',
		warning: 'Warning',
		danger: 'Error',
	};
</script>

<div
	data-kit="status-bar"
	class={cn(
		'kit-scroll w-full min-w-0 overflow-x-auto',
		'flex flex-wrap items-center gap-x-(--kit-space-md) gap-y-(--kit-space-xs)',
		'md:flex-nowrap',
		'border-t border-(--kit-border) bg-(--kit-status-bar)',
		'px-(--kit-space-md) py-(--kit-space-sm)',
		'text-[length:var(--kit-text-xs)] leading-(--kit-leading-xs) text-[color:var(--kit-muted-foreground)]',
		className,
	)}
	{...restProps}
>
	{#each items as item, index (index)}
		{@const tone = item.tone ?? 'neutral'}
		<div
			class={cn(
				'flex shrink-0 items-center gap-(--kit-space-xs)',
				// A vertical rule between items, as the crate's region separators do.
				index > 0 && 'md:border-l md:border-(--kit-border) md:pl-(--kit-space-md)',
			)}
		>
			<span class="shrink-0">{item.label}</span>
			<span
				class={cn(
					'shrink-0 text-[length:var(--kit-text-mono)] leading-(--kit-leading-mono) [font-family:var(--kit-font-mono)]',
					TONE_VALUE[tone],
				)}
			>
				<span class="sr-only">{TONE_NAME[tone]}: </span>
				{#if TONE_MARK[tone]}
					<span class="select-none" aria-hidden="true">{TONE_MARK[tone]}</span>
				{/if}
				{item.value}
			</span>
			{#if item.hint}
				<span class="shrink-0 text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">{item.hint}</span>
			{/if}
		</div>
	{/each}
</div>
