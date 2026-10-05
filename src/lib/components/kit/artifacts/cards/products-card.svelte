<script lang="ts">
	import { cn } from '#lib/utils.js';
	import type { ProductsPayload } from '../types.js';

	interface Props {
		data: ProductsPayload;
		class?: string;
	}

	let { data, class: className }: Props = $props();

	/**
	 * Stock is a word plus a glyph, never a colour alone: a colour-blind reader
	 * and a monochrome screenshot both still say whether it can be bought.
	 */
	const STOCK: Record<'in' | 'low' | 'out', { glyph: string; label: string; class: string }> = {
		in: { glyph: '✓', label: 'In stock', class: 'text-[color:var(--kit-success)]' },
		low: { glyph: '!', label: 'Low stock', class: 'text-[color:var(--kit-warning)]' },
		out: { glyph: '✕', label: 'Out of stock', class: 'text-[color:var(--kit-muted-foreground)]' }
	};
</script>

<div data-kit="products-card" class={cn('min-w-0', className)}>
	<div
		class="mb-(--kit-space-sm) flex flex-wrap items-center gap-(--kit-space-sm) text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]"
	>
		<span>{data.products.length} results</span>
		{#if data.currency}
			<span
				class="rounded-(--kit-radius-sm) border border-(--kit-border) px-(--kit-space-xs) py-(--kit-space-xxs) font-medium text-[color:var(--kit-foreground)]"
			>
				Prices in {data.currency}
			</span>
		{/if}
	</div>

	<ul
		class="m-0 grid list-none grid-cols-1 gap-(--kit-space-md) p-0 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4"
	>
		{#each data.products as product (product.url + product.name)}
			<li class="flex min-w-0 flex-col overflow-hidden rounded-(--kit-radius-md) border border-(--kit-border)">
				{#if product.imageUrl}
					<img
						src={product.imageUrl}
						alt=""
						loading="lazy"
						class="aspect-[4/3] max-h-[160px] w-full bg-(--kit-muted) object-cover"
					/>
				{:else}
					<div
					class="flex aspect-[4/3] max-h-[160px] w-full items-center justify-center bg-(--kit-muted) text-[length:var(--kit-text-xl)] text-[color:var(--kit-muted-foreground)]"
					>
						{product.name.slice(0, 1)}
					</div>
				{/if}

				<div class="flex min-w-0 flex-1 flex-col gap-(--kit-space-xs) p-(--kit-space-sm)">
					<p class="m-0 truncate text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">{product.vendor}</p>

					<h4 class="m-0 min-w-0 text-[length:var(--kit-text-sm)] leading-(--kit-leading-sm) font-medium">
						<a
							href={product.url}
							target="_blank"
							rel="noopener noreferrer"
							class="text-[color:var(--kit-link)] underline-offset-4 hover:underline"
						>
							<span class="break-words">{product.name}</span>
							<span aria-hidden="true" class="ml-(--kit-space-xxs) inline-block text-[length:var(--kit-text-xs)]">↗</span>
							<span class="sr-only">(opens in a new tab)</span>
						</a>
					</h4>

					{#if product.badges?.length}
						<ul class="m-0 flex list-none flex-wrap gap-(--kit-space-xs) p-0">
							{#each product.badges as badge (badge)}
								<li
									class="rounded-(--kit-radius-sm) bg-(--kit-secondary) px-(--kit-space-xs) py-(--kit-space-xxs) text-[length:var(--kit-text-xs)] text-[color:var(--kit-secondary-foreground)]"
								>
									{badge}
								</li>
							{/each}
						</ul>
					{/if}

					{#if product.rating}
						{@const ratingLabel = `Rated ${product.rating} out of 5${product.reviews === undefined ? '' : ` from ${product.reviews.toLocaleString()} reviews`}`}
						<p class="m-0 text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)] tabular-nums" aria-label={ratingLabel}>
							<span aria-hidden="true" class="text-[color:var(--kit-warning)]">★</span>
							<span aria-hidden="true">{product.rating}</span>
							{#if product.reviews !== undefined}
								<span aria-hidden="true">({product.reviews.toLocaleString()})</span>
							{/if}
						</p>
					{/if}

					<div class="mt-auto">
						<p class="m-0 text-[length:var(--kit-text-lg)] leading-(--kit-leading-lg) font-medium tabular-nums">
							{product.price}
						</p>
						{#if product.priceNote}
							<p class="m-0 text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">{product.priceNote}</p>
						{/if}
						{#if product.stock}
							{@const stock = STOCK[product.stock]}
							<p class={cn('m-0 flex items-center gap-(--kit-space-xs) text-[length:var(--kit-text-xs)]', stock.class)}>
								<span aria-hidden="true" class="font-mono">{stock.glyph}</span>
								{stock.label}
							</p>
						{/if}
					</div>
				</div>
			</li>
		{/each}
	</ul>
</div>