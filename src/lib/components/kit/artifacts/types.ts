/**
 * Artifact model for inline chat artifacts.
 *
 * An artifact is a self-contained thing the agent produced and the chat renders
 * inline, under the assistant message that introduced it. The model exists
 * because there are two different kinds of artifact and conflating them is the
 * mistake this file prevents:
 *
 *  - **Code artifacts** (`html`, `svelte`, `react`, `code`): the agent emitted
 *    source text. The chat runs it, it has no typed payload, and it is the only
 *    kind that can execute. They are rendered in a sandboxed region and their
 *    source is always reachable from the Code tab, never hidden behind the
 *    preview.
 *  - **Data artifacts** (everything else): the agent emitted a known shape that
 *    the host already knows how to draw, so the artifact is typed here, rendered
 *    natively, and carries its own interaction affordances. No sandbox, no
 *    execution, no source to inspect.
 *
 * An agent chooses `kind` when it emits the artifact. The host rejects unknown
 * kinds rather than guessing, so a new artifact type is a deliberate change to
 * this union plus a renderer, not a runtime fallback.
 */

export type ArtifactKind =
	// Code artifacts: executed, sandboxed, source shown.
	| 'html'
	| 'svelte'
	| 'react'
	| 'code'
	// Data artifacts: typed, rendered natively by the host.
	| 'table'
	| 'record'
	| 'products'
	| 'link'
	| 'metrics'
	| 'chart'
	| 'diff'
	| 'files'
	| 'checklist'
	| 'timeline'
	| 'json'
	| 'terminal'
	| 'media';

export type ArtifactLanguage = 'html' | 'css' | 'svelte' | 'js' | 'ts' | 'tsx' | 'rust' | 'json' | 'sh' | 'diff';

export type Artifact = {
	/** Stable id. Artifacts are keyed by it so a re-emitted artifact with new content does not remount. */
	id: string;
	kind: ArtifactKind;
	/** Short name for the frame header, e.g. "Price comparison". Never a filename unless it is one. */
	title: string;
	/** One line of context shown under the title. Optional: many artifacts speak for themselves. */
	summary?: string;
	/** Source text for code artifacts. */
	source?: string;
	language?: ArtifactLanguage;
	/** Preferred preview height. `tall` gives scrolling HTML artifacts room to be documents instead of slivers. */
	height?: 'auto' | 'tall' | 'full';
	/** Kind-specific payload for data artifacts. Unused for code artifacts. */
	data?: ArtifactPayload;
};

export type ArtifactPayload =
	| TablePayload
	| RecordPayload
	| ProductsPayload
	| LinkPayload
	| MetricsPayload
	| ChartPayload
	| DiffPayload
	| FilesPayload
	| ChecklistPayload
	| TimelinePayload
	| JsonPayload
	| TerminalPayload
	| MediaPayload;

/**
 * A data table. `columns` drives both header and cell alignment, so a numeric
 * column must declare `numeric: true` and the renderer right-aligns it rather
 * than the caller passing per-cell markup.
 */
export type TablePayload = {
	columns: Array<{
		key: string;
		label: string;
		numeric?: boolean;
		/** Truncates with an ellipsis and keeps the full value in `title`. */
		truncate?: boolean;
	}>;
	rows: Array<Record<string, string | number | boolean | null>>;
	/** Optional right-aligned summary line, e.g. "12 rows · 3 flagged". */
	footnote?: string;
};

export type RecordPayload = {
	items: Array<{ label: string; value: string; hint?: string }>;
	/** Rendered as a low-emphasis header when present. */
	caption?: string;
};

export type ProductsPayload = {
	products: Array<{
		name: string;
		vendor: string;
		price: string;
		/** Pre-formatted comparison, e.g. "£9.99 less". */
		priceNote?: string;
		rating?: string;
		reviews?: number;
		imageUrl?: string;
		/** External destination. This is the one artifact kind that always points off-product. */
		url: string;
		badges?: string[];
		stock?: 'in' | 'low' | 'out';
	}>;
	currency?: string;
};

export type LinkPayload = {
	url: string;
	site: string;
	title: string;
	description?: string;
	imageUrl?: string;
	/** Hosting of the page, e.g. "docs.gpui-kit.com". */
	host: string;
};

export type MetricsPayload = {
	metrics: Array<{
		label: string;
		value: string;
		delta?: string;
		/** Positive is good (green), negative is bad (red). Omit for a neutral metric. */
		direction?: 'up' | 'down' | 'flat';
		spark?: number[];
	}>;
	period?: string;
};

export type ChartPayload = {
	series: Array<{ label: string; color?: 1 | 2 | 3 | 4 | 5; points: number[] }>;
	labels: string[];
	kind: 'bar' | 'line' | 'area';
	unit?: string;
	title?: string;
};

export type DiffPayload = {
	path: string;
	language?: ArtifactLanguage;
	/** `0` context, `+` added, `-` removed. The renderer owns the colouring. */
	lines: Array<{ kind: 'context' | 'add' | 'remove'; text: string }>;
	added: number;
	removed: number;
};

export type FilesPayload = {
	root: string;
	files: Array<{
		path: string;
		/** Bytes or `size` label supplied by the agent; the renderer never formats sizes itself. */
		size: string;
		/** Marks the file an artifact created rather than read. */
		created?: boolean;
		modified?: boolean;
	}>;
};

export type ChecklistPayload = {
	/** Plan the agent is executing; `done` items may be clicked to toggle in the demo. */
	items: Array<{ label: string; done: boolean; detail?: string }>;
	total: number;
};

export type TimelinePayload = {
	title: string;
	startedAt: string;
	durationMs: number;
	/** Parent request and the sub-agents/tool calls it fanned out to. */
	steps: Array<{
		id: string;
		label: string;
		kind: 'request' | 'subagent' | 'tool';
		status: 'queued' | 'running' | 'succeeded' | 'failed' | 'cancelled';
		durationMs?: number;
		detail?: string;
		/** Nested children, e.g. the actions one sub-agent performed. */
		children?: Array<{ label: string; status: TimelinePayload['steps'][number]['status']; detail?: string }>;
	}>;
};

export type JsonPayload = {
	value: unknown;
	/** Pre-formatted so the artifact does not re-derive line numbers or truncation. */
	defaultCollapsed?: boolean;
};

export type TerminalPayload = {
	command: string;
	cwd?: string;
	exitCode: number;
	lines: Array<{ kind: 'stdout' | 'stderr' | 'dim' | 'command'; text: string }>;
};

export type MediaPayload = {
	imageUrl: string;
	alt: string;
	caption?: string;
	/** Natural aspect ratio, e.g. `16/9`. Reserved up front so the frame does not shift on load. */
	ratio: string;
};

/**
 * Kinds whose payload is source text. These execute inside a sandboxed frame
 * and always keep their source reachable from the Code tab; every other kind is
 * typed data the host draws itself.
 */
export const CODE_ARTIFACT_KIND: Record<string, true> = {
	html: true,
	svelte: true,
	react: true,
	code: true
};