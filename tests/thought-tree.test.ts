import { afterAll, beforeEach, describe, expect, test } from 'bun:test';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

// thought-tree.ts resolves its database from LEXIA_STATE_DIR when the module
// loads, so the temp directory has to exist before the import.
const workspace = mkdtempSync(join(tmpdir(), 'lexosa-tree-test-'));
process.env.LEXIA_STATE_DIR = workspace;
const tree = await import('../src/lib/server/thought-tree.js');

afterAll(() => {
	rmSync(workspace, { recursive: true, force: true });
});

const nodes = () => tree.listThoughtNodes();
const root = () => nodes().find((node) => node.parentId === null)!;
const byTitle = (title: string) => nodes().find((node) => node.title === title)!;
const childrenOf = (id: string) => nodes().filter((node) => node.parentId === id);

// Each case starts from the default pipeline again: the tests move and rename
// nodes, and a shared tree would make every later assertion depend on them.
beforeEach(() => {
	const existingRoot = nodes().find((node) => node.parentId === null);
	if (existingRoot) tree.deleteThoughtNode(existingRoot.id);
});

describe('the thought tree', () => {
	test('seeds the pipeline the host runs today', () => {
		expect(root().title).toBe('Take in a turn');
		expect(childrenOf(root().id).map((node) => node.title)).toEqual([
			'Capture the turn',
			'Retrieve memory',
			'Assemble the prompt',
			'Decide what this needs',
			'Run the work',
			'Reflect at the boundary'
		]);
	});

	test('saving an existing node updates it instead of duplicating it', () => {
		const before = nodes().length;

		const saved = tree.saveThoughtNode({ ...root(), question: 'What is wanted, exactly?' });

		expect(saved.id).toBe(root().id);
		expect(nodes()).toHaveLength(before);
		expect(nodes().find((node) => node.id === saved.id)?.question).toBe('What is wanted, exactly?');
	});

	test('re-parents a node under an existing one', () => {
		const source = byTitle('Retrieve memory');
		const newParent = byTitle('Run the work');

		const moved = tree.saveThoughtNode({ ...source, parentId: newParent.id });

		expect(moved.parentId).toBe(newParent.id);
		expect(childrenOf(newParent.id).map((node) => node.id)).toContain(source.id);
		expect(childrenOf(root().id).map((node) => node.id)).not.toContain(source.id);
	});

	test('refuses a parent that is not in the tree', () => {
		const node = byTitle('Retrieve memory');

		expect(() => tree.saveThoughtNode({ ...node, parentId: 'not-a-real-node' })).toThrow(/no longer exists/);
		expect(nodes().find((entry) => entry.id === node.id)?.parentId).toBe(node.parentId);
	});

	test('deleting a node takes its whole branch with it', () => {
		const branchParent = byTitle('Run the work');
		const leaf = byTitle('Reflect at the boundary');
		tree.saveThoughtNode({ ...leaf, parentId: branchParent.id });

		tree.deleteThoughtNode(branchParent.id);

		const remaining = nodes().map((node) => node.id);
		expect(remaining).toContain(root().id);
		expect(remaining).not.toContain(branchParent.id);
		expect(remaining).not.toContain(leaf.id);
	});

	test('re-adding the default pipeline fills only the gaps it finds', () => {
		tree.saveThoughtNode({ ...byTitle('Retrieve memory'), title: 'Retrieve memory (renamed)' });

		expect(tree.addDefaultPipeline().map((node) => node.title)).toEqual(['Retrieve memory']);
		expect(tree.addDefaultPipeline()).toEqual([]);
	});
});