<script lang="ts">
	import { cn } from '#lib/utils.js';
	import type { TablePayload } from '../types.js';

	interface Props {
		data: TablePayload;
		class?: string;
	}

	let { data, class: className }: Props = $props();

	/** The payload owns alignment: a numeric column is right-aligned everywhere, header included. */
	const cell = (numeric: boolean | undefined) =>
		cn('px-(--kit-space-sm) py-(--kit-space-xs)', numeric && 'text-right tabular-nums');
</script>

<div
	data-kit="table-card"
	class={cn('overflow-hidden rounded-(--kit-radius-md) border border-(--kit-border)', className)}
>
	<!-- A scrollable region has to be focusable, or a keyboard reader cannot scroll it. -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div role="region" aria-label="Table, scrollable sideways" tabindex="0" class="kit-scroll overflow-x-auto">
		<table class="w-full border-collapse text-left text-[length:var(--kit-text-sm)]">
			<thead class="bg-(--kit-table-head)">
				<tr>
					{#each data.columns as column (column.key)}
						<th
							scope="col"
							class={cn(
								cell(column.numeric),
								'text-[length:var(--kit-text-xs)] font-medium whitespace-nowrap text-[color:var(--kit-table-head-foreground)]'
							)}
						>
							{column.label}
						</th>
					{/each}
				</tr>
			</thead>
			<tbody>
				{#each data.rows as row, index (index)}
					<tr
						class={cn(
							'border-t border-(--kit-table-row-border) transition-colors duration-(--kit-duration-fast) hover:bg-(--kit-table-hover)',
							index % 2 === 1 && 'bg-(--kit-table-even)'
						)}
					>
						{#each data.columns as column (column.key)}
							{@const value = row[column.key]}
							<td class={cell(column.numeric)}>
								{#if value === null || value === undefined}
									<span class="text-[color:var(--kit-muted-foreground)]">—</span>
								{:else if column.truncate}
									<span class="block max-w-56 truncate" title={String(value)}>{value}</span>
								{:else}
									<span class="break-words">{value}</span>
								{/if}
							</td>
						{/each}
					</tr>
				{/each}
			</tbody>
			{#if data.footnote}
				<tfoot>
					<tr class="border-t border-(--kit-table-row-border)">
						<td colspan={data.columns.length} class="px-(--kit-space-sm) py-(--kit-space-xs) text-right text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">
							{data.footnote}
						</td>
					</tr>
				</tfoot>
			{/if}
		</table>
	</div>
</div>