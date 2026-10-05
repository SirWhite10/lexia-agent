import { fail } from '@sveltejs/kit';
import {
	addDefaultPipeline,
	deleteThoughtNode,
	GRID_SIZE,
	listThoughtNodes,
	saveThoughtNode,
	type ThoughtNodeKind
} from '#lib/server/thought-tree.js';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = () => ({
	nodes: listThoughtNodes(),
	gridSize: GRID_SIZE
});

const KINDS: ThoughtNodeKind[] = ['question', 'route', 'action'];

/** One editor payload for all three mutations: the canvas writes a node's shape,
 * the inspector writes its fields, and both post the same record. */
function readNode(formData: FormData) {
	const kind = String(formData.get('kind') ?? 'question') as ThoughtNodeKind;
	return {
		id: String(formData.get('id') ?? '') || undefined,
		parentId: String(formData.get('parentId') ?? '') || null,
		title: String(formData.get('title') ?? ''),
		kind: KINDS.includes(kind) ? kind : 'question',
		question: String(formData.get('question') ?? ''),
		// One option per line: the inspector is a textarea, not a repeater.
		options: String(formData.get('options') ?? '')
			.split('\n')
			.map((option) => option.trim())
			.filter(Boolean),
		notes: String(formData.get('notes') ?? ''),
		position: {
			x: Number(formData.get('x') ?? 0) || 0,
			y: Number(formData.get('y') ?? 0) || 0
		}
	};
}

export const actions: Actions = {
	save: async ({ request }) => ({ node: saveThoughtNode(readNode(await request.formData())) }),

	create: async ({ request }) => {
		const formData = await request.formData();
		const parentId = String(formData.get('parentId') ?? '') || null;
		const tree = listThoughtNodes();
		const parent = tree.find((node) => node.id === parentId) ?? null;
		const siblings = tree.filter((node) => node.parentId === parentId);

		// A new branch lands beside its siblings rather than on top of them. With
		// no siblings yet, the first child sits directly under its parent instead
		// of at the origin.
		const created = saveThoughtNode({
			parentId,
			title: String(formData.get('title') ?? '') || 'New node',
			kind: 'question',
			question: '',
			options: [],
			notes: '',
			position: {
				x: siblings[0]?.position.x ?? parent?.position.x ?? GRID_SIZE,
				y: (siblings.at(-1)?.position.y ?? parent?.position.y ?? 0) + GRID_SIZE * 6
			}
		});
		return { node: created };
	},

	remove: async ({ request }) => {
		const id = String((await request.formData()).get('id') ?? '');
		if (!id) return fail(400, { error: 'No node was named.' });
		deleteThoughtNode(id);
		return { removed: id };
	},

	/** Brings a hand-edited tree back in line with the pipeline the host
	 * actually runs. Only missing nodes are added: whatever the operator changed
	 * or re-parented is left exactly as they left it. */
	seedDefaults: async () => ({ added: addDefaultPipeline().length })
};