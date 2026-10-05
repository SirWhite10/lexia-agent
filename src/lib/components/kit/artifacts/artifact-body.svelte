<script lang="ts">
	import { cn } from '#lib/utils.js';
	import ChartCard from './cards/chart-card.svelte';
	import ChecklistCard from './cards/checklist-card.svelte';
	import DiffCard from './cards/diff-card.svelte';
	import FilesCard from './cards/files-card.svelte';
	import JsonCard from './cards/json-card.svelte';
	import LinkCard from './cards/link-card.svelte';
	import MediaCard from './cards/media-card.svelte';
	import MetricsCard from './cards/metrics-card.svelte';
	import ProductsCard from './cards/products-card.svelte';
	import RecordCard from './cards/record-card.svelte';
	import TableCard from './cards/table-card.svelte';
	import TerminalCard from './cards/terminal-card.svelte';
	import TimelineCard from './cards/timeline-card.svelte';
	import CodeView from './code-view.svelte';
	import RenderHtml from './render-html.svelte';
	import RenderReact from './render-react.svelte';
	import RenderSvelte from './render-svelte.svelte';
	import {
		CODE_ARTIFACT_KIND,
		type Artifact,
		type ChartPayload,
		type ChecklistPayload,
		type DiffPayload,
		type FilesPayload,
		type JsonPayload,
		type LinkPayload,
		type MediaPayload,
		type MetricsPayload,
		type ProductsPayload,
		type RecordPayload,
		type TablePayload,
		type TerminalPayload,
		type TimelinePayload
	} from './types.js';

	interface Props {
		artifact: Artifact;
		/** `preview` runs the artifact; `code` always shows its exact source. */
		view?: 'preview' | 'code';
		class?: string;
	}

	let { artifact, view = 'preview', class: className }: Props = $props();
	const isCode = $derived(CODE_ARTIFACT_KIND[artifact.kind] === true);
	const source = $derived(artifact.source ?? '');
	const payload = $derived(artifact.data);
</script>

<div data-kit="artifact-body" class={cn('min-w-0', className)}>
	{#if view === 'code'}
		{#if source}
			<CodeView
				{source}
				language={artifact.language}
				filename={artifact.kind === 'code' ? artifact.title : undefined}
				bodyClass="max-h-none"
			/>
		{:else}
			<p
				role="alert"
				class="m-0 rounded-(--kit-radius-md) border border-(--kit-danger) px-(--kit-space-sm) py-(--kit-space-xs) text-[length:var(--kit-text-xs)] text-[color:var(--kit-danger)]"
			>
				This artifact carries no source to show.
			</p>
		{/if}
	{:else if artifact.kind === 'html'}
		{#if source}
			<RenderHtml {source} title={`${artifact.title} preview`} />
		{:else}
			<p role="alert" class="m-0 text-[length:var(--kit-text-xs)] text-[color:var(--kit-danger)]">This artifact carries no markup.</p>
		{/if}
	{:else if artifact.kind === 'svelte'}
		{#if source}
			<RenderSvelte {source} />
		{:else}
			<p role="alert" class="m-0 text-[length:var(--kit-text-xs)] text-[color:var(--kit-danger)]">
				This artifact carries no component source.
			</p>
		{/if}
	{:else if artifact.kind === 'react'}
		{#if source}
			<RenderReact {source} />
		{:else}
			<p role="alert" class="m-0 text-[length:var(--kit-text-xs)] text-[color:var(--kit-danger)]">
				This artifact carries no component source.
			</p>
		{/if}
	{:else if artifact.kind === 'code'}
		{#if source}
			<CodeView {source} language={artifact.language} filename={artifact.title} />
		{:else}
			<p role="alert" class="m-0 text-[length:var(--kit-text-xs)] text-[color:var(--kit-danger)]">This artifact carries no source.</p>
		{/if}
	{:else if !payload}
		<p
			role="alert"
			class="m-0 rounded-(--kit-radius-md) border border-(--kit-danger) px-(--kit-space-sm) py-(--kit-space-xs) text-[length:var(--kit-text-xs)] text-[color:var(--kit-danger)]"
		>
			This {String(artifact.kind)} artifact carries no payload.
		</p>
	{:else if artifact.kind === 'table'}
		<TableCard data={payload as TablePayload} />
	{:else if artifact.kind === 'record'}
		<RecordCard data={payload as RecordPayload} />
	{:else if artifact.kind === 'products'}
		<ProductsCard data={payload as ProductsPayload} />
	{:else if artifact.kind === 'link'}
		<LinkCard data={payload as LinkPayload} />
	{:else if artifact.kind === 'metrics'}
		<MetricsCard data={payload as MetricsPayload} />
	{:else if artifact.kind === 'chart'}
		<ChartCard data={payload as ChartPayload} />
	{:else if artifact.kind === 'diff'}
		<DiffCard data={payload as DiffPayload} />
	{:else if artifact.kind === 'files'}
		<FilesCard data={payload as FilesPayload} />
	{:else if artifact.kind === 'checklist'}
		<ChecklistCard data={payload as ChecklistPayload} />
	{:else if artifact.kind === 'timeline'}
		<TimelineCard data={payload as TimelinePayload} />
	{:else if artifact.kind === 'json'}
		<JsonCard data={payload as JsonPayload} />
	{:else if artifact.kind === 'terminal'}
		<TerminalCard data={payload as TerminalPayload} />
	{:else if artifact.kind === 'media'}
		<MediaCard data={payload as MediaPayload} />
	{:else if isCode}
		<p role="alert" class="m-0 text-[length:var(--kit-text-xs)] text-[color:var(--kit-danger)]">This artifact carries no source.</p>
	{:else}
		<p
			role="alert"
			class="m-0 rounded-(--kit-radius-md) border border-(--kit-danger) px-(--kit-space-sm) py-(--kit-space-xs) text-[length:var(--kit-text-xs)] text-[color:var(--kit-danger)]"
		>
			Unsupported artifact kind <code class="font-mono">{String(artifact.kind)}</code>. The host draws the kinds
			it knows and says so when it meets one it does not.
		</p>
	{/if}
</div>