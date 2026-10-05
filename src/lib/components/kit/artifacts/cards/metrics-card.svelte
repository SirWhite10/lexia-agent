<script lang="ts">
	import { cn } from '#lib/utils.js';
	import type { MetricsPayload } from '../types.js';

	interface Props {
		data: MetricsPayload;
		class?: string;
	}

	let { data, class: className }: Props = $props();

	/**
	 * Direction is carried by an arrow as well as a colour, so the tiles read
	 * correctly in a monochrome screenshot or with colour vision that cannot
	 * separate the two.
	 */
	const DIRECTION: Record<'up' | 'down' | 'flat', { arrow: string; class: string }> = {
		up: { arrow: '↑', class: 'text-[color:var(--kit-chart-bullish)]' },
		down: { arrow: '↓', class: 'text-[color:var(--kit-chart-bearish)]' },
		flat: { arrow: '→', class: 'text-[color:var(--kit-muted-foreground)]' }
	};

	const SPARK_WIDTH = 100;
	const SPARK_HEIGHT = 24;

	/** Normalises a series into viewBox points; a flat series draws a level line rather than dividing by zero. */
	function sparkPoints(values: number[]): string {
		const min = Math.min(...values);
		const max = Math.max(...values);
		const span = max - min;
		return values
			.map((value, index) => {
				const x = (index / Math.max(values.length - 1, 1)) * SPARK_WIDTH;
				const y = span === 0 ? SPARK_HEIGHT / 2 : SPARK_HEIGHT - ((value - min) / span) * SPARK_HEIGHT;
				return `${x.toFixed(2)},${y.toFixed(2)}`;
			})
			.join(' ');
	}
</script>

<div data-kit="metrics-card" class={cn('min-w-0', className)}>
	{#if data.period}
		<p class="m-0 mb-(--kit-space-sm) text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">{data.period}</p>
	{/if}

	<ul class="m-0 grid list-none grid-cols-1 gap-(--kit-space-sm) p-0 sm:grid-cols-2 xl:grid-cols-4">
		{#each data.metrics as metric (metric.label)}
			{@const direction = metric.direction ? DIRECTION[metric.direction] : null}
			<li class="flex min-w-0 flex-col gap-(--kit-space-xxs) rounded-(--kit-radius-md) border border-(--kit-border) p-(--kit-space-sm)">
				<p class="m-0 truncate text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">{metric.label}</p>
				<p class="m-0 text-[length:var(--kit-text-xl)] leading-(--kit-leading-xl) font-medium tabular-nums">
					{metric.value}
				</p>
				{#if metric.delta}
					<p class={cn('m-0 flex items-center gap-(--kit-space-xs) text-[length:var(--kit-text-xs)]', direction?.class)}>
						{#if direction}
							<span aria-hidden="true">{direction.arrow}</span>
						{/if}
						<span class="truncate">{metric.delta}</span>
					</p>
				{/if}
				{#if metric.spark && metric.spark.length > 1}
					<svg
						viewBox={`0 0 ${SPARK_WIDTH} ${SPARK_HEIGHT}`}
						preserveAspectRatio="none"
						role="img"
						aria-label={`${metric.label} trend across ${metric.spark.length} samples`}
						class="mt-(--kit-space-xs) h-6 w-full"
					>
						<polyline
							points={sparkPoints(metric.spark)}
							fill="none"
							stroke-width="1.5"
							vector-effect="non-scaling-stroke"
							class={cn('stroke-(--kit-muted-foreground)', direction && metric.direction === 'up' && 'stroke-(--kit-chart-bullish)', direction && metric.direction === 'down' && 'stroke-(--kit-chart-bearish)')}
						/>
					</svg>
				{/if}
			</li>
		{/each}
	</ul>
</div>