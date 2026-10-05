<script lang="ts">
	import { deserialize, enhance } from '$app/forms';
	import { Button } from '#lib/components/ui/button/index.js';
	import NodeInspector from '#lib/components/features/thought-tree/node-inspector.svelte';
	import ThoughtCanvas from '#lib/components/features/thought-tree/thought-canvas.svelte';
	import type { ThoughtNode } from '#lib/server/thought-tree.js';

	let { data, form } = $props();

	// Seeded once from the load data rather than captured at init: this is the
	// server's record of the tree, and the canvas edits it from there.
	let nodes = $state<ThoughtNode[]>([]);
	let selectedId = $state<string | null>(null);
	let seeded = false;
	$effect(() => {
		if (seeded) return;
		nodes = data.nodes;
		selectedId = data.nodes[0]?.id ?? null;
		seeded = true;
	});

	let selected = $derived(nodes.find((node) => node.id === selectedId) ?? null);
	let connecting = $state(false);

	/** True when `candidate` sits on the branch above `ancestor`. Attaching there
	 * would make the tree cyclic and orphan every node between the two. */
	function isAncestor(ancestorId: string, candidate: ThoughtNode): boolean {
		let cursor = candidate.parentId;
		while (cursor) {
			if (cursor === ancestorId) return true;
			cursor = nodes.find((node) => node.id === cursor)?.parentId ?? null;
		}
		return false;
	}

	/** Arms the connection, or completes it when the canvas hands over a target:
	 * the selected node becomes that target's answer. */
	async function connect(targetId?: string): Promise<void> {
		if (!selectedId) return;
		if (targetId === undefined) {
			connecting = !connecting;
			return;
		}


		connecting = false;
		const source = selected;
		const target = nodes.find((node) => node.id === targetId);
		if (!source || !target || target.id === source.id || isAncestor(target.id, source)) return;

		const result = await post('save', {
			id: target.id,
			parentId: source.id,
			title: target.title,
			kind: target.kind,
			question: target.question,
			options: target.options.join('\n'),
			notes: target.notes,
			x: String(target.position.x),
			y: String(target.position.y)
		});
		if (result.type !== 'success') return;
		const saved = (result.data as { node?: ThoughtNode } | undefined)?.node;
		if (saved) nodes = nodes.map((node) => (node.id === saved.id ? saved : node));
	}
	/** Node mutations go through form actions with `deserialize` rather than a
	 * second round of API routes: the canvas edits positions in memory and the
	 * inspector edits fields, and both land on the same records. */
	async function post(body: string, payload: Record<string, string>) {
		const data = new FormData();
		for (const [key, value] of Object.entries(payload)) data.append(key, value);
		const response = await fetch(`?/${body}`, {
			method: 'POST',
			body: data,
			headers: { accept: 'application/json', 'x-sveltekit-action': 'true' }
		});
		return deserialize(await response.text());
	}

	async function createNode(parentId: string | null): Promise<void> {
		const result = await post('create', { parentId: parentId ?? '' });
		if (result.type !== 'success') return;
		const created = (result.data as { node?: ThoughtNode } | undefined)?.node;
		if (!created) return;
		nodes = [...nodes, created];
		selectedId = created.id;
	}

	async function deleteNode(id: string): Promise<void> {
		const result = await post('remove', { id });
		if (result.type !== 'success') return;
		// The delete takes the whole branch with it, so the local list mirrors the
		// server rather than dropping one card.
		const removed = new Set([id]);
		for (let index = nodes.length - 1; index >= 0; index -= 1) {
			const node = nodes[index];
			if (node.parentId && removed.has(node.parentId)) removed.add(node.id);
		}
		nodes = nodes.filter((node) => !removed.has(node.id));
		if (selectedId && removed.has(selectedId)) selectedId = nodes[0]?.id ?? null;
	}
</script>

<svelte:head>
	<title>Thinking — Lexosa</title>
	<meta name="description" content="The thought tree Lexosa routes with." />
</svelte:head>

<section class="mx-auto flex w-full max-w-none flex-1 flex-col gap-4 px-5 py-6 lg:flex-row" aria-label="Thinking">
	<div class="flex min-w-0 flex-1 flex-col gap-3">
		<div class="flex flex-wrap items-center justify-between gap-3">
			<div class="min-w-0">
				<h1 class="text-lg font-bold">Thought tree</h1>
				<p class="text-sm text-muted-foreground">
					How Lexosa reads a request: each node is a question it asks itself, and the
					answers open the branch below. Drag to arrange, click to edit.
				</p>
			</div>

			<form method="POST" action="?/seedDefaults" use:enhance>
				<Button type="submit" variant="outline" size="sm">Restore default pipeline</Button>
			</form>
		</div>

		{#if form?.error}
			<p class="text-sm text-destructive" role="alert">{form.error}</p>
		{/if}

		{#if form && 'added' in form}
			<p class="text-sm text-muted-foreground" role="status">
				{form.added === 0
					? 'The default pipeline is already in this tree.'
					: `Added ${form.added} node${form.added === 1 ? '' : 's'} from the default pipeline.`}
			</p>
		{/if}

		<p class="text-xs text-muted-foreground">
			These nodes describe the pipeline the host runs today: capture, retrieve, assemble,
			run, reflect. Nothing here changes how a turn runs yet — Jev, the routing decision,
			is the one step with no code behind it, and this tree is where its answer will land.
		</p>

		<ThoughtCanvas
			{nodes}
			{selectedId}
			gridSize={data.gridSize}
			{connecting}
			onselect={(id) => {
				selectedId = id;
				connecting = false;
			}}
			onmove={(id, position) => {
				nodes = nodes.map((node) => (node.id === id ? { ...node, position } : node));
			}}
			onnewroot={() => createNode(null)}
			onconnect={connect}
		/>
	</div>

	<NodeInspector node={selected} onsaved={(node) => {
		nodes = nodes.some((candidate) => candidate.id === node.id)
			? nodes.map((candidate) => (candidate.id === node.id ? node : candidate))
			: [...nodes, node];
		selectedId = node.id;
	}} oncreate={createNode} ondelete={deleteNode} />
</section>