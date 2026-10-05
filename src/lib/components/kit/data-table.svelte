<script lang="ts" module>
	/**
	 * Port of `gpui-component`'s `Table` / `DataTable`.
	 *
	 * Metrics come from the crate, not from taste:
	 *   - `Size::table_row_height` → 32px at medium, 40px at large (sizing.rs:57);
	 *     `comfortable` is the large height plus the large cell's vertical padding.
	 *   - `Size::table_cell_padding` → px 6 / py 3 at small, px 8 / py 4 at medium,
	 *     px 12 / py 8 at large (sizing.rs:69).
	 *   - `MIN_CELL_WIDTH` = 100px (table/table.rs:14). It is what lets the table
	 *     outgrow a narrow region and scroll inside its own box.
	 *   - Header surface `table_head` / `table_head_foreground`, row separators
	 *     `table_row_border`, zebra `table_even`, hover `table_hover`.
	 */

	import type { HTMLTableAttributes } from 'svelte/elements';

	export type KitTableValue = string | number | boolean | null;

	export type KitTableColumn = {
		key: string;
		label: string;
		/** Right-aligns the column and sets tabular numerals, so digits line up. */
		numeric?: boolean;
		/** Clamps the cell to one line with an ellipsis and keeps the value in `title`. */
		truncate?: boolean;
		/** A disabled column still shows, but its header cannot be sorted. */
		disabled?: boolean;
	};

	export type KitTableSort = {
		key: string;
		direction: 'asc' | 'desc';
	};

	export type KitTableDensity = 'compact' | 'normal' | 'comfortable';

	export type KitTableProps = HTMLTableAttributes & {
		columns: KitTableColumn[];
		rows: Record<string, KitTableValue>[];
		density?: KitTableDensity;
		/**
		 * Rendered under the table. The crate puts the footer on `table_foot` behind
		 * a 1px top rule; kit.css carries no `--kit-table-foot` token, so the
		 * footnote takes the muted foreground over the page surface instead.
		 */
		footnote?: string;
		emptyText?: string;
		/** Rendered above the table, muted and centred, as the crate's `TableCaption`. */
		caption?: string;
		/** Bindable so the caller can own the sort; the header buttons write it back. */
		sort?: KitTableSort | null;
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let {
		columns,
		rows,
		density = 'normal',
		footnote,
		emptyText,
		caption,
		sort = $bindable(null),
		class: className,
		...restProps
	}: KitTableProps = $props();

	const ROW_HEIGHT: Record<KitTableDensity, string> = {
		compact: 'h-8',
		normal: 'h-10',
		comfortable: 'h-12',
	};

	const CELL_PADDING: Record<KitTableDensity, string> = {
		compact: 'px-1.5 py-0.75',
		normal: 'px-2 py-1',
		comfortable: 'px-3 py-2',
	};

	/** A missing cell reads as missing, not as an empty word. */
	const EMPTY_CELL = '—';

	function formatValue(value: KitTableValue): string {
		if (value === null || value === undefined) return EMPTY_CELL;
		return String(value);
	}

	function compareValues(a: KitTableValue, b: KitTableValue): number {
		if (typeof a === 'number' && typeof b === 'number') return a - b;
		if (typeof a === 'boolean' && typeof b === 'boolean') return Number(a) - Number(b);
		const left = String(a).trim().toLowerCase();
		const right = String(b).trim().toLowerCase();
		if (left === right) return 0;
		return left < right ? -1 : 1;
	}

	const orderedRows = $derived.by(() => {
		if (!sort) return rows;
		const { key, direction } = sort;
		const sign = direction === 'asc' ? 1 : -1;
		// Sorting is stable on the caller's order, and a null sorts last in both
		// directions: absent data is not the smallest value, so flipping the sort
		// must not push it to the top.
		return rows
			.map((row, index) => ({ row, index }))
			.sort((a, b) => {
				const left = a.row[key] ?? null;
				const right = b.row[key] ?? null;
				if (left === null && right === null) return a.index - b.index;
				if (left === null) return 1;
				if (right === null) return -1;
				return compareValues(left, right) * sign || a.index - b.index;
			})
			.map((entry) => entry.row);
	});

	/**
	 * A `truncate` column needs a definite width to ellipsize against, and an
	 * auto-layout table ignores `max-width` on a cell. The crate clips the same
	 * way — `TableHead` and `TableCell` set `flex_shrink_1` with
	 * `min_w(MIN_CELL_WIDTH)` — so a table with a truncating column lays out
	 * fixed and shrinks columns toward that minimum instead of growing to fit
	 * the longest value.
	 */
	const fixedLayout = $derived(columns.some((column) => column.truncate));

	function sortState(key: string): 'none' | 'asc' | 'desc' {
		if (sort?.key !== key) return 'none';
		return sort.direction === 'asc' ? 'asc' : 'desc';
	}

	/** Ascending first, then descending — the crate's two-state sort cycle. */
	function toggleSort(key: string) {
		sort = sort?.key === key && sort.direction === 'asc'
			? { key, direction: 'desc' }
			: { key, direction: 'asc' };
	}
</script>

{#snippet sortGlyph(state: 'none' | 'asc' | 'desc')}
	{#if state === 'asc'}
		<svg class="size-3 shrink-0" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
			<path d="M8 3.5 13 10H3L8 3.5Z" />
		</svg>
	{:else if state === 'desc'}
		<svg class="size-3 shrink-0" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
			<path d="M8 12.5 3 6h10L8 12.5Z" />
		</svg>
	{:else}
		<!-- ChevronsUpDown at half opacity, as the crate dims the unsorted icon. -->
		<svg
			class="size-3 shrink-0 opacity-50"
			viewBox="0 0 16 16"
			fill="none"
			stroke="currentColor"
			stroke-width="1.5"
			stroke-linecap="round"
			stroke-linejoin="round"
			aria-hidden="true"
		>
			<path d="M4.5 6.5 8 3.5l3.5 3M4.5 9.5l3.5 3 3.5-3" />
		</svg>
	{/if}
{/snippet}

<div data-kit="data-table" class="flex w-full min-w-0 flex-col gap-(--kit-space-sm)">
	{#if caption}
		<p class="px-(--kit-space-sm) text-center text-[length:var(--kit-text-sm)] text-[color:var(--kit-muted-foreground)]">
			{caption}
		</p>
	{/if}

	<!-- The table owns its horizontal overflow, so on a phone it scrolls sideways
	     instead of widening the page. -->
	<div
		class="kit-scroll w-full min-w-0 overflow-x-auto overflow-y-hidden rounded-(--kit-radius-md) border border-(--kit-border) bg-(--kit-table)"
	>
		<table
			class={cn(
				'w-full border-collapse text-[length:var(--kit-text-sm)] text-[color:var(--kit-foreground)]',
				fixedLayout && 'table-fixed',
				className,
			)}
			{...restProps}
		>
			<thead class="bg-(--kit-table-head)">
				<tr>
					{#each columns as column (column.key)}
						{@const state = sortState(column.key)}
						<th
							scope="col"
							aria-sort={column.disabled
								? undefined
								: state === 'none'
									? 'none'
									: state === 'asc'
										? 'ascending'
										: 'descending'}
							class={cn(
								'min-w-25 overflow-hidden border-b border-(--kit-table-row-border) text-left font-medium text-[color:var(--kit-table-head-foreground)]',
								ROW_HEIGHT[density],
								CELL_PADDING[density],
								column.numeric && 'text-right tabular-nums',
							)}
						>
							<button
								type="button"
								class={cn(
									'flex w-full min-w-0 items-center gap-(--kit-space-xs) overflow-hidden rounded-(--kit-radius-sm) text-inherit',
									'transition-colors duration-(--kit-duration-fast)',
									'hover:bg-(--kit-muted) hover:text-[color:var(--kit-foreground)]',
									'active:bg-(--kit-secondary-active) active:text-[color:var(--kit-foreground)]',
									'disabled:pointer-events-none disabled:opacity-50',
									column.numeric && 'justify-end',
								)}
								onclick={() => toggleSort(column.key)}
								disabled={column.disabled}
							>
								<span class={cn(column.truncate && 'min-w-0 truncate')}>{column.label}</span>
								{@render sortGlyph(state)}
							</button>
						</th>
					{/each}
				</tr>
			</thead>
			<tbody>
				{#if orderedRows.length === 0}
					<tr>
						<td
							colspan={Math.max(columns.length, 1)}
							class="px-(--kit-space-md) py-(--kit-space-xl) text-center text-[color:var(--kit-muted-foreground)]"
						>
							{emptyText ?? 'No rows'}
						</td>
					</tr>
				{:else}
					{#each orderedRows as row, rowIndex (rowIndex)}
						{@const isLastRow = rowIndex === orderedRows.length - 1}
						<tr class={cn('even:bg-(--kit-table-even)', ROW_HEIGHT[density])}>
							{#each columns as column (column.key)}
								{@const text = formatValue(row[column.key])}
								<td
									class={cn(
										'hover:bg-(--kit-table-hover)',
										!isLastRow && 'border-b border-(--kit-table-row-border)',
										CELL_PADDING[density],
										column.numeric && 'text-right tabular-nums',
										column.truncate && 'truncate',
									)}
									title={column.truncate ? text : undefined}
								>
									{text}
								</td>
							{/each}
						</tr>
					{/each}
				{/if}
			</tbody>
		</table>
	</div>

	{#if footnote}
		<p class="px-(--kit-space-sm) text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">
			{footnote}
		</p>
	{/if}
</div>
