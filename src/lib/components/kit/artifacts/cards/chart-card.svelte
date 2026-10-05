<script lang="ts">
	import { cn } from '#lib/utils.js';
	import { formatAxisValue } from '../format.js';
	import type { ChartPayload } from '../types.js';

	interface Props {
		data: ChartPayload;
		class?: string;
	}

	let { data, class: className }: Props = $props();

	// Fixed drawing space; the SVG scales to the container and the type stays
	// proportional, so the chart is legible from 360px to a desktop column.
	const WIDTH = 720;
	const HEIGHT = 260;
	const PAD_LEFT = 48;
	const PAD_RIGHT = 8;
	const PAD_TOP = 8;
	const PAD_BOTTOM = 30;
	const PLOT_WIDTH = WIDTH - PAD_LEFT - PAD_RIGHT;
	const PLOT_HEIGHT = HEIGHT - PAD_TOP - PAD_BOTTOM;
	const GRID_LINES = 4;

	let svg = $state<SVGSVGElement | null>(null);
	let active = $state<number | null>(null);

	/** Only the five chart tokens exist, so a series index wraps within them. */
	const colour = (index: number, requested?: number) =>
		`var(--kit-chart-${Math.min(Math.max(requested ?? index + 1, 1), 5)})`;

	const peak = $derived(Math.max(0, ...data.series.flatMap((series) => series.points)));
	const axisMax = $derived.by(() => {
		if (peak <= 0) return 1;
		const magnitude = 10 ** Math.floor(Math.log10(peak));
		const steps = [1, 2, 2.5, 5, 10].find((step) => peak <= step * magnitude) ?? 10;
		return steps * magnitude;
	});

	const columnWidth = $derived(PLOT_WIDTH / Math.max(data.labels.length, 1));
	const barWidth = $derived(Math.max((columnWidth * 0.68) / Math.max(data.series.length, 1), 1));

	const centreOf = (index: number) => PAD_LEFT + (index + 0.5) * columnWidth;
	const topOf = (value: number) => PAD_TOP + PLOT_HEIGHT - (value / axisMax) * PLOT_HEIGHT;

	function linePoints(series: ChartPayload['series'][number]): string {
		return series.points
			.map((value, index) => `${centreOf(index).toFixed(2)},${topOf(value).toFixed(2)}`)
			.join(' ');
	}

	function areaPoints(series: ChartPayload['series'][number]): string {
		const baseline = PAD_TOP + PLOT_HEIGHT;
		return `${PAD_LEFT},${baseline} ${linePoints(series)} ${(PAD_LEFT + series.points.length * columnWidth).toFixed(2)},${baseline}`;
	}

	const xStep = $derived(Math.ceil(data.labels.length / 8));
	const tooltipLeft = $derived(
		active === null ? 0 : ((centreOf(active) / WIDTH) * 100).toFixed(2)
	);

	function columnLabel(index: number): string {
		return `${data.labels[index]}: ${data.series
			.map((series) => `${series.label} ${formatAxisValue(series.points[index] ?? 0)}`)
			.join(', ')}`;
	}

	function moveFocus(event: KeyboardEvent, index: number) {
		if (event.key !== 'ArrowRight' && event.key !== 'ArrowLeft') return;
		event.preventDefault();
		const step = event.key === 'ArrowRight' ? 1 : -1;
		const next = Math.min(Math.max(index + step, 0), data.labels.length - 1);
		svg?.querySelectorAll<SVGRectElement>('[data-kit-column]')[next]?.focus();
	}
</script>

