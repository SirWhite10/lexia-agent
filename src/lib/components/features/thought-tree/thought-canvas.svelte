<script lang="ts">
	import GitBranchIcon from '@lucide/svelte/icons/git-branch';
	import MaximizeIcon from '@lucide/svelte/icons/maximize';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import ZoomInIcon from '@lucide/svelte/icons/zoom-in';
	import ZoomOutIcon from '@lucide/svelte/icons/zoom-out';
	import { Button } from '#lib/components/ui/button/index.js';
	import type { ThoughtNode } from '#lib/server/thought-tree.js';

	let {
		nodes,
		selectedId,
		gridSize,
		connecting,
		onselect,
		onmove,
		onnewroot,
		onconnect
	}: {
		nodes: ThoughtNode[];
		selectedId: string | null;
		gridSize: number;
		/** Armed by the toolbar: the next node click attaches it to the selection. */
		connecting: boolean;
		onselect: (id: string) => void;
		onmove: (id: string, position: { x: number; y: number }) => void;
		onnewroot: () => void;
		onconnect: (targetId?: string) => void;
	} = $props();
	const NODE_WIDTH = 240;
	const NODE_HEIGHT = 96;
	const MIN_ZOOM = 0.3;
	const MAX_ZOOM = 2;

	let surface = $state<HTMLDivElement | null>(null);
	let pan = $state({ x: 0, y: 0 });
	let zoom = $state(1);

	// One drag state for both the background pan and a node move: two handlers
	// racing on the same pointer is how a canvas ends up fighting itself.
	type DragState =
		| { mode: 'pan'; pointerId: number; pointerX: number; pointerY: number; originX: number; originY: number; moved: boolean }
		| {
				mode: 'node';
				pointerId: number;
				id: string;
				pointerX: number;
				pointerY: number;
				originX: number;
				originY: number;
				moved: boolean;
			};
	let drag = $state<DragState | null>(null);

	let bounds = $derived.by(() => {
		if (nodes.length === 0) return { width: 800, height: 600 };
		const right = Math.max(...nodes.map((node) => node.position.x + NODE_WIDTH));
		const bottom = Math.max(...nodes.map((node) => node.position.y + NODE_HEIGHT));
		return { width: right + gridSize * 4, height: bottom + gridSize * 4 };
	});

	/** Edge from the bottom-centre of a parent to the top-centre of its child,
	 * which is how a blueprint reads: decisions flow downward. */
	let edges = $derived(
		nodes.flatMap((node) => {
			if (!node.parentId) return [];
			const parent = nodes.find((candidate) => candidate.id === node.parentId);
			if (!parent) return [];
			const fromX = parent.position.x + NODE_WIDTH / 2;
			const fromY = parent.position.y + NODE_HEIGHT;
			const toX = node.position.x + NODE_WIDTH / 2;
			const toY = node.position.y;
			const midY = (fromY + toY) / 2;
			return [{ id: `${parent.id}-${node.id}`, d: `M ${fromX} ${fromY} C ${fromX} ${midY}, ${toX} ${midY}, ${toX} ${toY}` }];
		})
	);

	/** Wheel listeners are passive by default, which would let the page scroll
	 * out from under the canvas while zooming. */
	function nonPassive(node: HTMLElement, handler: (event: WheelEvent) => void) {
		node.addEventListener('wheel', handler, { passive: false });
		return {
			destroy: () => node.removeEventListener('wheel', handler)
		};
	}

	function zoomAt(pointerX: number, pointerY: number, next: number): void {
		const clamped = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, next));
		pan = {
			x: pointerX - ((pointerX - pan.x) / zoom) * clamped,
			y: pointerY - ((pointerY - pan.y) / zoom) * clamped
		};
		zoom = clamped;
	}

	function onWheel(event: WheelEvent): void {
		const rect = surface?.getBoundingClientRect();
		if (!rect) return;
		zoomAt(event.clientX - rect.left, event.clientY - rect.top, zoom * (event.deltaY < 0 ? 1.1 : 0.9));
	}

	function zoomBy(factor: number): void {
		const rect = surface?.getBoundingClientRect();
		if (!rect) return;
		zoomAt(rect.width / 2, rect.height / 2, zoom * factor);
	}

	function fit(): void {
		const rect = surface?.getBoundingClientRect();
		if (!rect || bounds.width === 0 || bounds.height === 0) return;
		const next = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, Math.min(rect.width / bounds.width, rect.height / bounds.height)));
		zoom = next;
		pan = {
			x: (rect.width - bounds.width * next) / 2,
			y: (rect.height - bounds.height * next) / 2
		};
	}

	/**
	 * Pointer capture is taken only once a press turns into a drag, never on the
	 * press itself. Capturing during pointerdown swallows the follow-up `click`
	 * in some browsers, which left nodes selectable by hover but not by click;
	 * doing it late keeps a plain press a plain click.
	 */
	const DRAG_THRESHOLD_PX = 4;

	function onSurfacePointerDown(event: PointerEvent): void {
		const target = event.target as HTMLElement;
		const node = target.closest('.node') as HTMLElement | null;

		if (node) {
			const record = nodes.find((candidate) => candidate.id === node.dataset.id);
			if (!record) return;
			drag = {
				mode: 'node',
				pointerId: event.pointerId,
				id: record.id,
				pointerX: event.clientX,
				pointerY: event.clientY,
				originX: record.position.x,
				originY: record.position.y,
				moved: false
			};
			return;
		}

		drag = {
			mode: 'pan',
			pointerId: event.pointerId,
			pointerX: event.clientX,
			pointerY: event.clientY,
			originX: pan.x,
			originY: pan.y,
			moved: false
		};
	}

	function onSurfacePointerMove(event: PointerEvent): void {
		if (!drag || !surface) return;
		const distance = Math.hypot(event.clientX - drag.pointerX, event.clientY - drag.pointerY);

		if (!drag.moved && distance < DRAG_THRESHOLD_PX) return;
		if (!drag.moved) {
			drag.moved = true;
			surface.setPointerCapture(event.pointerId);
		}

		if (drag.mode === 'pan') {
			pan = { x: drag.originX + (event.clientX - drag.pointerX), y: drag.originY + (event.clientY - drag.pointerY) };
			return;
		}

		// Snapped to the grid so the tree stays readable after free dragging.
		onmove(drag.id, {
			x: Math.max(0, Math.round((drag.originX + (event.clientX - drag.pointerX) / zoom) / gridSize) * gridSize),
			y: Math.max(0, Math.round((drag.originY + (event.clientY - drag.pointerY) / zoom) / gridSize) * gridSize)
		});
	}

	function onSurfacePointerUp(event: PointerEvent): void {
		const finished = drag;
		drag = null;
		if (!finished) return;
		if (surface?.hasPointerCapture(event.pointerId)) surface.releasePointerCapture(event.pointerId);

		// A press that never moved is a click, resolved here rather than by the
		// node's own click handler: capture may have taken it.
		if (finished.mode === 'node' && !finished.moved) {
			if (connecting) onconnect(finished.id);
			else onselect(finished.id);
		}
	}

	/** Capture can be lost without a pointerup — a cancelled gesture, a
	 * context menu, the window losing focus — and a stale drag would then swallow
	 * every later press on the canvas. */
	function onLostPointerCapture(): void {
		drag = null;
	}
