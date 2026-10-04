<script lang="ts">
	import { enhance } from '$app/forms';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import SaveIcon from '@lucide/svelte/icons/save';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as NativeSelect from '#lib/components/ui/native-select/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';
	import type { ThoughtNode, ThoughtNodeKind } from '#lib/server/thought-tree.js';

	let {
		node,
		onsaved,
		oncreate,
		ondelete
	}: {
		node: ThoughtNode | null;
		onsaved: (node: ThoughtNode) => void;
		oncreate: (parentId: string | null) => void;
		ondelete: (id: string) => void;
	} = $props();

	const KINDS: { value: ThoughtNodeKind; label: string }[] = [
		{ value: 'question', label: 'Question — something Lexosa asks itself' },
		{ value: 'route', label: 'Route — a decision that picks a branch' },
		{ value: 'action', label: 'Action — work to carry out' }
	];

	// Edits are held locally and written on save, so a half-typed question never
	// round-trips to the database and a drag never fights a focused field.
	let draft = $state<ThoughtNode | null>(null);

	// Re-seeded whenever a different node is selected; a drag on the canvas moves
	// the same record, so the panel's coordinates stay truthful without a reload.
	$effect(() => {
		draft = node;
	});

	const KINDS_BY_VALUE = new Map(KINDS.map((kind) => [kind.value, kind.label]));
</script>

<aside class="inspector" aria-label="Node inspector">
	{#if !draft}
		<div class="space-y-3 p-4">
			<p class="text-sm text-muted-foreground">Select a node to edit it.</p>
			<Button type="button" variant="outline" size="sm" onclick={() => oncreate(null)}>
				<PlusIcon class="size-3.5" />
				Add a starting question
			</Button>
		</div>
	{:else}
		<form
			method="POST"
			action="?/save"
			class="space-y-4 p-4"
			use:enhance={() => {
				return async ({ result, update }) => {
					await update();
					if (result.type === 'success') {
						const saved = (result.data as { node?: ThoughtNode } | undefined)?.node;
						if (saved) onsaved(saved);
					}
				};
			}}
		>
			<input type="hidden" name="id" value={draft.id} />
			<input type="hidden" name="parentId" value={draft.parentId ?? ''} />
			<input type="hidden" name="x" value={draft.position.x} />
			<input type="hidden" name="y" value={draft.position.y} />

			<div>
				<p class="text-xs font-bold tracking-[0.16em] text-secondary">NODE</p>
				<p class="text-sm text-muted-foreground">{KINDS_BY_VALUE.get(draft.kind)}</p>
			</div>

			<Field.Field>
				<Field.FieldLabel for="node-title">Title</Field.FieldLabel>
				<Input id="node-title" name="title" bind:value={draft.title} maxlength={120} required />
			</Field.Field>

			<Field.Field>
				<Field.FieldLabel for="node-kind">Kind</Field.FieldLabel>
				<NativeSelect.Root id="node-kind" name="kind" bind:value={draft.kind} class="w-full">
					{#each KINDS as kind (kind.value)}
						<NativeSelect.Option value={kind.value}>{kind.label}</NativeSelect.Option>
					{/each}
				</NativeSelect.Root>
			</Field.Field>

			<Field.Field>
				<Field.FieldLabel for="node-question">Question</Field.FieldLabel>
				<Textarea id="node-question" name="question" bind:value={draft.question} rows={3} />
				<Field.FieldDescription>
					What Lexosa asks at this point in the tree.
				</Field.FieldDescription>
			</Field.Field>

			<Field.Field>
				<Field.FieldLabel for="node-options">Answers</Field.FieldLabel>
				<Textarea
					id="node-options"
					name="options"
					rows={4}
					value={draft.options.join('\n')}
					oninput={(event) => {
						const options = event.currentTarget.value.split('\n');
						if (draft) draft = { ...draft, options };
					}}
				/>
				<Field.FieldDescription>One answer per line.</Field.FieldDescription>
			</Field.Field>

			<Field.Field>
				<Field.FieldLabel for="node-notes">Notes</Field.FieldLabel>
				<Textarea id="node-notes" name="notes" bind:value={draft.notes} rows={3} />
			</Field.Field>

			<p class="text-xs text-muted-foreground">
				Position {Math.round(draft.position.x)}, {Math.round(draft.position.y)} — drag the card on the canvas to move it.
			</p>

			<div class="flex flex-wrap gap-2">
				<Button type="submit" size="sm">
					<SaveIcon class="size-3.5" />
					Save node
				</Button>
				<Button type="button" variant="outline" size="sm" onclick={() => draft && oncreate(draft.id)}>
					<PlusIcon class="size-3.5" />
					Add below
				</Button>
				<Button type="button" variant="ghost" size="sm" onclick={() => draft && ondelete(draft.id)}>
					Delete
				</Button>
			</div>
		</form>
	{/if}
</aside>

<style>
	.inspector {
		display: block;
		width: 22rem;
		flex-shrink: 0;
		border-left: 1px solid var(--border);
		background: var(--card);
		overflow-y: auto;
	}
</style>