<div data-kit="chart-card" class={cn('relative min-w-0', className)}>
	{#if data.title}
		<p class="m-0 mb-(--kit-space-sm) text-[length:var(--kit-text-sm)] font-medium">{data.title}</p>
	{/if}

	<ul class="m-0 mb-(--kit-space-sm) flex list-none flex-wrap gap-(--kit-space-md) p-0 text-[length:var(--kit-text-xs)]">
		{#each data.series as series, index (series.label)}
			<li class="flex items-center gap-(--kit-space-xs)">
				<span
					aria-hidden="true"
					class="h-2 w-2 rounded-(--kit-radius-full)"
					style:background={colour(index, series.color)}
				></span>
				{series.label}
			</li>
		{/each}
		{#if data.unit}
			<li class="text-[color:var(--kit-muted-foreground)]">Measured in {data.unit}</li>
		{/if}
	</ul>

	<div class="relative">
		<svg
			bind:this={svg}
			viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
			class="w-full"
			style:aspect-ratio={`${WIDTH} / ${HEIGHT}`}
			role="group"
			aria-label={`${data.kind} chart of ${data.series.map((s) => s.label).join(' and ')}`}
		>
			{#each Array.from({ length: GRID_LINES + 1 }, (_, step) => step) as step (step)}
				{@const value = (axisMax / GRID_LINES) * step}
				<line
					x1={PAD_LEFT}
					x2={WIDTH - PAD_RIGHT}
					y1={PAD_TOP + PLOT_HEIGHT - (step / GRID_LINES) * PLOT_HEIGHT}
					y2={PAD_TOP + PLOT_HEIGHT - (step / GRID_LINES) * PLOT_HEIGHT}
					stroke="var(--kit-chart-grid)"
					stroke-width="1"
				/>
				<text
					x={PAD_LEFT - 8}
					y={PAD_TOP + PLOT_HEIGHT - (step / GRID_LINES) * PLOT_HEIGHT + 4}
					text-anchor="end"
					fill="var(--kit-muted-foreground)"
					font-size="11"
				>
					{formatAxisValue(Math.round(value * 100) / 100)}
				</text>
			{/each}

			{#each data.labels as label, index (index)}
				<text
					x={centreOf(index)}
					y={HEIGHT - PAD_BOTTOM + 18}
					text-anchor="middle"
					fill="var(--kit-muted-foreground)"
					font-size="11"
				>
					{#if index % xStep === 0}{label}{/if}
				</text>
			{/each}

			{#each data.series as series, seriesIndex (series.label)}
				{#if data.kind === 'bar'}
					{#each series.points as value, index (index)}
						<rect
							x={PAD_LEFT + index * columnWidth + columnWidth * 0.16 + seriesIndex * barWidth}
							y={topOf(value)}
							width={barWidth}
							height={Math.max(topOf(0) - topOf(value), 0)}
							rx="2"
							fill={colour(seriesIndex, series.color)}
						/>
					{/each}
				{:else if data.kind === 'area'}
					<polygon points={areaPoints(series)} fill={colour(seriesIndex, series.color)} fill-opacity="0.18" />
					<polyline
						points={linePoints(series)}
						fill="none"
						stroke={colour(seriesIndex, series.color)}
						stroke-width="2"
						stroke-linejoin="round"
					/>
				{:else}
					<polyline
						points={linePoints(series)}
						fill="none"
						stroke={colour(seriesIndex, series.color)}
						stroke-width="2"
						stroke-linejoin="round"
					/>
				{/if}
				{#if data.kind !== 'bar'}
					{#each series.points as value, index (index)}
						<circle cx={centreOf(index)} cy={topOf(value)} r="3" fill={colour(seriesIndex, series.color)} />
					{/each}
				{/if}
			{/each}

			{#each data.labels as _, index (index)}
				<!--
					Each column is a focusable hit area carrying every series value
					for that column, so the tooltip is reachable from the keyboard
					with the arrow keys as well as from the pointer.
				-->
				<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
				<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<rect
					data-kit-column
					x={PAD_LEFT + index * columnWidth}
					y={PAD_TOP}
					width={columnWidth}
					height={PLOT_HEIGHT}
					fill={active === index ? 'var(--kit-chart-grid)' : 'transparent'}
					tabindex="0"
					role="img"
					aria-label={columnLabel(index)}
					onpointerenter={() => (active = index)}
					onpointerleave={() => (active = null)}
					onfocus={() => (active = index)}
					onblur={() => (active = null)}
					onkeydown={(event) => moveFocus(event, index)}
				/>
			{/each}
		</svg>

		{#if active !== null}
			<div
				class="pointer-events-none absolute top-0 z-10 min-w-32 -translate-x-1/2 rounded-(--kit-radius-sm) border border-(--kit-border) bg-(--kit-card) px-(--kit-space-sm) py-(--kit-space-xs) text-[length:var(--kit-text-xs)] shadow-lg"
				style:left={`${tooltipLeft}%`}
			>
				<p class="m-0 font-medium">{data.labels[active]}</p>
				{#each data.series as series, seriesIndex (series.label)}
					<p class="m-0 flex items-center gap-(--kit-space-xs) tabular-nums">
						<span
							aria-hidden="true"
							class="h-2 w-2 rounded-(--kit-radius-full)"
							style:background={colour(seriesIndex, series.color)}
						></span>
						{series.label}
						<span class="ml-auto text-[color:var(--kit-muted-foreground)]">
							{formatAxisValue(series.points[active] ?? 0)}{#if data.unit}&nbsp;{data.unit}{/if}
						</span>
					</p>
				{/each}
			</div>
		{/if}
	</div>

	<p class="m-0 mt-(--kit-space-xs) text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">
		Focus a column and use the arrow keys to read each value.
	</p>
</div>