</script>

<div
	role="application"
	aria-label="Thought tree canvas: drag to pan, scroll to zoom, click a node to edit it"
	class="canvas"
	bind:this={surface}
	use:nonPassive={onWheel}
	onpointerdown={onSurfacePointerDown}
	onpointermove={onSurfacePointerMove}
	onpointerup={onSurfacePointerUp}
	onpointercancel={onSurfacePointerUp}
	onlostpointercapture={onLostPointerCapture}
>
	<div
		class="layer"
		style="transform: translate({pan.x}px, {pan.y}px) scale({zoom}); transform-origin: 0 0;"
	>
		<svg class="edges" width={bounds.width} height={bounds.height} aria-hidden="true">
			{#each edges as edge (edge.id)}
				<path class="edge" d={edge.d} />
			{/each}
		</svg>

		{#each nodes as node (node.id)}
			<button
				type="button"
				class="node"
				class:selected={node.id === selectedId}
				class:connectable={connecting}
				data-id={node.id}
				style="left: {node.position.x}px; top: {node.position.y}px; width: {NODE_WIDTH}px;"
			>
				<span class="node-kind" data-kind={node.kind}>{node.kind}</span>
				<span class="node-title">{node.title}</span>
				{#if node.question}
					<span class="node-question">{node.question}</span>
				{/if}
				{#if node.options.length > 0}
					<span class="node-options">{node.options.length} answers</span>
				{/if}
			</button>
		{/each}
	</div>

	<!-- The toolbar sits over the canvas like a map control strip: creating and
	     wiring nodes is a canvas action, not something to hunt for in a header. -->
	<div class="canvas-toolbar">
		<Button type="button" variant="outline" size="sm" onclick={onnewroot}>
			<PlusIcon class="size-3.5" />
			New root
		</Button>
		<Button
			type="button"
			variant={connecting ? 'default' : 'outline'}
			size="sm"
			disabled={!selectedId}
			onclick={() => onconnect()}
		>
			<GitBranchIcon class="size-3.5" />
			{connecting ? 'Pick a node' : 'Connect'}
		</Button>
		<span class="toolbar-divider" aria-hidden="true"></span>
		<Button type="button" variant="outline" size="icon-sm" aria-label="Zoom out" onclick={() => zoomBy(0.8)}>
			<ZoomOutIcon class="size-3.5" />
		</Button>
		<Button type="button" variant="outline" size="icon-sm" aria-label="Zoom in" onclick={() => zoomBy(1.2)}>
			<ZoomInIcon class="size-3.5" />
		</Button>
		<Button type="button" variant="outline" size="icon-sm" aria-label="Fit tree to view" onclick={fit}>
			<MaximizeIcon class="size-3.5" />
		</Button>
	</div>

	{#if connecting}
		<p class="canvas-hint" role="status">Click the node this one should answer to.</p>
	{/if}
</div>

<style>
	/* One colour for the blueprint grid, because `color-mix` takes its mixes as
	   comma-separated arguments and cannot sit inside a gradient stop like this. */
	:global(:root) {
		--canvas-grid-line: color-mix(in oklch, var(--border) 60%, transparent);
	}

	.canvas {
		position: relative;
		width: 100%;
		/* A wide, shallow drawing surface: the tree reads left to right and the
		   inspector stays beside it without squeezing the canvas square. */
		aspect-ratio: 16 / 9;
		min-height: 22rem;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: 0.75rem;
		background-color: var(--background);
		background-image:
			linear-gradient(to right, var(--canvas-grid-line) 1px, transparent 1px),
			linear-gradient(to bottom, var(--canvas-grid-line) 1px, transparent 1px);
		background-size: 24px 24px;
		cursor: grab;
		touch-action: none;
	}

	.layer {
		position: absolute;
		inset: 0;
	}

	.edges {
		position: absolute;
		top: 0;
		left: 0;
		overflow: visible;
		pointer-events: none;
	}

	.edge {
		fill: none;
		stroke: color-mix(in oklch, var(--secondary) 70%, transparent);
		stroke-width: 2;
	}

	.node {
		position: absolute;
		display: grid;
		gap: 0.25rem;
		padding: 0.6rem 0.7rem;
		border: 1px solid var(--border);
		border-left: 3px solid var(--secondary);
		border-radius: 0.5rem;
		background: var(--card);
		text-align: left;
		cursor: grab;
		box-shadow: 0 1px 2rem oklch(0.04 0 0 / 18%);
	}

	.node:hover {
		border-color: var(--primary);
	}

	.node.selected {
		border-color: var(--primary);
		outline: 2px solid color-mix(in oklch, var(--primary) 45%, transparent);
		outline-offset: 2px;
	}

	.node-kind {
		font-size: 0.62rem;
		font-weight: 700;
		letter-spacing: 0.12em;
		text-transform: uppercase;
		color: var(--muted-foreground);
	}

	.node-kind[data-kind='route'] {
		color: var(--secondary);
	}

	.node-kind[data-kind='action'] {
		color: var(--primary);
	}

	.node-title {
		font-size: 0.85rem;
		font-weight: 700;
		line-height: 1.25;
	}

	.node-question {
		font-size: 0.75rem;
		color: var(--muted-foreground);
		line-height: 1.35;
	}

	.node-options {
		font-size: 0.7rem;
		color: var(--muted-foreground);
	}

	.canvas-toolbar {
		position: absolute;
		left: 0.75rem;
		top: 0.75rem;
		display: flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.35rem;
		border: 1px solid var(--border);
		border-radius: 0.6rem;
		background: color-mix(in oklch, var(--card) 92%, transparent);
		box-shadow: 0 1px 1.5rem oklch(0.04 0 0 / 22%);
		backdrop-filter: blur(6px);
		/* The toolbar floats over the drawing surface, and the top-left corner is
		   where nodes usually sit. Only its buttons take clicks: the strip and the
		   gaps between them belong to the canvas, or a node underneath would be
		   unselectable for no visible reason. */
		pointer-events: none;
	}

	.canvas-toolbar :global(button) {
		pointer-events: auto;
	}

	.toolbar-divider {
		width: 1px;
		height: 1.25rem;
		background: var(--border);
	}

	.canvas-hint {
		position: absolute;
		left: 0.75rem;
		top: 4rem;
		padding: 0.35rem 0.6rem;
		border-radius: 0.4rem;
		background: var(--primary);
		color: var(--primary-foreground);
		font-size: 0.75rem;
		/* Advisory, like the toolbar: it must never stand between the operator
		   and the node they are aiming at. */
		pointer-events: none;
	}

	.node.connectable {
		cursor: pointer;
		border-color: var(--primary);
	}

	.node.connectable:hover {
		transform: scale(1.02);
	}
</style>